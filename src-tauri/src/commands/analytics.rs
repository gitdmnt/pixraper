#[tauri::command]
pub fn show_analytics(data: &str) -> String {
    format!("Started analysis on data: {}", data)
}
