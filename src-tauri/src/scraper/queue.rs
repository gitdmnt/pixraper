//! `queue` モジュールは Actor パターンによるスクレイピングキューを実装します。
//!
//! 実装の意図:
//! - `QueryQueueActor` がコマンドを逐次処理して内部状態（キュー・進捗）を管理する。
//! - 外部は `QueryQueueHandle` を通じてコマンドを送り、複雑な同期処理を隠蔽する。
//! - `CancellationToken` を共有して実行中のキャンセルや停止を安全に行えるようにする。
//!
//! 注意: Actor は内部に `reqwest::Client` や `Config` を保持するため、長寿命であることを想定しています。

use std::sync::Arc;

use std::collections::VecDeque;
use tokio::sync::Mutex;

use crate::config::Config;
use crate::scraper::scrape::{ScrapingOption, ScrapingProgress, ScrapingStatus, Worker};

/// キュー操作用のコマンド列挙型。
///
/// 実装の意図:
/// - シンプルなメッセージパッシングで actor に指示を与える。将来的にコマンドを拡張しやすい。
enum Command {
    /// 新しいスクレイピングオプションをキューに追加する。
    Add(ScrapingOption),
    /// キューをクリアする。
    Clear,
    /// スクレイピングを開始する。
    Start(tokio_util::sync::CancellationToken),
    /// キューの次の要素を実行する（内部ループ用）。
    RunNext,
    /// 実行中のジョブを停止する（トークンをキャンセルする）。
    Stop,
    /// 進捗取得用の oneshot レスポンダを渡す。
    GetProgress(tokio::sync::oneshot::Sender<(usize, ScrapingProgress)>),
    /// キューの内容を取得する。
    GetQueue(tokio::sync::oneshot::Sender<Vec<ScrapingOption>>),
    /// キューから特定のインデックスの要素を削除する。
    Remove(usize),
}

/// Actor の実体。キューと進捗、HTTP クライアント、設定を保持する。
struct QueryQueueActor {
    queue: VecDeque<ScrapingOption>,
    progress: ScrapingProgress,
    client: reqwest::Client,
    cfg: Config,
    app_handle: tauri::AppHandle,
    scraping_token: Option<tokio_util::sync::CancellationToken>,
    receiver: tokio::sync::mpsc::Receiver<Command>,
    /// 自分自身への送信チャンネル（内部ループ用）
    sender: tokio::sync::mpsc::Sender<Command>,
}

impl QueryQueueActor {
    pub fn new(
        receiver: tokio::sync::mpsc::Receiver<Command>,
        sender: tokio::sync::mpsc::Sender<Command>,
        cfg: Config,
        app_handle: tauri::AppHandle,
    ) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:144.0) Gecko/20100101 Firefox/144.0",
            )
            .build()
            .unwrap();
        Self {
            queue: VecDeque::new(),
            progress: ScrapingProgress {
                status: ScrapingStatus::Stopped,
                total: None,
                current: None,
            },
            client,
            cfg,
            app_handle,
            scraping_token: None,
            receiver,
            sender,
        }
    }

    pub async fn run(&mut self) {
        while let Some(command) = self.receiver.recv().await {
            match command {
                Command::Add(option) => {
                    self.queue.push_back(option);
                }
                Command::Clear => {
                    self.queue.clear();
                }
                Command::Start(token) => {
                    // 既に実行中なら無視
                    if let Some(old) = &self.scraping_token {
                        if !old.is_cancelled() {
                            continue;
                        }
                    }
                    self.scraping_token = Some(token);

                    self.progress.status = ScrapingStatus::Running;
                    let _ = self.sender.send(Command::RunNext).await;
                }
                Command::RunNext => {
                    // トークンチェック
                    let token = match &self.scraping_token {
                        Some(t) if !t.is_cancelled() => t.clone(),
                        _ => {
                            // キャンセル済み、またはトークンなし停止
                            self.progress.status = ScrapingStatus::Stopped;
                            self.scraping_token = None;
                            continue;
                        }
                    };

                    if let Some(option) = self.queue.pop_front() {
                        // Workerに処理を委譲
                        Worker::run(
                            &option,
                            &self.client,
                            &Arc::new(Mutex::new(self.progress.clone())),
                            &self.cfg,
                            &token,
                            &self.app_handle,
                        )
                        .await
                        .unwrap_or_else(|e| {
                            println!("Error during scraping: {}", e);
                        });

                        // 次のループをスケジュール
                        // チャンネルが一杯でなければ送信
                        let _ = self.sender.send(Command::RunNext).await;
                    } else {
                        // キューが空
                        self.progress.status = ScrapingStatus::Stopped;
                        self.scraping_token = None;
                    }
                }
                Command::Stop => {
                    if let Some(token) = &self.scraping_token {
                        token.cancel();
                    }
                    self.progress.status = ScrapingStatus::Stopped;
                }
                Command::GetProgress(responder) => {
                    let _ = responder.send((self.queue.len(), self.progress.clone()));
                }
                Command::GetQueue(responder) => {
                    let _ = responder.send(self.queue.iter().cloned().collect());
                }
                Command::Remove(index) => {
                    if index < self.queue.len() {
                        self.queue.remove(index);
                    }
                }
            }
        }
    }
}

/// Queue への操作を簡潔に行うためのハンドル。
///
/// 実装の意図:
/// - 内部の `mpsc::Sender` にコマンドを流すことで、外部は同期的な複雑さを気にせずに利用できる。
#[derive(Clone)]
pub struct QueryQueueHandle {
    sender: tokio::sync::mpsc::Sender<Command>,
}
impl QueryQueueHandle {
    /// 新しいハンドルを作成します。内部で Actor を起動して受信ループを開始します。
    pub fn new(cfg: &Config, app_handle: tauri::AppHandle) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(100);
        let mut actor = QueryQueueActor::new(receiver, sender.clone(), cfg.clone(), app_handle);
        tauri::async_runtime::spawn(async move {
            actor.run().await;
        });
        Self { sender }
    }
    /// キューにオプションを追加します（非同期に enqueue されます）。
    pub async fn add(&self, option: ScrapingOption) {
        let _ = self.sender.send(Command::Add(option)).await;
    }
    /// キューをクリアします。
    pub async fn clear(&self) {
        let _ = self.sender.send(Command::Clear).await;
    }

    /// スクレイピングを開始します。
    pub async fn start(&self, token: tokio_util::sync::CancellationToken) {
        let _ = self.sender.send(Command::Start(token)).await;
    }

    /// 全体の停止を要求します（Actor 側でトークンをキャンセルする）。
    pub async fn stop(&self) {
        let _ = self.sender.send(Command::Stop).await;
    }

    /// 進捗を問い合わせて返します（キュー長と進捗）。
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

    /// キューの内容を取得して返します。
    pub async fn get_queue(&self) -> Vec<ScrapingOption> {
        let (responder, receiver) = tokio::sync::oneshot::channel();
        let _ = self.sender.send(Command::GetQueue(responder)).await;
        receiver.await.unwrap_or_else(|_| Vec::new())
    }

    /// 特定のインデックスの要素を削除します。
    pub async fn remove(&self, index: usize) {
        let _ = self.sender.send(Command::Remove(index)).await;
    }

    /// キューが空かどうかを判定します（ユーティリティ）。
    pub async fn is_empty(&self) -> bool {
        let (queue_length, _) = self.get_progress().await;
        queue_length == 0
    }
}
