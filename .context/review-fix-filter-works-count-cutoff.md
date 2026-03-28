# 仕様書: works_count_cutoff フィルタをタグ統計に適用する

## 背景

`Filter` 構造体に `works_count_cutoff: u64` フィールドがあるが、`filter_by()` も `calculate_tag_ranking` 内のいずれの処理でも参照されておらず、フロントエンドで設定した件数カットオフが結果に反映されない。

## 目的

タグ集計結果から `count < works_count_cutoff` のタグを除外し、カットオフ設定が有効に機能するようにする。

## スコープ

- `src-tauri/src/analytics/mod.rs`（`TagStatsVecExt` に `cutoff_filter` を追加）
- `src-tauri/src/commands/analytics.rs`（`calculate_tag_ranking` でソート後に呼び出す）

## 仕様

### 境界値の定義

`count == works_count_cutoff` のタグは除外しない（閾値と同値は保持する）。すなわち `count >= works_count_cutoff` を満たすタグのみを残す。`works_count_cutoff == 0` の場合は全タグを保持する（フィルタリング無効として扱う）。

### 型確認

`calculate_tag_ranking` は `Vec<TagStats>` を返す（`analytics.rs:51`）。`TagCount` は `get_all_tags` 専用の軽量型であり、`TagStatsVecExt` の対象型 `Vec<TagStats>` との不一致はない（コードで確認済み）。

### `TagStatsVecExt::cutoff_filter` を追加（`analytics/mod.rs`）

```rust
pub trait TagStatsVecExt {
    fn sort_by_key(&mut self, key: SortKey);
    fn search_query_filter(&mut self, query: &str) -> &mut Self;
    fn cutoff_filter(&mut self, min_count: u64) -> &mut Self;  // 追加
}

impl TagStatsVecExt for Vec<TagStats> {
    fn cutoff_filter(&mut self, min_count: u64) -> &mut Self {
        // min_count == 0 のとき u64 は常に 0 以上なので全件保持（フィルタリング無効）
        self.retain(|s| s.count >= min_count);
        self
    }
}
```

### `calculate_tag_ranking` でカットオフを適用（`analytics.rs`）

カットオフはソートの**前**に適用する。理由は、ソート対象データを事前に減らすことでソートコストを下げるためであり、カットオフ条件はタグ名と無関係なため検索クエリフィルタとは独立して先行適用できる。検索クエリフィルタはソート**後**に残す理由は、フロントエンドのページネーション UI でソート済みの並びを前提としているため順序を変えてはならないからである。適用順序はコード例のとおり `cutoff_filter` → `sort_by_key` → `search_query_filter`。

```rust
let mut stats = filtered_items.tag_stats(...);

// Works count cutoff（ソート前に適用してデータ量を削減）
stats.cutoff_filter(filter.works_count_cutoff);

// Sort
stats.sort_by_key(sort_key);

// Search Query Filter
if let Some(query) = &filter.search_query {
    stats.search_query_filter(query);
}
```

## テスト追加

`analytics/mod.rs` の `#[cfg(test)]` ブロックに以下を追加する：

- `cutoff_filter_removes_tags_below_threshold` — `count` が閾値未満のタグが除去されること。かつ閾値以上のタグが保持されること（除外側・保持側の両方をアサートすること）
- `cutoff_filter_keeps_tags_at_threshold` — `count == 閾値` のタグが保持されること（境界値）
- `cutoff_filter_zero_keeps_all` — 閾値 0 のとき全タグが保持されること

`TagStats` のフィールドは `tag: String, count: u64, view_count: u64, bookmark_count: u64, normalized_score: f64` であり、テストモックは以下のように構築する（全フィールド指定）：

```rust
fn make_stat(tag: &str, count: u64) -> TagStats {
    TagStats {
        tag: tag.to_string(),
        count,
        view_count: 0,
        bookmark_count: 0,
        normalized_score: 0.0,
    }
}
```

受け入れ条件4は `cutoff_filter` ユニットテストにより間接的に保証する。`calculate_tag_ranking` コマンド全体の統合テストは別途スコープ外とする（ネットワーク依存のため）。

## 受け入れ条件

1. `cargo test` が全パスすること
2. `cargo clippy` で `works_count_cutoff` の `dead_code` 警告が解消されること。これは `filter.works_count_cutoff` が `cutoff_filter` の引数として渡されていることの自動的な保証となる
3. 追加した3件のテストが通ること

## スコープ外

- `calculate_co_occurence` への `works_count_cutoff` 適用（共起分析での適用は別途検討）
- `filter_by()` でのアイテムレベルカットオフ（タグ単位の集計後に適用が正しい）
- `calculate_tag_ranking` の統合テスト追加（ネットワーク依存のため別タスクで対応）
