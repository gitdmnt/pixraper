//! `scraper` モジュールはいくつかの責務に分割されています。
//! - `types`: レスポンスや内部データ構造、スクレイピングオプションや進捗型
//! - `api`: HTTP を用いた取得ロジックの公開関数（`fetch_search_result`, `fetch_illust_data`）
//! - `csv`: CSV 出力ユーティリティ（`save_as_csv`）
//! - `queue`: Actor パターンによるキュー実装（`QueryQueueHandle`）

pub mod api;
pub mod csv;
pub mod queue;
pub mod types;

// 外部でよく使うシンボルを再エクスポートして既存コードの影響を最小化する
pub use api::{fetch_illust_data, fetch_search_result};
pub use csv::save_as_csv;
pub use queue::QueryQueueHandle;
pub use types::{ScrapingOption, ScrapingProgress};
