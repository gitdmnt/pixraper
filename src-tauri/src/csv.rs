//! `csv` モジュールは、スクレイピング結果の CSV 出力ロジックをまとめています。
//!
//! 実装の意図:
//! - ファイル書き出しの責務をこのモジュールに閉じることで、将来的に別実装（テンポラリファイルを使った原子書き込み等）へ置き換えやすくしています。
//! - エラーは呼び出し元にわかりやすく伝搬する設計です。

use std::fs;
use tauri::Manager;

use crate::scraper::scrape::ItemRecord;

/// テストや実環境で差し替え可能な AppHandle の抽象。
pub trait AppHandleLike: Send + Sync {
    fn document_dir(&self) -> Option<std::path::PathBuf>;
}

impl AppHandleLike for tauri::AppHandle {
    fn document_dir(&self) -> Option<std::path::PathBuf> {
        self.path().document_dir().ok()
    }
}

/// アイテム一覧を CSV に保存します。
///
/// 実装の意図:
/// - まず出力先ディレクトリを作成し、その後 CSV ライターでヘッダを書き、各アイテムを追記します。
/// - 将来的には一時ファイルを書いてからリネームする等、原子性の向上を検討できます。
pub async fn save_as_csv(
    items: &[ItemRecord],
    app_handle: &dyn AppHandleLike,
) -> Result<(), String> {
    let now = chrono::Local::now();
    let filename = format!("result_{}.csv", now.format("%Y%m%d_%H%M%S"));
    let output_path = app_handle
        .document_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("Pixraper")
        .join(filename);

    println!("Saving results to {:?}", output_path);
    if fs::create_dir_all(
        output_path
            .parent()
            .map_or_else(|| Err("親ディレクトリの作成に失敗しました".to_string()), Ok)?,
    )
    .is_ok()
    {
        let mut wtr = csv::Writer::from_path(output_path).map_err(|e| e.to_string())?;

        wtr.write_record([
            "Is Illust",
            "ID",
            "Title",
            "X Restrict",
            "Tags",
            "User ID",
            "Create Date",
            "AI Type",
            "Width",
            "Height",
            "Text Count",
            "Word Count",
            "Original",
            "Bookmark Count",
            "View Count",
        ])
        .map_err(|e| e.to_string())?;

        for item in items {
            wtr.write_record(&[
                item.is_illust.to_string(),
                item.id.to_string(),
                item.title.clone(),
                item.x_restrict.to_string(),
                item.tags.join(";"),
                item.user_id.to_string(),
                item.create_date.clone(),
                item.ai_type.to_string(),
                item.width.map_or("".to_string(), |v| v.to_string()),
                item.height.map_or("".to_string(), |v| v.to_string()),
                item.text_count.map_or("".to_string(), |v| v.to_string()),
                item.word_count.map_or("".to_string(), |v| v.to_string()),
                item.is_original.map_or("".to_string(), |v| v.to_string()),
                item.bookmark_count
                    .map_or("".to_string(), |v| v.to_string()),
                item.view_count.map_or("".to_string(), |v| v.to_string()),
            ])
            .map_err(|e| e.to_string())?;
        }

        wtr.flush().map_err(|e| e.to_string())?;
    } else {
        return Err("出力先ディレクトリの作成に失敗しました".to_string());
    }
    Ok(())
}

/// CSVファイルからアイテム一覧を読み込みます。
pub fn load_items(path: &str) -> Result<Vec<ItemRecord>, String> {
    let mut rdr = csv::Reader::from_path(path).map_err(|e| e.to_string())?;
    let mut items = Vec::new();

    // ヘッダーの手動マッピングなどを避けるため、各行をデシリアライズします。
    // ここでは一時的な構造体を定義して、CSVのヘッダー名 ("X Restrict" 等) と合わせます。
    #[derive(serde::Deserialize)]
    struct CsvRow {
        #[serde(rename = "Is Illust")]
        is_illust: String, // bool.to_string() -> "true"/"false
        #[serde(rename = "ID")]
        id: u64,
        #[serde(rename = "Title")]
        title: String,
        #[serde(rename = "X Restrict")]
        x_restrict: String, // bool.to_string() -> "true"/"false"
        #[serde(rename = "Tags")]
        tags: String, // "tag1;tag2"
        #[serde(rename = "User ID")]
        user_id: u64,
        #[serde(rename = "Create Date")]
        create_date: String,
        #[serde(rename = "AI Type")]
        ai_type: String, // bool.to_string() -> "true"/"false"
        #[serde(rename = "Width")]
        width: Option<u64>,
        #[serde(rename = "Height")]
        height: Option<u64>,
        #[serde(rename = "Text Count")]
        text_count: Option<u64>,
        #[serde(rename = "Word Count")]
        word_count: Option<u64>,
        #[serde(rename = "Original")]
        is_original: Option<bool>,
        #[serde(rename = "Bookmark Count")]
        bookmark_count: Option<u64>,
        #[serde(rename = "View Count")]
        view_count: Option<u64>,
    }

    for result in rdr.deserialize() {
        let record: CsvRow = result.map_err(|e| e.to_string())?;

        let tags = if record.tags.is_empty() {
            Vec::new()
        } else {
            record.tags.split(';').map(|s| s.to_string()).collect()
        };

        items.push(ItemRecord {
            is_illust: record.is_illust.parse().unwrap_or(true),
            id: record.id,
            title: record.title,
            x_restrict: record.x_restrict.parse().unwrap_or(false),
            tags,
            user_id: record.user_id,
            create_date: record.create_date,
            ai_type: record.ai_type.parse().unwrap_or(false),
            width: record.width,
            height: record.height,
            text_count: record.text_count,
            word_count: record.word_count,
            is_original: record.is_original,
            bookmark_count: record.bookmark_count,
            view_count: record.view_count,
        });
    }

    Ok(items)
}
