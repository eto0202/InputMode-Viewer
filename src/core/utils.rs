use anyhow::Context;
use check_elevation::is_elevated;
use directories::ProjectDirs;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_error::ErrorLayer;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use std::{collections::HashSet, path::Path};
use windows::Win32::{
    Foundation::{CloseHandle, HANDLE, HWND, INVALID_HANDLE_VALUE, MAX_PATH},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        },
        Threading::{
            OpenProcess, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
            QueryFullProcessImageNameW,
        },
    },
    UI::{
        Shell::ShellExecuteW,
        WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId, SW_SHOW},
    },
};
use windows_core::{HSTRING, w};

pub fn init_logger() -> anyhow::Result<WorkerGuard> {
    // 保存先
    let proj_dirs = ProjectDirs::from("com", "", "InputMode-Viewer")
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
    let log_dir = proj_dirs.data_local_dir().join("logs");

    // ディレクトリがない場合は作成しておく
    std::fs::create_dir_all(&log_dir).context("Failed to create log directory")?;

    // ファイル出力の設定
    // ファイル名のプレフィックスなどを設定
    let file_appender = tracing_appender::rolling::daily(&log_dir, "app.log");

    // 非ブロックで書き込む
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // ロガーの組み立て
    tracing_subscriber::registry()
        // ログレベルの設定
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")))
        // コンソール出力（標準出力）
        .with(fmt::layer().with_writer(std::io::stdout))
        // ファイル出力
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false) // ファイルには色コードを入れない
                .with_target(true) // どのモジュールからのログか表示
                .with_thread_ids(true), // スレッドIDを表示
        )
        .with(ErrorLayer::default())
        .init();

    Ok(guard)
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
                w!("explorer.exe"), // 実行ファイルはエクスプローラーにし普通権限で再起動
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
        tracing::info!("Restart process spawned successfully. Exiting current process.");
        std::process::exit(0);
    } else {
        tracing::error!(
            "Failed to restart application via ShellExecuteW: {:?}",
            result
        );
    }
}

pub fn elevated_check() -> bool {
    let current_is_elevated = is_elevated().unwrap_or(false);
    tracing::info!(
        "Administrator: {:?}",
        if current_is_elevated { "TRUE" } else { "FALSE" }
    );

    current_is_elevated
}

// 与えられたリストの要素が実行中プロセス一覧に含まれているか否か
pub fn included_in_running_process(black_or_white: &HashSet<String>) -> anyhow::Result<bool> {
    let process_set = get_focused_process_and_children_names()
        .context("Failed to get focused process and children names")?;

    Ok(process_set.iter().any(|name| black_or_white.contains(name)))
}

// 現在フォーカスされているプロセスとその子プロセス名をHashSetに
fn get_focused_process_and_children_names() -> Option<HashSet<String>> {
    let (parent_name, parent_pid) = get_foreground_process_name()?;

    let h_snapshot = create_process_snapshot()?;

    let mut set = HashSet::new();
    set.insert(parent_name); // 親自身の名前も判定対象に

    unsafe {
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        if Process32FirstW(h_snapshot, &mut entry).is_ok() {
            loop {
                // 親プロセスIDが現在のフォアグラウンドプロセスIDと一致するか確認
                if entry.th32ParentProcessID == parent_pid {
                    let name = String::from_utf16_lossy(&entry.szExeFile)
                        .trim_matches(char::from(0))
                        .to_lowercase();

                    if !name.is_empty() {
                        set.insert(name);
                    }
                }
                if Process32NextW(h_snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(h_snapshot);
    }
    Some(set)
}

// 現在フォーカスされているプロセスの名前とidを取得
fn get_foreground_process_name() -> Option<(String, u32)> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == HWND::default() {
            return None;
        }

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }

        let Ok(h_process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return None;
        };

        // 実行ファイルのフルパスを取得
        let mut buffer = [0u16; MAX_PATH as usize];
        let mut size = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            h_process,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut size,
        );

        let _ = CloseHandle(h_process);

        if result.is_ok() {
            // パス全体からファイル名のみを抽出
            let full_path = String::from_utf16_lossy(&buffer[..size as usize]);
            let file_name = Path::new(&full_path).file_name()?.to_str()?.to_lowercase(); // 小文字に統一

            Some((file_name, pid))
        } else {
            None
        }
    }
}

// GUIに表示されるプロセス一覧
pub fn get_running_process_names() -> anyhow::Result<Vec<String>> {
    let h_snapshot = create_process_snapshot().context("Not found process snapshot")?;
    let set = create_process_set(h_snapshot)?;
    unsafe {
        let _ = CloseHandle(h_snapshot);
    }
    let vec = set_to_vec(set);
    Ok(vec)
}

fn create_process_snapshot() -> Option<HANDLE> {
    let h_snapshot = unsafe {
        match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            Ok(h) => h,
            Err(e) => {
                tracing::error!("Failed to create a snapshot: {:?}", e);
                return None;
            }
        }
    };
    if h_snapshot == INVALID_HANDLE_VALUE {
        return None;
    }

    Some(h_snapshot)
}

// 全プロセス名のHashSetを作成
fn create_process_set(h_snapshot: HANDLE) -> anyhow::Result<HashSet<String>> {
    // エントリーの初期化
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut names: HashSet<String> = HashSet::new();

    unsafe {
        // 最初のプロセスを取得
        if Process32FirstW(h_snapshot, &mut entry).is_ok() {
            loop {
                let name = String::from_utf16_lossy(&entry.szExeFile)
                    .trim_matches(char::from(0))
                    .to_lowercase();

                if !name.is_empty() {
                    names.insert(name);
                }

                // なくなればループ終了
                if Process32NextW(h_snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
    }

    Ok(names)
}

// Vec<String> を HashSet<String> に変換する
// 重複を排除し小文字に統一
pub fn vec_to_set(vec: Vec<String>) -> HashSet<String> {
    vec.into_iter()
        .map(|s| s.to_lowercase()) // 小文字化
        .collect() // 重複は自動的に消える
}

// HashSet<String> を Vec<String> に変換する
// GUIライブラリで表示するためにアルファベット順にソートする
pub fn set_to_vec(set: HashSet<String>) -> Vec<String> {
    let mut v: Vec<String> = set.into_iter().collect();
    v.sort(); // リスト表示が見やすくなる
    v
}
