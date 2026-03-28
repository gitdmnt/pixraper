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
    /// Artist-normalized score (Z-score averaged per tag)
    NormalizedScore,
}

/// フィルタ条件をまとめた構造体。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Filter {
    pub works_count_cutoff: u64,
    #[serde(rename = "showAIGenerated")]
    pub show_ai_generated: bool,
    #[serde(rename = "showNotAIGenerated")]
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
    /// Artist-normalized score (average Z-score for works containing the tag)
    pub normalized_score: f64,
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
    fn filter_by(&self, f: &Filter) -> Vec<ItemRecord>;
    fn global_stats(&self) -> (f64, f64);
    fn artist_stats(&self) -> HashMap<u64, (f64, f64)>;
    fn tag_stats(
        &self,
        artist_stats: &HashMap<u64, (f64, f64)>,
        global_stats: (f64, f64),
    ) -> Vec<TagStats>;
}

impl ItemRecordVecExt for Vec<ItemRecord> {
    fn filter_by(&self, f: &Filter) -> Vec<ItemRecord> {
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
                true
            })
            .cloned()
            .collect()
    }
    fn global_stats(&self) -> (f64, f64) {
        let mut total_sum = 0.0_f64;
        let mut total_sumsq = 0.0_f64;
        let mut total_n: u64 = 0;

        for item in self {
            let b = item.bookmark_count.unwrap_or(0) as f64;
            let x = (b + 1.0).ln();
            total_sum += x;
            total_sumsq += x * x;
            total_n += 1;
        }

        let global_mean = if total_n > 0 {
            total_sum / total_n as f64
        } else {
            0.0
        };
        let global_var = if total_n > 0 {
            (total_sumsq / total_n as f64) - (global_mean * global_mean)
        } else {
            0.0
        };
        let global_std = global_var.max(0.0).sqrt();

        (global_mean, global_std)
    }

    // userid -> (log bookmark mean, log sd)
    fn artist_stats(&self) -> HashMap<u64, (f64, f64)> {
        let mut artist_acc: HashMap<u64, (f64, f64, u64)> = HashMap::new(); // user_id -> (sum, sumsq, n)
        let mut total_sum = 0.0_f64;
        let mut total_sumsq = 0.0_f64;
        let mut total_n: u64 = 0;

        for item in self {
            let b = item.bookmark_count.unwrap_or(0) as f64;
            let x = (b + 1.0).ln();
            let e = artist_acc.entry(item.user_id).or_insert((0.0, 0.0, 0));
            e.0 += x;
            e.1 += x * x;
            e.2 += 1;
            total_sum += x;
            total_sumsq += x * x;
            total_n += 1;
        }

        let global_mean = if total_n > 0 {
            total_sum / total_n as f64
        } else {
            0.0
        };
        let global_var = if total_n > 0 {
            (total_sumsq / total_n as f64) - (global_mean * global_mean)
        } else {
            0.0
        };
        let global_std = global_var.max(0.0).sqrt();
        let eps = 1e-9_f64;

        let mut artist_stats: HashMap<u64, (f64, f64)> = HashMap::new(); // user_id -> (mean, sd)
        for (user, (sum, sumsq, n)) in artist_acc.into_iter() {
            let mean = sum / n as f64;
            let var = if n > 1 {
                (sumsq / n as f64) - (mean * mean)
            } else {
                0.0
            };
            let sd = var.max(0.0).sqrt();
            artist_stats.insert(
                user,
                (mean, if sd < eps { global_std.max(eps) } else { sd }),
            );
        }
        artist_stats
    }

    fn tag_stats(
        &self,
        artist_stats: &HashMap<u64, (f64, f64)>,
        global_stats: (f64, f64),
    ) -> Vec<TagStats> {
        let mut stats_map: HashMap<String, (u64, u64, u64, f64)> = HashMap::new(); // (count, view, bookmark, normalized_score)

        for item in self {
            for tag in &item.tags {
                let entry = stats_map.entry(tag.clone()).or_insert((0, 0, 0, 0.0));
                entry.0 += 1;
                entry.1 += item.view_count.unwrap_or(0);
                entry.2 += item.bookmark_count.unwrap_or(0);
                entry.3 += {
                    let (artist_mean, artist_sd) = artist_stats
                        .get(&item.user_id)
                        .cloned()
                        .unwrap_or(global_stats);
                    let b = item.bookmark_count.unwrap_or(0) as f64;
                    let x = (b + 1.0).ln();
                    (x - artist_mean) / artist_sd
                }
            }
        }

        stats_map
            .into_iter()
            .map(
                |(tag, (count, view_count, bookmark_count, normalized_score))| TagStats {
                    tag,
                    count,
                    view_count,
                    bookmark_count,
                    normalized_score,
                },
            )
            .collect()
    }
}

/// タグごとの件数のみを表す軽量型。`get_all_tags` コマンド専用。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCount {
    pub tag: String,
    pub count: u64,
}

pub trait TagStatsVecExt {
    fn sort_by_key(&mut self, key: SortKey);
    fn search_query_filter(&mut self, query: &str) -> &mut Self;
}

impl TagStatsVecExt for Vec<TagStats> {
    fn sort_by_key(&mut self, key: SortKey) {
        match key {
            SortKey::WorkCount => self.sort_by(|a, b| b.count.cmp(&a.count)),
            SortKey::BookmarkCount => self.sort_by(|a, b| b.bookmark_count.cmp(&a.bookmark_count)),
            SortKey::ViewCount => self.sort_by(|a, b| b.view_count.cmp(&a.view_count)),
            SortKey::BookmarkPerWork => self.sort_by(|a, b| {
                (b.bookmark_count as f64 / b.count as f64)
                    .partial_cmp(&(a.bookmark_count as f64 / a.count as f64))
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortKey::ViewPerWork => self.sort_by(|a, b| {
                (b.view_count as f64 / b.count as f64)
                    .partial_cmp(&(a.view_count as f64 / a.count as f64))
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortKey::BookmarkPerView => self.sort_by(|a, b| {
                (b.bookmark_count as f64 / b.view_count.max(1) as f64)
                    .partial_cmp(&(a.bookmark_count as f64 / a.view_count.max(1) as f64))
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortKey::NormalizedScore => self.sort_by(|a, b| {
                b.normalized_score
                    .partial_cmp(&a.normalized_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
        }
    }

    fn search_query_filter(&mut self, query: &str) -> &mut Self {
        let query_lower = query.to_lowercase();
        self.retain(|tag_stat| tag_stat.tag.to_lowercase().contains(&query_lower));
        self
    }
}

/// `get_all_tags` ロジックの純粋関数（テスト容易性のため抽出）。
/// キャッシュ済みアイテム全件からタグ件数を集計し、count 降順でソートして返す。
pub fn count_tags(items: &[crate::scraper::scrape::ItemRecord]) -> Vec<TagCount> {
    let mut map: HashMap<String, u64> = HashMap::new();
    for item in items {
        for tag in &item.tags {
            *map.entry(tag.clone()).or_insert(0) += 1;
        }
    }
    let mut entries: Vec<TagCount> = map
        .into_iter()
        .map(|(tag, count)| TagCount { tag, count })
        .collect();
    entries.sort_by(|a, b| b.count.cmp(&a.count));
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scraper::scrape::ItemRecord;

    fn make_item(tags: Vec<&str>) -> ItemRecord {
        ItemRecord {
            is_illust: true,
            id: 0,
            title: String::new(),
            x_restrict: false,
            tags: tags.into_iter().map(String::from).collect(),
            user_id: 0,
            create_date: String::new(),
            ai_type: false,
            width: None,
            height: None,
            text_count: None,
            word_count: None,
            is_original: None,
            bookmark_count: None,
            view_count: None,
        }
    }

    #[test]
    fn count_tags_returns_tag_count_fields_only() {
        let items = vec![
            make_item(vec!["a", "b"]),
            make_item(vec!["a"]),
        ];
        let result = count_tags(&items);
        // TagCount には tag と count のみ存在する（コンパイルで保証）
        assert!(result.iter().any(|tc| tc.tag == "a" && tc.count == 2));
        assert!(result.iter().any(|tc| tc.tag == "b" && tc.count == 1));
    }

    #[test]
    fn count_tags_empty_items_returns_empty() {
        let result = count_tags(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn count_tags_sorted_descending_by_count() {
        // common=2, rare=1 なので common が先頭
        let items = vec![
            make_item(vec!["common"]),
            make_item(vec!["common"]),
            make_item(vec!["rare"]),
        ];
        let result = count_tags(&items);
        assert_eq!(result[0].tag, "common");
        assert_eq!(result[0].count, 2);
        assert_eq!(result[1].tag, "rare");
        assert_eq!(result[1].count, 1);
    }
}
