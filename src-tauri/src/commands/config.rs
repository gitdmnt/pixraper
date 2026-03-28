//! 設定関連のコマンドを提供します。
//!
//! 実装上の意図:
//! - 設定は共有状態（`AppConfig`）として管理し、コマンド間でロックしてアクセスします。
//! - 可能な限りロックを短時間に留めるため、`get_config` は設定をクローンして返します。
//! - `set_config` はまずインメモリを更新し、その後ディスクへ永続化します。永続化時のエラーは呼び出し元へ伝搬します。

use crate::AppConfig;
use reqwest::header::{ACCEPT, COOKIE, REFERER};
use tauri::State;

const PIXIV_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:144.0) Gecko/20100101 Firefox/144.0";
const PIXIV_STATUS_URL: &str = "https://www.pixiv.net/touch/ajax/user/self/status";
const PIXIV_REFERER: &str = "https://www.pixiv.net/";

/// 現在の設定を返します。ロックを長く保持しないよう、クローンを返す設計です。
#[tauri::command]
pub async fn get_config(config: State<'_, AppConfig>) -> Result<crate::config::Config, String> {
    Ok(config.lock().await.clone())
}

/// 設定を更新してファイルに保存します。
/// 実装の意図:
/// - まずインメモリの設定を置換してから、`config::save_config` で永続化することで、
///   同期的な UI 操作とディスク書き込みの責務を分離しています。
#[tauri::command]
pub async fn set_config(
    app_handle: tauri::AppHandle,
    config: State<'_, AppConfig>,
    new_config: crate::config::Config,
) -> Result<(), String> {
    println!("Config updating: {:?}", new_config);
    {
        let mut cfg = config.lock().await;
        *cfg = new_config;
    }

    crate::config::save_config(&app_handle, &*config.lock().await).map_err(|e| e.to_string())?;

    Ok(())
}

/// 指定した cookies 文字列で Pixiv にリクエストし、ログイン状態を返します。
/// Ok(true) = 有効, Ok(false) = 無効（ログアウト状態）, Err = 通信エラーまたは予期しないレスポンス
///
/// 判定ロジック:
/// 1. HTTP ステータスが 2xx でない → Ok(false)
/// 2. レスポンスが JSON でない（Cloudflare ブロック等） → Err
/// 3. `error` フィールドが true → Ok(false)
/// 4. `body.user_status.user_id` が非空文字列 → Ok(true)
/// 5. それ以外 → Ok(false)
#[tauri::command]
pub async fn validate_cookies(cookies: String) -> Result<bool, String> {
    let client = reqwest::Client::builder()
        .user_agent(PIXIV_USER_AGENT)
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(PIXIV_STATUS_URL)
        .header(COOKIE, &cookies)
        .header(REFERER, PIXIV_REFERER)
        .header(ACCEPT, "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Ok(false);
    }

    let text = resp.text().await.map_err(|e| e.to_string())?;

    if !text.trim_start().starts_with('{') {
        return Err("予期しないレスポンス形式です（Cloudflare等によるブロックの可能性）".to_string());
    }

    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("JSON パース失敗: {e}"))?;

    if value.get("error").and_then(|v| v.as_bool()).unwrap_or(true) {
        return Ok(false);
    }

    let user_id = value
        .pointer("/body/user_status/user_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    Ok(!user_id.is_empty())
}
