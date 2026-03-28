# Pixraper アーキテクチャドキュメント

> 作成日: 2026-03-28
> 作成エージェント: claude-sonnet-4-6

## 概要

Pixraper は Pixiv からデータを収集し、タグ統計・共起分析を行うデスクトップアプリケーション。
フロントエンドに SvelteKit、バックエンドに Rust + Tauri を採用し、Tauri IPC で通信する。

> **注意**: Pixiv のスクレイピングは規約違反。バイナリは配布しないこと。

---

## 技術スタック

| レイヤー | 技術 | バージョン |
|---------|------|-----------|
| UI フレームワーク | Svelte | 5.0.0 |
| メタフレームワーク | SvelteKit | 2.9.0 |
| ビルドツール | Vite | 6.0.3 |
| スタイリング | Tailwind CSS | 4.1.18 |
| デスクトップ化 | Tauri | 2.9.0 |
| 言語 | TypeScript / Rust | ~5.6.2 / Edition 2021 |
| 非同期実行時 | Tokio | 1.48.0 |
| HTTP 通信 | Reqwest | 0.12.24 |
| シリアライゼーション | Serde / serde_json | 1.0 |
| CSV 処理 | csv crate | 1.4.0 |
| パッケージマネージャ | bun | - |

---

## ディレクトリ構造

```
pixraper/
├── src/                            # フロントエンド (SvelteKit)
│   ├── routes/
│   │   ├── +page.svelte            # メインページ（タブ切り替え）
│   │   ├── +layout.svelte          # ルートレイアウト
│   │   ├── scraping/               # スクレイピング機能
│   │   │   ├── main.svelte         # スクレイピング画面
│   │   │   └── components/
│   │   │       ├── ProgressPanel.svelte   # 進捗バー
│   │   │       ├── ActivityPanel.svelte   # 結果ログ
│   │   │       ├── OptionsPanel.svelte    # スクレイピング設定
│   │   │       ├── QueueList.svelte       # キュー一覧
│   │   │       └── TipsPanel.svelte       # ヒント表示
│   │   ├── analytics/              # 分析機能
│   │   │   ├── main.svelte         # 分析画面（タブ切り替え）
│   │   │   ├── tagRanking/
│   │   │   │   ├── main.svelte     # タグランキング表示
│   │   │   │   └── OverviewCard.svelte
│   │   │   ├── cooccurrenceAnalyze.svelte  # 共起分析
│   │   │   ├── rawCSV.svelte       # 生データ表示
│   │   │   ├── timeAnalyze.svelte  # 時系列分析（実装予定）
│   │   │   ├── type.d.ts
│   │   │   └── components/
│   │   │       └── FiltersPanel.svelte
│   │   └── settings/               # 設定画面
│   │       ├── main.svelte
│   │       └── type.d.ts
│   ├── lib/
│   │   └── components/
│   │       ├── TopAppBar.svelte    # ヘッダーバー
│   │       ├── Button.svelte       # 汎用ボタン
│   │       └── TagList.svelte      # タグ一覧テーブル
│   ├── app.css                     # グローバルスタイル (Material Design 3 トークン)
│   └── app.html
├── src-tauri/                      # バックエンド (Rust + Tauri)
│   ├── src/
│   │   ├── lib.rs                  # Tauri アプリエントリポイント（コマンド登録）
│   │   ├── main.rs                 # バイナリエントリポイント
│   │   ├── config.rs               # 設定管理 (TOML)
│   │   ├── csv.rs                  # CSV 入出力
│   │   ├── scraper/
│   │   │   ├── mod.rs              # モジュール公開
│   │   │   ├── api.rs              # Pixiv API 通信
│   │   │   ├── scrape.rs           # スクレイピング実行ロジック
│   │   │   └── queue.rs            # キュー管理（Actor パターン）
│   │   ├── analytics/
│   │   │   └── mod.rs              # タグ統計・共起分析
│   │   └── commands/
│   │       ├── mod.rs
│   │       ├── config.rs           # 設定コマンド
│   │       ├── scraping.rs         # スクレイピングコマンド
│   │       └── analytics.rs        # 分析コマンド
│   ├── Cargo.toml
│   ├── tauri.conf.json             # Tauri 設定（Window: 800x600）
│   └── capabilities/
├── docs/                           # ドキュメント
│   ├── architecture.md             # このファイル
│   ├── adr/                        # Architecture Decision Records
│   ├── specifications/             # 機能仕様書
│   └── workflow/                   # 実装手順書
├── CLAUDE.md                       # 開発ルール
└── README.md
```

---

## レイヤー設計

```
┌─────────────────────────────────────────────────────┐
│                    UI Layer (Svelte)                 │
│  +page.svelte  /scraping  /analytics  /settings     │
└──────────────────────────┬──────────────────────────┘
                           │ invoke() / Tauri IPC
┌──────────────────────────▼──────────────────────────┐
│               Commands Layer (Rust)                  │
│  commands/config.rs  scraping.rs  analytics.rs      │
└──────┬──────────────────┬─────────────────┬─────────┘
       │                  │                 │
┌──────▼──────┐  ┌────────▼───────┐  ┌─────▼──────────┐
│ Config      │  │  Scraper       │  │  Analytics     │
│ config.rs   │  │  queue.rs      │  │  analytics/    │
│             │  │  scrape.rs     │  │  mod.rs        │
│ TOML ファイル│  │  api.rs        │  │                │
└─────────────┘  └────────┬───────┘  └─────────────────┘
                          │
                 ┌────────▼───────┐
                 │    csv.rs      │
                 │ Documents/     │
                 │ Pixraper/*.csv │
                 └────────────────┘
```

---

## モジュール詳細

### フロントエンド

#### 画面構成（3タブ）

| タブ | ルート | 役割 |
|-----|--------|------|
| Analytics | `/analytics` | CSV 読み込み・分析表示 |
| Scraping | `/scraping` | スクレイピング実行管理 |
| Settings | `/settings` | Cookie・出力先・間隔設定 |

#### Analytics 画面のサブタブ

| サブタブ | コンポーネント | 役割 |
|---------|--------------|------|
| Tag Ranking | `tagRanking/main.svelte` | タグ別集計・ソート・フィルタ |
| Co-occurrence | `cooccurrenceAnalyze.svelte` | タグ共起分析 |
| Raw CSV | `rawCSV.svelte` | 生データテーブル表示 |

---

### バックエンド

#### Config (`config.rs`)

設定の読み書きを担う。アプリ起動時に TOML から読み込み、変更時にディスクに永続化。

```rust
pub struct Config {
    pub cookies: Option<String>,         // Pixiv 認証 Cookie
    pub output: Option<String>,           // CSV 出力ディレクトリ
    pub scraping_interval_millis: u64,   // API 呼び出し間隔
}
```

- 保存先: OS のアプリ設定ディレクトリ (`Config.toml`)
- 関数: `load_config(app_handle)` / `save_config(app_handle, config)`

---

#### CSV (`csv.rs`)

CSV ファイルの入出力を担う。テスト可能性のため `AppHandleLike` トレイトで抽象化。

- 出力先: `Documents/Pixraper/result_YYYYmmdd_HHMMSS.csv`
- カラム: `is_illust, id, title, x_restrict, tags, user_id, create_date, ai_type, width, height, text_count, word_count, original, bookmark_count, view_count`
- 関数: `save_as_csv(items, app_handle)` / `load_items(path)`

---

#### Scraper

##### `api.rs` — Pixiv API 通信

Cookie 認証・カスタム User-Agent で Pixiv 非公式 API を叩く。

- `fetch_search_result()`: タグ検索（ページネーション対応）
- `fetch_detail_data()`: ブックマーク数・閲覧数の詳細取得

レスポンス型:
```rust
pub struct IllustData { id, title, tags, x_restrict, user_id, create_date, ai_type, width, height }
pub struct NovelData  { id, title, tags, x_restrict, user_id, text_count, word_count, is_original, ... }
```

##### `scrape.rs` — スクレイピング実行

Worker が 1 タスクを担当。キャンセルトークン付き。

```rust
pub struct ScrapingOption {
    tags: Vec<String>,
    search_mode: String,   // "s_tag" | "s_tag_full" | "s_tc"
    scd / ecd: String,     // 開始/終了日付フィルタ
    detailed: bool,        // 詳細情報取得フラグ
    is_illust: bool,       // イラスト or 小説
}
pub struct ScrapingProgress {
    status: ScrapingStatus,   // Running | Stopped
    total: Option<u64>,
    current: Option<u64>,
}
```

##### `queue.rs` — キュー管理（Actor パターン）

Tokio MPSC チャネルで状態を管理。外部からは `QueryQueueHandle` 経由でアクセス。

```
QueryQueueHandle   →(Command)→   QueryQueueActor
  add / clear /                    queue: VecDeque
  remove / start /                 progress: Arc<Mutex>
  stop / get_progress              scraping_token: CancellationToken
```

コマンド種別: `Add`, `Clear`, `Start`, `RunNext`, `Stop`, `GetProgress`, `GetQueue`, `Remove`, `WorkerFinished`

---

#### Analytics (`analytics/mod.rs`)

`ItemRecordVecExt` トレイトでデータ変換パイプラインを構成（関数型スタイル）。

```rust
pub enum SortKey {
    WorkCount, BookmarkCount, ViewCount,
    BookmarkPerWork, ViewPerWork, BookmarkPerView, NormalizedScore
}
pub struct Filter {
    works_count_cutoff: u64,
    show_ai_generated: bool,
    show_not_ai_generated: bool,
    show_x_restricted: bool,
    show_not_x_restricted: bool,
    search_query: Option<String>,
}
pub struct TagStats {
    tag: String,
    count: u64,
    view_count: u64,
    bookmark_count: u64,
    normalized_score: f64,  // アーティスト正規化 Z-score
}
```

**アーティスト正規化スコア**: `log(bookmark+1)` をアーティスト内で Z-score 化。人気作家バイアスを除去。

---

### Tauri コマンド一覧

#### 設定

| コマンド | 型 |
|---------|---|
| `get_config()` | `-> Result<Config>` |
| `set_config(new_config)` | `-> Result<()>` |

#### スクレイピング

| コマンド | 型 |
|---------|---|
| `add_queue(option)` | `-> Result<()>` |
| `clear_queue()` | `-> Result<()>` |
| `remove_queue_item(index)` | `-> Result<()>` |
| `start_scraping()` | `-> Result<()>` |
| `stop_scraping()` | `-> Result<String>` |
| `get_progress()` | `-> Result<ScrapingProgress>` |
| `get_queue()` | `-> Result<Vec<ScrapingOption>>` |

#### 分析

| コマンド | 型 |
|---------|---|
| `load_dataset(path)` | `-> Result<Vec<ItemRecord>>` |
| `get_all_tags()` | `-> Result<Vec<TagStats>>` |
| `calculate_tag_ranking(filter, sort_key)` | `-> Result<Vec<TagStats>>` |
| `calculate_co_occurence(tag)` | `-> Result<CooccurrenceResult>` |

---

## データフロー

### スクレイピングフロー

```
[OptionsPanel] タグ・期間・オプション入力
        ↓
invoke("add_queue", option)
        ↓
QueryQueueHandle::add() → Actor: Add コマンド
        ↓
invoke("start_scraping")
        ↓
CancellationToken 生成 → Actor: Start コマンド
        ↓
QueryQueueActor → RunNext → Worker::run()
        ↓
api::fetch_search_result()   ← Pixiv API
        ↓ (detailed=true)
api::fetch_detail_data()     ← Pixiv API (各アイテム)
        ↓
csv::save_as_csv()           → Documents/Pixraper/result_*.csv
        ↓
[ProgressPanel] ← 1秒ポーリング get_progress()
```

### 分析フロー

```
[FiltersPanel] ファイル選択ダイアログ (Tauri Dialog)
        ↓
invoke("load_dataset", path)
        ↓
AnalyticsState にメモリキャッシュ
        ↓
┌────────────────────┬─────────────────────────────┐
│  Tag Ranking       │  Co-occurrence               │
│ calculate_tag_     │  calculate_co_occurence(tag) │
│ ranking(filter,    │  → CooccurrenceResult        │
│ sort_key)          │                              │
│ filter_by()        │                              │
│ → tag_stats()      │                              │
│ → Vec<TagStats>    │                              │
└────────────────────┴─────────────────────────────┘
        ↓
フロントエンド表示
```

---

## 設計パターン

| パターン | 適用箇所 | 目的 |
|---------|---------|------|
| Actor パターン | `scraper/queue.rs` | キュー状態の非同期安全管理 |
| トレイト抽象化 | `csv.rs` (`AppHandleLike`) | テスト可能性の向上 |
| Extension トレイト | `analytics/mod.rs` (`ItemRecordVecExt`) | 関数型パイプライン構成 |
| メモリキャッシュ | `AnalyticsState` | 複数リクエスト時の効率化 |
| CancellationToken | `scraper/` | 長時間タスクの安全な中断 |

---

## スタイリング方針

- Tailwind CSS v4 を使用
- Material Design 3 カラートークンを CSS 変数で定義（Primary: `#6750a4`）
- 共通クラスは `app.css` に集約（`occ-button`, `occ-card` など）
- レイアウトは `flex` + `gap` 優先（`mt` / `mb` は避ける）

---

## 今後の実装予定

- `timeAnalyze.svelte`: 時系列分析（スケルトン実装済み）
- テスト整備: TDD ワークフローに沿って `docs/specifications/` + `docs/workflow/` を整備
