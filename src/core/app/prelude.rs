pub use crate::{
    common::{
        app_config::{AppConfig, WindowRole, WindowStyle},
        config,
    },
    core::{
        app::{
            app_core::*,
            calculation,
            controller::{self, Message},
            managed::ManagedWindow,
            prelude::ShowState,
            show_state::*,
            tray,
        },
        sys::{
            renderer::DCompRenderer,
            uia::{cap::InputCapability, text::InputMode},
            win32,
        },
    },
    ui,
};
pub use anyhow::Context;
pub use notify::{Error, Event, EventKind, Watcher};
pub use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
pub use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
pub use tray_icon::{TrayIcon, menu::MenuEvent};
pub use windows::Win32::{
    Foundation::{HWND, POINT},
    Graphics::Gdi::{
        GetMonitorInfoW, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
    },
    System::Threading::WaitForSingleObject,
    UI::{
        HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
        WindowsAndMessaging::GetCursorPos,
    },
};
pub use winit::{
    application::ApplicationHandler,
    dpi::{LogicalPosition, LogicalSize, PhysicalSize, Position},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoopProxy},
    platform::windows::WindowAttributesExtWindows,
    window::{Window, WindowAttributes, WindowId},
};
