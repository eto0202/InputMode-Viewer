use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::Foundation::HWND;
use winit::{dpi::LogicalSize, window::Window};

// ウィンドウハンドルを取得
pub fn convert_window_handle(window: &Window) -> anyhow::Result<HWND> {
    let h = window.window_handle()?.as_raw();
    if let RawWindowHandle::Win32(h) = h {
        Ok(HWND(h.hwnd.get() as _))
    } else {
        unreachable!();
    }
}

// 画面サイズを取得
pub fn monitor_size(window: &Window) -> Option<LogicalSize<f64>> {
    if let Some(monitor) = window.current_monitor() {
        let size = monitor.size().to_logical::<f64>(monitor.scale_factor());
        Some(size)
    } else {
        None
    }
}
