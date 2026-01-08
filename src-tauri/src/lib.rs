// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

mod config;
mod scraper;

use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub type AppConfig = Arc<Mutex<config::Config>>;
pub type ScrapingState = Arc<Mutex<Option<CancellationToken>>>;
pub type ScrapingHandle = scraper::QueryQueueHandle;

mod commands;

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
            commands::config::get_config,
            commands::config::set_config,
            commands::scraping::add_scraping_queue,
            commands::scraping::run_scraping_queue,
            commands::scraping::stop_scraping,
            commands::scraping::get_progress,
            commands::analytics::show_analytics,
            commands::scraping::start_rough_scraping,
            commands::scraping::start_detailed_scraping,
            commands::scraping::stop_scraping_old,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
