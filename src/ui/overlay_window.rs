use crate::ui::*;
use windows::Win32::Foundation::HWND;
use winit::dpi::LogicalSize;
use winit::platform::windows::CornerPreference;
use winit::window::{Window, WindowAttributes, WindowId};
use winit::{event_loop::ActiveEventLoop, platform::windows::WindowAttributesExtWindows};

use crate::sys::win32;

pub struct OverlayWindow {
    pub window: Window,
    // 自動消去用のID
    pub id: WindowId,
    pub hwnd: HWND,
    pub display_id: i32,
}

impl OverlayWindow {
    pub fn new(event_loop: &ActiveEventLoop) -> anyhow::Result<Self> {
        let attr = WindowAttributes::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_visible(false)
            .with_active(false)
            .with_skip_taskbar(true)
            .with_no_redirection_bitmap(false)
            .with_theme(None)
            .with_corner_preference(CornerPreference::RoundSmall)
            .with_max_inner_size(LogicalSize::new(300, 100));

        let window = event_loop.create_window(attr)?;
        let id = window.id();
        let hwnd = utils::convert_window_handle(&window)?;

        let (x, y) = {
            let monitor_size = utils::monitor_size(&window).unwrap();
            let x = monitor_size.width as i32 / 2 - 300;
            let y = monitor_size.height as i32 / 2 - 100;

            (x, y)
        };

        win32::set_window_style(hwnd)?;
        win32::set_window_position(hwnd, x, y)?;

        Ok(Self {
            window: window,
            id: id,
            hwnd: hwnd,
            display_id: 0,
        })
    }

    pub fn size(&self) -> (u32, u32) {
        (
            self.window.inner_size().width,
            self.window.inner_size().height,
        )
    }
}
