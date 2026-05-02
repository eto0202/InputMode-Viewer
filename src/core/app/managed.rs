use crate::core::app::prelude::*;

pub struct ManagedWindow {
    pub window: Arc<Window>,
    pub hwnd: HWND,
    pub role: WindowRole,
    pub show_state: ShowState,
    pub l_size: LogicalSize<f32>,
}

impl ManagedWindow {
    pub fn new(el: &ActiveEventLoop, role: WindowRole, info: MONITORINFO) -> anyhow::Result<Self> {
        let l_size = LogicalSize::new(
            (info.rcMonitor.right - info.rcMonitor.left) as f32,
            (info.rcMonitor.bottom - info.rcMonitor.top) as f32,
        );
        // 共通の属性定義
        let attr = WindowAttributes::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_visible(false)
            .with_active(false)
            .with_skip_taskbar(true)
            .with_no_redirection_bitmap(false)
            .with_position(Position::Logical(LogicalPosition::new(0.0, 0.0)))
            .with_inner_size(l_size);

        let window = Arc::new(el.create_window(attr)?);
        window.set_cursor_hittest(false)?;
        let hwnd = win32::get_hwnd(&window)?;

        Ok(Self {
            window,
            hwnd,
            role,
            show_state: ShowState::Hidden,
            l_size,
        })
    }
}
