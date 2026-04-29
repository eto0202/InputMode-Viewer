use crate::{core::sys::win32, ui::window};
use gpui::*;
use gpui_component::Root;
use gpui_component_assets::Assets;
use windows::Win32::{
    Foundation::CloseHandle,
    System::Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
};
use windows::Win32::{
    Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HANDLE},
    System::Threading::CreateMutexW,
};
use windows_core::PCWSTR;

pub fn run(parent_pid: Option<u32>) -> anyhow::Result<()> {
    // ユニークな名前でMutexを作成
    // すでに存在する場合は、既存のウィンドウを最前面に
    let _instance = match SingleInstance::new("SettingsUI_Mutex") {
        Some(inst) => inst,
        None => {
            return Ok(());
        }
    };

    // 親プロセスの監視スレッドを開始
    if let Some(pid) = parent_pid {
        spawn_parent_monitor(pid);
    }

    let options = WindowOptions {
        focus: true,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            // ウィンドウ位置
            // TODO: モニターサイズからトレイメニュー付近を指定
            // TODO: 最終位置を記憶
            origin: Point::new(px(700.0), px(250.0)),
            size: size(px(800.0), px(600.0)),
        })),
        window_min_size: Some(size(px(600.0), px(400.0))),
        ..Default::default()
    };

    Application::new().with_assets(Assets).run(move |cx| {
        gpui_component::init(cx);

        if let Ok(handle) = cx.open_window(options, |w, cx| {
            let v = cx.new(|cx| window::SettingsWindow::new(w, cx));
            cx.new(|cx| Root::new(v, w, cx))
        }) {
            let _ = handle.update(cx, |_, w, _| {
                let hwnd = win32::get_hwnd(&w).ok()?;
                win32::set_always_on_top(hwnd, true).ok()?;
                Some(())
            });
        }
    });

    Ok(())
}

fn spawn_parent_monitor(parent_pid: u32) {
    std::thread::spawn(move || {
        unsafe {
            // プロセスのハンドルを取得
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, parent_pid);
            if let Ok(h) = handle {
                loop {
                    // プロセスが終了していないかチェック
                    let mut exit_code: u32 = 0;
                    // 259 = STILL_ACTIVE
                    if GetExitCodeProcess(h, &mut exit_code).is_err() || exit_code != 259 {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
                let _ = CloseHandle(h);
            }
        }
        // 親がいなくなったら自分も終了
        std::process::exit(0);
    });
}

struct SingleInstance {
    handle: HANDLE,
}

impl SingleInstance {
    fn new(name: &str) -> Option<Self> {
        // 名前をUTF-16に変換
        let mut name_u16: Vec<u16> = name.encode_utf16().collect();
        name_u16.push(0);

        unsafe {
            // Mutexを作成
            let handle = CreateMutexW(None, false, PCWSTR(name_u16.as_ptr())).ok()?;

            // すでに存在していた場合はエラーコードをセット
            if GetLastError() == ERROR_ALREADY_EXISTS {
                return None;
            }

            Some(Self { handle })
        }
    }
}

// プロセス終了時にハンドルを閉じる
impl Drop for SingleInstance {
    fn drop(&mut self) {
        unsafe {
            use windows::Win32::Foundation::CloseHandle;
            let _ = CloseHandle(self.handle);
        }
    }
}
