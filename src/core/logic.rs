use crate::{
    common::config,
    core::{
        app::{
            controller::{self},
            watcher::spawn_config_watcher,
        },
        sys::{
            hooks,
            uia::{cap, mode},
        },
    },
};
use parking_lot::RwLock;
use std::sync::{Arc, mpsc};
use winit::event_loop::{ControlFlow, EventLoop};

pub fn run() -> anyhow::Result<()> {
    // 設定の初期ロード
    let cfg = Arc::new(RwLock::new(config::load_config()));
    log::info!("Initial load of AppConfig successful");

    let el = EventLoop::<controller::Message>::with_user_event().build()?;
    let proxy = el.create_proxy();
    log::info!("Create proxy successful");

    let (tx_uia, rx_uia) = mpsc::channel();
    let (tx_input, rx_input) = mpsc::channel();
    let rx_hooks = hooks::win_hooks();
    log::info!("Hooks channel created successfully");

    // ディスパッチャー
    std::thread::spawn(move || -> anyhow::Result<()> {
        while let Ok(e) = rx_hooks.recv() {
            tx_uia.send(e)?;
            tx_input.send(e)?;
        }
        log::info!("Dispatcher thread successful");
        Ok(())
    });

    let proxy_uia = proxy.clone();
    let proxy_input = proxy.clone();

    mode::mode_thread(proxy_uia, rx_uia);
    cap::cap_thread(proxy_input, rx_input);
    log::info!("Mode thread and Cap thread successful");

    let proxy_watcher = proxy.clone();
    let _watcher = spawn_config_watcher(proxy_watcher)?;
    log::info!("Spawn config watcher successful");

    el.set_control_flow(ControlFlow::Wait);
    let mut app = controller::Controller {
        cfg: Some(Arc::clone(&cfg)),
        ..Default::default()
    };

    if let Err(e) = el.run_app(&mut app) {
        log::error!("Main logic EventLoopError: {:?}", e);
    }
    Ok(())
}
