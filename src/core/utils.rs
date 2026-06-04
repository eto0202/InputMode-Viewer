use anyhow::Context;
use std::{collections::HashSet, path::Path};
use tracing::instrument;
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
    UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
};

pub struct RawIcon {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

// メモリ上のバイナリからRGBAピクセルデータにデコード
#[instrument(skip(bytes))]
pub fn decode_to_rgba(bytes: &[u8]) -> anyhow::Result<RawIcon> {
    let img = image::load_from_memory(bytes)
        .context(
            "Failed to decode image from memory. Format may be unsupported or data corrupted.",
        )?
        .into_rgba8();

    let (width, height) = img.dimensions();
    let rgba = img.into_raw();

    tracing::debug!(width, height, "Image decoded to RGBA pixels successfully");
    Ok(RawIcon { rgba, width, height })
}

// tray_icon クレート用のアイコン
pub fn to_tray_icon(raw: RawIcon) -> anyhow::Result<tray_icon::Icon> {
    tray_icon::Icon::from_rgba(raw.rgba, raw.width, raw.height).with_context(|| {
        format!(
            "Failed to create tray_icon::Icon ({}x{})",
            raw.width, raw.height
        )
    })
}
// winit クレート用のアイコン
pub fn to_winit_icon(raw: RawIcon) -> anyhow::Result<winit::window::Icon> {
    winit::window::Icon::from_rgba(raw.rgba, raw.width, raw.height).with_context(|| {
        format!(
            "Failed to create winit::window::Icon ({}x{})",
            raw.width, raw.height
        )
    })
}

#[instrument]
pub fn elevated_check() -> bool {
    let current_is_elevated = check_elevation::is_elevated().unwrap_or(false);
    tracing::info!(
        is_admin = current_is_elevated,
        "Checked process elevation status"
    );

    current_is_elevated
}

// 与えられたリストの要素が実行中プロセス一覧に含まれているか否か
#[instrument(skip(black_or_white))]
pub fn included_in_running_process(black_or_white: &HashSet<String>) -> anyhow::Result<bool> {
    let process_set = get_focused_process_and_children_names()
        .context("Failed to identify focused process and its children")?;

    let is_included = process_set.iter().any(|name| black_or_white.contains(name));
    tracing::debug!(
        is_included,
        "Checked if focused process is in the filter list"
    );

    Ok(is_included)
}

// 現在フォーカスされているプロセスとその子プロセス名をHashSetに
#[instrument]
fn get_focused_process_and_children_names() -> anyhow::Result<HashSet<String>> {
    let (parent_name, parent_pid) =
        get_foreground_process_name().context("Could not get foreground process info")?;

    let h_snapshot = create_process_snapshot()
        .context("Failed to capture system process snapshot for child search")?;

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
    tracing::debug!(
        count = set.len(),
        "Retrieved focused process and its children"
    );
    Ok(set)
}

// 現在フォーカスされているプロセスの名前とidを取得
#[instrument]
fn get_foreground_process_name() -> anyhow::Result<(String, u32)> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == HWND::default() {
            anyhow::bail!("No foreground window detected");
        }

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            anyhow::bail!("Failed to retrieve process ID for the foreground window");
        }

        let h_process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
            .with_context(|| format!("Failed to open process with PID: {}", pid))?;

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
            let file_name = Path::new(&full_path)
                .file_name()
                .and_then(|n| n.to_str())
                .context("Failed to extract file name from process path")?
                .to_lowercase(); // 小文字に統一

            Ok((file_name, pid))
        } else {
            anyhow::bail!("Failed to query image name for PID: {}", pid);
        }
    }
}

// GUIに表示されるプロセス一覧
#[instrument]
pub fn get_running_process_names() -> anyhow::Result<Vec<String>> {
    let h_snapshot =
        create_process_snapshot().context("Failed to capture system process snapshot")?;
    let set =
        create_process_set(h_snapshot).context("Failed to build process name set from snapshot")?;
    unsafe {
        let _ = CloseHandle(h_snapshot);
    }
    let vec = set_to_vec(set);

    tracing::info!(
        process_count = vec.len(),
        "Retrieved all running process names"
    );
    Ok(vec)
}

fn create_process_snapshot() -> anyhow::Result<HANDLE> {
    unsafe {
        let h_snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
            .context("Win32 API: CreateToolhelp32Snapshot failed")?;
        if h_snapshot == INVALID_HANDLE_VALUE {
            anyhow::bail!("CreateToolhelp32Snapshot returned an invalid handle");
        }

        Ok(h_snapshot)
    }
}

// 全プロセス名のHashSetを作成
#[instrument(skip(h_snapshot))]
fn create_process_set(h_snapshot: HANDLE) -> anyhow::Result<HashSet<String>> {
    // エントリーの初期化
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut names: HashSet<String> = HashSet::new();

    unsafe {
        // 最初のプロセスを取得
        // // ここで失敗するのはスナップショットが不正かOSレベルの異常
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
        } else {
            // 最初のプロセスすら取得できない場合はエラー
            anyhow::bail!("Failed to retrieve the first process from the snapshot");
        }
    }

    if names.is_empty() {
        // リストが空なのは不自然（自分自身は動いているはず）
        tracing::warn!("Process list is empty. This may indicate an issue with snapshot access.");
    } else {
        // 取得した個数をデバッグログに残す
        tracing::debug!(
            count = names.len(),
            "Successfully extracted process names from snapshot"
        );
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
