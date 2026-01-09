//! `api` モジュールは、HTTP を用いて Pixiv からデータを取得する公開 API を提供します。
//!
//! 実装の意図:
//! - `fetch_search_result` と `fetch_illust_data` は外部から呼び出すための薄いラッパーとして機能し、
//!   リクエスト送信・レスポンス検証・エラー正規化に責務を限定します。
//! - パーシングやデータ変換の詳細は `types` に任せ、ここは I/O とエラーの扱いに集中します。
//! - 将来的な拡張点: レート制御、リトライ、詳細なメトリクスの収集などをこの層で扱います。

use std::sync::Arc;

use reqwest::header::COOKIE;
use tokio::sync::Mutex;

use crate::config::Config;
use crate::scraper::types::{ItemRecord, ScrapingOption, ScrapingProgress, ScrapingStatus};

/// ラフ検索（ページ単位で複数ページを取得）を実行する公開API
/// 実装の意図:
/// - 実際のページ巡回ロジックは `ScrapingOption::fetch_rough` に委譲して再利用する
pub async fn fetch_search_result(
    cfg: &Config,
    scraping_option: &ScrapingOption,
    token: &tokio_util::sync::CancellationToken,
) -> Result<Vec<ItemRecord>, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:144.0) Gecko/20100101 Firefox/144.0",
        )
        .build()?;

    // 進捗はここで一時的に作成して渡す（呼び出し側は別の状態管理を使う場合がある）
    let progress = Arc::new(Mutex::new(ScrapingProgress {
        status: ScrapingStatus::Stopped,
        total: None,
        current: None,
    }));

    scraping_option
        .fetch_rough(&client, &progress, cfg, token)
        .await
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
    let illust_response: crate::scraper::types::IllustDetailResponse =
        serde_json::from_str(&resp.text().await?)?;

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
