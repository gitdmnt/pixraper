# 実装手順書: スクレイピング安定性・可視性の改善

## 実装順序

### 1. csv.rs の拡張

1. `AppHandleLike` に `config_dir() -> Option<PathBuf>` を追加
2. `tauri::AppHandle` の実装に `config_dir()` を追加
3. `generate_output_path(app_handle: &dyn AppHandleLike) -> PathBuf` を追加
4. `append_rows_to_csv(path: &Path, items: &[ItemRecord], write_header: bool) -> Result<(), String>` を追加
5. 既存 `save_as_csv` を `generate_output_path` + `append_rows_to_csv` を使って書き直す
6. テスト: `append_rows_to_csv` の新規作成・追記・ヘッダ有無

### 2. queue_state.rs の新規作成

1. `pub fn save_queue(app_handle: &dyn AppHandleLike, queue: &VecDeque<ScrapingOption>) -> Result<(), String>`
2. `pub fn load_queue(app_handle: &dyn AppHandleLike) -> Result<VecDeque<ScrapingOption>, String>`
   - ファイルなし or 解析失敗時は空のキューを返す
3. テスト: save → load ラウンドトリップ

### 3. api.rs の変更

1. `fetch_search_result` のページループを `fetch_one_page` として抽出
   - シグネチャ: `async fn fetch_one_page(cfg, option, client, page) -> Result<(Vec<ItemRecord>, u64), ...>`
   - 1ページ分の取得・レスポンス検証・ItemRecord 変換のみ担当
   - 進捗更新・キャンセルチェックは含めない
2. `fetch_search_result` を `fetch_one_page` のループラッパーに変更（テスト互換維持）

### 4. scrape.rs の変更

1. `ScrapingProgress` に `error: Option<String>` と `current_item: Option<String>` を追加
2. `Worker::run` に `output_path: &Path` 引数を追加
3. `Worker::run` のページループを変更:
   - `fetch_one_page` を呼ぶ
   - キャンセルチェック・進捗更新をここで行う
   - 各ページ後: `append_rows_to_csv(output_path, &page_items, page == 1)`
4. 詳細モードで各アイテム処理時: `progress.current_item = Some(title)` をセット
5. 詳細モード完了後: `append_rows_to_csv(output_path, &all_enriched, true)` で上書き
6. テスト: 詳細モード後に CSV が enriched データで上書きされることを確認

### 5. queue.rs の変更

1. `Command::WorkerError(String)` を追加
2. Worker spawn 部分を `match result { Ok => WorkerFinished, Err => WorkerError }` に変更
3. `WorkerError` ハンドラを追加:
   - `worker_running = false`
   - `queue.clear()`
   - `token.cancel()` + `scraping_token = None`
   - `progress.error = Some(msg)`, `status = Stopped`
   - `save_queue_state()` を呼ぶ
4. `Start` ハンドラで `progress.error = None` をクリア
5. `save_queue_state(&self)` メソッドを追加（queue_state::save_queue 呼び出し）
6. Add/Remove/Clear/RunNext の各ハンドラ末尾で `save_queue_state()` を呼ぶ
7. `QueryQueueHandle::new_with_queue` を追加（初期キューを受け取れるコンストラクタ）
8. テスト: WorkerError でキュークリア・ステータス停止

### 6. lib.rs の変更

1. `queue_state::load_queue` で起動時にキューをロード
2. `QueryQueueHandle::new_with_queue` を使って初期キューを渡す

### 7. フロントエンドの変更 (main.svelte)

1. `ScrapingProgress` 型に `error: string | null`, `current_item: string | null` を追加
2. `let currentItem: string | null = $state(null)` を追加
3. `let lastError: string | null = $state(null)` を追加
4. `pollProgress` と `onMount` で `currentItem` / `lastError` を更新
5. HTML に追加:
   - Running 時: `{#if currentItem}処理中: {currentItem}{/if}`
   - Stopped + error 時: エラーパネル（赤）に `lastError` を表示

### 8. ビルド確認

```bash
cd src-tauri && cargo clippy
cargo test
```
