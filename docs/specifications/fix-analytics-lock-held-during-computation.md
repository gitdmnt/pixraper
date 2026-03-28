# 仕様書: 分析コマンドのロック保持時間短縮とグローバル統計二重計算の排除

## 背景

`calculate_tag_ranking` と `get_all_tags` はキャッシュのロックを取得した後、ロックを保持したまま全件走査・集計・ソートを行う。大規模データセット（数万件）のとき、並行して他のコマンドが呼び出されると全コマンドがブロックされる。

加えて `artist_stats()` 内部でもグローバル統計を独立して全件走査して再計算しており、O(n) の走査が計3回発生する。

## 目的

1. ロック内ではデータを `clone()` するだけにし、計算はロック解放後に行う
2. `artist_stats()` に外部からグローバル統計を渡すことで重複走査を排除する

## スコープ

- `src-tauri/src/analytics/mod.rs`（`ItemRecordVecExt` トレイト・`artist_stats` シグネチャ変更・内部グローバル統計再計算の削除）
- `src-tauri/src/commands/analytics.rs`（`calculate_tag_ranking`、`get_all_tags`、`calculate_co_occurence` でロック解放後に計算）

## 型情報

- キャッシュの型: `Arc<Mutex<Option<Vec<ItemRecord>>>>`
- `items` の型: `Vec<ItemRecord>`
- `filter_by` は `ItemRecordVecExt` トレイト（`analytics/mod.rs`）のメソッド。`Vec<ItemRecord>` に実装済み。

## 仕様

### ロック保持時間の短縮（`analytics.rs`）

ロック内でデータを `clone()` してすぐ解放し、計算はロック解放後に行う。

#### `calculate_tag_ranking` の変更

```rust
pub async fn calculate_tag_ranking(...) -> Result<Vec<TagStats>, String> {
    let items = {
        let cache = state.0.lock().await;
        cache.as_ref().ok_or("No dataset loaded. Please import CSV first.")?.clone()
    }; // ここでロック解放

    let filtered_items = items.filter_by(&filter);
    let (global_mean, global_std) = filtered_items.global_stats();
    let artist_stats = filtered_items.artist_stats((global_mean, global_std));  // グローバル統計を渡す
    let mut stats = filtered_items.tag_stats(&artist_stats, (global_mean, global_std));

    stats.cutoff_filter(filter.works_count_cutoff);
    stats.sort_by_key(sort_key);
    if let Some(query) = &filter.search_query {
        stats.search_query_filter(query);
    }

    Ok(stats)
}
```

#### `get_all_tags` の変更

```rust
pub async fn get_all_tags(...) -> Result<Vec<TagCount>, String> {
    let items = {
        let cache = state.0.lock().await;
        match cache.as_ref() {
            Some(v) => v.clone(),
            None => return Ok(Vec::new()),
        }
    }; // ここでロック解放
    Ok(count_tags(&items))
}
```

#### `calculate_co_occurence` の変更

```rust
pub async fn calculate_co_occurence(
    filter: Filter,
    tag: String,
    state: tauri::State<'_, AnalyticsState>,
) -> Result<CooccurrenceResult, String> {
    let items = {
        let cache = state.0.lock().await;
        cache.as_ref().ok_or("No dataset loaded. Please import CSV first.")?.clone()
    }; // ここでロック解放

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
    entries.sort_by(|a, b| b.count.cmp(&a.count));

    Ok(CooccurrenceResult {
        counts: entries,
        total: total_in_subset,
    })
}
```

### `artist_stats` のシグネチャ変更（`analytics/mod.rs`）

グローバル統計の再計算を排除するため、呼び出し元から渡す。`ItemRecordVecExt` トレイト定義と実装の両方を変更する。

```rust
// 変更前（トレイト定義・実装ともに）
fn artist_stats(&self) -> HashMap<u64, (f64, f64)>;

// 変更後（トレイト定義・実装ともに）
fn artist_stats(&self, global_stats: (f64, f64)) -> HashMap<u64, (f64, f64)>;
```

`artist_stats` 内部のグローバル統計計算ブロック（`total_sum`, `total_sumsq`, `total_n` の蓄積と `global_mean`/`global_std` の計算）を削除し、引数から受け取った値を使う。

## テスト追加

`analytics/mod.rs` の `#[cfg(test)]` ブロックに以下を追加する：

- `artist_stats_uses_provided_global_stats` — `global_stats` を明示的に渡し、アーティストが1作品のみ（自身のSDが0）のとき global_std にフォールバックすることを確認する。具体的には `make_item_with_bookmark(1, 9)` 1件からなるリストに `global_stats = (0.0, 2.0)` を渡したとき、戻り値の `user_id=1` エントリの SD が `2.0`（global_std）となることをアサートする。
- `tag_stats_normalized_score_reflects_artist_stats` — `global_stats` → `artist_stats` → `tag_stats` の純粋関数チェーンが整合した `normalized_score` を返すことを確認する（計算全体の結合テスト）。入力: `user_id=1, bookmark_count=9` の1作品のみ（`tag = "a"`）、`global_stats = (0.0, 1.0)`。`bookmark_count=9` のとき `x = ln(10) ≒ 2.302`、アーティストの作品が1件なので SD は global_std=1.0 にフォールバック、`normalized_score = (ln(10) - 0.0) / 1.0 ≒ 2.302`。アサート: `tag "a"` のエントリが存在し `(result.normalized_score - 2.302585).abs() < 1e-4` であること（`ln(10) = 2.302585...` を基準とした絶対誤差 `1e-4` で評価）。
- `co_occurrence_counts_correctly` — `calculate_co_occurence` のロック変更後も共起カウントが正しく動作することを確認する。`Vec<ItemRecord>` に対して `filter_by` → タグ共起集計の純粋な計算ロジックをテストする。入力: `tags=["a","b"]` の1作品と `tags=["a","c"]` の1作品、対象タグ `"a"` で共起集計する。期待値: `"b"=1, "c"=1`。（`calculate_co_occurence` コマンド自体のロック解放パターンの構造変更はコンパイルで保証されるため追加テスト不要）

`ItemRecord` モックの最小構成：

```rust
fn make_item_with_bookmark(user_id: u64, bookmark_count: u64) -> ItemRecord {
    ItemRecord {
        is_illust: true,
        id: 0,
        title: String::new(),
        x_restrict: false,
        tags: vec![],
        user_id,
        create_date: String::new(),
        ai_type: false,
        width: None,
        height: None,
        text_count: None,
        word_count: None,
        is_original: None,
        bookmark_count: Some(bookmark_count),
        view_count: None,
    }
}
```

## 受け入れ条件

1. `cargo test` が全パスすること
2. `cargo clippy` で新規警告なし
3. `calculate_tag_ranking`、`get_all_tags`、`calculate_co_occurence` でロックがデータ clone 直後に解放されること（コードレビューで確認）
4. `artist_stats` のシグネチャが `(global_stats: (f64, f64))` を受け取る形になること（コンパイルで保証）
5. `artist_stats` 内部に `total_sum`, `total_sumsq`, `total_n` によるグローバル統計再計算が存在しないこと（コードレビューで確認）

## スコープ外

- ロック実装をRwLockへ変更（設計変更が大きいため別タスクで検討）
- `calculate_tag_ranking` の統合テスト（ネットワーク依存のため別タスク）
