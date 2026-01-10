//! `api` モジュールは、HTTP を用いて Pixiv からデータを取得する公開 API を提供します。
//!
//! 実装の意図:
//! - `fetch_search_result` と `fetch_illust_data` は外部から呼び出すための薄いラッパーとして機能し、
//!   リクエスト送信・レスポンス検証・エラー正規化に責務を限定します。
//! - パーシングやデータ変換の詳細は `types` に任せ、ここは I/O とエラーの扱いに集中します。
//! - 将来的な拡張点: レート制御、リトライ、詳細なメトリクスの収集などをこの層で扱います。

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use reqwest::header::COOKIE;
use tokio::sync::Mutex;

use crate::config::Config;
use crate::scraper::scrape::{ItemRecord, ScrapingOption, ScrapingProgress, ScrapingStatus};

// ----- 検索 API のレスポンス型 -----

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PixivSearchResponse {
    pub(crate) error: bool,
    pub(crate) message: Option<String>,
    pub(crate) body: ResponseBody,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResponseBody {
    pub(crate) illust_manga: Option<IllustMangaNovel>,
    pub(crate) novel: Option<IllustMangaNovel>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IllustMangaNovel {
    pub(crate) data: Vec<IllustNovelRecordOrAd>,
    pub(crate) total: u64,
    pub(crate) last_page: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum IllustNovelRecordOrAd {
    Ad(AdContainer),
    Illust(IllustData),
    Novel(NovelData),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdContainer {
    pub(crate) is_ad_container: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IllustData {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) x_restrict: i64,
    pub(crate) tags: Vec<String>,
    pub(crate) user_id: String,
    pub(crate) create_date: String,
    pub(crate) ai_type: i64,
    pub(crate) width: u64,
    pub(crate) height: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NovelData {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) x_restrict: i64,
    pub(crate) tags: Vec<String>,
    pub(crate) user_id: String,
    pub(crate) text_count: u64,
    pub(crate) word_count: u64,
    pub(crate) is_original: bool,
    pub(crate) create_date: String,
    pub(crate) ai_type: i64,
}

// ----- 詳細 API のレスポンス型 -----

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IllustDetailResponse {
    pub(crate) error: bool,
    pub(crate) message: String,
    pub(crate) body: Option<IllustDetailBody>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IllustDetailBody {
    pub(crate) bookmark_count: u64,
    pub(crate) view_count: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NovelDetailResponse {
    pub(crate) error: bool,
    pub(crate) message: String,
    pub(crate) body: Option<NovelDetailBody>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NovelDetailBody {
    pub(crate) bookmark_count: u64,
    pub(crate) view_count: u64,
}

/// ラフ検索（ページ単位で複数ページを取得）を実行する公開API
/// - ネットワーク、ページ巡回、レスポンスの検証と ItemRecord への変換を担当します。
/// - 進捗更新 (`ScrapingProgress`) とキャンセル (`CancellationToken`) に対応します。
pub async fn fetch_search_result(
    cfg: &Config,
    scraping_option: &ScrapingOption,
    client: &reqwest::Client,
    progress: &Arc<Mutex<ScrapingProgress>>,
    token: &tokio_util::sync::CancellationToken,
) -> Result<Vec<ItemRecord>, Box<dyn std::error::Error + Send + Sync>> {
    let cookie_header = cfg.cookies.clone();

    let tags = scraping_option.tags.join(" ");
    let base_url = format!(
        "https://www.pixiv.net/ajax/search/{}/{}",
        if scraping_option.is_illust {
            "artworks"
        } else {
            "novels"
        },
        utf8_percent_encode(&tags, NON_ALPHANUMERIC)
    );

    let mut curr_page: u64 = 1;
    let mut last_page: u64 = 1;
    let mut scraping_results: Vec<ItemRecord> = Vec::new();

    // 初期進捗
    {
        let mut p = progress.lock().await;
        p.status = ScrapingStatus::Running;
        p.current = Some(0);
        p.total = Some(0);
    }

    while curr_page <= last_page {
        if token.is_cancelled() {
            println!("Scraping was cancelled.");
            break;
        }

        tokio::time::sleep(Duration::from_millis(cfg.scraping_interval_millis)).await;

        let resp = client
            .get(&base_url)
            .query(&[
                ("word", tags.as_str()),
                ("order", "date_d"),
                ("mode", "all"),
                ("csw", "0"),
                ("s_mode", scraping_option.search_mode.as_str()),
                ("type", "all"),
                ("lang", "ja"),
                ("ai_type", "0"),
                ("scd", scraping_option.scd.as_str()),
                ("ecd", scraping_option.ecd.as_str()),
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

        let search_body = if scraping_option.is_illust {
            search_response
                .body
                .illust_manga
                .ok_or("レスポンスに 'illustManga' フィールドが含まれていません")?
        } else {
            search_response
                .body
                .novel
                .ok_or("レスポンスに 'novel' フィールドが含まれていません")?
        };

        let search_results: Vec<ItemRecord> = search_body
            .data
            .into_iter()
            .filter_map(|d| match d {
                IllustNovelRecordOrAd::Illust(item) => Some(item.into()),
                IllustNovelRecordOrAd::Novel(item) => Some(item.into()),
                IllustNovelRecordOrAd::Ad(_) => None,
            })
            .collect();

        if curr_page == 1 {
            last_page = search_body.last_page;
            println!("Total items found: {}", search_body.total);
            progress.lock().await.total = Some(
                last_page
                    + if scraping_option.detailed {
                        search_body.total
                    } else {
                        0
                    },
            );
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

// レスポンス共通処理を抽象化するトレイト
trait DetailResponse {
    fn is_error(&self) -> bool;
    fn message(&self) -> &str;
    fn body_counts(&self) -> Option<(u64, u64)>;
}

impl DetailResponse for IllustDetailResponse {
    fn is_error(&self) -> bool {
        self.error
    }
    fn message(&self) -> &str {
        &self.message
    }
    fn body_counts(&self) -> Option<(u64, u64)> {
        self.body.as_ref().map(|b| (b.bookmark_count, b.view_count))
    }
}

impl DetailResponse for NovelDetailResponse {
    fn is_error(&self) -> bool {
        self.error
    }
    fn message(&self) -> &str {
        &self.message
    }
    fn body_counts(&self) -> Option<(u64, u64)> {
        self.body.as_ref().map(|b| (b.bookmark_count, b.view_count))
    }
}

fn apply_detail<R: DetailResponse>(
    record: &mut ItemRecord,
    resp: &R,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if resp.is_error() {
        return Err(format!("APIエラーが発生しました: {}", resp.message()).into());
    }
    if let Some((b, v)) = resp.body_counts() {
        record.bookmark_count = Some(b);
        record.view_count = Some(v);
        Ok(())
    } else {
        Err("レスポンスに 'body' フィールドが含まれていません".into())
    }
}

/// イラスト詳細を取得する公開API
pub async fn fetch_detail_data(
    mut record: ItemRecord,
    client: &reqwest::Client,
    cookie_header: &Option<String>,
) -> Result<ItemRecord, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!(
        "https://www.pixiv.net/ajax/{}/{}",
        if record.is_illust { "illust" } else { "novel" },
        &record.id
    );

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let resp = client
        .get(&url)
        .header(COOKIE, cookie_header.as_deref().unwrap_or_default())
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(format!("取得失敗: HTTP {}, url: {}", resp.status(), &url).into());
    }

    // テキストを一度だけ取得してパースする
    let text = resp.text().await?;

    if record.is_illust {
        let illust_response: IllustDetailResponse = serde_json::from_str(&text)?;
        apply_detail(&mut record, &illust_response)?;
    } else {
        let novel_response: NovelDetailResponse = serde_json::from_str(&text)?;
        apply_detail(&mut record, &novel_response)?;
    }

    Ok(record)
}
