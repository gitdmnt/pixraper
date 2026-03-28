# 仕様書: キューアイテム削除をインデックスベースからIDベースに変更する

## 背景

`remove_queue_item` はキューの位置（インデックス）で削除対象を特定する。スクレイピング実行中に `Worker` がキューの先頭を `pop_front()` するタイミングとユーザーの削除操作が競合すると、意図と異なるアイテムが削除される。

## 目的

キューアイテムに安定した UUID を付与し、ID ベースで削除することで競合時の誤削除を防ぐ。

## スコープ

- `src-tauri/src/scraper/scrape.rs`（`ScrapingOption` に `id: String` を追加）
- `src-tauri/src/scraper/queue.rs`（`Command::Remove` を `(String)` に変更、`retain` で削除）
- `src-tauri/src/commands/scraping.rs`（`remove_queue_item` の引数を `id: String` に変更）
- `src/routes/scraping/main.svelte`（`ScrapingOption` インタフェースに `id` 追加、`add_queue` 前に `crypto.randomUUID()` で採番、`removeQueueItem` を ID ベースに変更）
- `src/routes/scraping/components/QueueList.svelte`（`removeQueueItem` の型を `(id: string) => void` に変更）

## 型情報

- `ScrapingOption.id` の型: `String`（Rust）/ `string`（TypeScript）
- UUID 生成: フロントエンドで `crypto.randomUUID()` を使用して `add_queue` 呼び出し前に採番する
- `get_queue` は `Vec<ScrapingOption>` を返す。バックエンドの `ScrapingOption` に `id` が追加されると、`get_queue` レスポンスにも自動的に `id` が含まれるようになる。フロントエンドの `ScrapingOption` インタフェースに `id: string` を追加することで型が一致する。

## 仕様

### `ScrapingOption` に `id` フィールドを追加（`scrape.rs`）

```rust
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScrapingOption {
    pub id: String,  // UUID（フロントエンドで採番）
    pub tags: Vec<String>,
    pub search_mode: String,
    pub scd: String,
    pub ecd: String,
    pub detailed: bool,
    pub is_illust: bool,
}
```

### `Command::Remove` を ID ベースに変更（`queue.rs`）

```rust
// 変更前
Remove(usize),

// 変更後
Remove(String),
```

アクター内の処理：

```rust
// 変更前
Command::Remove(index) => {
    if index < self.queue.len() {
        self.queue.remove(index);
    }
}

// 変更後
Command::Remove(id) => {
    self.queue.retain(|opt| opt.id != id);
}
```

### `remove_queue_item` コマンドの引数変更（`commands/scraping.rs`）

```rust
// 変更前
pub async fn remove_queue_item(queue: State<'_, ScrapingHandle>, index: usize) -> Result<(), String>

// 変更後
pub async fn remove_queue_item(queue: State<'_, ScrapingHandle>, id: String) -> Result<(), String>
```

### フロントエンド（`main.svelte`）

`ScrapingOption` インタフェースに `id` を追加し、`add_queue` 前に UUID を採番する：

```typescript
interface ScrapingOption {
  id: string;  // 追加
  tags: string[];
  searchMode: string;
  scd: string;
  ecd: string;
  detailed: boolean;
  isIllust: boolean;
}

// add_queue 呼び出し時
const option = { ...scrapingOption, id: crypto.randomUUID() };
invoke("add_queue", { option })

// remove_queue_item 呼び出し時
const removeQueueItem = (id: string) => {
  invoke("remove_queue_item", { id })
    .then(() => refreshQueue());
};
```

### フロントエンド（`QueueList.svelte`）

```typescript
// 変更前
export let removeQueueItem: ((index: number) => void) | undefined = undefined;

// 変更後
export let removeQueueItem: ((id: string) => void) | undefined = undefined;

// ボタンの onclick
// 変更前: onclick={() => removeQueueItem && removeQueueItem(i)}
// 変更後: onclick={() => removeQueueItem && removeQueueItem(item.id)}
```

## テスト追加

`scraper/queue.rs` の `#[cfg(test)]` ブロックに以下を追加する：

- `remove_by_id_removes_correct_item` — 3件のキューから中間のアイテムを ID で削除したとき、他の2件が残ること
- `remove_by_id_nonexistent_id_is_noop` — 存在しない ID で削除を試みても残りのアイテムが変化しないこと

テスト用のモック `ScrapingOption` の構築：

```rust
fn make_option(id: &str) -> ScrapingOption {
    ScrapingOption {
        id: id.to_string(),
        tags: vec![],
        search_mode: String::new(),
        scd: String::new(),
        ecd: String::new(),
        detailed: false,
        is_illust: true,
    }
}
```

## 受け入れ条件

1. `cargo test` が全パスすること
2. `cargo clippy` で新規警告なし
3. `Command::Remove` の型が `String` に、`remove_queue_item` コマンドの引数が `id: String` に変更されていること（コンパイルで保証）
4. `remove_by_id_removes_correct_item` テストが通ること
5. `remove_by_id_nonexistent_id_is_noop` テストが通ること
6. フロントエンド手動確認: 同一設定を2回キューに追加したとき、それぞれ異なる `id` が付与されること（`crypto.randomUUID()` の仕様上保証されるが、DevTools の Network タブまたは `get_queue` レスポンスで確認）

## スコープ外

- バックエンドでの UUID 採番（フロントエンド採番で十分であり、`add_queue` の戻り値変更が不要）
- `Worker` が処理中のアイテムを削除しようとした場合の挙動（`Worker` は `pop_front()` でアイテムを取り出し済みのため、キューからは既に削除されている）
