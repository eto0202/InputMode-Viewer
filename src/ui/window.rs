use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::Foundation::HWND;
use winit::dpi::Size;
use winit::window::{Window, WindowAttributes};
use winit::{event_loop::ActiveEventLoop, platform::windows::WindowAttributesExtWindows};

pub struct MainWindow {
    pub window: Window,
    // 自動消去用のID
    pub display_id: u64,
}

impl MainWindow {
    pub fn new(event_loop: &ActiveEventLoop) -> Self {
        let attr = WindowAttributes::default()
            .with_decorations(false)
            .with_active(false)
            .with_skip_taskbar(true)
            .with_theme(None)
            .with_corner_preference(winit::platform::windows::CornerPreference::Round)
            .with_max_inner_size(Size::Logical(winit::dpi::LogicalSize {
                width: 100f64,
                height: 30f64,
            }));
        let window = event_loop.create_window(attr).unwrap();

        Self {
            window: window,
            display_id: 0,
        }
    }

    pub fn hwnd(&self) -> HWND {
        convert_window_handle(&self.window)
    }
}

fn convert_window_handle(window: &Window) -> HWND {
    let h = window.window_handle().unwrap().as_raw();
    if let RawWindowHandle::Win32(h) = h {
        HWND(h.hwnd.get() as _)
    } else {
        unreachable!();
    }
}
