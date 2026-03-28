# 仕様書: load_dataset の返り値を件数のみに変更

## 背景

`load_dataset` は `Vec<ItemRecord>` 全件を返すが、フロントエンドは `loaded = true` のフラグ変更にしか使っておらず、IPC 経由の全件転送コストが無駄になっている。

## 目的

返り値を件数（`usize`）のみに変更し、不要なシリアライズ・デシリアライズコストを除去する。

## スコープ

- `src-tauri/src/commands/analytics.rs`（`load_dataset` 返り値を `Result<usize, String>` に変更）
- `src/routes/analytics/main.svelte`（`invoke<ItemRecord[]>` → `invoke<number>`、`ItemRecord` 型定義を削除）

## 仕様

### バックエンド（`analytics.rs`）

```rust
// 変更前
pub async fn load_dataset(...) -> Result<Vec<ItemRecord>, String> {
    let items = load_items(&path)?;
    let mut cache = state.0.lock().await;
    *cache = Some(items.clone());
    Ok(items)
}

// 変更後
pub async fn load_dataset(...) -> Result<usize, String> {
    let items = load_items(&path)?;
    let count = items.len();
    let mut cache = state.0.lock().await;
    *cache = Some(items);
    Ok(count)
}
```

`items.clone()` を削除し `items` を直接キャッシュに移動する（`count` を先に取ってから `move`）。

### フロントエンド（`main.svelte`）

```typescript
// 変更前
const data = await invoke<ItemRecord[]>("load_dataset", { path: selectedPath });
loaded = true;

// 変更後
await invoke<number>("load_dataset", { path: selectedPath });
loaded = true;
```

- `data` 変数を削除
- `invoke` の型パラメータを `ItemRecord[]` → `number` に変更
- `ItemRecord` インタフェース定義（`main.svelte:18-31`）は `load_dataset` の型チェックにのみ使われていたため削除（`ItemRecord` は `main.svelte` 内のみで定義・参照されており、外部 export なし・他ファイルからの import なしを確認済み）

## 受け入れ条件

1. `cargo test` が全パスすること
2. `cargo clippy` で新規警告なし
3. `load_dataset` の返り値が `Result<usize, String>` であること（コンパイルで保証）
4. `main.svelte` の `ItemRecord` インタフェース定義および `data` 変数が削除されていること
5. `items.clone()` が `analytics.rs` から削除されていること
6. CSVファイルを選択・読み込み後、`loaded` が `true` になりタブコンテンツが表示されること（手動確認）

## スコープ外

- 件数をフロントエンドの UI に表示すること（今回は `loaded = true` のフラグのみ）
- バックエンドのキャッシュ設計の変更
