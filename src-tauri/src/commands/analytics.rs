//! 分析系コマンド（現状は簡易実装）
//!
//! 実装の意図:
//! - 分析は将来的に独立した処理やワーカーに切り出す可能性があるため、
//!   現状はフロントエンドからの呼び出し用の薄いラッパーを用意しています。

use crate::csv::load_items;
use crate::scraper::scrape::ItemRecord;

#[tauri::command]
pub fn show_analytics(data: &str) -> String {
    // TODO: 実際の解析ロジックは将来追加する。現状は呼び出しの有無を確認するためのスタブ。
    format!("Started analysis on data: {}", data)
}

/// CSVファイルを読み込んでデータを返します。
#[tauri::command]
pub async fn load_dataset(path: String) -> Result<Vec<ItemRecord>, String> {
    load_items(&path)
}
