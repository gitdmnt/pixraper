//! `commands` モジュールは、フロントエンド（JS/TS）から呼び出される Tauri コマンドをまとめたサブモジュール群です。
//!
//! 各サブモジュールは単一責任の原則に従い、役割ごとにコマンドを分割しています：
//! - `config`: 設定の取得・更新と永続化
//! - `scraping`: スクレイピングのキュー制御と直接実行
//! - `analytics`: 分析関連のエンドポイント（現状は簡易ラッパー）

pub mod analytics;
pub mod config;
pub mod scraping;
