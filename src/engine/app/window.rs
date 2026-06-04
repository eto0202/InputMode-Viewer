use std::sync::Arc;

use anyhow::Context;
use tracing::instrument;
use windows::Win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, RegisterClassExW, WINDOW_EX_STYLE, WINDOW_STYLE,
        WNDCLASSEXW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
    },
};
use windows_core::{PCWSTR, w};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event_loop::ActiveEventLoop,
    platform::windows::WindowAttributesExtWindows,
    window::{Window, WindowAttributes},
};

use crate::{common::WindowRole, engine::utils};

pub struct MainWindow {
    pub window: Arc<Window>,
    pub hwnd: HWND,
    pub role: WindowRole,
}

impl MainWindow {
    #[instrument(skip(el, bytes))]
    pub fn new(
        el: &ActiveEventLoop,
        role: WindowRole,
        p_pos: PhysicalPosition<f32>,
        p_size: PhysicalSize<f32>,
        bytes: &[u8],
    ) -> anyhow::Result<Self> {
        // バックグラウンドプロセスにするため
        // ダミーウィンドウを作成し with_owner_window を設定
        let parent_hwnd = create_dummy_parent()
            .context("Failed to create dummy parent window for background process behavior")?;
        tracing::debug!(?parent_hwnd, "Dummy parent window created");

        let raw =
            utils::decode_to_rgba(bytes).context("Failed to decode tray icon image (icon.png)")?;
        let icon = utils::to_winit_icon(raw).context("Failed to create winit icon")?;

        let attr = WindowAttributes::default()
            .with_owner_window(parent_hwnd.0 as isize)
            .with_title("InputMode-Viewer")
            .with_decorations(false)
            .with_transparent(true)
            .with_visible(false)
            .with_active(false)
            .with_skip_taskbar(true)
            .with_no_redirection_bitmap(false)
            .with_position(p_pos)
            .with_inner_size(p_size)
            .with_window_icon(Some(icon));

        let window = el
            .create_window(attr)
            .context("Failed to create winit window instance")?;
        let window = Arc::new(window);

        window
            .set_cursor_hittest(false)
            .context("Failed to disable cursor hit-testing (click-through)")?;

        let hwnd = utils::get_hwnd(&window).context("Failed to retrieve HWND from winit window")?;

        tracing::info!(?hwnd, "MainWindow initialized successfully");
        Ok(Self { window, hwnd, role })
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

#[instrument]
pub fn create_dummy_parent() -> anyhow::Result<HWND> {
    unsafe {
        let instance =
            GetModuleHandleW(None).context("Failed to get module handle for dummy window")?;
        let class_name = w!("DummyParentClass");

        // ウィンドウクラスの登録
        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };

        // RegisterClassExW は、二回目以降の呼び出しで「既に存在します」というエラー（1410）を返す
        if RegisterClassExW(&wnd_class) == 0 {
            let err = windows::core::Error::from_thread();
            // 1410 = ERROR_CLASS_ALREADY_EXISTS
            if err.code().0 as u32 != 1410 {
                tracing::warn!(error = %err, "RegisterClassExW failed for dummy window class");
            }
        }

        // バックグラウンドプロセスにするためにはWS_VISIBLEが必要っぽい
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0),
            class_name,
            w!(""),
            WINDOW_STYLE(WS_POPUP.0 | WS_VISIBLE.0),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(HINSTANCE(instance.0)),
            None,
        )
        .context("Win32 API: CreateWindowExW failed for dummy parent")?;

        if hwnd.is_invalid() {
            anyhow::bail!("CreateWindowExW returned a null HWND");
        }

        Ok(hwnd)
    }
}
