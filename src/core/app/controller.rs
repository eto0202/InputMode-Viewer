use crate::{
    common::app_config::{DisplayStyle, PolicyMode},
    core::app::prelude::*,
};

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Cap(InputCapability), // 入力可能性
    Mode(InputMode),      // 入力タイプ
    ConfigUpdated,        // 設定更新
}

pub struct Controller {
    pub state: AppState,
    pub core: Option<AppCore>,
    pub cfg: Option<Arc<ArcSwap<AppConfig>>>, // アプリ設定
}

pub struct AppState {
    pub cap: InputCapability,
    pub mode: InputMode,
    pub displayed: bool,
    pub refresh_requested: bool,
    pub show_state: ShowState,
    pub v_screen: VirtualScreen,
    pub floating: POINT,
    pub fixed: POINT,
    pub metrics: DWRITE_TEXT_METRICS,
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            state: AppState {
                cap: InputCapability::default(),
                mode: InputMode::default(),
                displayed: false,
                refresh_requested: false,
                show_state: ShowState::Hidden,
                v_screen: VirtualScreen::default(),
                floating: POINT::default(),
                fixed: POINT::default(),
                metrics: DWRITE_TEXT_METRICS::default(),
            },
            core: None,
            cfg: None,
        }
    }
}

impl ApplicationHandler<Message> for Controller {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if let Err(e) = self.handle_resumed(el) {
            log::error!("Application error during resume: {}", e);
            el.exit();
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let Err(e) = self.handle_window_event(el, event) {
            log::error!("Window event error: {}", e);
            el.exit();
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        if let Err(e) = self.handle_about_to_wait(el) {
            log::error!("About to wait error: {}", e);
            el.exit();
        }
    }

    fn user_event(&mut self, el: &ActiveEventLoop, msg: Message) {
        if let Err(e) = self.handle_user_event(msg) {
            log::error!("User event error: {}", e);
            el.exit();
        }
    }
}

impl Controller {
    fn handle_resumed(&mut self, el: &ActiveEventLoop) -> anyhow::Result<()> {
        if self.core.is_some() {
            return Ok(());
        }

        self.state.v_screen = VirtualScreen::new();

        let cfg = self.cfg.as_ref().context("AppCore missing")?;
        let core = AppCore::new(el, cfg.clone(), self.state.mode, self.state.v_screen)?;
        log::info!("AppCore initialized");

        // ウィンドウを描画
        core.renderer.set_opacity(0.0)?;
        core.mw.window.request_redraw();

        self.core = Some(core);

        Ok(())
    }

    fn handle_window_event(&mut self, el: &ActiveEventLoop, e: WindowEvent) -> anyhow::Result<()> {
        match e {
            WindowEvent::RedrawRequested => {
                let core = self.core.as_mut().context("AppCore missing")?;
                let displayed = self.state.displayed;
                let mode = self.state.mode;
                let style = AppCore::get_style(&core.cfg, core.mw.role)?;
                let metrics =
                    handle_redraw_requested(core, &mut self.state, style, displayed, mode)?;
                self.state.metrics = metrics;
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                self.state.v_screen = VirtualScreen::new();
            }
            WindowEvent::CloseRequested => {
                el.exit();
            }
            _ => (),
        }
        Ok(())
    }

    fn handle_about_to_wait(&mut self, el: &ActiveEventLoop) -> anyhow::Result<()> {
        el.set_control_flow(ControlFlow::Wait);
        wait_tray_event(el); // タスクトレイイベント

        let core = self.core.as_ref().context("AppCore Missing")?;
        let cfg = core.cfg.load();
        let is_always = match cfg.active_role {
            WindowRole::Fixed => cfg.fixed.display_style == DisplayStyle::Always,
            WindowRole::Floating => cfg.floating.display_style == DisplayStyle::Always,
        };

        if !self.state.displayed && !is_always {
            return Ok(());
        }

        let mut pt = POINT::default();
        unsafe { GetCursorPos(&mut pt) }?;

        match cfg.active_role {
            WindowRole::Floating => {
                set_pos_floating(core, cfg, &self.state, pt)?;
                self.state.floating = pt; // 現在のマウス座標を保存
            }
            WindowRole::Fixed => {
                let pos = set_pos_fixed(core, cfg, &self.state, pt)?;
                self.state.fixed = pos;
            }
        }
        core.mw.window.request_redraw();

        Ok(())
    }

    fn handle_user_event(&mut self, msg: Message) -> anyhow::Result<()> {
        match msg {
            Message::Cap(cap) => {
                self.state.cap = cap;
            }
            Message::Mode(mode) => {
                // モードが変化した時に、ウィンドウサイズを再計算してリサイズ要求
                let core = self.core.as_ref().context("AppCore Missing")?;
                if self.state.mode != mode {
                    resize_request(mode, core)?;
                    self.state.refresh_requested = true; // リフレッシュ要求
                }
                self.state.mode = mode;
                let old_displayed = self.state.displayed;
                self.state.displayed = check_displayed(&self.state, core)?;

                // 非表示から表示に変更する場合（新規フェードイン）
                if !old_displayed && self.state.displayed {
                    self.state.refresh_requested = true;
                }
            }
            Message::ConfigUpdated => {
                let core = self.core.as_mut().context("AppCore Missing")?;
                let new_cfg = config::load_config();
                config_update(self.cfg.clone(), &new_cfg, core, &self.state)?;
            }
        }
        Ok(())
    }
}

fn check_displayed(state: &AppState, core: &AppCore) -> anyhow::Result<bool> {
    let cfg = core.cfg.load().process_cfg.clone();

    let displayed = match state.cap {
        InputCapability::No => false,
        InputCapability::Yes => state.mode != InputMode::Unknown,
        InputCapability::Unknown => state.mode.is_on(), // 不明の場合はONの時だけ表示
    };

    // 失敗した場合は表示する
    let result = match cfg.mode {
        PolicyMode::BlackList => {
            if !utils::included_in_running_process(&utils::vec_to_set(cfg.blacklist.processes))
                .unwrap_or(false)
            {
                displayed
            } else {
                false
            }
        }
        PolicyMode::WhiteList => {
            if utils::included_in_running_process(&utils::vec_to_set(cfg.whitelist.processes))
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
    core.renderer
        .resize(new_size.width as u32, new_size.height as u32)?;
    core.mw.window.request_redraw();

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
        log::debug!("config updated!");
    }
    // 最新データを直接渡して反映
    apply_config_to_all(core, state, new_cfg)?;
    log::debug!("Apply config to all");

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

    // Rendererのリソース（色、フォント）を更新
    core.renderer.update_config(style)?;
    // サイズの再計算とリサイズ
    let metrics = core.renderer.calc_metrics(state.mode, style.text_style)?;
    let p = style.padding;
    let p_size = PhysicalSize::new(
        (metrics.width + p * 2.0).ceil(),
        (metrics.height + p * 2.0).ceil(),
    );
    core.renderer
        .resize(p_size.width as u32, p_size.height as u32)?;
    core.mw.window.request_redraw();

    Ok(())
}

fn handle_redraw_requested(
    core: &mut AppCore,
    state: &mut AppState,
    style: WindowStyle,
    displayed: bool,
    mode: InputMode,
) -> anyhow::Result<DWRITE_TEXT_METRICS> {
    let metrics = core.renderer.calc_metrics(mode, style.text_style)?;
    let (w, h) = (
        metrics.width + style.padding * 2.0,
        metrics.height + style.padding * 2.0,
    );
    let cfg = core.cfg.load();
    let (is_always, auto_hide_enabled, auto_hide_time) = match cfg.active_role {
        WindowRole::Fixed => {
            let is_always = cfg.fixed.display_style == DisplayStyle::Always;
            let auto_hide_enable = cfg.fixed.auto_hide.enabled;
            let auto_hide_time = cfg.fixed.auto_hide.time;
            (is_always, auto_hide_enable, auto_hide_time)
        }
        WindowRole::Floating => {
            let is_always = cfg.floating.display_style == DisplayStyle::Always;
            let auto_hide_enable = cfg.floating.auto_hide.enabled;
            let auto_hide_time = cfg.floating.auto_hide.time;
            (is_always, auto_hide_enable, auto_hide_time)
        }
    };

    // displayed: 今表示すべきかどうか
    // is_always: 常に表示設定か
    // auto_hide_enabled: 自動非表示が設定でONか
    let should_show = displayed || is_always;
    let action = state
        .show_state
        .update(should_show, state.refresh_requested);
    state.refresh_requested = false;

    if should_show {
        core.renderer.draw(mode, &style, w, h, style.padding)?;

        log::debug!("RENDER: Visible path");
        if is_always {
            core.renderer.set_opacity(style.opacity)?;
        } else {
            match action {
                AnimationAction::StartFadeIn => {
                    if auto_hide_enabled {
                        log::debug!("RENDER: Start AutoHide");
                        core.renderer
                            .auto_hide(style.opacity, auto_hide_time, false)?;
                    } else {
                        core.renderer.fade_in(style.opacity)?;
                    }
                }
                AnimationAction::Refresh => {
                    if auto_hide_enabled {
                        log::debug!("RENDER: Start AutoHide Refresh");
                        core.renderer
                            .auto_hide(style.opacity, auto_hide_time, true)?;
                    } else {
                        // 常に表示モードなら、Refreshが来ても何もしない
                    }
                }
                _ => {
                    if !auto_hide_enabled {
                        core.renderer.set_opacity(style.opacity)?;
                    }
                }
            }
        }
    } else {
        log::debug!("RENDER: Hide path (Set 0.0)");
        core.renderer.set_opacity(0.0)?;
    }
    Ok(metrics)
}

fn set_pos_floating(
    core: &AppCore,
    cfg: Guard<Arc<AppConfig>>,
    state: &AppState,
    pt: POINT,
) -> anyhow::Result<()> {
    let o = cfg.floating.offset;
    let f = cfg.floating.frequency;
    let v_screen = state.v_screen;

    core.renderer.mouse_tracking(
        state.floating.x - v_screen.x + o.x,
        state.floating.y - v_screen.y + o.y,
        pt.x - v_screen.x + o.x,
        pt.y - v_screen.y + o.y,
        f,
    )?;
    Ok(())
}

fn set_pos_fixed(
    core: &AppCore,
    cfg: Guard<Arc<AppConfig>>,
    state: &AppState,
    pt: POINT,
) -> anyhow::Result<POINT> {
    let (info, s) = calc::monitor_info(pt)?;
    let pos = calc::fixed_position(
        state.metrics,
        &cfg.fixed.pos,
        cfg.fixed.margin,
        cfg.fixed.style.padding,
        info,
        s,
    )?;
    // DComp の SetOffset はウィンドウの左上を基準とした相対座標で計算
    // 画面全体を透明なウィンドウで覆っているため、- 仮想スクリーンのx,y軸
    core.renderer.set_position(
        (pos.x - state.v_screen.x) as f32,
        (pos.y - state.v_screen.y) as f32,
    )?;

    Ok(pos)
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
    log::info!("Administrator setting changed. Restarting...");
    if new_cfg.administrator != utils::elevated_check() {
        // 権限降格
        log::info!("Dropping privileges via explorer.exe...");
        utils::restart_application(true);
    } else {
        utils::restart_application(false);
    }
    Ok(())
}

fn apply_startup_changed(new_cfg: &AppConfig) -> anyhow::Result<()> {
    if utils::elevated_check() {
        if new_cfg.startup {
            log::info!("Syncing startup task imme diately (Admin mode)");
            run::register_startup_task(true)?;
        } else {
            run::unregister_startup_task()?;
        }
    } else if new_cfg.startup {
        // タスク登録（管理者権限が必要）のため昇格再起動が必要
        log::info!("Startup enabled in normal mode. Restarting for elevation...");
        utils::restart_application(false);
    } else {
        log::info!("Startup disabled in normal mode. Restarting for elevation...");
        utils::restart_application(false);
    }
    Ok(())
}
