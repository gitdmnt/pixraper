use std::env;
use std::time::{Duration, Instant};

use futures::stream::StreamExt;
use tauri::{Manager, State};

use crate::{ScrapingHandle, ScrapingState};
use tokio_util::sync::CancellationToken;

#[tauri::command]
pub async fn add_scraping_queue(
    queue: State<'_, ScrapingHandle>,
    option: crate::scraper::ScrapingOption,
) -> Result<(), String> {
    queue.add(option).await;
    Ok(())
}

#[tauri::command]
pub async fn run_scraping_queue(queue: State<'_, ScrapingHandle>) -> Result<(), String> {
    let token = CancellationToken::new();
    queue.set_token(token.clone()).await;
    loop {
        queue.run_next().await?;
        if queue.is_empty().await {
            break;
        }
    }
    queue.clear_token().await;
    Ok(())
}

#[tauri::command]
pub async fn stop_scraping(queue: State<'_, ScrapingHandle>) -> Result<String, String> {
    queue.stop().await;
    Ok("Scraping queue stop signal sent.".to_string())
}

#[tauri::command]
pub async fn get_progress(
    queue: State<'_, ScrapingHandle>,
) -> Result<crate::scraper::ScrapingProgress, String> {
    let (_, progress) = queue.get_progress().await;
    Ok(progress)
}

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
        if state.is_some() {
            return Err("Another scraping process is already running.".to_string());
        }
        *state = Some(token.clone());
    }

    let cfg = &config.lock().await.clone();
    let res = crate::scraper::fetch_search_result(cfg, &scraping_option, &token).await;

    *scraping_state.lock().await = None;

    let res = res.map_err(|e| e.to_string())?;

    if token.is_cancelled() {
        return Ok("Rough scraping was cancelled by user.".to_string());
    }

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

    let message = format!("Rough scraping finished. Found {} items.", res.len());
    println!("{}", message);
    Ok(message)
}

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

    let res = crate::scraper::fetch_search_result(cfg, &scraping_option, &token).await;

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

    *scraping_state.lock().await = None;

    let message = format!("Detailed scraping finished. Found {} items.", res.len());
    println!("{}", message);
    Ok(message)
}

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
