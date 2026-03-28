//! 設定関連のコマンドを提供します。
//!
//! 実装上の意図:
//! - 設定は共有状態（`AppConfig`）として管理し、コマンド間でロックしてアクセスします。
//! - 可能な限りロックを短時間に留めるため、`get_config` は設定をクローンして返します。
//! - `set_config` はまずインメモリを更新し、その後ディスクへ永続化します。永続化時のエラーは呼び出し元へ伝搬します。

use crate::AppConfig;
use reqwest::header::COOKIE;
use serde::Deserialize;
use tauri::State;

const PIXIV_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:144.0) Gecko/20100101 Firefox/144.0";
const PIXIV_STATUS_URL: &str = "https://www.pixiv.net/touch/ajax/user/self/status";

#[derive(Deserialize)]
struct StatusBody {
    #[serde(rename = "isLoggedIn")]
    is_logged_in: bool,
}

#[derive(Deserialize)]
struct StatusResponse {
    error: bool,
    body: Option<StatusBody>,
}

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
/// Ok(true) = 有効, Ok(false) = 無効（ログアウト状態）, Err = 通信エラー
#[tauri::command]
pub async fn validate_cookies(cookies: String) -> Result<bool, String> {
    let client = reqwest::Client::builder()
        .user_agent(PIXIV_USER_AGENT)
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(PIXIV_STATUS_URL)
        .header(COOKIE, &cookies)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let text = resp.text().await.map_err(|e| e.to_string())?;
    let status: StatusResponse =
        serde_json::from_str(&text).map_err(|e| format!("レスポンスのパースに失敗: {e}"))?;

    if status.error {
        return Ok(false);
    }

    Ok(status.body.map(|b| b.is_logged_in).unwrap_or(false))
}
