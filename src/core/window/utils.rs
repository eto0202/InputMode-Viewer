use winit::{dpi::LogicalSize, window::Window};

// 画面サイズを取得
pub fn monitor_size(window: &Window) -> Option<LogicalSize<f64>> {
    if let Some(monitor) = window.current_monitor() {
        let size = monitor.size().to_logical::<f64>(monitor.scale_factor());
        Some(size)
    } else {
        None
    }
}
