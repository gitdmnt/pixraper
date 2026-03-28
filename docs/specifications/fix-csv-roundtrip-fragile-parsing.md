# 仕様書: CSV bool パースエラーを明示的エラーに変換し is_illust 型を統一する

## 背景

`load_items` の bool フィールドパースが `unwrap_or(false)` でサイレントに失敗する。CSVが手編集・Excel等で生成された場合（`"TRUE"`, `"1"` 形式など）にエラーなく `false` に置き換えられ、データ破損に気づけない。

加えて `is_illust` フィールドのみ `CsvRow` の型が `Option<String>` になっており、他の bool フィールド（`String`）と型が一致していない。

## 目的

1. `x_restrict`・`ai_type`・`is_illust` のパース失敗を `Err` として返し、サイレント失敗を排除する
2. `CsvRow.is_illust` の型を `String` に統一し、欠損時の挙動を明示する

## スコープ

- `src-tauri/src/csv.rs`（`CsvRow` 構造体・`load_items` のパースロジック・`load_items_from_reader` のテスト用内部関数抽出）

## 型情報

- 書き込み側は `bool.to_string()` で `"true"`/`"false"` を出力（変更なし）
- 読み込み側は `"true"`/`"false"` 形式のみ正規形式として受け付ける

## 関数シグネチャ

- `load_items(path: &str) -> Result<Vec<ItemRecord>, String>` — エラー型は `String`（既存シグネチャを維持）
- `parse_bool_field(value: &str, field_name: &str, row_id: u64) -> Result<bool, String>` — エラー型は `String`（`load_items` と一致するため `?` で伝播可能）

テスト内では `load_items_from_reader` を使用し `std::io::Cursor<&[u8]>` を渡す。`load_items_from_reader<R: std::io::Read>(rdr: csv::Reader<R>) -> Result<Vec<ItemRecord>, String>` を `csv.rs` 内の `#[cfg(test)]` 外にプライベート関数として定義し、`load_items` はそのラッパーとする。テストモジュールからは `super::load_items_from_reader(...)` で呼び出す。

## 前提

- このアプリケーションが保存する CSV には必ず `is_illust` 列が存在する（書き込み側で常に出力）
- 手編集・外部ツール生成の CSV で列欠損が発生した場合はエラーで失敗させることを意図した仕様変更である

## 仕様

### `CsvRow` の `is_illust` 型変更

```rust
// 変更前
is_illust: Option<String>,

// 変更後
is_illust: String,
```

`serde` のデシリアライズ時に列が存在しない場合はエラーとする（`Option` を廃止して CSV 形式の整合性を強制）。

### パースロジックの変更（`load_items` 内）

```rust
// 変更前（サイレント失敗）
is_illust: record.is_illust.is_none_or(|v| v.parse().unwrap_or(true)),
x_restrict: record.x_restrict.parse().unwrap_or(false),
ai_type: record.ai_type.parse().unwrap_or(false),

// 変更後（明示的エラー）
is_illust: record.is_illust.parse::<bool>()
    .map_err(|_| format!("'Is Illust' の値 '{}' を bool に変換できません（行ID: {}）", record.is_illust, record.id))?,
x_restrict: record.x_restrict.parse::<bool>()
    .map_err(|_| format!("'X Restrict' の値 '{}' を bool に変換できません（行ID: {}）", record.x_restrict, record.id))?,
ai_type: record.ai_type.parse::<bool>()
    .map_err(|_| format!("'AI Type' の値 '{}' を bool に変換できません（行ID: {}）", record.ai_type, record.id))?,
```

エラーメッセージには列名・不正な値・行ID を含め、デバッグを容易にする。

## 純粋関数の抽出

テスト容易性のため、パースロジックを純粋関数として抽出する：

```rust
fn parse_bool_field(value: &str, field_name: &str, row_id: u64) -> Result<bool, String> {
    value.parse::<bool>()
        .map_err(|_| format!("'{}' の値 '{}' を bool に変換できません（行ID: {}）", field_name, value, row_id))
}
```

## テスト追加

`csv.rs` の `#[cfg(test)]` ブロックに以下を追加する：

以下のテストはすべて `load_items_from_reader` に `Cursor<&[u8]>` を渡して検証する：

- `load_items_valid_csv_parses_bool_fields_correctly` — `"true"`/`"false"` 形式の CSV で `is_illust`・`x_restrict`・`ai_type` が正しく `bool` に変換されること（ハッピーパス）
- `load_items_missing_is_illust_column_returns_error` — `is_illust` 列が存在しない CSV で `Err` が返ること（serde による列欠損エラーの保証）
- `load_items_invalid_bool_value_returns_error` — `x_restrict` に `"TRUE"` などの不正値を含む CSV で `Err` が返ること（エラー伝播の確認）

`parse_bool_field` の単体テスト（`load_items_from_reader` を介さず直接呼ぶ）：

- `parse_bool_field_accepts_true_false` — `"true"` → `Ok(true)`、`"false"` → `Ok(false)` を確認
- `parse_bool_field_rejects_invalid_value` — `"TRUE"` / `"1"` / `"yes"` が `Err` を返すことを確認
- `parse_bool_field_error_message_contains_field_name_and_value` — エラーメッセージに列名・値・行IDが含まれることを確認

## 受け入れ条件

1. `cargo test` が全パスすること
2. `cargo clippy` で新規警告なし
3. `"true"`/`"false"` 形式の CSV は正常に読み込めること（テストで確認）
4. `"TRUE"` / `"1"` などの不正な値で `Err` が返ること（テストで確認）
5. `CsvRow.is_illust` の型が `String` に変更されていること（コンパイルで保証）
6. `is_illust` 列が欠損した CSV を `load_items` に渡すと `Err` が返ること（テストで確認）

## スコープ外

- 書き込み側の変更（`bool.to_string()` で `"true"`/`"false"` を出力する現状を維持）
- `"TRUE"`, `"1"` など代替形式の許容（今回は正規形式 `"true"`/`"false"` に固定）
- `is_original` フィールド（`CsvRow` で `Option<bool>` として定義されており、CSV の `"true"`/`"false"`/空文字列を serde が正常にデシリアライズできることを既存アプリケーション動作で確認済み。また `is_original` は illust/novel 混在データの都合で `Option` を許容する設計意図があるため `String` への変換対象外）
