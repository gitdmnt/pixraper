# 仕様書: 詳細取得のウェイトをハードコーディングから設定値に変更

## 背景

`fetch_search_result` は `cfg.scraping_interval_millis` を使うのに対し、`fetch_detail_data` は固定1秒（`Duration::from_secs(1)`）をハードコーディングしている。設定画面でスクレイピング間隔を変更しても詳細取得には反映されない。

## 目的

`fetch_detail_data` が設定値のインターバルを使用するようにし、ラフ検索と詳細取得で一貫した間隔設定を実現する。

## スコープ

- `src-tauri/src/scraper/api.rs`（`fetch_detail_data` シグネチャ変更・実装修正）
- `src-tauri/src/scraper/scrape.rs`（`Worker::run` 内の呼び出し元に `cfg.scraping_interval_millis` を渡す）

## 仕様

### `fetch_detail_data` シグネチャ変更（`api.rs:292`）

```rust
pub async fn fetch_detail_data(
    mut record: ItemRecord,
    client: &reqwest::Client,
    cookie_header: &Option<String>,
    interval_millis: u64,  // 追加
) -> Result<ItemRecord, Box<dyn std::error::Error + Send + Sync>>
```

### `fetch_detail_data` 実装変更（`api.rs:303`）

```rust
// 変更前
tokio::time::sleep(std::time::Duration::from_secs(1)).await;

// 変更後
tokio::time::sleep(std::time::Duration::from_millis(interval_millis)).await;
```

### 呼び出し元変更（`scrape.rs` の `Worker::run` 内、144行目）

```rust
// 変更前
let enriched = fetch_detail_data(rec.clone(), client, &cfg.cookies).await?;

// 変更後
let enriched = fetch_detail_data(rec.clone(), client, &cfg.cookies, cfg.scraping_interval_millis).await?;
```

### コメント更新（`scrape.rs:141`）

```rust
// 変更前
// fetch_detail_data performs its own light throttling

// 変更後
// fetch_detail_data uses the configured scraping interval
```

## テスト追加（TDD: 先に追加してコンパイルエラーにする）

`api.rs` の `#[cfg(test)]` ブロックに以下を追加する。

`tokio::time::pause()` + `advance()` + `spawn` でスリープ時間を検証することも可能だが、`fetch_detail_data` がネットワーク依存のため spawn + advance 構成が必要で実装が複雑になる。そのためコンパイル保証のみとし、`Duration::from_secs(1)` の除去は受け入れ条件 4 の grep で確認する。

`api.rs` に `#[cfg(test)]` ブロックが存在しないため新規作成する。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    // ItemRecord は super::* 経由で use crate::scraper::scrape::ItemRecord が取り込まれる

    fn make_record() -> ItemRecord {
        ItemRecord {
            is_illust: true,
            id: 0,
            title: String::new(),
            x_restrict: false,
            tags: vec![],
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

    // fetch_detail_data が interval_millis: u64 を受け取るシグネチャであること
    // （コンパイル時チェック。ネットワーク呼び出しはしない）
    #[allow(dead_code)]
    fn _assert_fetch_detail_data_signature_compiles() {
        let _client = reqwest::Client::new();
        let _record = make_record();
        // 型チェックのみ（実行しない）
        let _ = fetch_detail_data(_record, &_client, &None, 1000u64);
    }
}
```

## 受け入れ条件

1. `cargo test` が全パスすること
2. `cargo clippy` で新規警告なし
3. `fetch_detail_data` が `interval_millis: u64` パラメータを受け取ること（コンパイルで保証）
4. `api.rs` 内に `Duration::from_secs(1)` が残っていないこと（`grep "from_secs(1)" src/scraper/api.rs` で0件）
5. `scrape.rs` の `Worker::run` 内呼び出しが `cfg.scraping_interval_millis` を渡すこと（シグネチャ不一致はコンパイルエラーになるため型チェックで保証済み）
6. `scrape.rs:141` のコメントが更新されていること

## スコープ外

- `Config` 型の変更
- `scraping_interval_millis` のデフォルト値の変更
- 詳細取得用に別の間隔設定を追加すること（用途は1つの設定値で統一）
