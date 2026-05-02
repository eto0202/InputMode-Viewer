use anyhow::Context;
use std::{os::windows::process::CommandExt, process::Command};

// Windowsのプロセス作成フラグ
const CREATE_NO_WINDOW: u32 = 0x08000000;
const DETACHED_PROCESS: u32 = 0x00000008; // コンソールを出さず親から切り離す

pub fn spawn_settings_ui() -> anyhow::Result<()> {
    // 自分自身のEXEファイルの場所
    let exe_path = std::env::current_exe().context("Failed to get current exe path")?;
    let pid = std::process::id(); // 自分のPIDを取得

    // 自分自身をオプション付きで新しく起動
    Command::new(exe_path)
        .arg("--ui")
        .arg("--parent-pid")
        .arg(pid.to_string()) // 引数で渡す
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
        .spawn()
        .context("Failed to spawn Settings UI")?;
    Ok(())
}
