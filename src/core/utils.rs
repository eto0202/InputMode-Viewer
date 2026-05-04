use crate::common::{app_config::AppConfig, config};
use anyhow::Context;
use check_elevation::is_elevated;
use directories::ProjectDirs;
use flexi_logger::{Cleanup, Criterion, FileSpec, Logger, Naming};
use std::{env, process};
use windows::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOW};
use windows_core::{HSTRING, w};

pub fn restart_as_admin(cfg: &mut AppConfig) -> anyhow::Result<()> {
    let current_is_elevated = is_elevated().unwrap_or(false);
    // 権限状態の同期と昇格チェック
    if current_is_elevated {
        // 現在管理者なら設定を同期
        if !cfg.administrator {
            cfg.administrator = true;
            config::save_config(cfg)?;
        }
        log::info!("Running as administrator.");
    } else {
        // 現在一般権限で、設定では管理者として実行となっている場合
        if cfg.administrator {
            log::info!("Attempting to elevate privileges...");

            // 自らの実行ファイルパスを取得
            let exe_path = env::current_exe().context("Failed to retrieve the execution path")?;
            let exe_path_str = HSTRING::from(exe_path.as_os_str());

            // 現在の引数（exeパスを除く）を結合して1つの文字列にする
            let args: Vec<String> = env::args().skip(1).collect();
            let args_string = if args.is_empty() {
                HSTRING::new()
            } else {
                HSTRING::from(args.join(" "))
            };

            let result = unsafe {
                ShellExecuteW(
                    None,
                    w!("runas"), // 昇格
                    &exe_path_str,
                    &args_string, // 引数を渡す
                    None,
                    SW_SHOW,
                )
            };

            if result.0 as usize > 32 {
                process::exit(0);
            } else {
                // UACで拒否された場合などはここに来る
                log::warn!("Failed to elevate. Falling back to normal user.");
                // 拒否されたのに設定をtrueのままにすると無限ループになるため、
                // 一時的にfalseにするか、エラーを表示して終了するのが安全
                cfg.administrator = false;
                config::save_config(cfg)?;
            }
        }
        log::info!("Not running as administrator.");
    }
    Ok(())
}

pub fn init_logger() -> anyhow::Result<()> {
    // 保存先
    let proj_dirs = ProjectDirs::from("com", "", "input_mode_viewer")
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
    let log_dir = proj_dirs.data_local_dir().join("logs");

    // ロガーの初期化
    Logger::try_with_str("info")?
        .log_to_file(FileSpec::default().directory(log_dir).basename("app"))
        .rotate(
            Criterion::Size(10 * 1024 * 1024), // 10MBごとに新しいファイルへ
            Naming::Timestamps,
            Cleanup::KeepLogFiles(5), // 最新の3つだけ残して古いのは消す
        )
        .start()?;

    Ok(())
}

// メモリ上のバイト列から画像をデコードしアイコンを生成
// アプリケーション内に画像が保存される
pub fn load_icon(to_include_bytes: &[u8]) -> tray_icon::Icon {
    let img = image::load_from_memory(to_include_bytes)
        .unwrap()
        .into_rgba8();
    let (width, height) = img.dimensions();
    let rgba = img.into_raw();

    tray_icon::Icon::from_rgba(rgba, width, height).unwrap()
}
