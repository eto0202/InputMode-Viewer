use crate::common::app_config::AppConfig;
use anyhow::Context;
use directories::ProjectDirs;
use std::{
    fs,
    io::Write,
    path::PathBuf,
    sync::{
        LazyLock,
        mpsc::{self, Sender},
    },
    time::Duration,
};
use tracing::instrument;

pub fn get_config_path() -> anyhow::Result<PathBuf> {
    let project_dirs = ProjectDirs::from("com", "", "InputMode-Viewer")
        .context("Failed to get AppData directory")?;

    let config_dirs = project_dirs.config_dir();

    // ディレクトリが無ければ作成
    std::fs::create_dir_all(config_dirs).context("Failed to create log directory")?;

    Ok(config_dirs.join("config.toml"))
}

pub fn load_config() -> AppConfig {
    // パスの取得
    let path = match get_config_path() {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(
                error = ?e,
                fallback = "AppConfig::default()",
                "Could not determine config path"
            );
            return AppConfig::default();
        }
    };
    if !path.exists() {
        let default_config = AppConfig::default();
        if let Err(e) = save_config(&default_config) {
            tracing::error!(
                error = ?e,
                "Failed to save config.toml."
            );
        }
        return default_config;
    }

    // 読み込みとパース
    let res = fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!(e).context("File read failed"))
        .and_then(|content| {
            toml::from_str(&content).map_err(|e| anyhow::anyhow!(e).context("Parse failed"))
        });

    match res {
        Ok(config) => config,
        Err(e) => {
            tracing::warn!(
                error = ?e,
                path = %path.display(), // Display トレイトを使って出力する
                fallback = "AppConfig::default()",
                "Failed to load config, using default values:\n{:#?}", e
            );
            AppConfig::default()
        }
    }
}

#[instrument(skip(config))] // 設定内容は巨大になる可能性があるためログ引数からは外す
pub fn save_config(config: &AppConfig) -> anyhow::Result<()> {
    let path = get_config_path().context("Failed to get config path")?;
    let tmp_path = path.with_extension("toml.tmp");

    // シリアライズ
    let contents = toml::to_string_pretty(config).context("Failed to serialize config to TOML")?;

    // 一時ファイルに書き込み
    // 確実に書き込むためにスコープを分ける
    {
        let mut file = fs::File::create(&tmp_path)
            .with_context(|| format!("Failed to create temp file: {:?}", tmp_path))?;

        file.write_all(contents.as_bytes())
            .context("Failed to write to temp file")?;

        file.sync_all()
            .context("Failed to sync temp file to disk")?; // OSのバッファを物理ディスクに強制フラッシュ
    }

    // リネーム
    // 同じドライブ内であればこの操作はアトミック
    fs::rename(&tmp_path, &path)
        .with_context(|| format!("Failed to rename temp file to {:?}", path))?;

    // どこに保存したか残しておく
    tracing::info!(path = %path.display(), "Configuration saved successfully");
    Ok(())
}

// グローバルな送信機を定義（LazyLockで初回アクセス時にスレッド起動）
static CONFIG_SAVE_TX: LazyLock<Sender<AppConfig>> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel::<AppConfig>();

    std::thread::spawn(move || {
        let mut pending_config: Option<AppConfig> = None;
        let debounce_timeout = Duration::from_millis(1000); // 1秒待機

        loop {
            // 保留中のデータがある場合はタイムアウト付きで待機、ない場合は無限に待機
            let wait_timeout = if pending_config.is_some() {
                debounce_timeout
            } else {
                Duration::MAX
            };

            match rx.recv_timeout(wait_timeout) {
                Ok(new_config) => {
                    // 新しい変更が届いたので、保留してタイマーをリセット
                    pending_config = Some(new_config);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // 1秒間次の変更が来なかったので、ディスクに書き込む
                    if let Some(config) = pending_config.take() {
                        tracing::info!("Debounce timeout: Saving configuration to disk...");
                        if let Err(e) = save_config(&config) {
                            tracing::error!(error = ?e, "Background save failed: {:#?}", e);
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    tracing::info!("Config saver thread shutting down");
                    break;
                }
            }
        }
    });

    tx
});

// 外部から呼ぶためのインターフェース
pub fn request_config_save(config: AppConfig) {
    if let Err(e) = CONFIG_SAVE_TX.send(config) {
        tracing::warn!(error = %e, "Failed to send save request to background thread");
    }
}
