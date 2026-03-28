以下の仕様書をレビューしてください。

確認観点:
- ユーザー要求から外れていないか
- スキルを適切に使用しているか
- 実装者に追加判断を残していないか
- 根拠のない決定や先送りが入っていないか
- スコープ外として扱う項目を、理由なく無視していないか
- スコープ外の変更や過剰設計が入っていないか
- 必要なテスト観点と受け入れ条件が欠けていないか

各観点について PASS / ISSUE / SUGGESTION で評価し、ISSUE があれば簡潔な修正案を述べてください。

---

# 仕様書: get_all_tags の戻り値型修正

## 背景

`get_all_tags` コマンドは `TagStats` 型を返しているが、`view_count` / `bookmark_count` / `normalized_score` が常に 0 になっており、型の誤用が発生している。

## 目的

`get_all_tags` の戻り値を用途に特化した軽量型 `TagCount` に変更し、`TagStats` の誤用を型レベルで排除する。

## スコープ

- バックエンド: `src-tauri/src/analytics/mod.rs`, `src-tauri/src/commands/analytics.rs`
- フロントエンド: `src/routes/analytics/cooccurrenceAnalyze.svelte`
- テスト: `src-tauri/src/analytics/mod.rs` 内の単体テスト

## 仕様

### 新型 `TagCount`

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCount {
    pub tag: String,
    pub count: u64,
}
```

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

- 旧 `TagStats` インタフェースの `_viewCount` / `_bookmarkCount` フィールドを削除
- 変数名 `tagCounts` の型を `TagCount[]` に変更

## 受け入れ条件

1. `cargo test` が通ること
2. `cargo clippy` で警告がないこと
3. `get_all_tags` の戻り値が `Vec<TagCount>` であること
4. `TagStats` は `calculate_tag_ranking` でのみ使用されること
5. フロントエンドの `cooccurrenceAnalyze.svelte` が `TagCount` 型を使うこと
6. タグサジェスト機能（`suggestedTags`）が引き続き正しく動作すること

## スコープ外

- `calculate_tag_ranking` の変更
- `TagStats` 型の変更
- フィルタ機能の付与（`get_all_tags` はフィルタなし全件対象のまま）
