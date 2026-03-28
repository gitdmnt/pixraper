# 仕様書: Worker が Stopped をセットすることでポーリングが途中で止まる問題の修正

## 背景

`Worker::run()` はタスク完了時に自ら `status = Stopped` を設定する。その後 `WorkerFinished → RunNext` でキューに次のアイテムがあれば `status = Running` に戻るが、その一瞬の間にフロントエンドのポーリング（1秒間隔）が走ると `Stopped` と判断してポーリングを止めてしまう。

## 目的

ステータス遷移を `QueryQueueActor` に一元化し、Worker はステータス管理に関与しないようにする。

## スコープ

- `src-tauri/src/scraper/scrape.rs`（Worker の `Stopped` セットを削除）

## 仕様

`Worker::run()` 末尾の以下ブロックを削除する：

```rust
// 削除対象（scrape.rs:158-162）
{
    let mut p = progress.lock().await;
    p.status = ScrapingStatus::Stopped;
}
```

### 変更しない箇所

以下の `Stopped` セットは正当な停止経路であり変更しない：

- `queue.rs:155-156` — `RunNext` 時にトークンがキャンセル済み or なしの場合の停止。キャンセル操作による正常終了パス。
- `queue.rs:214-216` — キューが空になったときの停止。正常完了パス。
- `queue.rs:229-230` — `Stop` コマンド受信時の停止。

## 実行パスへの影響

`queue.rs` には非同期パス（`spawn_workers_async=true`、本番使用）と同期パス（`spawn_workers_async=false`、テスト補助用）の2つがある。両パスとも Worker::run を呼ぶため、削除の効果は両パスに及ぶ。

## テスト方針

既存テスト `simulate_queue_processing_completes_all_items` は `simulate_workers=true` で動作するため `Worker::run` を実行しない。このテストは最終状態（`Stopped`）を確認するが、今回の変更箇所（`scrape.rs:158-162`）を通過しないため、変更の回帰検証としての意義はない。

`simulate_workers=false` で実際の Worker::run を動かすテストはネットワーク API へのアクセスが必要なため追加しない。変更の効果（`Worker::run` 内で `Stopped` を設定しなくなったこと）は受け入れ条件 3 の静的検査（grep）で確認する。

## 受け入れ条件

1. `cargo test` が全パスすること（既存テストは変更箇所を通過しないため、ビルドと他テストの非破壊確認としての位置づけ）
2. `cargo clippy` で新規警告なし
3. `scrape.rs` に `p.status = ScrapingStatus::Stopped` が存在しないこと（`grep "ScrapingStatus::Stopped" scrape.rs` で0件）

## スコープ外

- `queue.rs` の変更
- `WorkerFinished` → `RunNext` フロー全体のリファクタリング
- ネットワークアクセスを伴う統合テストの追加（Worker::run のモック化は別途 Issue として管理する）
