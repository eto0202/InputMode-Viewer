pub use crate::{
    common::{
        app_config::{AppConfig, WindowRole, WindowStyle},
        config,
    },
    core::{
        app::{
            app_core::*,
            calc::{self, VirtualScreen},
            controller::{self, Message},
            prelude::ShowState,
            show_state::*,
            tray,
            window::MainWindow,
        },
        sys::{
            new_renderer::DCompRenderer,
            uia::{cap::InputCapability, text::InputMode},
            win_style,
        },
        utils,
    },
    run, ui,
};

pub use anyhow::Context;
pub use arc_swap::{ArcSwap, Guard};
pub use notify::{Error, Event, EventKind, Watcher};
pub use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
pub use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};
pub use windows::Win32::{
    Foundation::{HWND, POINT},
    Graphics::{
        DirectWrite::DWRITE_TEXT_METRICS,
        Gdi::{GetMonitorInfoW, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint},
    },
    System::Threading::WaitForSingleObject,
    UI::{
        HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
        WindowsAndMessaging::{
            GetCursorPos, GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
            SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
        },
    },
};
pub use winit::{
    application::ApplicationHandler,
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize, Position},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy},
    platform::windows::WindowAttributesExtWindows,
    window::{Window, WindowAttributes, WindowId},
};
