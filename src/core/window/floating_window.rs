use crate::core::sys::win32;
use windows::Win32::Foundation::HWND;
use winit::dpi::LogicalSize;
use winit::platform::windows::CornerPreference;
use winit::window::{Window, WindowAttributes, WindowId};
use winit::{event_loop::ActiveEventLoop, platform::windows::WindowAttributesExtWindows};

pub struct FloatingWindow {
    pub window: Window,
    // 自動消去用のID
    pub id: WindowId,
    pub hwnd: HWND,
    pub display_id: i32,
}

impl FloatingWindow {
    pub fn new(event_loop: &ActiveEventLoop) -> anyhow::Result<Self> {
        let attr = WindowAttributes::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_visible(false)
            .with_active(false)
            .with_skip_taskbar(true)
            .with_no_redirection_bitmap(false)
            .with_theme(None)
            .with_corner_preference(CornerPreference::Round)
            .with_max_inner_size(LogicalSize::new(100, 30));

        let window = event_loop.create_window(attr)?;
        let id = window.id();
        let hwnd = win32::get_hwnd(&window)?;

        win32::set_window_style(hwnd)?;

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
