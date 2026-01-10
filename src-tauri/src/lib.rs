// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

mod analytics;
mod config;
mod csv;
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
            app.manage(scraper::QueryQueueHandle::new(&config, app_handle.clone()));
            app.manage(Arc::new(Mutex::new(None::<CancellationToken>)));
            // Initialize analytics cache state (None initially)
            app.manage(commands::analytics::AnalyticsState(Arc::new(Mutex::new(
                None,
            ))));
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            commands::config::get_config,
            commands::config::set_config,
            commands::scraping::add_queue,
            commands::scraping::clear_queue,
            commands::scraping::remove_queue_item,
            commands::scraping::start_scraping,
            commands::scraping::stop_scraping,
            commands::scraping::get_progress,
            commands::scraping::get_queue,
            commands::analytics::load_dataset,
            commands::analytics::get_all_tags,
            commands::analytics::calculate_tag_ranking,
            commands::analytics::calculate_co_occurence,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
