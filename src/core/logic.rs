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
use std::sync::Arc;
use std::sync::{RwLock, mpsc};
use winit::event_loop::{ControlFlow, EventLoop};

pub fn run() -> anyhow::Result<()> {
    // 設定の初期ロード
    let config = Arc::new(RwLock::new(config::load_config()));

    let event_loop = EventLoop::<controller::Message>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();

    let (tx_uia, rx_uia) = mpsc::channel();
    let (tx_input, rx_input) = mpsc::channel();
    let rx_hooks = hooks::win_hooks();

    // ディスパッチャー
    std::thread::spawn(move || -> anyhow::Result<()> {
        while let Ok(event) = rx_hooks.recv() {
            tx_uia.send(event.clone())?;
            tx_input.send(event.clone())?;
        }
        Ok(())
    });

    let proxy_uia = proxy.clone();
    let proxy_input = proxy.clone();

    mode::mode_thread(proxy_uia, rx_uia);
    cap::cap_thread(proxy_input, rx_input);

    let proxy_watcher = proxy.clone();
    let _watcher = spawn_config_watcher(proxy_watcher)?;

    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = controller::Controller {
        config: Some(Arc::clone(&config)),
        ..Default::default()
    };

    event_loop.run_app(&mut app).unwrap();
    Ok(())
}
