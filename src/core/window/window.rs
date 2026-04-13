use std::sync::Arc;

use crate::common::app_config::{WindowRole, WindowStyle};
use crate::core::sys::win32;
use windows::Win32::Foundation::HWND;
use winit::platform::windows::CornerPreference;
use winit::window::{Window, WindowAttributes};
use winit::{event_loop::ActiveEventLoop, platform::windows::WindowAttributesExtWindows};

pub struct ManagedWindow {
    pub window: Arc<Window>,
    pub hwnd: HWND,
    pub role: WindowRole,
}

impl ManagedWindow {
    pub fn new(
        el: &ActiveEventLoop,
        role: WindowRole,
        _style: &WindowStyle,
    ) -> anyhow::Result<Self> {
        // 共通の属性定義
        let attr = WindowAttributes::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_visible(false)
            .with_active(false)
            .with_skip_taskbar(true)
            .with_no_redirection_bitmap(false)
            .with_theme(None)
            .with_corner_preference(CornerPreference::Round);

        let window = Arc::new(el.create_window(attr)?);
        let hwnd = win32::get_hwnd(&window)?;

        win32::set_window_style(hwnd)?;

        Ok(Self {
            window: window,
            hwnd: hwnd,
            role,
        })
    }

    pub fn size(&self) -> (u32, u32) {
        (
            self.window.inner_size().width,
            self.window.inner_size().height,
        )
    }
}
