## Coding Rule

### 禁止

1. URL のハードコーディングの禁止
2. 800 行以上のファイルの禁止

### 必ず実行

1. 計画の自己レビュー
2. 型定義
3. 関数型プログラミング
4. Test Driven Development（要求 → 仕様書 → 失敗テストの追加 → 実装）
5. 単一責任原則に従ったファイル / モジュール / 関数の分割
6. MVC の分離
7. MCP ではなく CI を使用
8. cargo clippy / bun / bunx  の使用

## TDDワークフロー

1. ユーザーに聞き取りを行って要求の確定
2. worktree およびブランチを作成 `feature/PR-[number]-[feature]/`
3. 仕様書の作成 `docs/specifications/[機能名].md`
4. サブエージェントによる仕様書の自己レビュー `以下の仕様書をレビューし、ISSUEとSUGGESTIONを詳述してください.`
5. 実装手順書の作成 `docs/workflow/[機能名].md`
6. 実装
7. テストの実行
8. PR を作成
9. PR の自己レビュー
10. `main` へマージ

## Design Rule

1. Tailwind CSS v4 を使用する
2. `style.css` に共通の `occ-button` や `occ-card` の class を作成し再利用する
3. `mt` や `mb` class ではなく `flex` と `gap` を使用すること

## Docs Routing

`docs/`にドキュメントを配置する予定。

## 開発環境

### コマンド

| 目的     | コマンド        |
| -------- | --------------- |
| ウォッチ | `bun tauri dev` |
| テスト   | ``              |



