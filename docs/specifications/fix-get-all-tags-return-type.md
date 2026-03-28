# 仕様書: get_all_tags の戻り値型修正

## 背景

`get_all_tags` コマンドは `TagStats` 型を返しているが、`view_count` / `bookmark_count` / `normalized_score` が常に 0 になっており、型の誤用が発生している。

## 目的

`get_all_tags` の戻り値を用途に特化した軽量型 `TagCount` に変更し、`TagStats` の誤用を型レベルで排除する。

## スコープ

- バックエンド: `src-tauri/src/analytics/mod.rs`（型定義）, `src-tauri/src/commands/analytics.rs`（コマンド修正）
- フロントエンド: `src/routes/analytics/cooccurrenceAnalyze.svelte`
- テスト: `src-tauri/src/analytics/mod.rs` 内の単体テスト

## 仕様

### 新型 `TagCount`

定義場所: `src-tauri/src/analytics/mod.rs`（`CooccurrenceEntry` と同じファイル）

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCount {
    pub tag: String,
    pub count: u64,
}
```

derive は `Debug, Serialize` のみ（`CooccurrenceEntry` に揃える）。`Clone` は不要。

- `tag`: タグ文字列
- `count`: そのタグを持つアイテム数（全データセット対象、フィルタなし）

### `get_all_tags` コマンド

| 項目 | 値 |
|------|---|
| 入力 | `AnalyticsState`（キャッシュ済みデータセット） |
| 出力 | `Result<Vec<TagCount>, String>` |
| ソート | count 降順 |
| データなし | 空 Vec を返す（エラーではない） |

### フロントエンド型定義 (`cooccurrenceAnalyze.svelte`)

```typescript
interface TagCount {
  tag: string;
  count: number;
}
```

- 旧 `TagStats` インタフェース（`_viewCount` / `_bookmarkCount` フィールドを含む）を `TagCount` に置き換える
- 変数 `tagCounts` の型を `TagCount[]` に変更
- `invoke<TagStats[]>` を `invoke<TagCount[]>` に変更

### スコープ外: `cooccurrenceResults` の型不一致

`cooccurrenceResults: TagStats[]` という変数が存在し、実際は `CooccurrenceEntry[]` と一致していない既存バグがあるが、本 Issue（`get_all_tags` の戻り値型修正）とは独立した問題であるため、別 Issue で対応する。本仕様では変更しない。

## テストケース

以下の3ケースを `src-tauri/src/analytics/mod.rs` の `#[cfg(test)]` ブロックに追加する：

1. **正常系: タグあり** — 複数アイテムを持つ Vec を渡し、返り値が `TagCount` の Vec（tag・count フィールドのみ）で count 降順にソートされていること
2. **正常系: データなし** — 空の Vec を渡し、空の Vec が返ること
3. **正常系: ソート順** — count の異なる複数タグを渡し、降順に並んでいること

## 受け入れ条件

1. `cargo test` が通ること
2. `cargo clippy` で警告がないこと
3. `get_all_tags` の戻り値型が `Vec<TagCount>` であること
4. `TagStats` の import が `commands/analytics.rs` から削除されないこと（`calculate_tag_ranking` で引き続き使用する）
5. フロントエンドの `cooccurrenceAnalyze.svelte` が `TagCount` 型を使うこと
6. タグサジェスト機能（`suggestedTags`）が引き続き正しく動作すること

## スコープ外

- `calculate_tag_ranking` の変更
- `TagStats` 型の変更
- フィルタ機能の付与（`get_all_tags` はフィルタなし全件対象のまま）
- `cooccurrenceResults` の型修正（別 Issue で対応）
