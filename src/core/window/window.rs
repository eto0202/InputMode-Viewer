use std::sync::Arc;

use crate::common::app_config::{WindowRole};
use crate::core::app::prelude::DCompRenderer;
use crate::core::sys::win32;
use windows::Win32::Foundation::HWND;
use winit::dpi::LogicalSize;
use winit::window::{Window, WindowAttributes};
use winit::{event_loop::ActiveEventLoop, platform::windows::WindowAttributesExtWindows};

pub struct ManagedWindow {
    pub window: Arc<Window>,
    pub hwnd: HWND,
    pub role: WindowRole,
    pub render_stack: Option<DCompRenderer>,
    pub current_size: LogicalSize<f32>,
}

impl ManagedWindow {
    pub fn new(el: &ActiveEventLoop, role: WindowRole) -> anyhow::Result<Self> {
        let current_size = LogicalSize::new(1.0, 1.0);
        // 共通の属性定義
        let attr = WindowAttributes::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_visible(false)
            .with_active(false)
            .with_skip_taskbar(true)
            .with_no_redirection_bitmap(false)
            .with_theme(None)
            .with_inner_size(current_size);

        let window = Arc::new(el.create_window(attr)?);
        let hwnd = win32::get_hwnd(&window)?;

        win32::set_window_style(hwnd)?;

        Ok(Self {
            window: window,
            hwnd: hwnd,
            role,
            render_stack: None,
            current_size,
        })
    }
}
