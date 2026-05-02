use crate::common::app_config::AppConfig;
use anyhow::Context;
use directories::ProjectDirs;
use std::{fs, io::Write, path::PathBuf};

pub fn get_config_path() -> anyhow::Result<PathBuf> {
    let project_dirs = ProjectDirs::from("com", "", "input_mode_viewer")
        .context("Failed to get AppData directory")?;

    let config_dirs = project_dirs.config_dir();

    // フォルダが無ければ作成
    if let Err(e) = fs::create_dir_all(config_dirs) {
        log::warn!("Folder not found{:?}", e);
    };
    Ok(config_dirs.join("config.toml"))
}

pub fn load_config() -> AppConfig {
    if let Ok(path) = get_config_path() {
        // 初回起動時はデフォルト値を保存
        if !path.exists() {
            let default_config = AppConfig::default();
            if let Err(e) = save_config(&default_config) {
                log::warn!("Failed to create default config: {:?}", e);
            }
            return default_config;
        }

        // ファイルがある場合は読み込み
        match fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content).unwrap_or_else(|e| {
                log::warn!("Config parse error. Using default: {:?}", e);
                AppConfig::default()
            }),
            Err(_) => {
                log::warn!("Failed to read file");
                AppConfig::default()
            }
        }
    } else {
        log::warn!("Failed to get config path");
        AppConfig::default()
    }
}

pub fn save_config(config: &AppConfig) -> anyhow::Result<()> {
    let path = get_config_path()?;
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

    log::debug!("AppConfig saved successfully");
    Ok(())
}
