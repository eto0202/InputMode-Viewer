use crate::{
    common::app_config::{AppConfig, ConfigTheme},
    core::sys::win32,
    ui::window,
};
use anyhow::Context;
use gpui::*;
use gpui_component::{Root, Theme};
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
    let pid = parent_pid.context("Parent pid not found")?;
    spawn_parent_monitor(pid);
    log::info!("Spawn parent monitor successful");

    let options = WindowOptions {
        focus: true,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            // ウィンドウ位置
            // TODO: モニターサイズからトレイメニュー付近を指定
            // TODO: 最終位置を記憶
            origin: Point::new(px(550.0), px(250.0)),
            size: size(px(1000.0), px(700.0)),
        })),
        window_min_size: Some(size(px(800.0), px(400.0))),
        ..Default::default()
    };

    Application::new().with_assets(Assets).run(move |cx| {
        gpui_component::init(cx);

        if let Err(e) = cx.open_window(options, |w, cx| {
            let s_v = cx.new(|cx| window::SettingsWindow::new(w, cx));
            log::info!("Create SettingsWindow successful");
            cx.new(|cx| {
                let root = Root::new(s_v, w, cx);
                log::info!("Create Root successful");

                cx.on_next_frame(w, |_, w, cx| {
                    cx.observe_window_appearance(w, |_, w, cx| {
                        if AppConfig::global(cx).cfg_theme == ConfigTheme::System {
                            let appearance = cx.window_appearance();
                            Theme::change(appearance, Some(w), cx);
                            cx.refresh_windows();
                        }
                    })
                    .detach();

                    let mode = AppConfig::global(cx).cfg_theme;
                    mode.theme_change(cx);

                    match win32::get_hwnd(&w) {
                        Ok(hwnd) => {
                            if let Err(e) = win32::set_always_on_top(hwnd, true) {
                                log::warn!("Failed to set always on top: {:?}", e);
                            };
                        }
                        Err(e) => log::warn!("Failed to get HWND: {:?}", e),
                    }
                });
                root
            })
        }) {
            log::error!("Faild to open window{:?}", e);
        };
    });

    log::info!("Build Application successful");
    Ok(())
}

fn spawn_parent_monitor(parent_pid: u32) {
    std::thread::spawn(move || {
        unsafe {
            // プロセスのハンドルを取得
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, parent_pid);
            match handle {
                Ok(h) => {
                    loop {
                        // プロセスが終了していないかチェック
                        let mut exit_code: u32 = 0;
                        // 259 = STILL_ACTIVE
                        if GetExitCodeProcess(h, &mut exit_code).is_err() || exit_code != 259 {
                            log::warn!("Parent Process Terminated");
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    }
                    let _ = CloseHandle(h);
                }
                Err(e) => {
                    log::error!("Failed to get parent process handle: {:?}", e);
                }
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
