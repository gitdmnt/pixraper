//! `api` モジュールは、HTTP を用いて Pixiv からデータを取得する公開 API を提供します。
//!
//! 実装の意図:
//! - `fetch_search_result` と `fetch_illust_data` は外部から呼び出すための薄いラッパーとして機能し、
//!   リクエスト送信・レスポンス検証・エラー正規化に責務を限定します。
//! - パーシングやデータ変換の詳細は `types` に任せ、ここは I/O とエラーの扱いに集中します。
//! - 将来的な拡張点: レート制御、リトライ、詳細なメトリクスの収集などをこの層で扱います。

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use reqwest::header::COOKIE;
use tokio::sync::Mutex;

fn random_interval(min: u64, max: u64) -> Duration {
    let effective_max = max.max(min);
    let millis = rand::thread_rng().gen_range(min..=effective_max);
    Duration::from_millis(millis)
}

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

/// 検索URLのベースを構築する純粋関数。
fn build_search_base_url(option: &ScrapingOption) -> (String, String) {
    let tags = option.tags.join(" ");
    let base_url = format!(
        "https://www.pixiv.net/ajax/search/{}/{}",
        if option.is_illust {
            "artworks"
        } else {
            "novels"
        },
        utf8_percent_encode(&tags, NON_ALPHANUMERIC)
    );
    (base_url, tags)
}

/// レスポンスから IllustMangaNovel を抽出する純粋関数。
fn extract_search_body(
    response: PixivSearchResponse,
    is_illust: bool,
) -> Result<IllustMangaNovel, Box<dyn std::error::Error + Send + Sync>> {
    if response.error {
        return Err(format!(
            "APIエラーが発生しました: {}",
            response.message.unwrap_or_default()
        )
        .into());
    }
    if is_illust {
        response
            .body
            .illust_manga
            .ok_or_else(|| "レスポンスに 'illustManga' フィールドが含まれていません".into())
    } else {
        response
            .body
            .novel
            .ok_or_else(|| "レスポンスに 'novel' フィールドが含まれていません".into())
    }
}

/// IllustMangaNovel のデータを ItemRecord のリストに変換する純粋関数。
fn to_item_records(data: Vec<IllustNovelRecordOrAd>) -> Vec<ItemRecord> {
    data.into_iter()
        .filter_map(|d| match d {
            IllustNovelRecordOrAd::Illust(item) => Some(item.into()),
            IllustNovelRecordOrAd::Novel(item) => Some(item.into()),
            IllustNovelRecordOrAd::Ad(_) => None,
        })
        .collect()
}

/// 1ページ分の検索結果を取得する内部関数。
/// - 1ページのリクエスト → レスポンス検証 → Vec<ItemRecord> + last_page を返す。
/// - インターバルスリープをここで行う。
/// - 進捗更新・キャンセルチェックは呼び出し元（Worker）に任せる。
pub(crate) async fn fetch_one_page(
    cfg: &Config,
    option: &ScrapingOption,
    client: &reqwest::Client,
    page: u64,
) -> Result<(Vec<ItemRecord>, u64), Box<dyn std::error::Error + Send + Sync>> {
    tokio::time::sleep(random_interval(
        cfg.scraping_interval_min_millis,
        cfg.scraping_interval_max_millis,
    ))
    .await;

    let (base_url, tags) = build_search_base_url(option);

    let resp = client
        .get(&base_url)
        .query(&[
            ("word", tags.as_str()),
            ("order", "date_d"),
            ("mode", "all"),
            ("csw", "0"),
            ("s_mode", option.search_mode.as_str()),
            ("type", "all"),
            ("lang", "ja"),
            ("ai_type", "0"),
            ("scd", option.scd.as_str()),
            ("ecd", option.ecd.as_str()),
            ("p", &page.to_string()),
        ])
        .header(COOKIE, cfg.cookies.as_deref().unwrap_or_default())
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(format!("取得失敗: HTTP {}", resp.status()).into());
    }

    let search_response: PixivSearchResponse = serde_json::from_str(&resp.text().await?)?;
    let search_body = extract_search_body(search_response, option.is_illust)?;

    let last_page = search_body.last_page;
    let items = to_item_records(search_body.data);

    Ok((items, last_page))
}

/// ラフ検索（ページ単位で複数ページを取得）を実行する公開API
/// - ネットワーク、ページ巡回、レスポンスの検証と ItemRecord への変換を担当します。
/// - 進捗更新 (`ScrapingProgress`) とキャンセル (`CancellationToken`) に対応します。
/// - `fetch_one_page` のループラッパーとして機能します（テスト互換性維持）。
#[allow(dead_code)]
pub async fn fetch_search_result(
    cfg: &Config,
    scraping_option: &ScrapingOption,
    client: &reqwest::Client,
    progress: &Arc<Mutex<ScrapingProgress>>,
    token: &tokio_util::sync::CancellationToken,
) -> Result<Vec<ItemRecord>, Box<dyn std::error::Error + Send + Sync>> {
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

        let (page_items, page_last_page) =
            fetch_one_page(cfg, scraping_option, client, curr_page).await?;

        if curr_page == 1 {
            last_page = page_last_page;
            println!("Total last_page: {}", last_page);
            progress.lock().await.total = Some(
                last_page
                    + if scraping_option.detailed {
                        // total items is approximated from last_page for now
                        0
                    } else {
                        0
                    },
            );
        }

        scraping_results.extend(page_items);

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
    interval_min_millis: u64,
    interval_max_millis: u64,
) -> Result<ItemRecord, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!(
        "https://www.pixiv.net/ajax/{}/{}",
        if record.is_illust { "illust" } else { "novel" },
        &record.id
    );

    tokio::time::sleep(random_interval(interval_min_millis, interval_max_millis)).await;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record() -> crate::scraper::scrape::ItemRecord {
        crate::scraper::scrape::ItemRecord {
            is_illust: true,
            id: 0,
            title: String::new(),
            x_restrict: false,
            tags: vec![],
            user_id: 0,
            create_date: String::new(),
            ai_type: false,
            width: None,
            height: None,
            text_count: None,
            word_count: None,
            is_original: None,
            bookmark_count: None,
            view_count: None,
        }
    }

    #[allow(dead_code)]
    fn _assert_fetch_detail_data_compiles() {
        let client = reqwest::Client::new();
        let record = make_record();
        let _ = fetch_detail_data(record, &client, &None, 1000u64, 2000u64);
    }

    #[test]
    fn test_random_interval_within_range() {
        for _ in 0..100 {
            let d = random_interval(1000, 2000);
            assert!(d >= Duration::from_millis(1000));
            assert!(d <= Duration::from_millis(2000));
        }
    }

    #[test]
    fn test_random_interval_min_equals_max() {
        let d = random_interval(1000, 1000);
        assert_eq!(d, Duration::from_millis(1000));
    }

    #[test]
    fn test_random_interval_min_greater_than_max() {
        let d = random_interval(2000, 1000);
        assert_eq!(d, Duration::from_millis(2000));
    }

    #[test]
    fn test_random_interval_zero() {
        // 0ms は許容する（無間隔スクレイピング）
        let d = random_interval(0, 0);
        assert_eq!(d, Duration::from_millis(0));
    }

    #[test]
    fn build_search_base_url_illust() {
        let option = crate::scraper::scrape::ScrapingOption {
            id: "1".to_string(),
            tags: vec!["夕焼け".to_string(), "空".to_string()],
            search_mode: "s_tag".to_string(),
            scd: "".to_string(),
            ecd: "".to_string(),
            detailed: false,
            is_illust: true,
        };
        let (url, tags) = build_search_base_url(&option);
        assert!(url.contains("artworks"), "イラスト検索URLにartworksが含まれること");
        assert_eq!(tags, "夕焼け 空");
    }

    #[test]
    fn build_search_base_url_novel() {
        let option = crate::scraper::scrape::ScrapingOption {
            id: "2".to_string(),
            tags: vec!["ファンタジー".to_string()],
            search_mode: "s_tag".to_string(),
            scd: "".to_string(),
            ecd: "".to_string(),
            detailed: false,
            is_illust: false,
        };
        let (url, _) = build_search_base_url(&option);
        assert!(url.contains("novels"), "小説検索URLにnovelsが含まれること");
    }

    #[test]
    fn to_item_records_filters_ads() {
        let data = vec![
            IllustNovelRecordOrAd::Ad(AdContainer {
                is_ad_container: true,
            }),
            IllustNovelRecordOrAd::Illust(IllustData {
                id: "1".to_string(),
                title: "test".to_string(),
                x_restrict: 0,
                tags: vec![],
                user_id: "100".to_string(),
                create_date: "2024-01-01".to_string(),
                ai_type: 1,
                width: 1920,
                height: 1080,
            }),
        ];
        let records = to_item_records(data);
        assert_eq!(records.len(), 1, "広告はフィルタされること");
        assert_eq!(records[0].id, 1);
    }
}
