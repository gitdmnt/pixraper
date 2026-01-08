use crate::AppConfig;
use tauri::State;

#[tauri::command]
pub async fn get_config(config: State<'_, AppConfig>) -> Result<crate::config::Config, String> {
    Ok(config.lock().await.clone())
}

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
