//! 分析系コマンド（現状は簡易実装）
//!
//! 実装の意図:
//! - 分析は将来的に独立した処理やワーカーに切り出す可能性があるため、
//!   現状はフロントエンドからの呼び出し用の薄いラッパーを用意しています。

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::analytics::{
    CooccurrenceEntry, CooccurrenceResult, Filter, ItemRecordVecExt, SortKey, TagCount, TagStats,
    TagStatsVecExt, count_tags,
};
use crate::csv::load_items;
use crate::scraper::scrape::ItemRecord;

// State to cache the loaded dataset in memory
pub struct AnalyticsState(pub Arc<Mutex<Option<Vec<ItemRecord>>>>);

#[tauri::command]
pub async fn get_all_tags(
    state: tauri::State<'_, AnalyticsState>,
) -> Result<Vec<TagCount>, String> {
    let cache = state.0.lock().await;
    let items = match cache.as_ref() {
        Some(v) => v,
        None => return Ok(Vec::new()),
    };
    Ok(count_tags(items))
}

/// CSVファイルを読み込んでメモリにキャッシュし、件数を返します。
#[tauri::command]
pub async fn load_dataset(
    path: String,
    state: tauri::State<'_, AnalyticsState>,
) -> Result<usize, String> {
    let items = load_items(&path)?;
    let count = items.len();

    // Update cache
    let mut cache = state.0.lock().await;
    *cache = Some(items);

    Ok(count)
}

/// キャッシュされたデータセットに対して集計とソートを行います。
#[tauri::command]
pub async fn calculate_tag_ranking(
    filter: Filter,
    sort_key: SortKey,
    state: tauri::State<'_, AnalyticsState>,
) -> Result<Vec<TagStats>, String> {
    let cache = state.0.lock().await;
    let items = cache
        .as_ref()
        .ok_or("No dataset loaded. Please import CSV first.")?;

    // 1) Filter
    let filtered_items = items.filter_by(&filter);

    // 2) PASS 1: Compute per-artist statistics (log(bookmark+1)), and global stats for fallback
    let (global_mean, global_std) = filtered_items.global_stats();
    let artist_stats = filtered_items.artist_stats();
    // 3) PASS 2: Aggregate tags and accumulate normalized Z-scores
    let mut stats: Vec<TagStats> =
        filtered_items.tag_stats(&artist_stats, (global_mean, global_std));

    // 4) Works count cutoff（ソート前に適用してデータ量を削減）
    stats.cutoff_filter(filter.works_count_cutoff);

    // 5) Sort
    stats.sort_by_key(sort_key);

    // 6) Search Query Filter
    if let Some(query) = &filter.search_query {
        stats.search_query_filter(query);
    }

    Ok(stats)
}

#[tauri::command]
pub async fn calculate_co_occurence(
    filter: Filter,
    tag: String,
    state: tauri::State<'_, AnalyticsState>,
) -> Result<CooccurrenceResult, String> {
    let cache = state.0.lock().await;
    let items = cache
        .as_ref()
        .ok_or("No dataset loaded. Please import CSV first.")?;
    let filtered_items = items.filter_by(&filter);

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
        .map(|(tag, count)| CooccurrenceEntry { tag, count })
        .collect();

    // sort descending by count
    entries.sort_by(|a, b| b.count.cmp(&a.count));

    Ok(CooccurrenceResult {
        counts: entries,
        total: total_in_subset,
    })
}
