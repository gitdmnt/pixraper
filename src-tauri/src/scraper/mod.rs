//! `scraper` モジュールはいくつかの責務に分割されています。
//! - `api` サブモジュール: Pixiv の非公式 API との通信を担当し、レスポンスのデシリアライズを行います。
//! - `scrape` サブモジュール: スクレイピングの主要ロジックを実装し、`ScrapingOption` に基づいてデータ取得を行います。
//! - `queue` サブモジュール: スクレイピングタスクのキューイングと管理を担当します。
//! - `csv` サブモジュール: 取得したデータを CSV ファイルに保存する機能を提供します。
//!
//! 動作の流れは以下の通りです:
//! 1. UIからのadd_queueコマンドを受信します。
//!    コマンドはqueue::QueryQueueHandle::addを実行し、QueryQueueActorへオプションを追加します。
//!
//! 2. UIからのstart_scrapingコマンドを受信します。
//!    コマンドはqueue::QueryQueueHandle::start(cancel_token)を実行します。
//!    Actorに開始シグナルを送り、フロントエンドへのレスポンスは即座に返されます。
//!
//! 3. QueryQueueActorはStartコマンドを受信すると、内部でループ処理を開始します。
//!    a. キューからオプションを取り出します（空なら終了）。
//!    b. キャンセルトークンがキャンセル済みかチェックします。
//!    c. scrape::Worker::run(option) を呼び出します。
//!       WorkerはAPI通信、進捗更新、CSV保存の一連の流れを処理します。
//!    d. 完了後、次のループへ進みます（RunNextメッセージを自己送信）。
//!
//! 4. 取得したデータはWorkerによって自動的にCSVファイルに保存されます。

pub mod api;
pub mod csv;
pub mod queue;
pub mod scrape;

// 外部でよく使うシンボルを再エクスポートして既存コードの影響を最小化する

pub use queue::QueryQueueHandle;
pub use scrape::{ScrapingOption, ScrapingProgress};
