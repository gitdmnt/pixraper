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
    pub(crate) illust_manga: IllustManga,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IllustManga {
    pub(crate) data: Vec<ItemRecordOrAd>,
    pub(crate) total: u64,
    pub(crate) last_page: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum ItemRecordOrAd {
    Ad(AdContainer),
    Item(IllustData),
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
        "https://www.pixiv.net/ajax/search/artworks/{}",
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

        let search_body = search_response.body.illust_manga;

        let search_results: Vec<ItemRecord> = search_body
            .data
            .into_iter()
            .filter_map(|d| match d {
                ItemRecordOrAd::Item(item) => Some(item.into()),
                ItemRecordOrAd::Ad(_) => None,
            })
            .collect();

        if curr_page == 1 {
            last_page = search_body.last_page;
            println!("Total items found: {}", search_body.total);
            progress.lock().await.total = Some(search_body.total);
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

/// イラスト詳細を取得する公開API
pub async fn fetch_illust_data(
    mut record: ItemRecord,
    client: &reqwest::Client,
    cookie_header: &Option<String>,
) -> Result<ItemRecord, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("https://www.pixiv.net/ajax/illust/{}", &record.id);

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let resp = client
        .get(&url)
        .header(COOKIE, cookie_header.as_deref().unwrap_or_default())
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(format!("取得失敗: HTTP {}, url: {}", resp.status(), &url).into());
    }

    // 型は types.rs 内で定義されている `IllustDetailResponse` を使ってパースする
    let illust_response: IllustDetailResponse = serde_json::from_str(&resp.text().await?)?;

    if illust_response.error {
        return Err(format!("APIエラーが発生しました: {}", illust_response.message).into());
    }

    if let Some(body) = illust_response.body {
        record.bookmark_count = Some(body.bookmark_count);
        record.view_count = Some(body.view_count);
    } else {
        return Err("レスポンスに 'body' フィールドが含まれていません".into());
    }
    Ok(record)
}
