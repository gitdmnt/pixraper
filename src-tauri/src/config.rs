use serde::{Deserialize, Serialize};
use std::fs;
use tauri::Manager;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CookieProfile {
    pub id: String,
    pub name: String,
    pub cookies: String,
    pub is_valid: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub cookies: Option<String>,
    pub output: Option<String>,
    pub scraping_interval_min_millis: u64,
    pub scraping_interval_max_millis: u64,
    #[serde(default)]
    pub cookie_profiles: Vec<CookieProfile>,
    pub active_profile_id: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            cookies: None,
            output: None,
            scraping_interval_min_millis: 1000,
            scraping_interval_max_millis: 2000,
            cookie_profiles: vec![],
            active_profile_id: None,
        }
    }
}

pub fn load_config(app_handle: &tauri::AppHandle) -> Config {
    let config_dir = match app_handle.path().app_config_dir() {
        Ok(dir) => dir,
        Err(_) => {
            eprintln!("config ディレクトリの解決に失敗しました");
            return Config::default();
        }
    };

    let path = config_dir.join("Config.toml");
    println!("Loading config from {:?}", path);

    match fs::read_to_string(path) {
        Ok(s) => match toml::from_str::<Config>(&s) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("config TOML のパースに失敗しました: {}", e);
                Config::default()
            }
        },
        Err(_) => {
            eprintln!("config TOML の読み込みに失敗しました");
            let config = Config::default();
            // create default config file
            if let Ok(toml_str) = toml::to_string_pretty(&config) {
                // ディレクトリが存在しない場合に再帰的に作成
                if fs::create_dir_all(&config_dir).is_ok() {
                    let _ = fs::write(config_dir.join("Config.toml"), toml_str);
                }
            }
            config
        }
    }
}

pub fn save_config(app_handle: &tauri::AppHandle, config: &Config) -> Result<(), std::io::Error> {
    let config_dir = match app_handle.path().app_config_dir() {
        Ok(dir) => dir,
        Err(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Config directory not found",
            ));
        }
    };

    // ディレクトリが存在しない場合に再帰的に作成
    fs::create_dir_all(&config_dir)?;

    let path = config_dir.join("Config.toml");

    // 更新されたConfigをTOML文字列に変換して書き込む
    let new_toml_str = toml::to_string_pretty(config).unwrap();
    fs::write(path, new_toml_str)?;

    Ok(())
}
