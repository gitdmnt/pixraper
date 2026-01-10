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
    pub works_count_cutoff: u64,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagEntry {
    pub tag: String,
    pub count: u64,
}

#[tauri::command]
pub async fn get_all_tags(
    state: tauri::State<'_, AnalyticsState>,
) -> Result<Vec<TagEntry>, String> {
    let cache = state.0.lock().await;
    let items = match cache.as_ref() {
        Some(v) => v,
        None => return Ok(Vec::new()),
    };

    let mut map: HashMap<String, u64> = HashMap::new();
    for item in items {
        for tag in &item.tags {
            *map.entry(tag.clone()).or_insert(0) += 1;
        }
    }

    let mut entries: Vec<TagEntry> = map
        .into_iter()
        .map(|(tag, count)| TagEntry { tag, count })
        .collect();

    entries.sort_by(|a, b| b.count.cmp(&a.count));

    Ok(entries)
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

trait ItemRecordVecExt {
    fn filter_by(&self, f: TagRankingFilters) -> Vec<ItemRecord>;
    fn aggregated_tag_stats(&self) -> Vec<TagStats>;
}

impl ItemRecordVecExt for Vec<ItemRecord> {
    fn filter_by(&self, f: TagRankingFilters) -> Vec<ItemRecord> {
        self.iter()
            .filter(|item| {
                if !f.show_ai_generated && item.ai_type {
                    return false;
                }
                if !f.show_not_ai_generated && !item.ai_type {
                    return false;
                }
                if !f.show_x_restricted && item.x_restrict {
                    return false;
                }
                if !f.show_not_x_restricted && !item.x_restrict {
                    return false;
                }
                if let Some(ref query) = f.search_query {
                    let query_lower = query.to_lowercase();
                    if !item
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
                    {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }

    fn aggregated_tag_stats(&self) -> Vec<TagStats> {
        let mut stats_map: HashMap<String, (u64, u64, u64)> = HashMap::new(); // (count, view, bookmark)

        for item in self {
            for tag in &item.tags {
                let entry = stats_map.entry(tag.clone()).or_insert((0, 0, 0));
                entry.0 += 1;
                entry.1 += item.view_count.unwrap_or(0);
                entry.2 += item.bookmark_count.unwrap_or(0);
            }
        }

        stats_map
            .into_iter()
            .map(|(tag, (count, view_count, bookmark_count))| TagStats {
                tag,
                count,
                view_count,
                bookmark_count,
            })
            .collect()
    }
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

    let filtered_items = items.filter_by(filters);
    let mut stats = filtered_items.aggregated_tag_stats();

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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CooccurrenceEntry {
    pub tag: String,
    pub count: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CooccurrenceResult {
    pub counts: Vec<CooccurrenceEntry>,
    pub total: u64,
}

#[tauri::command]
pub async fn calculate_co_occurence(
    filters: TagRankingFilters,
    tag: String,
    state: tauri::State<'_, AnalyticsState>,
) -> Result<CooccurrenceResult, String> {
    let cache = state.0.lock().await;
    let items = cache
        .as_ref()
        .ok_or("No dataset loaded. Please import CSV first.")?;
    let filtered_items = items.filter_by(filters);

    let mut co_occurrence: HashMap<String, u64> = HashMap::new();
    let mut total_in_subset: u64 = 0;

    for item in &filtered_items {
        if item.tags.contains(&tag) {
            total_in_subset += 1;
            for t in &item.tags {
                if t != &tag {
                    *co_occurrence.entry(t.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    let mut entries: Vec<CooccurrenceEntry> = co_occurrence
        .into_iter()
        .map(|(tag, count)| {
            let rate = if total_in_subset == 0 {
                0.0
            } else {
                (count as f64) / (total_in_subset as f64)
            };
            CooccurrenceEntry { tag, count }
        })
        .collect();

    // sort descending by count
    entries.sort_by(|a, b| b.count.cmp(&a.count));

    Ok(CooccurrenceResult {
        counts: entries,
        total: total_in_subset,
    })
}
