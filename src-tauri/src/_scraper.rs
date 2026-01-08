use std::env;
use std::fs;
use std::path::PathBuf;

use std::time::Duration;

use csv::Writer;
use futures::stream::{self, StreamExt};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::header::COOKIE;
use serde::de;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Config を TOML から読み込むための構造体
#[derive(Deserialize, Debug)]
struct Config {
    cookies: Option<String>,
    output: Option<String>, // パス文字列
    scraping_interval_millis: u64,
    detailed_logging: bool,
    tags: Option<Vec<String>>,
    x_mode: Option<String>,
    s_mode: Option<String>,
    ai_type: Option<String>,
    scd: Option<String>,
    ecd: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            cookies: None,
            output: None,
            scraping_interval_millis: 1000,
            detailed_logging: false,
            tags: None,
            x_mode: None,
            s_mode: None,
            ai_type: None,
            scd: None,
            ecd: None,
        }
    }
}

fn load_config(path: &PathBuf) -> Config {
    match fs::read_to_string(path) {
        Ok(s) => match toml::from_str::<Config>(&s) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("config TOML のパースに失敗しました: {}", e);
                Config::default()
            }
        },
        Err(_) => {
            eprintln!("config TOML の読み込みに失敗しました");
            Config::default()
        }
    }
}

#[derive(Debug, Serialize)]
struct ItemRecord {
    id: u64,
    title: String,
    x_restrict: bool,
    tags: Vec<String>,
    create_date: String,
    ai_type: bool,
    width: u64,
    height: u64,
    bookmark_count: Option<u64>,
    view_count: Option<u64>,
}

async fn fetch_search_result(
    client: &reqwest::Client,
    cookie_header: &Option<String>,
    config: &Config,
) -> Result<Vec<ItemRecord>, Box<dyn std::error::Error>> {
    let mut res_vec = Vec::new();
    let mut page = 0;
    let mut last_page = 1;

    let tags = config.tags.as_ref().unwrap_or(&vec![]).join(" ");
    let url = format!(
        "https://www.pixiv.net/ajax/search/artworks/{}?word={}&order=date_d&mode={}&csw=0&s_mode={}&type=all&lang=ja&ai_type={}&scd={}&ecd={}",
        utf8_percent_encode(&tags, NON_ALPHANUMERIC),
        &tags,
        config.x_mode.as_deref().unwrap_or("all"),
        config.s_mode.as_deref().unwrap_or("s_tag"),
        config.ai_type.as_deref().unwrap_or("0"),
        config.scd.as_deref().unwrap_or(""),
        config.ecd.as_deref().unwrap_or(""),
    );

    while page <= last_page {
        page += 1;
        let paged_url = format!("{}&p={}", url, page);

        if page > 1 {
            tokio::time::sleep(Duration::from_millis(config.scraping_interval_millis)).await;
        }

        let mut req = client.get(&paged_url);
        if let Some(ch) = cookie_header {
            req = req.header(COOKIE, ch.clone());
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            return Err(format!("取得失敗: HTTP {}, url: {}", resp.status(), paged_url).into());
        }

        let body = resp.text().await?;
        let json: Value = serde_json::from_str(&body).map_err(|e| {
            eprintln!("レスポンスが JSON としてパースできません: {}", e);
            e
        })?;

        let res: Vec<ItemRecord> = json
            .get("body")
            .and_then(|v| v.get("illustManga"))
            .and_then(|v| v.get("data"))
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![])
            .iter()
            .map(|v| ItemRecord {
                id: v
                    .get("id")
                    .and_then(|id| id.as_str())
                    .and_then(|id| id.parse::<u64>().ok())
                    .unwrap_or(0),
                title: v
                    .get("title")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string(),
                x_restrict: v
                    .get("xRestrict")
                    .and_then(|xr| xr.as_i64())
                    .map(|n| n != 0)
                    .unwrap_or(false),
                tags: v
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
                    .collect::<Vec<String>>(),
                create_date: v
                    .get("createDate")
                    .and_then(|cd| cd.as_str())
                    .unwrap_or("")
                    .to_string(),
                ai_type: v
                    .get("aiType")
                    .and_then(|at| at.as_i64())
                    .map(|n| n != 1)
                    .unwrap_or(false),
                width: v.get("width").and_then(|w| w.as_u64()).unwrap_or(0),
                height: v.get("height").and_then(|h| h.as_u64()).unwrap_or(0),
                bookmark_count: None,
                view_count: None,
            })
            .collect();

        res_vec.extend(res);

        last_page = json
            .get("body")
            .and_then(|v| v.get("illustManga"))
            .and_then(|v| v.get("lastPage"))
            .and_then(|v| v.as_u64())
            .unwrap_or(last_page);

        if page == 1 {
            let total = json
                .get("body")
                .and_then(|v| v.get("illustManga"))
                .and_then(|v| v.get("total"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            println!("Total items found: {}", total);
        }

        println!("Fetched page {} / {}", page, last_page + 1);
    }

    Ok(res_vec)
}

fn extract_bookmark_count(body: &Value) -> Result<u64, Box<dyn std::error::Error>> {
    let res = body
        .get("bookmarkCount")
        .and_then(|v| v.as_u64())
        .ok_or("bookmarkCount フィールドが見つかりません")?;
    Ok(res)
}

fn extract_view_count(body: &Value) -> Result<u64, Box<dyn std::error::Error>> {
    let res = body
        .get("viewCount")
        .and_then(|v| v.as_u64())
        .ok_or("viewCount フィールドが見つかりません")?;
    Ok(res)
}

// 各ページにアクセスしてデータを収集するための関数

async fn fetch_illust_data(
    record: ItemRecord,
    client: &reqwest::Client,
    cookie_header: &Option<String>,
) -> Result<ItemRecord, Box<dyn std::error::Error>> {
    let url = format!("https://www.pixiv.net/ajax/illust/{}", &record.id);

    tokio::time::sleep(Duration::from_secs(1)).await;

    let mut req = client.get(&url);
    if let Some(ch) = cookie_header {
        req = req.header(COOKIE, ch.clone());
    }

    let resp = req.send().await?;
    if !resp.status().is_success() {
        return Err(format!("取得失敗: HTTP {}, url: {}", resp.status(), &url).into());
    }

    let body = resp.text().await?;
    let json: Value = serde_json::from_str(&body).map_err(|e| {
        eprintln!("レスポンスが JSON としてパースできません: {}", e);
        e
    })?;

    let is_error = json.get("error").and_then(|v| v.as_bool()).unwrap_or(false);
    if is_error {
        let message = json
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("不明なエラー");
        return Err(format!("エラーが発生しました: {}", message).into());
    }

    let body = json.get("body").ok_or("body フィールドがありません")?;

    let bookmark_count = extract_bookmark_count(body)?;
    let view_count = extract_view_count(body)?;

    let mut record = record;

    record.bookmark_count = Some(bookmark_count);
    record.view_count = Some(view_count);

    Ok(record)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // config.toml を読み込む（無ければ警告）
    let cfg_path = PathBuf::from("user/config.toml");
    let config = load_config(&cfg_path);

    // Cookie ヘッダを取得（優先順: 環境変数 PIXIV_COOKIES -> config）
    let cookie_header = env::var("PIXIV_COOKIES").ok().or(config.cookies.clone());

    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:144.0) Gecko/20100101 Firefox/144.0",
        )
        .build()?;

    println!("Using tags: {:?}", config.tags);
    println!("Using mode: {:?}", config.x_mode);
    println!("Using s_mode: {:?}", config.s_mode);
    println!("Using ai_type: {:?}", config.ai_type);
    println!("Using scd: {:?}", config.scd);
    println!("Using ecd: {:?}", config.ecd);
    println!("Starting scraping...");

    // 実際のスクレイピング

    let mut item_records = fetch_search_result(&client, &cookie_header, &config).await?;

    if config.detailed_logging {
        println!("Detailed logging is enabled.");
        println!(
            "Initial fetch complete. Total items to process: {}",
            item_records.len()
        );
        let now = std::time::Instant::now();
        let estimated_end = now + Duration::from_secs(item_records.len() as u64); // 1秒/件で計算
        println!("Estimated completion time: {:?}", estimated_end);

        let len = item_records.len();

        item_records = stream::iter(item_records)
            .enumerate()
            .map(|(index, item)| {
                println!(
                    "Fetching illust data for ID: {}, title: {}\t({}/{} ({}%)",
                    &item.id,
                    &item.title,
                    index + 1,
                    len,
                    (index + 1) * 100 / len
                );
                fetch_illust_data(item, &client, &cookie_header)
            })
            .buffer_unordered(1) // 1件ずつ処理
            .filter_map(|res| async { res.ok() })
            .collect()
            .await;
    }

    // 出力 CSV ヘッダ
    let mut rows: Vec<(
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    )> = Vec::new();
    for item in item_records {
        let tags_joined = item.tags.join(";");
        rows.push((
            item.id.to_string(),
            item.title,
            item.x_restrict.to_string(),
            tags_joined,
            item.ai_type.to_string(),
            item.create_date,
            item.width.to_string(),
            item.height.to_string(),
            item.bookmark_count.unwrap_or_default().to_string(),
            item.view_count.unwrap_or_default().to_string(),
        ));
    }

    // CSV 出力（config.output があればファイルへ、それ以外は stdout）
    if let Some(path) = config.output {
        let mut wtr = Writer::from_path(path)?;
        wtr.write_record([
            "id",
            "title",
            "xRestrict",
            "tags",
            "aiType",
            "createDate",
            "width",
            "height",
            "bookmarkCount",
            "viewCount",
        ])?;
        for (id, title, xr, tags, ai, cd, w, h, bc, vc) in rows {
            wtr.write_record([id, title, xr, tags, ai, cd, w, h, bc, vc])?;
        }
        wtr.flush()?;
    } else {
        let mut wtr = Writer::from_writer(std::io::stdout());
        wtr.write_record([
            "id",
            "title",
            "xRestrict",
            "tags",
            "bookmarkCount",
            "viewCount",
            "aiType",
        ])?;
        for (id, title, xr, tags, ai, cd, w, h, bc, vc) in rows {
            wtr.write_record([id, title, xr, tags, ai, cd, w, h, bc, vc])?;
        }
        wtr.flush()?;
    }

    // 分析をする

    Ok(())
}
