use crate::scraper::scrape::ItemRecord;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 結果データのソートキーの列挙型。
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

/// フィルタ条件をまとめた構造体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Filter {
    pub works_count_cutoff: u64,
    pub show_ai_generated: bool,
    pub show_not_ai_generated: bool,
    pub show_x_restricted: bool,
    pub show_not_x_restricted: bool,
    pub search_query: Option<String>,
}

/// タグごとの集計統計情報を表す構造体。
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

/// 共起解析の結果エントリを表す構造体。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CooccurrenceEntry {
    pub tag: String,
    pub count: u64,
}

/// 共起解析の結果全体を表す構造体。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CooccurrenceResult {
    pub counts: Vec<CooccurrenceEntry>,
    pub total: u64,
}

/// `Vec<ItemRecord>` に対する拡張トレイト。
pub trait ItemRecordVecExt {
    fn filter_by(&self, f: Filter) -> Vec<ItemRecord>;
    fn aggregated_tag_stats(&self) -> Vec<TagStats>;
}

impl ItemRecordVecExt for Vec<ItemRecord> {
    fn filter_by(&self, f: Filter) -> Vec<ItemRecord> {
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
