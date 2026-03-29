//! `queue_state` モジュールは、スクレイピングキューの永続化を担います。
//!
//! 実装の意図:
//! - キューを JSON ファイルとして app_config_dir に保存し、アプリ再起動後も復元できるようにします。
//! - ファイルが存在しない場合・読み込み失敗の場合は空のキューを返し、呼び出し元が graceful に扱えるようにします。

use std::collections::VecDeque;
use std::fs;

use crate::csv::AppHandleLike;
use crate::scraper::scrape::ScrapingOption;

const QUEUE_FILE: &str = "queue.json";

/// キューの保存先パスを生成する純粋関数。
fn queue_file_path(app_handle: &dyn AppHandleLike) -> Option<std::path::PathBuf> {
    app_handle
        .config_dir()
        .map(|dir| dir.join(QUEUE_FILE))
}

/// キューの状態を app_config_dir/queue.json に保存する。
pub fn save_queue(
    app_handle: &dyn AppHandleLike,
    queue: &VecDeque<ScrapingOption>,
) -> Result<(), String> {
    let path = queue_file_path(app_handle)
        .ok_or_else(|| "設定ディレクトリが取得できません".to_string())?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let json = serde_json::to_string(queue).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

/// queue.json からキューを読み込む（ファイルが存在しない場合は空を返す）。
pub fn load_queue(app_handle: &dyn AppHandleLike) -> Result<VecDeque<ScrapingOption>, String> {
    let path = queue_file_path(app_handle)
        .ok_or_else(|| "設定ディレクトリが取得できません".to_string())?;

    if !path.exists() {
        return Ok(VecDeque::new());
    }

    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct MockAppHandle {
        config_dir: PathBuf,
    }

    impl crate::csv::AppHandleLike for MockAppHandle {
        fn document_dir(&self) -> Option<PathBuf> {
            Some(self.config_dir.clone())
        }
        fn config_dir(&self) -> Option<PathBuf> {
            Some(self.config_dir.clone())
        }
    }

    fn make_option(id: &str) -> ScrapingOption {
        ScrapingOption {
            id: id.to_string(),
            tags: vec!["tag1".to_string()],
            search_mode: "s_tag".to_string(),
            scd: "2024-01-01".to_string(),
            ecd: "2024-12-31".to_string(),
            detailed: false,
            is_illust: true,
        }
    }

    #[test]
    fn save_and_load_queue_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let handle = MockAppHandle {
            config_dir: dir.path().to_path_buf(),
        };

        let mut queue = VecDeque::new();
        queue.push_back(make_option("opt-a"));
        queue.push_back(make_option("opt-b"));

        save_queue(&handle, &queue).unwrap();
        let loaded = load_queue(&handle).unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].id, "opt-a");
        assert_eq!(loaded[1].id, "opt-b");
    }

    #[test]
    fn load_queue_returns_empty_when_file_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let handle = MockAppHandle {
            config_dir: dir.path().to_path_buf(),
        };

        let loaded = load_queue(&handle).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn save_queue_overwrites_previous() {
        let dir = tempfile::tempdir().unwrap();
        let handle = MockAppHandle {
            config_dir: dir.path().to_path_buf(),
        };

        let mut queue1 = VecDeque::new();
        queue1.push_back(make_option("first"));
        save_queue(&handle, &queue1).unwrap();

        let mut queue2 = VecDeque::new();
        queue2.push_back(make_option("second"));
        save_queue(&handle, &queue2).unwrap();

        let loaded = load_queue(&handle).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "second");
    }
}
