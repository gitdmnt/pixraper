//! 分析系コマンド（現状は簡易実装）
//!
//! 実装の意図:
//! - 分析は将来的に独立した処理やワーカーに切り出す可能性があるため、
//!   現状はフロントエンドからの呼び出し用の薄いラッパーを用意しています。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::csv::load_items;
use crate::scraper::scrape::ItemRecord;

// State to cache the loaded dataset in memory
pub struct AnalyticsState(pub Arc<Mutex<Option<Vec<ItemRecord>>>>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SortKey {
    WorkCount,
    BookmarkCount,
    ViewCount,
    BookmarkPerWork,
    ViewPerWork,
    BookmarkPerView,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagRankingFilters {
    pub show_ai_generated: bool,
    pub show_not_ai_generated: bool,
    pub show_x_restricted: bool,
    pub show_not_x_restricted: bool,
    pub search_query: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagStats {
    pub tag: String,
    pub count: u64,
    pub view_count: u64,
    pub bookmark_count: u64,
    // Derived metrics for display if needed, but primarily used for sorting in Rust
    // values below are calculated on the fly
}

#[tauri::command]
pub fn show_analytics(data: &str) -> String {
    // TODO: 実際の解析ロジックは将来追加する。現状は呼び出しの有無を確認するためのスタブ。
    format!("Started analysis on data: {}", data)
}

/// CSVファイルを読み込んでメモリにキャッシュし、生データを返します。
#[tauri::command]
pub async fn load_dataset(
    path: String,
    state: tauri::State<'_, AnalyticsState>,
) -> Result<Vec<ItemRecord>, String> {
    let items = load_items(&path)?;

    // Update cache
    let mut cache = state.0.lock().await;
    *cache = Some(items.clone());

    Ok(items)
}

/// キャッシュされたデータセットに対して集計とソートを行います。
#[tauri::command]
pub async fn calculate_tag_ranking(
    filters: TagRankingFilters,
    sort_key: SortKey,
    state: tauri::State<'_, AnalyticsState>,
) -> Result<Vec<TagStats>, String> {
    let cache = state.0.lock().await;
    let items = cache
        .as_ref()
        .ok_or("No dataset loaded. Please import CSV first.")?;

    let query_lower = filters.search_query.as_deref().map(|s| s.to_lowercase());

    // 1. Filter and Aggregate
    let mut stats_map: HashMap<String, (u64, u64, u64)> = HashMap::new(); // (count, view, bookmark)

    for item in items {
        // Filter logic
        if !filters.show_ai_generated && item.ai_type {
            continue;
        }
        if !filters.show_not_ai_generated && !item.ai_type {
            continue;
        }
        if !filters.show_x_restricted && item.x_restrict {
            continue;
        }
        if !filters.show_not_x_restricted && !item.x_restrict {
            continue;
        }

        for tag in &item.tags {
            if let Some(q) = &query_lower {
                if !tag.to_lowercase().contains(q) {
                    continue;
                }
            }

            let entry = stats_map.entry(tag.clone()).or_insert((0, 0, 0));
            entry.0 += 1;
            entry.1 += item.view_count.unwrap_or(0);
            entry.2 += item.bookmark_count.unwrap_or(0);
        }
    }

    // 2. Convert to Vec
    let mut stats: Vec<TagStats> = stats_map
        .into_iter()
        .map(|(tag, (count, view_count, bookmark_count))| TagStats {
            tag,
            count,
            view_count,
            bookmark_count,
        })
        .collect();

    // 3. Sort
    stats.sort_by(|a, b| match sort_key {
        SortKey::WorkCount => b.count.cmp(&a.count),
        SortKey::BookmarkCount => b.bookmark_count.cmp(&a.bookmark_count),
        SortKey::ViewCount => b.view_count.cmp(&a.view_count),
        SortKey::BookmarkPerWork => {
            let a_rate = a.bookmark_count as f64 / a.count.max(1) as f64;
            let b_rate = b.bookmark_count as f64 / b.count.max(1) as f64;
            b_rate
                .partial_cmp(&a_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
        }
        SortKey::ViewPerWork => {
            let a_rate = a.view_count as f64 / a.count.max(1) as f64;
            let b_rate = b.view_count as f64 / b.count.max(1) as f64;
            b_rate
                .partial_cmp(&a_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
        }
        SortKey::BookmarkPerView => {
            let a_rate = a.bookmark_count as f64 / a.view_count.max(1) as f64;
            let b_rate = b.bookmark_count as f64 / b.view_count.max(1) as f64;
            b_rate
                .partial_cmp(&a_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
        }
    });

    Ok(stats)
}
