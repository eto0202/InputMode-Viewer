pub use crate::{
    common::{
        app_config::{AppConfig, RenderingQuality, WindowRole, WindowStyle},
        config,
    },
    core::{
        app::{
            app_core::*,
            calc::{self, VirtualScreen},
            controller::{self, AppState, Message, SharedState},
            prelude::ShowState,
            show_state::*,
            tray,
            window::MainWindow,
        },
        sys::{
            new_renderer::{DCompRenderer, RendererController},
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
    sync::{
        Arc,
        atomic::Ordering,
        mpsc::{self, Receiver, Sender},
    },
    time::{Duration, Instant},
};
pub use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};
pub use windows::Win32::{
    Foundation::{CloseHandle, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM, *},
    Graphics::{
        DirectWrite::DWRITE_TEXT_METRICS,
        Dwm::{
            DWMWA_TRANSITIONS_FORCEDISABLED, DwmExtendFrameIntoClientArea, DwmSetWindowAttribute,
        },
        Gdi::{GetMonitorInfoW, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint},
    },
    System::{
        LibraryLoader::GetModuleHandleW,
        Threading::{
            CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, CreateEventW, CreateWaitableTimerExW,
            GetCurrentThread, INFINITE, SetEvent, SetThreadPriority, SetWaitableTimer,
            THREAD_PRIORITY_HIGHEST, TIMER_ALL_ACCESS, WaitForSingleObject,
        },
        WinRT::{RO_INIT_MULTITHREADED, RO_INIT_TYPE, RoInitialize, RoUninitialize},
    },
    UI::{
        Controls::MARGINS,
        HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
        Input::{RAWINPUTDEVICE, RIDEV_DEVNOTIFY, RIDEV_INPUTSINK, RegisterRawInputDevices},
        Shell::ShellExecuteW,
        WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DispatchMessageW, GetCursorPos, GetSystemMetrics,
            HWND_MESSAGE, MSG, MWMO_INPUTAVAILABLE, MsgWaitForMultipleObjectsEx, PM_REMOVE,
            PeekMessageW, QS_POSTMESSAGE, QS_RAWINPUT, RegisterClassExW, SM_CXVIRTUALSCREEN,
            SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_SHOW, WINDOW_EX_STYLE,
            WINDOW_STYLE, WM_INPUT, WNDCLASSEXW, *,
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

pub use tracing::instrument;
pub use windows_core::{BOOL, HSTRING, PCWSTR, w};
