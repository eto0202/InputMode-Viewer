use crate::{
    core::app::{calculation::VirtualScreen, prelude::*},
    guard_opt, guard_res,
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
    pub cfg: Option<Arc<RwLock<AppConfig>>>, // アプリ設定
}

pub struct AppState {
    pub cap: InputCapability,
    pub mode: InputMode,
    pub displayed: bool,
    pub v_screen: VirtualScreen,
    pub floating: POINT,
    pub fixed: POINT,
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

    fn window_event(&mut self, el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if let Err(e) = self.handle_window_event(el, id, event) {
            log::error!("Window_event error during resume: {}", e);
            el.exit();
        }
    }

    // 特にイベントがない時に何をするかを決定
    // メインウィンドウが表示されている間は常に最新のマウス位置を追いかける
    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        // タスクトレイイベント
        if let Ok(e) = MenuEvent::receiver().try_recv() {
            match e.id.as_ref() {
                tray::ID_QUIT => el.exit(),
                tray::ID_SETTING => {
                    let _ = ui::spawn::spawn_settings_ui();
                }
                _ => {}
            }
        }

        if self.state.displayed {
            let core = guard_opt!(self.core.as_mut());
            let cfg = core.cfg.read();
            let mut pt = POINT::default();
            let _ = unsafe { GetCursorPos(&mut pt) };

            match cfg.active_role {
                WindowRole::Floating => {
                    let o = cfg.floating.offset;
                    let (x, y) = (self.state.v_screen.x, self.state.v_screen.y);

                    let _ = core.renderer.mouse_tracking(
                        self.state.floating.x - x + o.x,
                        self.state.floating.y - y + o.y,
                        pt.x - x + o.x,
                        pt.y - y + o.y,
                    );
                    (self.state.floating.x, self.state.floating.y) = (pt.x, pt.y);
                }
                WindowRole::Fixed => {
                    let (info, scale) = guard_res!(calculation::monitor_info(pt));
                    if let Ok((x, y)) = calculation::calc_fixed_position(
                        core.mw.l_size,
                        &cfg.fixed.position,
                        cfg.fixed.margin,
                        &info,
                        scale,
                    ) {
                        let _ = core.renderer.set_position(
                            (x - self.state.v_screen.x) as f32,
                            (y - self.state.v_screen.y) as f32,
                        );
                        (self.state.fixed.x, self.state.fixed.y) = (x, y);
                    }
                }
            }
            core.mw.window.request_redraw();
        }
    }

    fn user_event(&mut self, _el: &ActiveEventLoop, msg: Message) {
        match msg {
            Message::Cap(cap) => {
                self.state.cap = cap;
            }
            Message::Mode(mode) => {
                // モードが変化した時に、ウィンドウサイズを再計算してリサイズ要求
                if self.state.mode != mode {
                    let core = guard_opt!(self.core.as_mut());

                    if let Ok(new_size) =
                        AppCore::try_resize(&core.cfg, &core.renderer, mode, core.mw.role)
                    {
                        let _ = core
                            .renderer
                            .resize(new_size.width as u32, new_size.height as u32);
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

    fn handle_window_event(
        &mut self,
        el: &ActiveEventLoop,
        id: WindowId,
        event: WindowEvent,
    ) -> anyhow::Result<()> {
        let core = self.core.as_mut().context("AppCore missing")?;

        // プロキシウィンドウのイベントなら無視
        if id == core.proxy_window.id() {
            // CloseRequestedなら処理
            if let WindowEvent::CloseRequested = event {
                el.exit();
            }
            return Ok(());
        }

        // 表示判定
        match event {
            WindowEvent::RedrawRequested => {
                let style = AppCore::get_style(&core.cfg, core.mw.role)?;
                let (w, h) = core.renderer.calc_metrics(self.state.mode)?;
                let (w, h) = (w + style.padding * 2.0, h + style.padding * 2.0);
                let is_animation = core.mw.show_state.update(self.state.displayed);

                if self.state.displayed {
                    core.renderer
                        .draw(self.state.mode, &style, w, h, style.padding)?;

                    if is_animation {
                        core.renderer.fade_in(style.opacity)?
                    }
                } else {
                    // 非表示なら画面外に飛ばし透明に
                    core.renderer.set_opacity(0.0)?;
                }
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
        if let Ok((w, h)) = core.renderer.calc_metrics(self.state.mode) {
            let p = style.padding;
            let p_size = PhysicalSize::new((w + p * 2.0).ceil(), (h + p * 2.0).ceil());

            core.renderer
                .resize(p_size.width as u32, p_size.height as u32)?;
            core.mw.window.request_redraw();
        }

        core.mw.window.request_redraw();

        Ok(())
    }
}
