# 仕様書: 未使用の ScrapingState を削除

## 背景

`lib.rs` に `ScrapingState = Arc<Mutex<Option<CancellationToken>>>` 型エイリアスと `app.manage()` 呼び出しが残っているが、`tauri::State<ScrapingState>` で取得している箇所はどこにもない。キャンセルトークンの管理は `QueryQueueActor` 内部に移行済み。

## 目的

デッドコードを削除し、将来の読者への誤解と `app.manage()` の型衝突リスクを排除する。

## スコープ

- `src-tauri/src/lib.rs` のみ

## 仕様

以下を削除する：

1. `use tokio_util::sync::CancellationToken;` import（`lib.rs` 内で他に使用箇所がないことを確認済み）
2. `pub type ScrapingState = Arc<Mutex<Option<CancellationToken>>>;` 型エイリアス
3. `app.manage(Arc::new(Mutex::new(None::<CancellationToken>)));` 呼び出し

`tokio::sync::Mutex` と `std::sync::Arc` は `AppConfig` で引き続き使用するため残す。

## 受け入れ条件

1. `cargo build` が通ること（コンパイルエラーなし）
2. `cargo clippy` で新規警告なし
3. `lib.rs` に `ScrapingState` / `CancellationToken` が残っていないこと
4. 既存の `cargo test` が引き続き全パスすること

## スコープ外

- `commands/scraping.rs` の `CancellationToken` 使用（独立した import で問題なし）
- `scraper/queue.rs` の `CancellationToken` 使用（変更不要）
