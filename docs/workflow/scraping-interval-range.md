# 実装手順書: スクレイピング間隔レンジ指定

## 手順

### 1. `Cargo.toml` に `rand` を追加

```toml
rand = "0.8"
```

### 2. `config.rs` の Config 構造体を変更

- `scraping_interval_millis: u64` を削除
- `scraping_interval_min_millis: u64` と `scraping_interval_max_millis: u64` を追加
- `Default` 実装を min=1000, max=2000 に更新

### 3. `api.rs` を変更

1. `use rand::Rng;` を追加
2. `random_interval(min: u64, max: u64) -> Duration` 関数を追加（min > max 時は max=min）
3. `fetch_search_result` 内の sleep を `random_interval(cfg.scraping_interval_min_millis, cfg.scraping_interval_max_millis)` に変更
4. `fetch_detail_data` のシグネチャに `interval_min_millis: u64, interval_max_millis: u64` を追加（旧 `interval_millis` を置換）
5. `fetch_detail_data` 内の sleep を `random_interval(interval_min_millis, interval_max_millis)` に変更
6. テスト内の `_assert_fetch_detail_data_accepts_interval_millis` を新しい2引数シグネチャに更新
7. `random_interval` のユニットテストを追加

### 4. `scrape.rs` の呼び出し元を変更

```rust
let enriched = fetch_detail_data(
    rec.clone(),
    client,
    &cfg.cookies,
    cfg.scraping_interval_min_millis,
    cfg.scraping_interval_max_millis,
).await?;
```

### 5. `src/routes/settings/type.d.ts` を変更

- `scraping_interval_millis: number` を削除
- `scraping_interval_min_millis: number` と `scraping_interval_max_millis: number` を追加

### 6. `src/routes/settings/main.svelte` を変更

- 既存の「スクレイピング間隔」input を「最小間隔（ms）」「最大間隔（ms）」の2つに置換
- 初期 state を `scraping_interval_min_millis: 1000, scraping_interval_max_millis: 2000` に更新

### 7. ビルド確認

```bash
cd src-tauri && cargo clippy
```
