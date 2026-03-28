# 実装手順書: Cookie プロファイル管理

作成日: 2026-03-28
Agent: claude-sonnet-4-6

---

## 実装順序

### 1. Rust バックエンド

#### 1-1. `src-tauri/src/config.rs`
- `CookieProfile { id, name, cookies, is_valid: Option<bool> }` を追加
- `Config` に `cookie_profiles: Vec<CookieProfile>` と `active_profile_id: Option<String>` を追加
- `Default` impl で `cookie_profiles: vec![]`, `active_profile_id: None`

#### 1-2. `src-tauri/src/commands/config.rs`
- `validate_cookies(cookies: String) -> Result<bool, String>` コマンドを追加
  - reqwest クライアントを生成してリクエスト
  - Cookie ヘッダー + User-Agent を付与
  - レスポンスの `body.isLoggedIn` を返す

#### 1-3. `src-tauri/src/lib.rs`
- `validate_cookies` をコマンドリストに登録

### 2. フロントエンド TypeScript

#### 2-1. `src/routes/settings/type.d.ts`
- `CookieProfile` interface を追加
- `Config` を更新（`cookieProfiles`, `activeProfileId` を追加）

#### 2-2. `src/routes/settings/cookieParser.ts` (新規)
- `IMPORTANT_KEYS = ["PHPSESSID", "device_token"]`
- `parseCookies(raw: string): ParsedCookie[]`
  - 空文字列は空配列
  - `;` で分割 → 各要素を trim → `=` で分割
  - key が空または `=` なしの要素は除外
  - key が IMPORTANT_KEYS に含まれれば `important: true`

### 3. フロントエンド コンポーネント

#### 3-1. `src/routes/settings/CookieProfileList.svelte` (新規)
- props: `profiles`, `activeProfileId`, `selectedId`
- emit: `select(id)`, `add()`, `delete(id)`, `setActive(id)`
- 各プロファイル行に: 名前, 有効バッジ (green/red/gray), アクティブ印

#### 3-2. `src/routes/settings/CookieProfileEditor.svelte` (新規)
- props: `profile`, `activeProfileId`
- emit: `save(profile)`, `validate(cookies)`, `setActive(id)`
- Cookie テキストエリア（bind:value に raw cookies）
- パース結果テーブル: key / value / important
- PHPSESSID 不在の場合は黄色の警告バナー
- Validate ボタン → invoke validate_cookies → isValid 更新
- Set as Active ボタン
- Save ボタン

#### 3-3. `src/routes/settings/main.svelte` (更新)
- 既存の出力・間隔設定は保持
- Cookie セクションを `CookieProfileList` + `CookieProfileEditor` に置き換え
- 「+ Add Profile」で UUID 生成, name="Profile N", cookies="", isValid=null
- Save 時: 全プロファイルと activeProfileId を含む Config を set_config に送る
  - active profile の cookies を Config.cookies にも設定する
