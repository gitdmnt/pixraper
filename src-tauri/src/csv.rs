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

/// bool フィールドのパースエラーを明示的なメッセージで返す純粋関数。
fn parse_bool_field(value: &str, field_name: &str, row_id: u64) -> Result<bool, String> {
    value.parse::<bool>().map_err(|_| {
        format!(
            "'{}' の値 '{}' を bool に変換できません（行ID: {}）",
            field_name, value, row_id
        )
    })
}

/// CSVファイルからアイテム一覧を読み込みます。
pub fn load_items(path: &str) -> Result<Vec<ItemRecord>, String> {
    let rdr = csv::Reader::from_path(path).map_err(|e| e.to_string())?;
    load_items_from_reader(rdr)
}

/// CSV リーダーからアイテム一覧を読み込む内部関数（テスト容易性のため抽出）。
fn load_items_from_reader<R: std::io::Read>(
    mut rdr: csv::Reader<R>,
) -> Result<Vec<ItemRecord>, String> {
    let mut items = Vec::new();

    // ヘッダーの手動マッピングなどを避けるため、各行をデシリアライズします。
    // ここでは一時的な構造体を定義して、CSVのヘッダー名 ("X Restrict" 等) と合わせます。
    #[derive(serde::Deserialize)]
    struct CsvRow {
        #[serde(rename = "Is Illust")]
        is_illust: String, // bool.to_string() -> "true"/"false"
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
            is_illust: parse_bool_field(&record.is_illust, "Is Illust", record.id)?,
            id: record.id,
            title: record.title,
            x_restrict: parse_bool_field(&record.x_restrict, "X Restrict", record.id)?,
            tags,
            user_id: record.user_id,
            create_date: record.create_date,
            ai_type: parse_bool_field(&record.ai_type, "AI Type", record.id)?,
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

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_CSV: &str = "\
Is Illust,ID,Title,X Restrict,Tags,User ID,Create Date,AI Type,Width,Height,Text Count,Word Count,Original,Bookmark Count,View Count\n\
true,1,Test,false,tag1;tag2,100,2024-01-01,false,1920,1080,,,false,,\n\
";

    const INVALID_BOOL_CSV: &str = "\
Is Illust,ID,Title,X Restrict,Tags,User ID,Create Date,AI Type,Width,Height,Text Count,Word Count,Original,Bookmark Count,View Count\n\
TRUE,1,Test,false,tag1,100,2024-01-01,false,,,,,false,,\n\
";

    const MISSING_IS_ILLUST_CSV: &str = "\
ID,Title,X Restrict,Tags,User ID,Create Date,AI Type,Width,Height,Text Count,Word Count,Original,Bookmark Count,View Count\n\
1,Test,false,tag1,100,2024-01-01,false,,,,,false,,\n\
";

    #[test]
    fn parse_bool_field_accepts_true_false() {
        assert_eq!(parse_bool_field("true", "F", 1).unwrap(), true);
        assert_eq!(parse_bool_field("false", "F", 1).unwrap(), false);
    }

    #[test]
    fn parse_bool_field_rejects_invalid_value() {
        assert!(parse_bool_field("TRUE", "F", 1).is_err());
        assert!(parse_bool_field("1", "F", 1).is_err());
        assert!(parse_bool_field("yes", "F", 1).is_err());
    }

    #[test]
    fn parse_bool_field_error_message_contains_field_name_and_value() {
        let err = parse_bool_field("BAD", "X Restrict", 42).unwrap_err();
        assert!(err.contains("X Restrict"), "error should contain field name");
        assert!(err.contains("BAD"), "error should contain value");
        assert!(err.contains("42"), "error should contain row ID");
    }

    #[test]
    fn load_items_valid_csv_parses_bool_fields_correctly() {
        let rdr = csv::Reader::from_reader(VALID_CSV.as_bytes());
        let items = load_items_from_reader(rdr).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].is_illust, true);
        assert_eq!(items[0].x_restrict, false);
        assert_eq!(items[0].ai_type, false);
    }

    #[test]
    fn load_items_invalid_bool_value_returns_error() {
        let rdr = csv::Reader::from_reader(INVALID_BOOL_CSV.as_bytes());
        assert!(load_items_from_reader(rdr).is_err());
    }

    #[test]
    fn load_items_missing_is_illust_column_returns_error() {
        let rdr = csv::Reader::from_reader(MISSING_IS_ILLUST_CSV.as_bytes());
        assert!(load_items_from_reader(rdr).is_err());
    }
}
