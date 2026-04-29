pub use crate::{
    common::{
        app_config::{AppConfig, WindowRole, WindowStyle},
        config,
    },
    core::{
        app::{app_core::*, show_state::*, tray, utils},
        sys::{
            renderer::DCompRenderer,
            uia::{cap::InputCapability, text::InputMode},
            win32,
        },
        window::managed::ManagedWindow,
    },
    ui,
};
pub use anyhow::Context;
pub use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
pub use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
pub use tray_icon::{TrayIcon, menu::MenuEvent};
pub use winit::{
    application::ApplicationHandler,
    dpi::{LogicalPosition, LogicalSize},
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    platform::windows::WindowAttributesExtWindows,
    window::{Window, WindowAttributes, WindowId},
};
