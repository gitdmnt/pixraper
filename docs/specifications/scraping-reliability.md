# 仕様書: スクレイピング安定性・可視性の改善

作業日時: 2026-03-29
Agent: claude-sonnet-4-6

## 背景

スクレイピングが途中で止まったり、異常終了時にデータが失われたり、ユーザーが現状を把握できないという問題がある。

## 解決する問題

1. **CSV 一括保存による異常終了耐性の低さ** → ページ取得ごとに append 保存する
2. **エラーの握りつぶし** → Worker エラーをキュー停止・UI 表示につなげる
3. **画面遷移後の UI 状態リセット** → onMount でエラー・処理中アイテムも含めて完全復元
4. **アプリ再起動時のキュー消失** → キューを JSON ファイルに永続化する
5. **現在の処理状況が見えない** → 処理中の作品タイトルや検索クエリを UI に表示する

## スコープ

### バックエンド (Rust)
- `src-tauri/src/scraper/scrape.rs`: `ScrapingProgress` 拡張・`Worker::run` のページループ移管
- `src-tauri/src/scraper/api.rs`: `fetch_one_page` 内部関数の追加
- `src-tauri/src/scraper/queue.rs`: `WorkerError` コマンド・キュー永続化
- `src-tauri/src/csv.rs`: `append_rows_to_csv` / `generate_output_path` の追加
- `src-tauri/src/queue_state.rs`: キュー永続化の保存・読み込み（新規）
- `src-tauri/src/lib.rs`: 起動時のキュー永続化ロード

### フロントエンド (Svelte/TypeScript)
- `src/routes/scraping/main.svelte`: エラー表示・現在アイテム表示・状態復元改善

---

## 仕様

### 1. ScrapingProgress の拡張

```rust
pub struct ScrapingProgress {
    pub status: ScrapingStatus,
    pub total: Option<u64>,
    pub current: Option<u64>,
    pub error: Option<String>,          // 追加: 直近のエラーメッセージ
    pub current_item: Option<String>,   // 追加: 処理中の作品タイトルまたはクエリ
}
```

- `error`: Worker エラー発生時にセット。次の Start コマンドでクリア。
- `current_item`: 詳細取得中は作品タイトル、検索中はタグ文字列をセット。

### 2. CSV ページ単位 append 保存

#### csv.rs に追加する関数

```rust
/// 出力ファイルパスを生成する（テスト差し替え可能）
pub fn generate_output_path(app_handle: &dyn AppHandleLike) -> PathBuf

/// ファイルに行を追記する（write_header=true のとき先頭にヘッダ行を書く）
pub fn append_rows_to_csv(
    path: &Path,
    items: &[ItemRecord],
    write_header: bool,
) -> Result<(), String>
```

- ファイルが存在しない場合は新規作成
- `write_header=true` → ヘッダ + 行を書く
- `write_header=false` → 行のみ append（既存ファイルに続けて書く）

#### api.rs の変更

```rust
/// 1ページ分の検索結果を取得する内部関数
async fn fetch_one_page(
    cfg: &Config,
    option: &ScrapingOption,
    client: &reqwest::Client,
    page: u64,
) -> Result<(Vec<ItemRecord>, u64), Box<dyn Error + Send + Sync>>
// 戻り値: (ページのアイテム一覧, last_page)
```

- `fetch_search_result` は `fetch_one_page` のループラッパーとして残す（テスト互換性維持）
- 進捗更新・キャンセルチェックは `fetch_one_page` 側で行わず、呼び出し元（Worker）に任せる

#### scrape.rs Worker::run の変更

```rust
// Worker::run 引数に output_path を追加
pub async fn run(
    option: &ScrapingOption,
    client: &reqwest::Client,
    progress: &Arc<Mutex<ScrapingProgress>>,
    cfg: &Config,
    token: &CancellationToken,
    app_handle: &dyn AppHandleLike,
    output_path: &Path,          // 追加
) -> Result<(), ...>
```

- `Worker::run` 内でページループを管理
- 各ページ取得後: `append_rows_to_csv(output_path, &page_items, page == 1)` を呼ぶ
- 詳細モード: `progress.current_item = Some(item.title.clone())` をセット
- 詳細モード完了後: Worker がメモリ上に保持する全 enriched `Vec<ItemRecord>` を `append_rows_to_csv(output_path, &all_enriched_items, true)` で上書き（ヘッダ付き新規書き込み）

#### queue.rs での output_path 生成

```rust
// Worker を spawn する前に出力パスを生成し渡す
let output_path = generate_output_path(&*self.app_handle);
```

### 3. エラー時のキュー全停止

#### 新しいコマンド

```rust
enum Command {
    // ...既存...
    WorkerError(String),  // 追加
}
```

#### Worker spawn の変更

```rust
tokio::spawn(async move {
    match Worker::run(...).await {
        Ok(()) => { sender.send(Command::WorkerFinished).await; }
        Err(e) => { sender.send(Command::WorkerError(e.to_string())).await; }
    }
});
```

#### WorkerError ハンドラ

```rust
Command::WorkerError(msg) => {
    self.worker_running = false;
    self.queue.clear();
    if let Some(token) = &self.scraping_token {
        token.cancel();
    }
    self.scraping_token = None;
    let mut p = self.progress.lock().await;
    p.status = ScrapingStatus::Stopped;
    p.error = Some(msg);
    // キュー永続化: クリア後に保存
    self.save_queue_state();
}
```

### 4. キューの永続化

#### 新規ファイル: `src-tauri/src/queue_state.rs`

```rust
/// キューの状態を app_config_dir/queue.json に保存する
pub fn save_queue(
    app_handle: &dyn AppHandleLike,
    queue: &VecDeque<ScrapingOption>,
) -> Result<(), String>

/// queue.json からキューを読み込む（ファイルが存在しない場合は空を返す）
pub fn load_queue(
    app_handle: &dyn AppHandleLike,
) -> Result<VecDeque<ScrapingOption>, String>
```

`AppHandleLike` に `config_dir() -> Option<PathBuf>` メソッドを追加する。
既存のテスト用スタブ（`csv.rs` の `MockAppHandle` 等、`AppHandleLike` を実装している全箇所）にも `config_dir()` の実装を追加する（テストでは `Some(std::env::temp_dir())` を返す）。

#### 保存タイミング (queue.rs)

以下のコマンドハンドラの末尾で `save_queue_state()` を呼ぶ：
- `Command::Add`
- `Command::Remove`
- `Command::Clear`
- `Command::RunNext`（queue.pop_front() 後）
- `Command::WorkerError`

#### 起動時ロード (lib.rs)

```rust
let persisted_queue = queue_state::load_queue(&app_handle).unwrap_or_default();
app.manage(scraper::QueryQueueHandle::new_with_queue(&config, app_handle.clone(), persisted_queue));
```

`QueryQueueHandle::new_with_queue` を追加し、初期キューを受け取れるようにする。

### 5. フロントエンドの変更

#### ScrapingProgress 型の拡張

```typescript
interface ScrapingProgress {
  status: "Running" | "Stopped";
  total: number | null;
  current: number | null;
  error: string | null;         // 追加
  current_item: string | null;  // 追加
}
```

#### onMount での状態復元

```javascript
onMount(async () => {
  fetchQueue();
  const progress = await invoke<ScrapingProgress>("get_progress");
  isRunning = progress.status === "Running";
  scrapedItems = progress.current ?? 0;
  totalItems = progress.total ?? 0;
  currentItem = progress.current_item ?? null;
  lastError = progress.error ?? null;
  if (isRunning) startPolling();
});
```

#### UI 追加要素

- `currentItem`: 進捗バーの下に「処理中: {currentItem}」を表示（Running 時のみ）
- `lastError`: 赤い通知エリアにエラーメッセージを表示（Stopped かつ error がある場合）
  - 「次の開始時にクリア」とメモを添える

---

## バリデーション・動作保証

- `append_rows_to_csv` の単体テスト（新規作成・追記・ヘッダ有無）
- `Worker::run` がエラーを返すと `WorkerError` が送られてキューがクリアされることのテスト
- `Worker::run` 詳細モード完了後に CSV が enriched データ（basic でなく enriched の値を持つ）で上書きされることのテスト
- `save_queue` → `load_queue` のラウンドトリップテスト
- `fetch_one_page` の単体テストは既存の `fetch_search_result` のテストで兼用

## 受け入れ条件

1. `cargo clippy` で新規警告なし
2. ページ取得後、クラッシュを模倣しても取得済みページ分の CSV が残ること
3. Worker エラー時: `progress.error` にメッセージがセット、queue がクリア、status が Stopped
4. アプリ再起動後、前回のキューが復元されること（queue.json 経由）
5. 詳細モード実行中に UI に「処理中: {作品タイトル}」が表示されること
6. Worker エラー時に UI にエラーメッセージが表示されること

## スコープ外

- エラー発生後の自動リトライ
- キュー永続化の暗号化・セキュリティ強化
- 詳細モードのアイテム単位 CSV 上書き（ページ単位で basic 保存、完了時に enriched で上書き）
- キュー永続化ロジックは `queue_state.rs` に分離済み（queue.rs の肥大化防止のため）
