//! `types` モジュールは、スクレイピングで扱うデータ型と進捗管理を定義します。
//!
//! 実装の意図:
//! - API レスポンスを内部で扱いやすい `ItemRecord` に変換して処理の境界を明確にする。
//! - `ScrapingOption` に URL 生成やページ巡回等のラフスクレイピングに関するロジック（`fetch_rough`）を持たせ、
//!   その責務をこの型にまとめることで再利用性を高める。
//! - 進捗情報 (`ScrapingProgress`, `ScrapingStatus`) は Queue や UI と共有されるため、シンプルな形で定義する。
//!
//! 注意: HTTP の副作用は `api` モジュールに委譲できる部分もありますが、利便性のために `ScrapingOption::fetch_rough` に一部実装を残しています。

use std::sync::Arc;
use std::time::Duration;

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use reqwest::header::COOKIE;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::config::Config;

/// スクレイピングのオプションを表す構造体。
/// - `tags`、`search_mode`、期間（`scd`/`ecd`）を保持する
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrapingOption {
    pub tags: Vec<String>,
    pub search_mode: String,
    pub scd: String,
    pub ecd: String,
    pub detailed: bool,
}

// ----- レスポンス関連の型 -----

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct IllustData {
    id: String,
    title: String,
    x_restrict: i64,
    tags: Vec<String>,
    user_id: String,
    create_date: String,
    ai_type: i64,
    width: u64,
    height: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct IllustManga {
    data: Vec<ItemRecordOrAd>,
    total: u64,
    last_page: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResponseBody {
    illust_manga: IllustManga,
}

#[derive(Debug, Deserialize, Serialize)]
struct PixivSearchResponse {
    error: bool,
    message: Option<String>,
    body: ResponseBody,
}

/// 内部で使用する簡易的なアイテム表現（CSV出力やUIに渡すため）
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemRecord {
    pub id: u64,
    pub title: String,
    pub x_restrict: bool,
    pub tags: Vec<String>,
    pub user_id: u64,
    pub create_date: String,
    pub ai_type: bool,
    pub width: u64,
    pub height: u64,
    pub bookmark_count: Option<u64>,
    pub view_count: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdContainer {
    is_ad_container: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum ItemRecordOrAd {
    Ad(AdContainer),
    Item(IllustData),
}

impl From<IllustData> for ItemRecord {
    fn from(data: IllustData) -> Self {
        ItemRecord {
            id: data.id.parse().unwrap_or(0),
            title: data.title,
            x_restrict: data.x_restrict != 0,
            tags: data.tags,
            user_id: data.user_id.parse().unwrap_or(0),
            create_date: data.create_date,
            ai_type: data.ai_type != 1,
            width: data.width,
            height: data.height,
            bookmark_count: None,
            view_count: None,
        }
    }
}

// ----- 詳細 API のレスポンス型 -----

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IllustDetailBody {
    pub(crate) bookmark_count: u64,
    pub(crate) view_count: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct IllustDetailResponse {
    pub(crate) error: bool,
    pub(crate) message: String,
    pub(crate) body: Option<IllustDetailBody>,
}

// ----- 進捗関連の型 -----

/// スクレイピング進捗を表す構造体。Queue の状態取得等で使う。
#[derive(Debug, Serialize, Clone)]
pub struct ScrapingProgress {
    pub status: ScrapingStatus,
    pub total: Option<u64>,
    pub current: Option<u64>,
}

/// スクレイピングの状態（実行中か停止中か）
#[derive(Debug, Serialize, Clone, PartialEq)]
pub enum ScrapingStatus {
    Running,
    Stopped,
}

// ----- ScrapingOption の補助メソッド -----
impl ScrapingOption {
    /// このオプションに基いてスクレイピングを実行する。
    pub async fn fetch(
        &self,
        client: &reqwest::Client,
        progress: &Arc<Mutex<ScrapingProgress>>,
        cfg: &Config,
        token: &tokio_util::sync::CancellationToken,
    ) -> Result<Vec<ItemRecord>, Box<dyn std::error::Error + Send + Sync>> {
        let cookie_header = cfg.cookies.clone();

        let tags = self.tags.join(" ");
        let base_url = format!(
            "https://www.pixiv.net/ajax/search/artworks/{}",
            utf8_percent_encode(&tags, NON_ALPHANUMERIC)
        );

        // logging
        println!("Using tags: {:?}", self.tags);
        println!("Using search_mode: {:?}", self.search_mode);
        println!("Using scd: {:?}", self.scd);
        println!("Using ecd: {:?}", self.ecd);
        println!("Starting scraping...");

        // スクレイピング処理開始
        let mut curr_page = 1;
        let mut last_page = 1;
        let mut scraping_results: Vec<ItemRecord> = Vec::new();

        // 進捗初期化
        {
            let mut p = progress.lock().await;
            p.status = ScrapingStatus::Running;
            p.current = Some(0);
            p.total = Some(0);
        }

        while curr_page <= last_page {
            if token.is_cancelled() {
                println!("Rough scraping was cancelled.");
                return Ok(scraping_results);
            }

            tokio::time::sleep(Duration::from_millis(cfg.scraping_interval_millis)).await;

            let resp = client
                .get(&base_url)
                .query(&[
                    ("word", tags.as_str()),
                    ("order", "date_d"),
                    ("mode", "all"),
                    ("csw", "0"),
                    ("s_mode", self.search_mode.as_str()),
                    ("type", "all"),
                    ("lang", "ja"),
                    ("ai_type", "0"),
                    ("scd", self.scd.as_str()),
                    ("ecd", self.ecd.as_str()),
                    ("p", &curr_page.to_string()),
                ])
                .header(COOKIE, cookie_header.as_deref().unwrap_or_default())
                .send()
                .await?;

            if !resp.status().is_success() {
                return Err(format!("取得失敗: HTTP {}", resp.status()).into());
            }

            let search_response: PixivSearchResponse = serde_json::from_str(&resp.text().await?)?;

            if search_response.error {
                return Err(format!(
                    "APIエラーが発生しました: {}",
                    search_response.message.unwrap_or_default()
                )
                .into());
            }
            let search_response = search_response.body.illust_manga;

            let search_results: Vec<ItemRecord> = search_response
                .data
                .into_iter()
                .filter_map(|d| match d {
                    ItemRecordOrAd::Item(item) => Some(item),
                    ItemRecordOrAd::Ad(_) => None,
                })
                .map(|data| data.into())
                .collect();

            if curr_page == 1 {
                last_page = search_response.last_page;
                println!("Total items found: {}", search_response.total);
                progress.lock().await.total = Some(search_response.total);
            }
            scraping_results.extend(search_results);

            progress.lock().await.current = Some(curr_page);
            println!("Fetched page {} / {}", curr_page, last_page);

            curr_page += 1;
        }

        println!(
            "Scraping completed. All pages prefetched: {}",
            scraping_results.len()
        );
        Ok(scraping_results)
    }
}

// ----- テスト -----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_illust_data() {
        let json = r#"
{
  "error": false,
  "message": null,
  "body": {
    "illustManga": {
      "data": [
        {
          "id": "123456",
          "title": "Test Illust",
          "xRestrict": false,
          "tags": ["tag1", "tag2"],
          "userId": "78910",
          "createDate": "2023-01-01T00:00:00+00:00",
          "aiType": false,
          "width": 800,
          "height": 600,
          "bookmarkCount": null,
          "viewCount": null
        },
        { "isAdContainer": true }
      ],
      "total": 2,
      "lastPage": 1
    }
  }
}
        "#;
        let response: PixivSearchResponse = serde_json::from_str(json).unwrap();
        assert!(!response.error);
    }

    #[test]
    fn test_item_record_to_dummy_response() {
        let illust_data = IllustData {
            id: "123456".to_string(),
            title: "Test Illust".to_string(),
            x_restrict: 0,
            tags: vec!["tag1".to_string(), "tag2".to_string()],
            user_id: "78910".to_string(),
            create_date: "2023-01-01T00:00:00+00:00".to_string(),
            ai_type: 0,
            width: 800,
            height: 600,
        };
        let ad_container = AdContainer {
            is_ad_container: true,
        };
        let dummy_response = PixivSearchResponse {
            error: false,
            message: None,
            body: ResponseBody {
                illust_manga: IllustManga {
                    data: vec![
                        ItemRecordOrAd::Item(illust_data),
                        ItemRecordOrAd::Ad(ad_container),
                    ],
                    total: 2,
                    last_page: 1,
                },
            },
        };

        let json = serde_json::to_string(&dummy_response).unwrap();

        dbg!(json);
    }
}
