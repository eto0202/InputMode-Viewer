use windows::Win32::Graphics::DirectWrite::DWRITE_TEXT_METRICS;

use crate::core::app::{calc::VirtualScreen, prelude::*};

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Cap(InputCapability), // 入力可能性
    Mode(InputMode),      // 入力タイプ
    ConfigUpdated,        // 設定更新
}

pub struct Controller {
    pub state: AppState,
    pub core: Option<AppCore>,
    pub cfg: Option<Arc<RwLock<AppConfig>>>, // アプリ設定
}

pub struct AppState {
    pub cap: InputCapability,
    pub mode: InputMode,
    pub displayed: bool,
    pub v_screen: VirtualScreen,
    pub floating: POINT,
    pub fixed: POINT,
    pub metrics: DWRITE_TEXT_METRICS,
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            state: AppState {
                cap: InputCapability::Unknown,
                mode: InputMode::Unknown,
                displayed: false,
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
        let core = self.core.as_mut().context("AppCore missing")?;
        // 表示判定
        match e {
            WindowEvent::RedrawRequested => {
                let style = AppCore::get_style(&core.cfg, core.mw.role)?;
                let is_animation = core.mw.show_state.is_animation(self.state.displayed);
                let metrics = core.renderer.calc_metrics(self.state.mode)?;
                let (w, h) = (
                    metrics.width + style.padding * 2.0,
                    metrics.height + style.padding * 2.0,
                );

                if self.state.displayed {
                    core.renderer
                        .draw(self.state.mode, &style, w, h, style.padding)?;

                    if is_animation {
                        core.renderer.fade_in(style.opacity)?;
                    }
                } else {
                    core.renderer.set_opacity(0.0)?;
                }
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
        if self.state.displayed {
            let core = self.core.as_ref().context("AppCore Missing")?;
            let cfg = core.cfg.read();
            let mut pt = POINT::default();
            unsafe { GetCursorPos(&mut pt) }?;

            match cfg.active_role {
                WindowRole::Floating => {
                    let o = cfg.floating.offset;
                    let v_screen = self.state.v_screen;
                    core.renderer.mouse_tracking(
                        self.state.floating.x - v_screen.x + o.x,
                        self.state.floating.y - v_screen.y + o.y,
                        pt.x - v_screen.x + o.x,
                        pt.y - v_screen.y + o.y,
                    )?;
                    self.state.floating = pt;
                }
                WindowRole::Fixed => {
                    let (info, s) = calc::monitor_info(pt)?;
                    let pos = calc::fixed_position(
                        self.state.metrics,
                        &cfg.fixed.pos,
                        cfg.fixed.margin,
                        cfg.fixed.style.padding,
                        info,
                        s,
                    )?;
                    // DCompの SetOffset はウィンドウの左上を0として計算する
                    core.renderer.set_position(
                        (pos.x - self.state.v_screen.x) as f32,
                        (pos.y - self.state.v_screen.y) as f32,
                    )?;
                    self.state.fixed = pos;
                }
            }
            core.mw.window.request_redraw();
        }

        // タスクトレイイベント
        if let Ok(e) = MenuEvent::receiver().try_recv() {
            match e.id.as_ref() {
                tray::ID_QUIT => el.exit(),
                tray::ID_SETTING => {
                    let _ = ui::spawn::spawn_settings_ui();
                }
                _ => {}
            }
        };
        Ok(())
    }

    fn handle_user_event(&mut self, msg: Message) -> anyhow::Result<()> {
        match msg {
            Message::Cap(cap) => {
                self.state.cap = cap;
            }
            Message::Mode(mode) => {
                // モードが変化した時に、ウィンドウサイズを再計算してリサイズ要求
                if self.state.mode != mode {
                    let core = self.core.as_ref().context("AppCore Missing")?;

                    if let Ok(new_size) =
                        AppCore::try_resize(&core.cfg, &core.renderer, mode, core.mw.role)
                    {
                        core.renderer
                            .resize(new_size.width as u32, new_size.height as u32)?;
                        core.mw.window.request_redraw();
                    }
                }
                self.state.mode = mode;
                self.state.displayed = match self.state.cap {
                    InputCapability::No => false,
                    InputCapability::Yes => self.state.mode != InputMode::Unknown,
                    InputCapability::Unknown => self.state.mode.is_on(), // 不明の場合はONの時だけ表示
                };
            }
            Message::ConfigUpdated => {
                let new_cfg = config::load_config();

                if let Some(cfg) = &self.cfg {
                    let mut lock = cfg.write();
                    *lock = new_cfg.clone();
                    log::debug!("config updated!");
                }
                // 最新データを直接渡して反映させる
                let _ = self.apply_config_to_all(&new_cfg);
            }
        }
        Ok(())
    }

    // 再描画を伝播
    pub fn apply_config_to_all(&mut self, cfg: &AppConfig) -> anyhow::Result<()> {
        let core = self.core.as_mut().context("AppCore missing")?;

        // 現在の active_role に基づいてスタイルを取得
        let style = match cfg.active_role {
            WindowRole::Floating => &cfg.floating.style,
            WindowRole::Fixed => &cfg.fixed.style,
        };
        // ここに各設定
        // Rendererのリソース（色、フォント）を更新
        core.renderer.update_config(style)?;
        // サイズの再計算とリサイズ
        if let Ok(metrics) = core.renderer.calc_metrics(self.state.mode) {
            let p = style.padding;
            let p_size = PhysicalSize::new(
                (metrics.width + p * 2.0).ceil(),
                (metrics.height + p * 2.0).ceil(),
            );

            core.renderer
                .resize(p_size.width as u32, p_size.height as u32)?;
            core.mw.window.request_redraw();
        }

        core.mw.window.request_redraw();

        Ok(())
    }
}
