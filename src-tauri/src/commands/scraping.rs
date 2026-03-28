//! スクレイピング関連のコマンド群。
//!
//! 実装の意図（要点）:
//! - フロントエンドからキュー操作や直接実行を行うための薄いラッパーを提供する。
//! - 長時間実行される処理は `CancellationToken` でキャンセル可能にする。
//! - キューを扱う場合、トークンは `QueryQueueActor` 側でも共有しておき、
//!   `run_scraping_queue` ではトークンをセット→実行→クリアする流れを明示することで、
//!   同時実行防止と停止制御を簡潔にしている。

use tauri::State;

use crate::{scraper::ScrapingOption, ScrapingHandle};
use tokio_util::sync::CancellationToken;

/// キューにスクレイピングオプションを追加します。
///
/// 実装の意図:
/// - フロントエンドから受け取ったオプションを `QueryQueueHandle` に転送するだけの責務に限定。
#[tauri::command]
pub async fn add_queue(
    queue: State<'_, ScrapingHandle>,
    option: ScrapingOption,
) -> Result<(), String> {
    queue.add(option).await;
    Ok(())
}

#[tauri::command]
pub async fn clear_queue(queue: State<'_, ScrapingHandle>) -> Result<(), String> {
    queue.clear().await;
    Ok(())
}

#[tauri::command]
pub async fn remove_queue_item(
    queue: State<'_, ScrapingHandle>,
    id: String,
) -> Result<(), String> {
    queue.remove_by_id(id).await;
    Ok(())
}

/// キューの先頭要素を順次実行します。
///
/// 実装の意図:
/// - 実行時に専用の `CancellationToken` を生成して Actor に渡します。
/// - Actor は内部でループしてキューを消化します（非同期ループ）。
#[tauri::command]
pub async fn start_scraping(queue: State<'_, ScrapingHandle>) -> Result<(), String> {
    let token = CancellationToken::new();
    // Actor 側にトークンを渡して開始シグナルを送る
    queue.start(token).await;
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

/// キューの現在の内容を取得します。
#[tauri::command]
pub async fn get_queue(
    queue: State<'_, ScrapingHandle>,
) -> Result<Vec<crate::scraper::ScrapingOption>, String> {
    let items = queue.get_queue().await;
    Ok(items)
}
