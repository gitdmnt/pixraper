# 仕様書: Cookie プロファイル管理

作成日: 2026-03-28
Agent: claude-sonnet-4-6

---

## 概要

Settings 画面において、Pixiv の Cookie を複数プロファイルとして管理する。
各プロファイルは色付きパース表示・有効性チェックを持ち、スクレイパーが使用するアクティブプロファイルを切り替えられる。

---

## 機能要件

### FR-1: Cookie プロファイルの複数管理

- プロファイルは `{ id, name, cookies, isValid }` で表現される
- プロファイルは複数登録・削除・名前変更できる
- プロファイルのうち1つを「アクティブ」に設定でき、スクレイパーはそのプロファイルの cookies 文字列を使用する
- プロファイルは Config.toml に永続化される

### FR-2: Cookie のパースと色付き表示

- Raw テキスト（`key=value; key2=value2; ...` 形式）を入力するとパースされる
- パースした key-value ペアを一覧表示する
- 重要なキー（PHPSESSID, device_token）は強調表示する
- PHPSESSID が存在しない場合は警告を表示する

### FR-3: Cookie の有効性チェック

- 「Validate」ボタン押下で Pixiv API にリクエストし、ログイン状態を確認する
- バックエンドコマンド `validate_cookies(cookies: String) -> Result<bool, String>`
  - エンドポイント: `https://www.pixiv.net/touch/ajax/user/self/status`
  - Cookie ヘッダーを付与してリクエスト
  - レスポンス JSON の `body.isLoggedIn` が true なら有効
- 結果はプロファイルの `isValid: bool | null` に保存される
- 有効=緑バッジ, 無効=赤バッジ, 未確認=グレーバッジ

---

## 非機能要件

### NFR-1: 既存の scraper との互換性

- `Config.cookies: Option<String>` はアクティブプロファイルの cookies を常に反映する
- バックエンドの scraper 側は変更なし

### NFR-2: 永続化形式

Config.toml に以下のフィールドを追加:

```toml
active_profile_id = "uuid"

[[cookie_profiles]]
id = "uuid"
name = "Main Account"
cookies = "PHPSESSID=..."
is_valid = true  # or false or omitted (unknown)
```

---

## データモデル

### Rust 側 (config.rs)

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CookieProfile {
    pub id: String,
    pub name: String,
    pub cookies: String,
    pub is_valid: Option<bool>,
}
```

Config に追加:

```rust
pub cookie_profiles: Vec<CookieProfile>,
pub active_profile_id: Option<String>,
```

### TypeScript 側 (type.d.ts)

```ts
export interface CookieProfile {
  id: string;
  name: string;
  cookies: string;
  isValid: boolean | null;
}

export interface Config {
  cookies: string;
  output: string;
  scrapingIntervalMillis: number;
  cookieProfiles: CookieProfile[];
  activeProfileId: string | null;
}
```

---

## コンポーネント設計

### `settings/main.svelte`

- 上部: 出力ディレクトリ・スクレイピング間隔（既存）
- 下部: Cookie プロファイルセクション
  - プロファイルリスト（`CookieProfileList.svelte`）
  - 選択中プロファイルのエディタ（`CookieProfileEditor.svelte`）

### `settings/CookieProfileList.svelte`

- プロファイル一覧（名前・有効バッジ・アクティブ表示）
- 「+ 追加」ボタン
- プロファイルを選択すると editor に表示
- 削除ボタン（アクティブプロファイルは削除不可）

### `settings/CookieProfileEditor.svelte`

- 名前入力フィールド
- Cookie テキストエリア（Raw 入力）
- パース結果表示: key-value ペアのテーブル
  - PHPSESSID / device_token は強調 (primary 色)
  - その他は muted 色
  - PHPSESSID がない場合は警告バナー
- 「Validate」ボタン（バックエンド呼び出し）
- 「Set as Active」ボタン
- 「Save」ボタン

### `settings/cookieParser.ts`

```ts
export interface ParsedCookie { key: string; value: string; important: boolean; }
export const parseCookies = (raw: string): ParsedCookie[]
export const IMPORTANT_KEYS = ["PHPSESSID", "device_token"]
```

---

## バックエンドコマンド

### `validate_cookies(cookies: String) -> Result<bool, String>`

- GET `https://www.pixiv.net/touch/ajax/user/self/status`
- Header: `Cookie: {cookies}`
- Header: `User-Agent: Mozilla/5.0` (src-tauri/src/scraper/api.rs の fetch_search_result と同じ値)
- レスポンス型: `{ error: bool, body: { isLoggedIn: bool } }`
- isLoggedIn が true → Ok(true), false → Ok(false), エラー → Err(msg)

### アクティブプロファイル切り替え・保存の経路

- `set_config` コマンドに `cookie_profiles` と `active_profile_id` を含めて保存する
- Save ボタン押下時に `set_config` を呼び出す
- アクティブプロファイル切り替え時: `active_profile_id` を更新し `Config.cookies` を該当プロファイルの cookies 文字列に更新して `set_config` を呼ぶ
- `active_profile_id` が None または対応プロファイルが存在しない場合は `Config.cookies` を空文字列にする
- 新規プロファイルの `is_valid` 初期値は `None`（未確認）

---

## テスト観点

### cookieParser.ts (フロントエンド単体テスト)

- 正常系: `key=value; key2=value2` をパースして2件返す
- 正常系: PHPSESSID を `important: true` でフラグ付けする
- 正常系: device_token を `important: true` でフラグ付けする
- 異常系: 空文字列で空配列を返す
- 異常系: `=value` や `key=` など不正形式を除外する

### validate_cookies (Rust)

- 正常系: isLoggedIn=true で Ok(true) を返す（モック不可のため手動確認）
- 正常系: isLoggedIn=false で Ok(false) を返す
- 異常系: ネットワークエラーで Err を返す

### 受け入れ条件

- プロファイルを2件追加し、それぞれ別の Cookie を保存できる
- Validate ボタン押下後に有効バッジ/無効バッジが表示される
- アクティブ切り替え後、Config.cookies が新しい cookies に更新される
- アプリ再起動後もプロファイル一覧と active_profile_id が保持される
- PHPSESSID が存在しないプロファイルでは警告が表示される

---

## スコープ外

- Cookie の自動リフレッシュ
- プロファイルのインポート/エクスポート
- Cookie の有効期限日時の表示（Set-Cookie ヘッダー未使用のため）
