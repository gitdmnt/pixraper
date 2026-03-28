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
use crate::csv::AppHandleLike;
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
    /// キューから特定の ID の要素を削除する。
    Remove(String),
    /// Workerの処理が完了したことを通知する。
    WorkerFinished,
    /// Worker が動作中かどうかを問い合わせる（主にテスト用）
    IsWorkerRunning(tokio::sync::oneshot::Sender<bool>),
    /// 現在のプロファイルインデックスを取得する（主にテスト用）
    GetProfileIndex(tokio::sync::oneshot::Sender<usize>),
}

/// Actor の実体。キューと進捗、HTTP クライアント、設定を保持する。
struct QueryQueueActor {
    queue: VecDeque<ScrapingOption>,
    progress: Arc<Mutex<ScrapingProgress>>,
    client: reqwest::Client,
    cfg: Config,
    app_handle: Arc<dyn AppHandleLike>,
    scraping_token: Option<tokio_util::sync::CancellationToken>,
    /// 現在Workerが実行中かどうかを管理するフラグ
    worker_running: bool,
    receiver: tokio::sync::mpsc::Receiver<Command>,
    /// 自分自身への送信チャンネル（内部ループ用）
    sender: tokio::sync::mpsc::Sender<Command>,
    /// テストやオプションで同期実行／疑似実行するためのフラグ
    spawn_workers_async: bool,
    /// テスト用: Worker の実行を模擬する（実際の処理を行わず完了通知だけ送る）
    simulate_workers: bool,
    /// cookie_profiles のローテーション用インデックス
    profile_index: usize,
}

impl QueryQueueActor {
    pub fn new(
        receiver: tokio::sync::mpsc::Receiver<Command>,
        sender: tokio::sync::mpsc::Sender<Command>,
        cfg: Config,
        app_handle: tauri::AppHandle,
    ) -> Self {
        // デフォルトは非テスト用（非同期 spawn を行う）
        let client = reqwest::Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:144.0) Gecko/20100101 Firefox/144.0",
            )
            .build()
            .unwrap();
        let app_handle_arc: Arc<dyn AppHandleLike> = Arc::new(app_handle);
        Self::new_with_client_and_apphandle(
            receiver,
            sender,
            cfg,
            app_handle_arc,
            client,
            true,
            false,
        )
    }

    /// テストやカスタム設定用に Client / AppHandle / 実行モードを注入できるコンストラクタ
    pub fn new_with_client_and_apphandle(
        receiver: tokio::sync::mpsc::Receiver<Command>,
        sender: tokio::sync::mpsc::Sender<Command>,
        cfg: Config,
        app_handle: Arc<dyn AppHandleLike>,
        client: reqwest::Client,
        spawn_workers_async: bool,
        simulate_workers: bool,
    ) -> Self {
        Self {
            queue: VecDeque::new(),
            progress: Arc::new(Mutex::new(ScrapingProgress {
                status: ScrapingStatus::Stopped,
                total: None,
                current: None,
            })),
            client,
            cfg,
            app_handle,
            scraping_token: None,
            worker_running: false,
            receiver,
            sender,
            spawn_workers_async,
            simulate_workers,
            profile_index: 0,
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

                    {
                        let mut p = self.progress.lock().await;
                        p.status = ScrapingStatus::Running;
                    }
                    let _ = self.sender.send(Command::RunNext).await;
                }
                Command::RunNext => {
                    // 実行中なら多重起動を防止するために何もしない
                    if self.worker_running {
                        continue;
                    }

                    // トークンチェック
                    let token = match &self.scraping_token {
                        Some(t) if !t.is_cancelled() => t.clone(),
                        _ => {
                            // キャンセル済み、またはトークンなし停止
                            let mut p = self.progress.lock().await;
                            p.status = ScrapingStatus::Stopped;
                            self.scraping_token = None;
                            continue;
                        }
                    };

                    // 無事起動した場合
                    if let Some(option) = self.queue.pop_front() {
                        let client = self.client.clone();
                        let progress = self.progress.clone(); // Arc<Mutex>

                        // cookie_profiles が存在する場合はローテーションで使用し、
                        // 空の場合は Config.cookies にフォールバックする。
                        let mut cfg = self.cfg.clone();
                        if !cfg.cookie_profiles.is_empty() {
                            let idx = self.profile_index % cfg.cookie_profiles.len();
                            cfg.cookies = Some(cfg.cookie_profiles[idx].cookies.clone());
                            self.profile_index += 1;
                        }

                        let app_handle = self.app_handle.clone();
                        let sender = self.sender.clone(); // Self sender

                        // フラグを立てて実行中にする
                        self.worker_running = true;

                        // テストモード: Worker を模擬してすぐ完了通知を送る
                        if self.simulate_workers {
                            let sender_clone = sender.clone();
                            tokio::spawn(async move {
                                // 即時完了を模擬
                                let _ = sender_clone.send(Command::WorkerFinished).await;
                            });
                            continue;
                        }

                        // Workerに処理を委譲 (Spawnして非同期実行) または同期実行
                        if self.spawn_workers_async {
                            tokio::spawn(async move {
                                Worker::run(
                                    &option,
                                    &client,
                                    &progress,
                                    &cfg,
                                    &token,
                                    &*app_handle,
                                )
                                .await
                                .unwrap_or_else(|e| {
                                    println!("Error during scraping: {}", e);
                                });

                                // 完了通知を送る (RunNextではなくWorkerFinished)
                                let _ = sender.send(Command::WorkerFinished).await;
                            });
                        } else {
                            // 同期実行 (テストでの確認や制御がしやすい)
                            Worker::run(&option, &client, &progress, &cfg, &token, &*app_handle)
                                .await
                                .unwrap_or_else(|e| {
                                    println!("Error during scraping: {}", e);
                                });

                            let _ = self.sender.send(Command::WorkerFinished).await;
                        }
                    } else {
                        // キューが空
                        let mut p = self.progress.lock().await;
                        p.status = ScrapingStatus::Stopped;
                        self.scraping_token = None;
                    }
                }
                Command::WorkerFinished => {
                    self.worker_running = false;
                    // 次があれば実行する
                    let _ = self.sender.send(Command::RunNext).await;
                }
                Command::Stop => {
                    println!("Stopping scraping process...");
                    if let Some(token) = &self.scraping_token {
                        token.cancel();
                    }
                    let mut p = self.progress.lock().await;
                    p.status = ScrapingStatus::Stopped;
                }
                Command::IsWorkerRunning(responder) => {
                    let _ = responder.send(self.worker_running);
                }
                Command::GetProfileIndex(responder) => {
                    let _ = responder.send(self.profile_index);
                }
                Command::GetProgress(responder) => {
                    let p = self.progress.lock().await;
                    let _ = responder.send((self.queue.len(), p.clone()));
                }
                Command::GetQueue(responder) => {
                    let _ = responder.send(self.queue.iter().cloned().collect());
                }
                Command::Remove(id) => {
                    self.queue.retain(|opt| opt.id != id);
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

    /// テスト用 / カスタム用コンストラクタ。AppHandle は trait オブジェクトで渡す。
    pub fn new_with_client_and_app(
        cfg: &Config,
        app_handle: Arc<dyn AppHandleLike>,
        client: reqwest::Client,
        spawn_workers_async: bool,
        simulate_workers: bool,
    ) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(100);
        let mut actor = QueryQueueActor::new_with_client_and_apphandle(
            receiver,
            sender.clone(),
            cfg.clone(),
            app_handle,
            client,
            spawn_workers_async,
            simulate_workers,
        );
        // Use tokio runtime for tests to control lifetime (tests use tokio::test)
        tokio::spawn(async move {
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

    /// テスト用: Worker が動作中かどうかを問い合わせます。
    pub async fn is_worker_running(&self) -> bool {
        let (responder, receiver) = tokio::sync::oneshot::channel();
        let _ = self.sender.send(Command::IsWorkerRunning(responder)).await;
        receiver.await.unwrap_or(false)
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

    /// 特定の ID の要素を削除します。
    pub async fn remove_by_id(&self, id: String) {
        let _ = self.sender.send(Command::Remove(id)).await;
    }

    /// キューが空かどうかを判定します（ユーティリティ）。
    pub async fn is_empty(&self) -> bool {
        let (queue_length, _) = self.get_progress().await;
        queue_length == 0
    }

    /// 現在のプロファイルインデックスを取得します（テスト用）。
    pub async fn get_profile_index(&self) -> usize {
        let (responder, receiver) = tokio::sync::oneshot::channel();
        let _ = self
            .sender
            .send(Command::GetProfileIndex(responder))
            .await;
        receiver.await.unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::sync::Arc;
    use tokio::time::{timeout, Duration};

    struct DummyAppHandle {
        base: std::path::PathBuf,
    }

    impl DummyAppHandle {
        fn new(base: std::path::PathBuf) -> Self {
            Self { base }
        }
    }

    impl AppHandleLike for DummyAppHandle {
        fn document_dir(&self) -> Option<std::path::PathBuf> {
            Some(self.base.clone())
        }
    }

    fn make_option_with_id(id: &str) -> ScrapingOption {
        ScrapingOption {
            id: id.to_string(),
            tags: vec!["test".into()],
            search_mode: "mode".into(),
            scd: "".into(),
            ecd: "".into(),
            detailed: false,
            is_illust: true,
        }
    }

    fn make_option() -> ScrapingOption {
        make_option_with_id("default-id")
    }

    #[tokio::test]
    async fn remove_by_id_removes_correct_item() {
        let cfg = Config::default();
        let temp = std::env::temp_dir().join("pixraper_test_remove");
        let app = Arc::new(DummyAppHandle::new(temp));
        let client = reqwest::Client::builder().build().unwrap();
        let handle = QueryQueueHandle::new_with_client_and_app(&cfg, app, client, true, true);

        handle.add(make_option_with_id("a")).await;
        handle.add(make_option_with_id("b")).await;
        handle.add(make_option_with_id("c")).await;

        handle.remove_by_id("b".to_string()).await;

        // allow actor to process
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let queue = handle.get_queue().await;
        assert_eq!(queue.len(), 2);
        assert!(queue.iter().any(|o| o.id == "a"));
        assert!(queue.iter().any(|o| o.id == "c"));
        assert!(!queue.iter().any(|o| o.id == "b"), "b should be removed");
    }

    #[tokio::test]
    async fn remove_by_id_nonexistent_id_is_noop() {
        let cfg = Config::default();
        let temp = std::env::temp_dir().join("pixraper_test_remove_noop");
        let app = Arc::new(DummyAppHandle::new(temp));
        let client = reqwest::Client::builder().build().unwrap();
        let handle = QueryQueueHandle::new_with_client_and_app(&cfg, app, client, true, true);

        handle.add(make_option_with_id("a")).await;
        handle.add(make_option_with_id("b")).await;

        handle.remove_by_id("nonexistent".to_string()).await;

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let queue = handle.get_queue().await;
        assert_eq!(queue.len(), 2);
    }

    #[tokio::test]
    async fn simulate_queue_processing_completes_all_items() {
        let cfg = Config::default();
        let temp = std::env::temp_dir().join("pixraper_test");
        let app = Arc::new(DummyAppHandle::new(temp));
        let client = reqwest::Client::builder().build().unwrap();

        // simulate_workers=true で実際の Worker::run を実行せず、完了通知だけを送る
        let handle = QueryQueueHandle::new_with_client_and_app(&cfg, app, client, true, true);

        handle.add(make_option()).await;
        handle.add(make_option()).await;

        let token = tokio_util::sync::CancellationToken::new();
        handle.start(token.clone()).await;

        // ある程度の猶予をもってキューが空になるのを待つ
        let res = timeout(Duration::from_secs(3), async {
            loop {
                if handle.is_empty().await {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await;

        assert!(res.is_ok(), "Queue did not process in time");
        let (q_len, progress) = handle.get_progress().await;
        assert_eq!(q_len, 0);
        assert_eq!(progress.status, ScrapingStatus::Stopped);
    }

    fn make_config_with_profiles(cookies: Vec<&str>) -> Config {
        use crate::config::CookieProfile;
        let mut cfg = Config::default();
        cfg.cookie_profiles = cookies
            .into_iter()
            .enumerate()
            .map(|(i, c)| CookieProfile {
                id: format!("profile-{i}"),
                name: format!("Profile {i}"),
                cookies: c.to_string(),
                is_valid: None,
            })
            .collect();
        cfg
    }

    #[tokio::test]
    async fn profile_index_increments_per_job() {
        let cfg = make_config_with_profiles(vec!["cookie_a", "cookie_b", "cookie_c"]);
        let temp = std::env::temp_dir().join("pixraper_test_profile_rotation");
        let app = Arc::new(DummyAppHandle::new(temp));
        let client = reqwest::Client::builder().build().unwrap();
        let handle = QueryQueueHandle::new_with_client_and_app(&cfg, app, client, true, true);

        handle.add(make_option_with_id("j1")).await;
        handle.add(make_option_with_id("j2")).await;
        handle.add(make_option_with_id("j3")).await;

        let token = tokio_util::sync::CancellationToken::new();
        handle.start(token).await;

        let res = timeout(Duration::from_secs(3), async {
            loop {
                if handle.is_empty().await {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await;

        assert!(res.is_ok(), "Queue did not empty in time");

        // 3件処理したのでインデックスは3になっているはず
        let idx = handle.get_profile_index().await;
        assert_eq!(idx, 3, "profile_index should be 3 after 3 jobs");
    }

    #[tokio::test]
    async fn profile_index_does_not_increment_when_no_profiles() {
        let cfg = Config::default(); // cookie_profiles は空
        let temp = std::env::temp_dir().join("pixraper_test_no_profiles");
        let app = Arc::new(DummyAppHandle::new(temp));
        let client = reqwest::Client::builder().build().unwrap();
        let handle = QueryQueueHandle::new_with_client_and_app(&cfg, app, client, true, true);

        handle.add(make_option_with_id("j1")).await;
        handle.add(make_option_with_id("j2")).await;

        let token = tokio_util::sync::CancellationToken::new();
        handle.start(token).await;

        let _ = timeout(Duration::from_secs(3), async {
            loop {
                if handle.is_empty().await {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await;

        // プロファイルが空の場合はインデックスが増えない
        let idx = handle.get_profile_index().await;
        assert_eq!(idx, 0, "profile_index should stay 0 when no profiles");
    }

    #[tokio::test]
    async fn stop_signal_stops_processing() {
        let cfg = Config::default();
        let temp = std::env::temp_dir().join("pixraper_test_stop");
        let app = Arc::new(DummyAppHandle::new(temp));
        let client = reqwest::Client::builder().build().unwrap();

        let handle = QueryQueueHandle::new_with_client_and_app(&cfg, app, client, true, true);

        handle.add(make_option()).await;

        let token = tokio_util::sync::CancellationToken::new();
        handle.start(token.clone()).await;

        // すぐに停止を送る
        handle.stop().await;

        let res = timeout(Duration::from_secs(2), async {
            loop {
                let (_, progress) = handle.get_progress().await;
                if progress.status == ScrapingStatus::Stopped {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await;

        assert!(res.is_ok(), "Stop did not take effect in time");
    }
}
