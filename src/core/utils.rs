use check_elevation::is_elevated;
use directories::ProjectDirs;
use flexi_logger::{Cleanup, Criterion, FileSpec, Logger, Naming};
use std::collections::HashSet;
use windows::Win32::{
    Foundation::CloseHandle,
    System::{
        ProcessStatus::{EnumProcesses, GetModuleBaseNameW},
        Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
    },
    UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOW},
};
use windows_core::{HSTRING, w};

pub fn init_logger() -> anyhow::Result<()> {
    // 保存先
    let proj_dirs = ProjectDirs::from("com", "", "input_mode_viewer")
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
    let log_dir = proj_dirs.data_local_dir().join("logs");

    // ロガーの初期化
    Logger::try_with_str("debug")?
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

pub fn restart_application(dropping_privileges: bool) {
    // 自らの実行ファイルパスを取得
    let exe_path = std::env::current_exe().expect("Failed to get exe path");

    let result = if dropping_privileges {
        let quoted_path = format!("\"{}\"", exe_path.display());
        let args_str = HSTRING::from(quoted_path);
        unsafe {
            ShellExecuteW(
                None,
                w!("open"),
                w!("explorer.exe"), // 実行ファイルはエクスプローラー
                &args_str,
                None,
                SW_SHOW,
            )
        }
    } else {
        let exe_path_str = HSTRING::from(exe_path.as_os_str());
        let args_str = HSTRING::from("");
        unsafe { ShellExecuteW(None, None, &exe_path_str, &args_str, None, SW_SHOW) }
    };

    if result.0 as usize > 32 {
        log::info!("Restart process spawned successfully. Exiting current process.");
        std::process::exit(0);
    } else {
        log::error!(
            "Failed to restart application via ShellExecuteW: {:?}",
            result
        );
    }
}

pub fn elevated_check() -> bool {
    let current_is_elevated = is_elevated().unwrap_or(false);
    log::info!(
        "Administrator: {:?}",
        if current_is_elevated { "TRUE" } else { "FALSE" }
    );

    current_is_elevated
}

pub fn get_running_process_names() -> Vec<String> {
    let mut process_ids = [0u32; 1024];
    let mut bytes_returned = 0u32;

    unsafe {
        // 実行中の全プロセスIDを取得
        if EnumProcesses(
            process_ids.as_mut_ptr(),
            std::mem::size_of_val(&process_ids) as u32,
            &mut bytes_returned,
        )
        .is_err()
        {
            return Vec::new();
        }

        let count = bytes_returned as usize / std::mem::size_of::<u32>();
        let mut names = HashSet::new();

        for &pid in &process_ids[0..count] {
            if pid == 0 {
                continue;
            } // System Idle Process はスキップ

            // プロセスのハンドルを開く (情報の取得とメモリ読み取り権限)
            let Ok(handle) = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid)
            else {
                continue; // 権限がないプロセスはスキップ
            };

            // モジュール名を取得
            let mut buffer = [0u16; 260];
            let len = GetModuleBaseNameW(handle, None, &mut buffer);

            if len > 0 {
                let name = String::from_utf16_lossy(&buffer[..len as usize]);
                names.insert(name);
            }

            let _ = CloseHandle(handle);
        }

        let mut sorted_names: Vec<String> = names.into_iter().collect();
        sorted_names.sort_by_key(|n| n.to_lowercase());
        sorted_names
    }
}
