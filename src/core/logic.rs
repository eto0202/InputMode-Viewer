use crate::{
    common::config,
    core::{
        app::controller::{self},
        sys::{
            hooks::{self, AppEvent},
            uia::{cap, mode},
        },
    },
};
use anyhow::Context;
use arc_swap::ArcSwap;
use notify::{Error, Event, EventKind, Watcher};
use std::sync::{
    Arc,
    mpsc::{self, Receiver, Sender},
};
use tracing::instrument;
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};

#[instrument]
pub fn run() -> anyhow::Result<()> {
    // 設定の初期ロード
    let cfg_data = config::load_config();
    tracing::info!("Configuration loaded for main logic");
    let cfg = Arc::new(ArcSwap::from_pointee(cfg_data));

    let el = EventLoop::<controller::Message>::with_user_event()
        .build()
        .context("Failed to build winit EventLoop")?;
    let proxy = el.create_proxy();

    let (tx_mode, rx_mode) = mpsc::channel();
    let (tx_cap, rx_cap) = mpsc::channel();

    let rx_hooks = hooks::win_hooks();
    tracing::info!("Windows event hooks initialized");

    set_dispatcher(rx_hooks, tx_mode, tx_cap).context("Failed to spawn event dispatcher thread")?;

    let proxy_mode = proxy.clone();
    let proxy_cap = proxy.clone();

    // 各スレッドの起動
    mode::mode_thread(proxy_mode, rx_mode);
    cap::cap_thread(proxy_cap, rx_cap);
    tracing::info!("Sub-threads (mode, cap) spawned successfully");

    let proxy_watcher = proxy.clone();
    let _watcher = spawn_config_watcher(proxy_watcher)
        .context("Failed to initialize configuration file watcher")?;

    el.set_control_flow(ControlFlow::Wait);
    let mut app = controller::Controller {
        cfg: Some(cfg),
        ..Default::default()
    };

    if let Err(e) = el.run_app(&mut app) {
        tracing::error!(error = ?e, "Critical error in winit event loop: {:#?}", e);
        return Err(e).context("Event loop execution failed");
    }
    Ok(())
}

#[instrument(skip_all)]
fn set_dispatcher(
    rx_hooks: Receiver<AppEvent>,
    tx_mode: Sender<AppEvent>,
    tx_cap: Sender<AppEvent>,
) -> anyhow::Result<()> {
    // ログにスレッド名を表示するため
    std::thread::Builder::new()
        .name("event_dispatcher".into())
        .spawn(move || -> anyhow::Result<()> {
            tracing::debug!("Event dispatcher thread started");
            while let Ok(e) = rx_hooks.recv() {
                tx_mode
                    .send(e)
                    .context("Failed to dispatch event to mode thread")?;
                tx_cap
                    .send(e)
                    .context("Failed to dispatch event to cap thread")?;
            }
            tracing::warn!("Event dispatcher thread exiting: rx_hooks channel closed");
            Ok(())
        })
        .context("Failed to build dispatcher thread")?;
    Ok(())
}

#[instrument(skip(proxy))]
fn spawn_config_watcher(
    proxy: EventLoopProxy<controller::Message>,
) -> anyhow::Result<impl Watcher> {
    let path = config::get_config_path().context("Failed to determine config file path")?;
    let parent_dir = path
        .parent()
        .context("Config path has no parent directory")?
        .to_path_buf();

    // 親ディレクトリは確実に作成
    std::fs::create_dir_all(&parent_dir)
        .with_context(|| format!("Failed to create config directory: {:?}", parent_dir))?;

    let mut watcher = notify::recommended_watcher(move |res: Result<Event, Error>| match res {
        Ok(e) => {
            // config.tomlが含まれているかチェック
            if e.paths.iter().any(|p| p.ends_with("config.toml")) {
                match e.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {
                        tracing::debug!("Config file change detected, sending reload signal");
                        let _ = proxy.send_event(controller::Message::ConfigUpdated);
                    }
                    _ => {}
                }
            }
        }
        Err(e) => tracing::error!(error = ?e, "Error occurred in file system watcher"),
    })
    .context("Failed to create notify watcher instance")?;

    watcher
        .watch(&parent_dir, notify::RecursiveMode::NonRecursive)
        .with_context(|| format!("Failed to start watching directory: {:?}", parent_dir))?;

    tracing::info!(target_dir = ?parent_dir, "Configuration watcher started successfully");
    Ok(watcher)
}
