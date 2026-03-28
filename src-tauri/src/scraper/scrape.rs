//! `types` モジュールは、スクレイピングで扱うデータ型と進捗管理を定義します。
//!
//! 実装の意図:
//! - API レスポンスを内部で扱いやすい `ItemRecord` に変換して処理の境界を明確にする。
//! - `ScrapingOption` に URL 生成やページ巡回等のラフスクレイピングに関するロジック（`fetch_rough`）を持たせ、
//!   その責務をこの型にまとめることで再利用性を高める。
//! - 進捗情報 (`ScrapingProgress`, `ScrapingStatus`) は Queue や UI と共有されるため、シンプルな形で定義する。
//!
//! 注意: HTTP の副作用は `api` モジュールに委譲できる部分もありますが、利便性のために `ScrapingOption::fetch_rough` に一部実装を残しています。

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::Config;
use crate::csv::AppHandleLike;

use crate::scraper::api::{fetch_detail_data, fetch_search_result, IllustData, NovelData};

/// 内部で使用する簡易的なアイテム表現（CSV出力やUIに渡すため）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemRecord {
    pub is_illust: bool,
    pub id: u64,
    pub title: String,
    pub x_restrict: bool,
    pub tags: Vec<String>,
    pub user_id: u64,
    pub create_date: String,
    pub ai_type: bool,
    pub width: Option<u64>,
    pub height: Option<u64>,
    pub text_count: Option<u64>,
    pub word_count: Option<u64>,
    pub is_original: Option<bool>,
    pub bookmark_count: Option<u64>,
    pub view_count: Option<u64>,
}
impl From<IllustData> for ItemRecord {
    fn from(data: IllustData) -> Self {
        ItemRecord {
            is_illust: true,
            id: data.id.parse().unwrap_or(0),
            title: data.title,
            x_restrict: data.x_restrict != 0,
            tags: data.tags,
            user_id: data.user_id.parse().unwrap_or(0),
            create_date: data.create_date,
            ai_type: data.ai_type != 1,
            width: Some(data.width),
            height: Some(data.height),
            text_count: None,
            word_count: None,
            is_original: None,
            bookmark_count: None,
            view_count: None,
        }
    }
}

impl From<NovelData> for ItemRecord {
    fn from(data: NovelData) -> Self {
        ItemRecord {
            is_illust: false,
            id: data.id.parse().unwrap_or(0),
            title: data.title,
            x_restrict: data.x_restrict != 0,
            tags: data.tags,
            user_id: data.user_id.parse().unwrap_or(0),
            create_date: data.create_date,
            ai_type: data.ai_type != 1,
            width: None,
            height: None,
            text_count: Some(data.text_count),
            word_count: Some(data.word_count),
            is_original: Some(data.is_original),
            bookmark_count: None,
            view_count: None,
        }
    }
}

/// スクレイピングのオプションを表す構造体。
/// - `tags`、`search_mode`、期間（`scd`/`ecd`）を保持する
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScrapingOption {
    pub id: String,
    pub tags: Vec<String>,
    pub search_mode: String,
    pub scd: String,
    pub ecd: String,
    pub detailed: bool,
    pub is_illust: bool,
}

// ----- 進捗関連の型 -----

/// スクレイピング進捗を表す構造体。Queue の状態取得等で使う。
#[derive(Debug, Serialize, Clone)]
pub struct ScrapingProgress {
    pub status: ScrapingStatus,
    pub total: Option<u64>,
    pub current: Option<u64>,
}

/// スクレイピングの状態（実行中か停止中か）
#[derive(Debug, Serialize, Clone, PartialEq)]
pub enum ScrapingStatus {
    Running,
    Stopped,
}

// ----- Worker (Logic) -----

pub struct Worker;

impl Worker {
    /// オプションに基づいてスクレイピングを実行し、結果をCSV保存する。
    pub async fn run(
        option: &ScrapingOption,
        client: &reqwest::Client,
        progress: &Arc<Mutex<ScrapingProgress>>,
        cfg: &Config,
        token: &tokio_util::sync::CancellationToken,
        app_handle: &dyn AppHandleLike,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Prepare and delegate network/paging to the api layer
        println!("Starting scraping with options: {:?}", option);

        // Fetch basic list
        let mut items = fetch_search_result(cfg, option, client, progress, token).await?;

        // If detailed mode is requested, enrich each item with detail API calls
        if option.detailed {
            for rec in &mut items {
                if token.is_cancelled() {
                    println!("Detailed scraping cancelled");
                    break;
                }
                // fetch_detail_data uses the configured scraping interval
                let current = progress.lock().await.current.unwrap_or(0);
                progress.lock().await.current = Some(current + 1);
                let enriched = fetch_detail_data(rec.clone(), client, &cfg.cookies, cfg.scraping_interval_min_millis, cfg.scraping_interval_max_millis).await?;
                *rec = enriched;
            }
        }

        // Save to CSV if we have items
        if !items.is_empty() {
            if let Err(e) = crate::csv::save_as_csv(&items, app_handle).await {
                println!("Failed to save CSV: {}", e);
                // We might want to return error or just log it.
                // For now, logging is safer to keep loop running if needed.
            }
        }

        Ok(())
    }
}
