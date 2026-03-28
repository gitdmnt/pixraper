# 仕様書: スクレイピング間隔を最小・最大レンジで指定する

## 背景

現在 `Config` は `scraping_interval_millis: u64`（固定値）でスクレイピング間隔を管理しているが、
ユーザーが min〜max の範囲を指定し、その範囲内のランダム値を間隔として使いたい。

## 目的

設定画面で最小・最大間隔（ms）を指定できるようにし、API 呼び出しごとにその範囲内のランダム値をスリープ時間として使う。

## スコープ

- `src-tauri/src/config.rs`（Config 構造体の変更）
- `src-tauri/Cargo.toml`（`rand` クレートの追加）
- `src-tauri/src/scraper/api.rs`（`random_interval` 関数の追加・sleep 箇所の変更）
- `src-tauri/src/scraper/scrape.rs`（`fetch_detail_data` 呼び出し元の変更）
- `src/routes/settings/type.d.ts`（TypeScript 型の変更）
- `src/routes/settings/main.svelte`（UI の変更）

## 仕様

### Config 構造体の変更

```rust
// 変更前
pub scraping_interval_millis: u64,

// 変更後
pub scraping_interval_min_millis: u64,
pub scraping_interval_max_millis: u64,
```

デフォルト値:
- `scraping_interval_min_millis`: 1000
- `scraping_interval_max_millis`: 2000

### ランダム間隔生成

`api.rs` にプライベート関数を追加:

```rust
fn random_interval(min: u64, max: u64) -> Duration {
    let effective_max = max.max(min); // min > max のとき min を上限として使う
    let millis = rand::thread_rng().gen_range(min..=effective_max);
    Duration::from_millis(millis)
}
```

`min > max` の場合は `max = min` として扱い、パニックしない。
（UI 側での min ≤ max の保証が主防衛ラインだが、バックエンドでも安全に動作させる）

### fetch_search_result での使用

既存の `tokio::time::sleep(Duration::from_millis(cfg.scraping_interval_millis))` を以下に変更:

```rust
tokio::time::sleep(random_interval(cfg.scraping_interval_min_millis, cfg.scraping_interval_max_millis)).await;
```

### fetch_detail_data シグネチャ変更

```rust
// 変更前
pub async fn fetch_detail_data(
    mut record: ItemRecord,
    client: &reqwest::Client,
    cookie_header: &Option<String>,
    interval_millis: u64,
) -> ...

// 変更後
pub async fn fetch_detail_data(
    mut record: ItemRecord,
    client: &reqwest::Client,
    cookie_header: &Option<String>,
    interval_min_millis: u64,
    interval_max_millis: u64,
) -> ...
```

内部の sleep:
```rust
tokio::time::sleep(random_interval(interval_min_millis, interval_max_millis)).await;
```

### scrape.rs の呼び出し元

```rust
// 変更後
let enriched = fetch_detail_data(
    rec.clone(),
    client,
    &cfg.cookies,
    cfg.scraping_interval_min_millis,
    cfg.scraping_interval_max_millis,
).await?;
```

### TypeScript 型（type.d.ts）

```typescript
// 変更前
scraping_interval_millis: number;

// 変更後
scraping_interval_min_millis: number;
scraping_interval_max_millis: number;
```

### 設定 UI（main.svelte）

- 既存の「スクレイピング間隔」単一フィールドを削除
- 「最小間隔（ms）」と「最大間隔（ms）」の2つの number input を追加
- `bind:value={config.scraping_interval_min_millis}` / `bind:value={config.scraping_interval_max_millis}`

## バリデーション

- `min <= max` であること（UI では視覚的なヒントのみ、バックエンドでの強制はしない）

## 受け入れ条件

1. `cargo clippy` で新規警告なし
2. `rand` クレートが `Cargo.toml` に追加されている
3. `Config` に `scraping_interval_min_millis` と `scraping_interval_max_millis` が存在し、`scraping_interval_millis` が削除されている
4. `fetch_detail_data` が `interval_min_millis: u64, interval_max_millis: u64` を受け取る（コンパイルで保証）
5. 設定画面に最小・最大の2つの入力欄が表示される
6. `api.rs` 内のコンパイル確認テスト関数（旧 `_assert_fetch_detail_data_accepts_interval_millis`）が新しい2引数シグネチャに更新されている
7. `random_interval` のユニットテストで以下を検証する:
   - `random_interval(1000, 2000)` の返値が `1000ms <= x <= 2000ms` の範囲内であること
   - `random_interval(1000, 1000)` の返値が常に `1000ms` であること（min == max）
   - `random_interval(2000, 1000)` がパニックせず `2000ms` を返すこと（min > max の fallback）

## スコープ外

- 既存の toml ファイルのマイグレーション: `Config` は `#[serde(deny_unknown_fields)]` を使用していないため、旧フィールド `scraping_interval_millis` が toml に残っていても serde は無視する。新フィールドは `Default` 実装で補完される。
