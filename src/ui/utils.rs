use crate::common::{app_config::AppConfig, config};
use anyhow::Context;
use check_elevation::is_elevated;
use gpui::App;
use std::{env, process};
use windows::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOW};
use windows_core::{HSTRING, w};

pub fn restart_as_admin_for_gpui(cx: &mut App) -> anyhow::Result<()> {
    let current_is_elevated = is_elevated().unwrap_or(false);
    // 権限状態の同期と昇格チェック
    if current_is_elevated {
        log::info!("Running as administrator.");
    } else {
        // 現在一般権限で、設定では管理者として実行となっている場合
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
            AppConfig::global_mut(cx).administrator = false;
            config::save_config(AppConfig::global(cx))?;
        }

        log::info!("Not running as administrator.");
    }
    Ok(())
}
