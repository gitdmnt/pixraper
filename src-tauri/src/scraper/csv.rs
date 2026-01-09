//! `csv` モジュールは、スクレイピング結果の CSV 出力ロジックをまとめています。
//!
//! 実装の意図:
//! - ファイル書き出しの責務をこのモジュールに閉じることで、将来的に別実装（テンポラリファイルを使った原子書き込み等）へ置き換えやすくしています。
//! - エラーは呼び出し元にわかりやすく伝搬する設計です。

use std::fs;
use tauri::Manager;

use crate::scraper::types::ItemRecord;

/// アイテム一覧を CSV に保存します。
///
/// 実装の意図:
/// - まず出力先ディレクトリを作成し、その後 CSV ライターでヘッダを書き、各アイテムを追記します。
/// - 将来的には一時ファイルを書いてからリネームする等、原子性の向上を検討できます。
pub async fn save_as_csv(
    items: &[ItemRecord],
    app_handle: &tauri::AppHandle,
) -> Result<(), String> {
    let now = chrono::Local::now();
    let filename = format!("result_{}.csv", now.format("%Y%m%d_%H%M%S"));
    let output_path = app_handle
        .path()
        .document_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
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
            "ID",
            "Title",
            "X Restrict",
            "Tags",
            "User ID",
            "Create Date",
            "AI Type",
            "Width",
            "Height",
            "Bookmark Count",
            "View Count",
        ])
        .map_err(|e| e.to_string())?;

        for item in items {
            wtr.write_record(&[
                item.id.to_string(),
                item.title.clone(),
                item.x_restrict.to_string(),
                item.tags.join(";"),
                item.user_id.to_string(),
                item.create_date.clone(),
                item.ai_type.to_string(),
                item.width.to_string(),
                item.height.to_string(),
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
