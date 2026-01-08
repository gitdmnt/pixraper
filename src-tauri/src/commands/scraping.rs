//! スクレイピング関連のコマンド群。
//!
//! 実装の意図（要点）:
//! - フロントエンドからキュー操作や直接実行を行うための薄いラッパーを提供する。
//! - 長時間実行される処理は `CancellationToken` でキャンセル可能にする。
//! - キューを扱う場合、トークンは `QueryQueueActor` 側でも共有しておき、
//!   `run_scraping_queue` ではトークンをセット→実行→クリアする流れを明示することで、
//!   同時実行防止と停止制御を簡潔にしている。

use std::env;
use std::time::{Duration, Instant};

use futures::stream::StreamExt;
use tauri::{Manager, State};

use crate::{ScrapingHandle, ScrapingState};
use tokio_util::sync::CancellationToken;

/// キューにスクレイピングオプションを追加します。
///
/// 実装の意図:
/// - フロントエンドから受け取ったオプションを `QueryQueueHandle` に転送するだけの責務に限定。
#[tauri::command]
pub async fn add_scraping_queue(
    queue: State<'_, ScrapingHandle>,
    option: crate::scraper::ScrapingOption,
) -> Result<(), String> {
    queue.add(option).await;
    Ok(())
}

/// キューの先頭要素を順次実行します。
///
/// 実装の意図:
/// - 実行時に専用の `CancellationToken` をセットしてから `run_next` を繰り返し、
///   終了時にトークンをクリアします。これによりキュー実行中のキャンセルや重複実行を制御します。
#[tauri::command]
pub async fn run_scraping_queue(queue: State<'_, ScrapingHandle>) -> Result<(), String> {
    let token = CancellationToken::new();
    // QueryQueueActor 側にトークンを渡して、RunNext で利用できるようにする
    queue.set_token(token.clone()).await;
    loop {
        queue.run_next().await?;
        if queue.is_empty().await {
            break;
        }
    }
    // 実行終了後はトークンをクリアして、次回実行時に新しいトークンを使えるようにする
    queue.clear_token().await;
    Ok(())
}

/// 実行中のキューを停止します。
///
/// 実装の意図:
/// - キューの `Stop` をトリガし、actor 側でトークンをキャンセルする責務に限定する。
#[tauri::command]
pub async fn stop_scraping(queue: State<'_, ScrapingHandle>) -> Result<String, String> {
    queue.stop().await;
    Ok("Scraping queue stop signal sent.".to_string())
}

/// キューの進捗を取得します。
///
/// 実装の意図:
/// - 非同期で一時的に状態を問い合わせるだけの責務。
#[tauri::command]
pub async fn get_progress(
    queue: State<'_, ScrapingHandle>,
) -> Result<crate::scraper::ScrapingProgress, String> {
    let (_, progress) = queue.get_progress().await;
    Ok(progress)
}

/// ラフスクレイピングを直接実行します（単発実行）。
///
/// 実装の意図:
/// - 並列実行は防止し、`scraping_state` で実行中フラグを保持する。
/// - 設定はロックして取得後にクローンして使う（ロックを長引かせないため）。
/// - 実行後は必ず `scraping_state` をクリアして次を受け付ける。
#[tauri::command]
pub async fn start_rough_scraping(
    app_handle: tauri::AppHandle,
    config: State<'_, crate::AppConfig>,
    scraping_option: crate::scraper::ScrapingOption,
    scraping_state: State<'_, ScrapingState>,
) -> Result<String, String> {
    let token = CancellationToken::new();
    {
        let mut state = scraping_state.lock().await;
        // 既に実行中ならエラーを返す（UI 側で通知する想定）
        if state.is_some() {
            return Err("Another scraping process is already running.".to_string());
        }
        *state = Some(token.clone());
    }

    // 設定はロックを短くするためにクローンして保持する
    let cfg = &config.lock().await.clone();
    let res = crate::scraper::fetch_search_result(cfg, &scraping_option, &token).await;

    // 終了時は状態をリセット（成功／失敗問わず）
    *scraping_state.lock().await = None;

    let res = res.map_err(|e| e.to_string())?;

    if token.is_cancelled() {
        return Ok("Rough scraping was cancelled by user.".to_string());
    }

    // 出力ファイルパスの決定。`AppHandle::path()` を使うために `Manager` が導入されている。
    let now = chrono::Local::now();
    let default_filename = format!("result_{}.csv", now.format("%Y%m%d_%H%M%S"));
    let user_input_path = cfg.output.as_deref().unwrap_or(&default_filename);
    let user_input_path = std::path::Path::new(user_input_path);
    let output_path = app_handle
        .path()
        .document_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("Pixraper")
        .join(user_input_path);

    // CSV 保存は別関数に委譲（責務の分離）
    crate::scraper::save_as_csv(&res, &output_path)
        .await
        .map_err(|e| e.to_string())?;

    let message = format!("Rough scraping finished. Found {} items.", res.len());
    println!("{}", message);
    Ok(message)
}

/// 詳細スクレイピングを実行します（ラフ結果を元に個別詳細をフェッチ）。
///
/// 実装の意図:
/// - ラフスクレイピングで得た一覧を逐次処理して詳細を取得する。キャンセルを随所で確認して中断できるようにする。
/// - ネットワークリクエスト用の `reqwest::Client` はここで生成して再利用する。
#[tauri::command]
pub async fn start_detailed_scraping(
    app_handle: tauri::AppHandle,
    config: State<'_, crate::AppConfig>,
    scraping_option: crate::scraper::ScrapingOption,
    scraping_state: State<'_, ScrapingState>,
) -> Result<String, String> {
    let token = CancellationToken::new();
    {
        let mut state = scraping_state.lock().await;
        if state.is_some() {
            return Err("Another scraping process is already running.".to_string());
        }
        *state = Some(token.clone());
    }

    let cfg = &config.lock().await.clone();

    // 初期フェーズ（ラフスクレイピング）を実行
    let res = crate::scraper::fetch_search_result(cfg, &scraping_option, &token).await;

    // キャンセルされていたら状態リセットして終了
    if token.is_cancelled() {
        *scraping_state.lock().await = None;
        return Ok("Initial rough scraping was cancelled by user.".to_string());
    }

    let mut res = res.map_err(|e| e.to_string())?;

    println!("Detailed logging is enabled.");
    println!(
        "Initial fetch complete. Total items to process: {}",
        res.len()
    );
    let now = Instant::now();
    let estimated_end = now + Duration::from_secs(res.len() as u64);
    println!("Estimated completion time: {:?}", estimated_end);

    // クッキーは環境変数優先で設定からフォールバックする
    let len = res.len();
    let cookie_header = env::var("PIXIV_COOKIES").ok().or(cfg.cookies.clone());
    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:144.0) Gecko/20100101 Firefox/144.0",
        )
        .build()
        .map_err(|e| e.to_string())?;

    // 順次処理するためストリームで処理
    let mut processed_items = Vec::new();
    let mut stream = futures::stream::iter(res).enumerate().boxed();

    loop {
        tokio::select! {
            _ = token.cancelled() => {
                println!("Scraping process was cancelled.");
                break;
            }
            maybe_item = stream.next() => {
                if let Some((index, item)) = maybe_item {
                    println!(
                        "Fetching illust data for ID: {}, title: {}\t({}/{} ({}%)",
                        &item.id,
                        &item.title,
                        index + 1,
                        len,
                        (index + 1) * 100 / len
                    );

                    if let Ok(processed) = crate::scraper::fetch_illust_data(item, &client, &cookie_header).await {
                        processed_items.push(processed);
                    }
                } else {
                    break;
                }
            }
        }
    }

    // 結果を置き換えて CSV に保存
    res = processed_items;

    let now = chrono::Local::now();
    let default_filename = format!("result_{}.csv", now.format("%Y%m%d_%H%M%S"));
    let user_input_path = cfg.output.as_deref().unwrap_or(&default_filename);
    let user_input_path = std::path::Path::new(user_input_path);
    let output_path = app_handle
        .path()
        .document_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("Pixraper")
        .join(user_input_path);

    crate::scraper::save_as_csv(&res, &output_path)
        .await
        .map_err(|e| e.to_string())?;

    // 実行終了時に状態をクリア
    *scraping_state.lock().await = None;

    let message = format!("Detailed scraping finished. Found {} items.", res.len());
    println!("{}", message);
    Ok(message)
}

/// 実行中の単発スクレイピング（start_* の旧API）を停止します。
///
/// 実装の意図:
/// - GUI 側からの旧インターフェース互換性を保つためのラッパー。
#[tauri::command]
pub async fn stop_scraping_old(scraping_state: State<'_, ScrapingState>) -> Result<String, String> {
    let mut state = scraping_state.lock().await;
    if let Some(token) = state.as_ref() {
        token.cancel();
        *state = None;
        Ok("Scraping stop signal sent.".to_string())
    } else {
        Err("No scraping process is currently running.".to_string())
    }
}
