//! `queue` モジュールは Actor パターンによるスクレイピングキューを実装します。
//!
//! 実装の意図:
//! - `QueryQueueActor` がコマンドを逐次処理して内部状態（キュー・進捗）を管理する。
//! - 外部は `QueryQueueHandle` を通じてコマンドを送り、複雑な同期処理を隠蔽する。
//! - `CancellationToken` を共有して実行中のキャンセルや停止を安全に行えるようにする。
//!
//! 注意: Actor は内部に `reqwest::Client` や `Config` を保持するため、長寿命であることを想定しています。

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::config::Config;
use crate::scraper::types::{ScrapingOption, ScrapingProgress, ScrapingStatus};

/// キュー操作用のコマンド列挙型。
///
/// 実装の意図:
/// - シンプルなメッセージパッシングで actor に指示を与える。将来的にコマンドを拡張しやすい。
enum Command {
    /// 新しいスクレイピングオプションをキューに追加する。
    Add(ScrapingOption),
    /// キューをクリアする。
    Clear,
    /// キューの次の要素を実行する。
    RunNext,
    /// 実行中のジョブを停止する（トークンをキャンセルする）。
    Stop,
    /// Actor 側に `CancellationToken` をセットする。
    SetToken(tokio_util::sync::CancellationToken),
    /// Actor 側のトークンをクリアする（実行終了後に呼ぶ）。
    ClearToken,
    /// 進捗取得用の oneshot レスポンダを渡す。
    GetProgress(tokio::sync::oneshot::Sender<(usize, ScrapingProgress)>),
}

/// Actor の実体。キューと進捗、HTTP クライアント、設定を保持する。
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
                Command::Clear => {
                    self.queue.clear();
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
    pub fn new(cfg: &Config) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(100);
        let mut actor = QueryQueueActor::new(receiver, cfg.clone());
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

    /// キューの次を実行する合図を送ります。
    ///
    /// 実装の意図: Actor 側で `RunNext` を受け取ると内部で単体実行を行います。
    pub async fn run_next(&self) -> Result<(), String> {
        let response = self.sender.send(Command::RunNext).await;
        response.map_err(|e| e.to_string())
    }
    /// 全体の停止を要求します（Actor 側でトークンをキャンセルする）。
    pub async fn stop(&self) {
        let _ = self.sender.send(Command::Stop).await;
    }
    /// Actor 側にキャンセレーショントークンをセットします。
    pub async fn set_token(&self, token: tokio_util::sync::CancellationToken) {
        let _ = self.sender.send(Command::SetToken(token)).await;
    }
    /// Actor 側のトークンをクリアします。
    pub async fn clear_token(&self) {
        let _ = self.sender.send(Command::ClearToken).await;
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
    /// キューが空かどうかを判定します（ユーティリティ）。
    pub async fn is_empty(&self) -> bool {
        let (queue_length, _) = self.get_progress().await;
        queue_length == 0
    }
}
