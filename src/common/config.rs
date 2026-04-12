use crate::common::app_config::AppConfig;
use directories::ProjectDirs;
use std::{fs, io::Write, path::PathBuf};

pub fn get_config_path() -> PathBuf {
    let project_dirs =
        ProjectDirs::from("com", "", "input_mode_viewer").expect("Failed to get AppData directory");

    let config_dirs = project_dirs.config_dir();

    // フォルダが無ければ作成
    fs::create_dir_all(config_dirs).ok();
    config_dirs.join("config.toml")
}

pub fn load_config() -> AppConfig {
    let path = get_config_path();

    // 初回起動時はデフォルト値を保存
    if !path.exists() {
        let default_config = AppConfig::default();
        if let Err(e) = save_config(&default_config) {
            eprintln!("Failed to create default config: {:?}", e);
        }
        return default_config;
    }

    // ファイルがある場合は読み込み
    match fs::read_to_string(&path) {
        Ok(content) => toml::from_str(&content).unwrap_or_else(|_| {
            eprintln!("Config parse error. Using default.");
            AppConfig::default()
        }),
        Err(_) => AppConfig::default(),
    }
}

pub fn save_config(config: &AppConfig) -> anyhow::Result<()> {
    let path = get_config_path();
    let tmp_path = path.with_extension("toml.tmp");

    // シリアライズ
    let contents = toml::to_string_pretty(config)?;

    // 一時ファイルに書き込み
    // 確実に書き込むためにスコープを分ける
    {
        let mut file = fs::File::create(&tmp_path)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?; // OSのバッファを物理ディスクに強制フラッシュ
    }

    // リネーム
    // 同じドライブ内であればこの操作はアトミック
    fs::rename(&tmp_path, &path)?;
    Ok(())
}
