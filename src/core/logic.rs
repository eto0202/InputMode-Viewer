use crate::{
    common::config,
    core::{
        app::{
            controller::{self},
            watcher::spawn_config_watcher,
        },
        sys::{
            hooks::{self, AppEvent},
            uia::{cap, mode},
        },
    },
};
use arc_swap::ArcSwap;
use std::sync::{
    Arc,
    mpsc::{self, Receiver, Sender},
};
use winit::event_loop::{ControlFlow, EventLoop};

pub fn run() -> anyhow::Result<()> {
    // 設定の初期ロード
    let cfg = Arc::new(ArcSwap::from_pointee(config::load_config()));

    tracing::info!("Initial load of AppConfig successful");

    let el = EventLoop::<controller::Message>::with_user_event().build()?;
    let proxy = el.create_proxy();
    tracing::info!("Create proxy successful");

    let (tx_mode, rx_mode) = mpsc::channel();
    let (tx_cap, rx_cap) = mpsc::channel();

    let rx_hooks = hooks::win_hooks();
    tracing::info!("Hooks channel created successfully");

    set_dispatcher(rx_hooks, tx_mode, tx_cap)?;
    tracing::info!("Dispatcher thread successful");

    let proxy_mode = proxy.clone();
    let proxy_cap = proxy.clone();

    mode::mode_thread(proxy_mode, rx_mode);
    cap::cap_thread(proxy_cap, rx_cap);
    tracing::info!("Mode and Cap thread and Cap thread successful");

    let proxy_watcher = proxy.clone();
    let _watcher = spawn_config_watcher(proxy_watcher)?;
    tracing::info!("Spawn config watcher successful");

    el.set_control_flow(ControlFlow::Wait);
    let mut app = controller::Controller {
        cfg: Some(cfg),
        ..Default::default()
    };

    if let Err(e) = el.run_app(&mut app) {
        tracing::error!("Main logic EventLoopError: {:?}", e);
    }
    Ok(())
}

fn set_dispatcher(
    rx_hooks: Receiver<AppEvent>,
    tx_mode: Sender<AppEvent>,
    tx_cap: Sender<AppEvent>,
) -> anyhow::Result<()> {
    std::thread::spawn(move || -> anyhow::Result<()> {
        while let Ok(e) = rx_hooks.recv() {
            tx_mode.send(e)?;
            tx_cap.send(e)?;
        }
        Ok(())
    });
    Ok(())
}
