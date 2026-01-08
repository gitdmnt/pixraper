// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

mod config;
mod scraper;

use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{Manager, State};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use futures::stream::StreamExt;

type AppConfig = Arc<Mutex<config::Config>>;
type ScrapingState = Arc<Mutex<Option<CancellationToken>>>;
type ScrapingHandle = scraper::QueryQueueHandle;

#[tauri::command]
async fn get_config(config: State<'_, AppConfig>) -> Result<config::Config, String> {
    Ok(config.lock().await.clone())
}

#[tauri::command]
async fn set_config(
    app_handle: tauri::AppHandle,
    config: State<'_, AppConfig>,
    new_config: config::Config,
) -> Result<(), String> {
    // インメモリの設定を更新
    println!("Config updating: {:?}", new_config);
    {
        let mut cfg = config.lock().await;
        *cfg = new_config;
    } // ここでロックが解放される

    // ファイルに保存
    config::save_config(&app_handle, &*config.lock().await).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn add_scraping_queue(
    queue: State<'_, ScrapingHandle>,
    option: scraper::ScrapingOption,
) -> Result<(), String> {
    queue.add(option).await;
    Ok(())
}

#[tauri::command]
async fn run_scraping_queue(queue: State<'_, ScrapingHandle>) -> Result<(), String> {
    // RunNext 実行に使うキャンセレーショントークンをセット
    let token = tokio_util::sync::CancellationToken::new();
    queue.set_token(token.clone()).await;
    loop {
        let _ = queue.run_next().await?;
        // キューが空の場合は終了
        if queue.is_empty().await {
            break;
        }
    }
    // トークンをクリア
    queue.clear_token().await;
    Ok(())
}

#[tauri::command]
async fn stop_scraping(queue: State<'_, ScrapingHandle>) -> Result<String, String> {
    queue.stop().await;
    Ok("Scraping queue stop signal sent.".to_string())
}

#[tauri::command]
async fn get_progress(
    queue: State<'_, ScrapingHandle>,
) -> Result<scraper::ScrapingProgress, String> {
    let (_, progress) = queue.get_progress().await;
    Ok(progress)
}

#[tauri::command]
fn show_analytics(data: &str) -> String {
    // Here you would add logic to start data analysis on the provided data
    format!("Started analysis on data: {}", data)
}

#[tauri::command]
async fn start_rough_scraping(
    app_handle: tauri::AppHandle,
    config: State<'_, AppConfig>,
    scraping_option: scraper::ScrapingOption,
    scraping_state: State<'_, ScrapingState>,
) -> Result<String, String> {
    let token = CancellationToken::new();
    {
        let mut state = scraping_state.lock().await;
        // 既に他のタスクが実行中ならエラー（オプション）
        if state.is_some() {
            return Err("Another scraping process is already running.".to_string());
        }
        // 新しいトークンを状態としてセット
        *state = Some(token.clone());
    }

    let cfg = &config.lock().await.clone();
    let res = scraper::fetch_search_result(cfg, &scraping_option, &token).await;

    // 処理が終了したら、状態をリセット
    *scraping_state.lock().await = None;

    let res = res.map_err(|e| e.to_string())?;

    if token.is_cancelled() {
        return Ok("Rough scraping was cancelled by user.".to_string());
    }

    // Save as CSV
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
    // let output_path = cfg.output.as_deref().unwrap_or(&default_filename);

    scraper::save_as_csv(&res, &output_path)
        .await
        .map_err(|e| e.to_string())?;

    // Return message
    let message = format!("Rough scraping finished. Found {} items.", res.len());
    println!("{}", message);
    Ok(message)
}

#[tauri::command]
async fn start_detailed_scraping(
    app_handle: tauri::AppHandle,
    config: State<'_, AppConfig>,
    scraping_option: scraper::ScrapingOption,
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

    // Rough Scraping
    let res = scraper::fetch_search_result(cfg, &scraping_option, &token).await;

    // この時点でキャンセルされていたら、詳細スクレイピングに進まずに終了
    if token.is_cancelled() {
        *scraping_state.lock().await = None; // 状態をリセット
        return Ok("Initial rough scraping was cancelled by user.".to_string());
    }

    let mut res = res.map_err(|e| e.to_string())?;

    // Get additional details for each item
    println!("Detailed logging is enabled.");
    println!(
        "Initial fetch complete. Total items to process: {}",
        res.len()
    );
    let now = Instant::now();
    let estimated_end = now + Duration::from_secs(res.len() as u64); // 1秒/件で計算
    println!("Estimated completion time: {:?}", estimated_end);

    let len = res.len();
    let cookie_header = env::var("PIXIV_COOKIES").ok().or(cfg.cookies.clone());
    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:144.0) Gecko/20100101 Firefox/144.0",
        )
        .build()
        .map_err(|e| e.to_string())?;

    let mut processed_items = Vec::new();
    let mut stream = futures::stream::iter(res).enumerate().boxed();

    loop {
        tokio::select! {
            // キャンセルされたか確認
            _ = token.cancelled() => {
                println!("Scraping process was cancelled.");
                break; // ループを抜ける
            }
            // ストリームから次のアイテムを取得
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

                    // fetch_illust_dataも非同期なので、結果を待つ
                    if let Ok(processed) = scraper::fetch_illust_data(item, &client, &cookie_header).await {
                        processed_items.push(processed);
                    }
                } else {
                    break; // ストリームが終了したらループを抜ける
                }
            }
        }
    }

    // resを処理済みのアイテムで上書き
    res = processed_items;

    // Save as CSV
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

    scraper::save_as_csv(&res, &output_path)
        .await
        .map_err(|e| e.to_string())?;

    // 処理が終了したら、状態をリセット
    *scraping_state.lock().await = None;

    // Return message
    let message = format!("Detailed scraping finished. Found {} items.", res.len());
    println!("{}", message);
    Ok(message)
}

#[tauri::command]
async fn stop_scraping_old(scraping_state: State<'_, ScrapingState>) -> Result<String, String> {
    let mut state = scraping_state.lock().await;
    if let Some(token) = state.as_ref() {
        token.cancel();
        *state = None;
        Ok("Scraping stop signal sent.".to_string())
    } else {
        Err("No scraping process is currently running.".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle();
            let config = config::load_config(app_handle);
            app.manage(Arc::new(Mutex::new(config.clone())));
            app.manage(scraper::QueryQueueHandle::new(&config));
            app.manage(Arc::new(Mutex::new(None::<CancellationToken>)));
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            get_config,
            set_config,
            add_scraping_queue,
            run_scraping_queue,
            stop_scraping,
            get_progress,
            show_analytics,
            start_rough_scraping,
            start_detailed_scraping,
            stop_scraping_old,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
