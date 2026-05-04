use winit::dpi::PhysicalPosition;

use crate::core::app::prelude::*;

pub struct MainWindow {
    pub window: Arc<Window>,
    pub hwnd: HWND,
    pub role: WindowRole,
    pub show_state: ShowState,
    pub l_size: LogicalSize<f32>,
}

impl MainWindow {
    pub fn new(
        el: &ActiveEventLoop,
        role: WindowRole,
        p_pos: PhysicalPosition<f32>,
        p_size: PhysicalSize<f32>,
    ) -> anyhow::Result<Self> {
        let attr = WindowAttributes::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_visible(false)
            .with_active(false)
            .with_skip_taskbar(true)
            .with_no_redirection_bitmap(false)
            .with_position(p_pos)
            .with_inner_size(p_size);

        let window = Arc::new(el.create_window(attr)?);
        let s = window.scale_factor();
        window.set_cursor_hittest(false)?;
        let hwnd = win32::get_hwnd(&window)?;

        Ok(Self {
            window,
            hwnd,
            role,
            show_state: ShowState::Hidden,
            l_size: p_size.to_logical(s),
        })
    }
}
