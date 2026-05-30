use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;

use crate::{
    common::app_config::{DisplayStyle, PolicyMode, RenderingQuality},
    core::app::{
        mouse_tracking::{ControlMessage, PositionController, spawn_position_thread},
        prelude::*,
    },
};

#[derive(Debug, Clone)]
pub enum Message {
    Cap(InputCapability), // 入力可能性
    Mode(InputMode<'static>),      // 入力タイプ
    ConfigUpdated,        // 設定更新
}

pub struct AppState {
    pub shared: Arc<SharedState>,
    pub cap: InputCapability,
    pub mode: InputMode<'static>,
    pub currently_visible: bool,
    pub show_state: ShowState,
    pub refresh_requested: bool,
}

pub struct SharedState {
    pub displayed: AtomicBool,
    pub floating: Mutex<POINT>,
    pub fixed: Mutex<POINT>,
    pub v_screen: ArcSwap<VirtualScreen>,
    pub metrics: ArcSwap<DWRITE_TEXT_METRICS>,
}

impl AppState {
    pub fn new_shared() -> Arc<SharedState> {
        Arc::new(SharedState {
            displayed: AtomicBool::new(false),
            floating: Mutex::new(POINT::default()),
            fixed: Mutex::new(POINT::default()),
            v_screen: ArcSwap::from_pointee(VirtualScreen::default()),
            metrics: ArcSwap::from_pointee(DWRITE_TEXT_METRICS::default()),
        })
    }
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            state: AppState {
                shared: AppState::new_shared(),
                cap: InputCapability::default(),
                mode: InputMode::default(),
                currently_visible: false,
                show_state: ShowState::Hidden,
                refresh_requested: false,
            },
            core: None,
            cfg: None,
            position_thread_tx: None,
        }
    }
}

pub struct Controller {
    pub state: AppState,
    pub core: Option<AppCore>,
    pub cfg: Option<Arc<ArcSwap<AppConfig>>>, // アプリ設定
    pub position_thread_tx: Option<PositionController>,
}

impl ApplicationHandler<Message> for Controller {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if let Err(e) = self.handle_resumed(el) {
            tracing::error!("Application error during resume: {}", e);
            el.exit();
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let Err(e) = self.handle_window_event(el, event) {
            tracing::error!("Window event error: {}", e);
            el.exit();
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        if let Err(e) = self.handle_about_to_wait(el) {
            tracing::error!("About to wait error: {}", e);
            el.exit();
        }
    }

    fn user_event(&mut self, el: &ActiveEventLoop, msg: Message) {
        if let Err(e) = self.handle_user_event(msg) {
            tracing::error!("User event error: {}", e);
            el.exit();
        }
    }
}

impl Controller {
    fn handle_resumed(&mut self, el: &ActiveEventLoop) -> anyhow::Result<()> {
        if self.core.is_some() {
            return Ok(());
        }

        self.state
            .shared
            .v_screen
            .store(Arc::new(VirtualScreen::new()));

        let cfg = self.cfg.as_ref().context("AppCore missing")?;
        let v_screen = self.state.shared.v_screen.load();
        let core = AppCore::new(el, cfg.clone(), self.state.mode.clone(), v_screen)?;
        tracing::info!("AppCore initialized");

        // マウス監視スレッドの起動
        if self.position_thread_tx.is_none() {
            let pos_ctrl =
                spawn_position_thread(&core, &self.state).expect("Failed to spawn thread");

            self.position_thread_tx = Some(pos_ctrl);
            tracing::info!("Position tracking thread spawned.");
        }

        self.core = Some(core);

        // 初回ウィンドウを描画
        self.redraw_requested().unwrap();
        tracing::info!("First redraw requested");

        Ok(())
    }

    fn handle_window_event(&mut self, el: &ActiveEventLoop, e: WindowEvent) -> anyhow::Result<()> {
        match e {
            WindowEvent::ScaleFactorChanged { .. } => {
                self.state
                    .shared
                    .v_screen
                    .store(Arc::new(VirtualScreen::new()));
            }
            WindowEvent::CloseRequested => {
                el.exit();
            }
            _ => (),
        }
        Ok(())
    }

    fn redraw_requested(&mut self) -> anyhow::Result<()> {
        let core = self.core.as_mut().context("AppCore Missing")?;
        let mode = self.state.mode.clone();
        let metrics = handle_redraw_requested(core, &self.state, mode)?;
        self.state.shared.metrics.store(Arc::new(metrics));
        Ok(())
    }

    fn handle_about_to_wait(&mut self, el: &ActiveEventLoop) -> anyhow::Result<()> {
        el.set_control_flow(ControlFlow::Wait);
        wait_tray_event(el); // タスクトレイイベント
        Ok(())
    }

    fn handle_user_event(&mut self, msg: Message) -> anyhow::Result<()> {
        let core = self.core.as_mut().context("AppCore Missing")?;
        match msg {
            Message::Cap(cap) => {
                if self.state.cap != cap {
                    self.state.refresh_requested = true;
                }
                self.state.cap = cap;
            }
            Message::Mode(mode) => {
                // モードが変化した時に、ウィンドウサイズを再計算
                if self.state.mode != mode {
                    resize_request(mode.clone(), core)?;
                    self.state.refresh_requested = true;
                }
                self.state.mode = mode;
                let old_displayed = self.state.shared.displayed.load(Ordering::Relaxed);
                self.state
                    .shared
                    .displayed
                    .store(check_displayed(&self.state, core)?, Ordering::Relaxed);

                if self.state.shared.displayed.load(Ordering::Relaxed) {
                    let tx = self
                        .position_thread_tx
                        .as_ref()
                        .context("Missing Position thread")?;
                    if let Err(e) = tx.send(ControlMessage::Refresh) {
                        tracing::error!("Failed to send Refresh: {:?}", e);
                    }
                }

                if old_displayed != self.state.shared.displayed.load(Ordering::Relaxed) {
                    self.state.refresh_requested = true;
                }

                trigger_display(&mut self.state, core, true)?;
                self.redraw_requested()?;
            }
            Message::ConfigUpdated => {
                let old_role = core.cfg.load().active_role;
                let new_cfg = config::load_config();
                config_update(self.cfg.clone(), &new_cfg, core, &self.state)?;

                let new_role = core.cfg.load().active_role;
                if (old_role != new_role) && new_role == WindowRole::Floating {
                    let tx = self
                        .position_thread_tx
                        .as_ref()
                        .context("Missing Position thread")?;
                    if let Err(e) = tx.send(ControlMessage::ResetPosition) {
                        tracing::error!("Failed to send ResetPosition: {:?}", e);
                    }
                }
            }
        }
        Ok(())
    }
}

pub fn trigger_display(
    state: &mut AppState,
    core: &AppCore,
    force_fade_in: bool,
) -> anyhow::Result<()> {
    let displayed = state.shared.displayed.load(Ordering::Relaxed);

    let update = state.show_state.update(displayed);
    let result_fade_in = update || force_fade_in;

    let rendering_quality = core.cfg.load().quality;

    match rendering_quality {
        RenderingQuality::Performance => no_animation(state, core, displayed, result_fade_in)?,
        _ => with_animation(state, core, displayed, result_fade_in)?,
    }

    Ok(())
}

pub fn with_animation(
    state: &mut AppState,
    core: &AppCore,
    displayed: bool,
    result_fade_in: bool,
) -> anyhow::Result<()> {
    let style = AppCore::get_style(&core.cfg, core.mw.role)?;
    let opacity = style.opacity;
    let (auto_hide_enabled, auto_hide_time) = set_auto_hide(core)?;

    if auto_hide_enabled {
        if displayed && state.refresh_requested {
            if result_fade_in {
                core.renderer.auto_hide(opacity, auto_hide_time, false)?;
                state.refresh_requested = false;
            } else {
                core.renderer.auto_hide(opacity, auto_hide_time, true)?;
                state.refresh_requested = false;
            }
        } else if !state.refresh_requested {
            core.renderer.set_opacity(0.0)?;
        } else {
            core.renderer.fade_out()?;
        }
    } else if displayed {
        if result_fade_in {
            core.renderer.fade_in(opacity)?;
        }
    } else {
        core.renderer.fade_out()?;
    }
    Ok(())
}

pub fn no_animation(
    state: &mut AppState,
    core: &AppCore,
    displayed: bool,
    result_fade_in: bool,
) -> anyhow::Result<()> {
    let style = AppCore::get_style(&core.cfg, core.mw.role)?;
    let opacity = style.opacity;
    let (auto_hide_enabled, auto_hide_time) = set_auto_hide(core)?;

    if auto_hide_enabled {
        if displayed && state.refresh_requested {
            if result_fade_in {
                core.renderer
                    .auto_hide_no_animaition(opacity, auto_hide_time)?;
                state.refresh_requested = false;
            }
        } else if !state.refresh_requested {
            core.renderer.set_opacity(0.0)?;
        }
    } else if displayed {
        if result_fade_in {
            core.renderer.set_opacity(opacity)?;
        }
    } else {
        core.renderer.set_opacity(0.0)?;
    }
    Ok(())
}

fn check_displayed(state: &AppState, core: &AppCore) -> anyhow::Result<bool> {
    let cfg = core.cfg.load();

    let displayed = match state.cap {
        InputCapability::No => false,
        InputCapability::Yes => state.mode != InputMode::Unknown,
        InputCapability::Unknown => state.mode.is_on(), // 不明の場合はONの時だけ表示
    };

    let is_always = match cfg.active_role {
        WindowRole::Fixed => cfg.fixed.display_style == DisplayStyle::Always,
        WindowRole::Floating => cfg.floating.display_style == DisplayStyle::Always,
    };

    let displayed = displayed || is_always;

    // 失敗した場合は表示する
    let process = cfg.process_cfg.clone();
    let result = match process.mode {
        PolicyMode::BlackList => {
            if !utils::included_in_running_process(&utils::vec_to_set(process.blacklist.processes))
                .unwrap_or(false)
            {
                displayed
            } else {
                false
            }
        }
        PolicyMode::WhiteList => {
            if utils::included_in_running_process(&utils::vec_to_set(process.whitelist.processes))
                .unwrap_or(true)
            {
                displayed
            } else {
                false
            }
        }
    };

    Ok(result)
}

// ウィンドウサイズを再計算してリサイズ要求
fn resize_request(mode: InputMode, core: &AppCore) -> anyhow::Result<()> {
    let new_size = AppCore::try_resize(&core.cfg, &core.renderer, mode, core.mw.role)?;
    let scale = core.mw.window.scale_factor();
    core.renderer
        .resize(new_size.width as u32, new_size.height as u32, scale)?;
    Ok(())
}

fn config_update(
    current_cfg: Option<Arc<ArcSwap<AppConfig>>>,
    new_cfg: &AppConfig,
    core: &mut AppCore,
    state: &AppState,
) -> anyhow::Result<()> {
    let mut admin_changed = false;
    let mut startup_changed = false;

    if let Some(cfg) = current_cfg {
        let lock = cfg.load();
        admin_changed = lock.administrator != new_cfg.administrator;
        startup_changed = lock.startup != new_cfg.startup;

        cfg.store(Arc::new(new_cfg.clone()));
        tracing::debug!("config updated!");
    }
    // 最新データを直接渡して反映
    apply_config_to_all(core, state, new_cfg)?;
    tracing::debug!("Apply config to all");

    if admin_changed {
        apply_admin_changed(new_cfg)?;
        return Ok(());
    }

    if startup_changed {
        apply_startup_changed(new_cfg)?;
    }
    Ok(())
}

// 再描画を伝播
pub fn apply_config_to_all(
    core: &mut AppCore,
    state: &AppState,
    cfg: &AppConfig,
) -> anyhow::Result<()> {
    // 現在の active_role に基づいてスタイルを取得
    let style = match cfg.active_role {
        WindowRole::Floating => &cfg.floating.style,
        WindowRole::Fixed => &cfg.fixed.style,
    };

    let scale = core.mw.window.scale_factor();

    // Rendererのリソース（色、フォント）を更新
    core.renderer.request_alpha_mode(cfg.transparent);
    core.renderer
        .update_config(style, cfg.transparent, state.mode.clone(), scale)?;
    // サイズの再計算とリサイズ
    let metrics = core
        .renderer
        .calc_metrics(state.mode.clone(), style.text_format)?;
    let p = style.padding;
    let p_size = PhysicalSize::new(
        (metrics.width + p * 2.0).ceil(),
        (metrics.height + p * 2.0).ceil(),
    );

    core.renderer
        .resize(p_size.width as u32, p_size.height as u32, scale)?;

    // テキスト更新
    handle_redraw_requested(core, state, state.mode.clone())?;

    Ok(())
}

fn handle_redraw_requested(
    core: &mut AppCore,
    state: &AppState,
    mode: InputMode,
) -> anyhow::Result<DWRITE_TEXT_METRICS> {
    let style = AppCore::get_style(&core.cfg, core.mw.role)?;
    let metrics = core.renderer.calc_metrics(mode.clone(), style.text_format)?;
    let (w, h) = (
        metrics.width + style.padding * 2.0,
        metrics.height + style.padding * 2.0,
    );
    let displayed = state.shared.displayed.load(Ordering::Relaxed);

    if displayed {
        let scale = core.mw.window.scale_factor();
        let cfg = core.cfg.load();

        core.renderer
            .draw(mode, &style, w, h, scale, cfg.transparent)?;
    }
    Ok(metrics)
}

pub fn set_auto_hide(core: &AppCore) -> anyhow::Result<(bool, f32)> {
    let cfg = core.cfg.load();
    let (auto_hide_enabled, auto_hide_time) = match cfg.active_role {
        WindowRole::Fixed => {
            let auto_hide_enable = cfg.fixed.auto_hide.enabled;
            let auto_hide_time = cfg.fixed.auto_hide.time;
            (auto_hide_enable, auto_hide_time)
        }
        WindowRole::Floating => {
            let auto_hide_enable = cfg.floating.auto_hide.enabled;
            let auto_hide_time = cfg.floating.auto_hide.time;
            (auto_hide_enable, auto_hide_time)
        }
    };
    Ok((auto_hide_enabled, auto_hide_time))
}

fn wait_tray_event(el: &ActiveEventLoop) {
    if let Ok(e) = MenuEvent::receiver().try_recv() {
        match e.id.as_ref() {
            tray::ID_QUIT => el.exit(),
            tray::ID_RESTART => {
                // 権限降格は行わずそのまま再起動
                // もし管理者権限に変更があった場合、再起動時に昇格/降格処理が行われる
                utils::restart_application(false);
            }
            tray::ID_SETTING => {
                let _ = ui::spawn::spawn_settings_ui();
            }
            _ => {}
        }
    };
}

fn apply_admin_changed(new_cfg: &AppConfig) -> anyhow::Result<()> {
    tracing::info!("Administrator setting changed. Restarting...");
    if new_cfg.administrator != utils::elevated_check() {
        // 権限降格
        tracing::info!("Dropping privileges via explorer.exe...");
        utils::restart_application(true);
    } else {
        utils::restart_application(false);
    }
    Ok(())
}

fn apply_startup_changed(new_cfg: &AppConfig) -> anyhow::Result<()> {
    if utils::elevated_check() {
        if new_cfg.startup {
            tracing::info!("Syncing startup task imme diately (Admin mode)");
            run::register_startup_task(true)?;
        } else {
            run::unregister_startup_task()?;
        }
    } else if new_cfg.startup {
        // タスク登録（管理者権限が必要）のため昇格再起動が必要
        tracing::info!("Startup enabled in normal mode. Restarting for elevation...");
        utils::restart_application(false);
    } else {
        tracing::info!("Startup disabled in normal mode. Restarting for elevation...");
        utils::restart_application(false);
    }
    Ok(())
}
