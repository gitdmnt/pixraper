//! 設定関連のコマンドを提供します。
//!
//! 実装上の意図:
//! - 設定は共有状態（`AppConfig`）として管理し、コマンド間でロックしてアクセスします。
//! - 可能な限りロックを短時間に留めるため、`get_config` は設定をクローンして返します。
//! - `set_config` はまずインメモリを更新し、その後ディスクへ永続化します。永続化時のエラーは呼び出し元へ伝搬します。

use crate::AppConfig;
use tauri::State;

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
