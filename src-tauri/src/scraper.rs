use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use reqwest::header::COOKIE;
use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrapingOption {
    pub tags: Vec<String>,
    pub search_mode: String,
    pub scd: String,
    pub ecd: String,
}

// JSONレスポンスの 'data' 配列の要素に対応する構造体
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

// JSONレスポンスの 'illustManga' オブジェクトに対応する構造体
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct IllustManga {
    data: Vec<ItemRecordOrAd>,
    total: u64,
    last_page: u64,
}

// JSONレスポンスの 'body' オブジェクトに対応する構造体
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResponseBody {
    illust_manga: IllustManga,
}

// JSONレスポンス全体に対応するトップレベルの構造体
#[derive(Debug, Deserialize, Serialize)]
struct PixivSearchResponse {
    error: bool,
    message: Option<String>,
    body: ResponseBody,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemRecord {
    pub id: u64,
    pub title: String,
    x_restrict: bool,
    tags: Vec<String>,
    user_id: u64,
    create_date: String,
    ai_type: bool,
    width: u64,
    height: u64,
    bookmark_count: Option<u64>,
    view_count: Option<u64>,
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

pub async fn fetch_search_result(
    cfg: &Config,
    scraping_option: &ScrapingOption,
    token: &tokio_util::sync::CancellationToken,
) -> Result<Vec<ItemRecord>, Box<dyn std::error::Error + Send + Sync>> {
    let cookie_header = env::var("PIXIV_COOKIES").ok().or(cfg.cookies.clone());

    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:144.0) Gecko/20100101 Firefox/144.0",
        )
        .build()?;

    let tags = scraping_option.tags.join(" ");
    let base_url = format!(
        "https://www.pixiv.net/ajax/search/artworks/{}",
        utf8_percent_encode(&tags, NON_ALPHANUMERIC)
    );

    println!("Using tags: {:?}", scraping_option.tags);
    println!("Using search_mode: {:?}", scraping_option.search_mode);
    println!("Using scd: {:?}", scraping_option.scd);
    println!("Using ecd: {:?}", scraping_option.ecd);
    println!("Starting scraping...");

    let mut page = 0;
    let mut last_page = 1;
    let mut res_vec: Vec<ItemRecord> = Vec::new();

    while page <= last_page {
        // ループの開始時にキャンセルトークンをチェック
        if token.is_cancelled() {
            println!("Rough scraping was cancelled.");
            // 中断された場合は、それまでに収集したアイテムを返す
            return Ok(res_vec);
        }

        tokio::time::sleep(Duration::from_millis(cfg.scraping_interval_millis)).await;

        page += 1;

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
                ("p", &page.to_string()),
            ])
            .header(COOKIE, cookie_header.as_deref().unwrap_or_default())
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(format!("取得失敗: HTTP {}", resp.status()).into());
        }
        // return Err(format!("{:?}", &resp.text().await?).into());

        let search_response: PixivSearchResponse = serde_json::from_str(&resp.text().await?)?;

        if search_response.error {
            return Err(format!(
                "APIエラーが発生しました: {}",
                search_response.message.unwrap_or_default()
            )
            .into());
        }
        let illust_manga = search_response.body.illust_manga;

        let res: Vec<ItemRecord> = illust_manga
            .data
            .into_iter()
            .filter_map(|d| match d {
                ItemRecordOrAd::Item(item) => Some(item),
                ItemRecordOrAd::Ad(_) => None,
            })
            .map(|data| data.into())
            .collect();
        res_vec.extend(res);

        last_page = illust_manga.last_page;

        if page == 1 {
            println!("Total items found: {}", illust_manga.total);
        }

        println!("Fetched page {} / {}", page, last_page);
    }

    println!(
        "Scraping completed. All pages prefetched: {}",
        res_vec.len()
    );
    Ok(res_vec)
}

// イラスト詳細APIのレスポンスボディに対応する構造体
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IllustDetailBody {
    bookmark_count: u64,
    view_count: u64,
}

// イラスト詳細APIのトップレベルレスポンスに対応する構造体
#[derive(Debug, Deserialize)]
struct IllustDetailResponse {
    error: bool,
    message: String,
    body: Option<IllustDetailBody>,
}

pub async fn fetch_illust_data(
    mut record: ItemRecord,
    client: &reqwest::Client,
    cookie_header: &Option<String>,
) -> Result<ItemRecord, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("https://www.pixiv.net/ajax/illust/{}", &record.id);

    tokio::time::sleep(Duration::from_secs(1)).await;

    let resp = client
        .get(&url)
        .header(COOKIE, cookie_header.as_deref().unwrap_or_default())
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(format!("取得失敗: HTTP {}, url: {}", resp.status(), &url).into());
    }

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

pub async fn save_as_csv(items: &[ItemRecord], output_path: &PathBuf) -> Result<(), String> {
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

#[test]
fn test_fetch_illust_data() {
    // テスト用のセットアップ
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

// Actor Model パターンによるクエリキューの実装 (書き直し)

#[derive(Debug, Serialize, Clone)]
pub struct ScrapingProgress {
    pub status: ScrapingStatus,
    pub total: Option<u64>,
    pub current: Option<u64>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub enum ScrapingStatus {
    Running,
    Stopped,
}

impl ScrapingOption {
    pub async fn fetch_rough(
        &self,
        client: &reqwest::Client,
        progress: &Arc<Mutex<ScrapingProgress>>,
        cfg: &Config,
        token: &tokio_util::sync::CancellationToken,
    ) -> Result<Vec<ItemRecord>, Box<dyn std::error::Error + Send + Sync>> {
        // ここで作成したURL, クッキーを使ってスクレイピングを実行
        let tags = self.tags.join(" ");
        let base_url = format!(
            "https://www.pixiv.net/ajax/search/artworks/{}",
            utf8_percent_encode(&tags, NON_ALPHANUMERIC)
        );
        let cookie_header = cfg.cookies.clone();

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

        // スクレイピング進行状況の初期化
        {
            let mut p = progress.lock().await;
            p.status = ScrapingStatus::Running;
            p.current = Some(0);
            p.total = Some(0);
        }

        while curr_page <= last_page {
            // ループ開始時処理ここから
            // キャンセルトークンをチェック。指示があれば中断。
            if token.is_cancelled() {
                println!("Rough scraping was cancelled.");
                // 中断された場合は、それまでに収集したアイテムを返す
                return Ok(scraping_results);
            }

            // スクレイピング間隔の待機
            tokio::time::sleep(Duration::from_millis(cfg.scraping_interval_millis)).await;

            // ループ開始時処理ここまで

            // HTTPリクエストを送信
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

            // レスポンスをパース
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

            // ループ後処理ここから
            // 取得結果のマージ
            if curr_page == 1 {
                last_page = search_response.last_page;
                println!("Total items found: {}", search_response.total);
                progress.lock().await.total = Some(search_response.total);
            }
            scraping_results.extend(search_results);

            // 進行状況の更新
            progress.lock().await.current = Some(curr_page);
            println!("Fetched page {} / {}", curr_page, last_page);

            // ページ番号をインクリメント
            println!("Fetched page {} / {}", curr_page, last_page);

            // ページ番号をインクリメント
            curr_page += 1;

            // ループ後処理ここまで
        }

        println!(
            "Scraping completed. All pages prefetched: {}",
            scraping_results.len()
        );
        Ok(scraping_results)
    }
}

enum Command {
    Add(ScrapingOption),
    RunNext,
    Stop,
    SetToken(tokio_util::sync::CancellationToken),
    ClearToken,
    GetProgress(tokio::sync::oneshot::Sender<(usize, ScrapingProgress)>),
}

struct QueryQueueActor {
    queue: Vec<ScrapingOption>,
    progress: ScrapingProgress,
    client: reqwest::Client,
    cfg: Config,
    scraping_token: Option<tokio_util::sync::CancellationToken>,
    receiver: tokio::sync::mpsc::Receiver<Command>,
}

impl QueryQueueActor {
    pub fn new(receiver: tokio::sync::mpsc::Receiver<Command>, cfg: Config) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:144.0) Gecko/20100101 Firefox/144.0",
            )
            .build()
            .unwrap();
        Self {
            queue: Vec::new(),
            progress: ScrapingProgress {
                status: ScrapingStatus::Stopped,
                total: None,
                current: None,
            },
            client,
            cfg,
            scraping_token: None,
            receiver,
        }
    }
    pub async fn run(&mut self) {
        while let Some(command) = self.receiver.recv().await {
            match command {
                Command::Add(option) => {
                    self.queue.push(option);
                }
                Command::RunNext => {
                    // 重複実行防止
                    if self.progress.status == ScrapingStatus::Running {
                        continue;
                    }

                    if let Some(option) = self.queue.pop() {
                        // トークンがセットされていない場合は実行しない
                        let token = match &self.scraping_token {
                            Some(t) => t.clone(),
                            None => {
                                println!("No scraping token set, skipping RunNext");
                                continue;
                            }
                        };
                        option
                            .fetch_rough(
                                &self.client,
                                &Arc::new(Mutex::new(self.progress.clone())),
                                &self.cfg,
                                &token,
                            )
                            .await
                            .unwrap_or_else(|e| {
                                println!("Error during scraping: {}", e);
                                Vec::new()
                            });
                    }
                }
                Command::Stop => {
                    if let Some(token) = &self.scraping_token {
                        token.cancel();
                    }
                }
                Command::SetToken(token) => {
                    self.scraping_token = Some(token);
                }
                Command::ClearToken => {
                    self.scraping_token = None;
                }
                Command::GetProgress(responder) => {
                    let _ = responder.send((self.queue.len(), self.progress.clone()));
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct QueryQueueHandle {
    sender: tokio::sync::mpsc::Sender<Command>,
}
impl QueryQueueHandle {
    pub fn new(cfg: &Config) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(100);
        let mut actor = QueryQueueActor::new(receiver, cfg.clone());
        tauri::async_runtime::spawn(async move {
            actor.run().await;
        });
        Self { sender }
    }
    pub async fn add(&self, option: ScrapingOption) {
        let _ = self.sender.send(Command::Add(option)).await;
    }
    pub async fn run_next(&self) -> Result<(), String> {
        let response = self.sender.send(Command::RunNext).await;
        response.map_err(|e| e.to_string())
    }
    pub async fn stop(&self) {
        let _ = self.sender.send(Command::Stop).await;
    }
    pub async fn set_token(&self, token: tokio_util::sync::CancellationToken) {
        let _ = self.sender.send(Command::SetToken(token)).await;
    }
    pub async fn clear_token(&self) {
        let _ = self.sender.send(Command::ClearToken).await;
    }
    pub async fn get_progress(&self) -> (usize, ScrapingProgress) {
        let (responder, receiver) = tokio::sync::oneshot::channel();
        let _ = self.sender.send(Command::GetProgress(responder)).await;
        receiver.await.unwrap_or((
            0,
            ScrapingProgress {
                status: ScrapingStatus::Stopped,
                total: None,
                current: None,
            },
        ))
    }
    pub async fn is_empty(&self) -> bool {
        let (queue_length, _) = self.get_progress().await;
        queue_length == 0
    }
}
