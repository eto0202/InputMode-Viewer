use crate::{core::app::prelude::*, guard_opt};
use windows::Win32::System::Threading::WaitForSingleObject;

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
    pub mx: i32, // マウス座標
    pub my: i32,
    pub wx: i32, // ウィンドウ座標
    pub wy: i32,
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            state: AppState {
                cap: InputCapability::Unknown,
                mode: InputMode::Unknown,
                displayed: false,
                mx: 0,
                my: 0,
                wx: 0,
                wy: 0,
            },
            core: None,
            cfg: None,
        }
    }
}

impl ApplicationHandler<Message> for Controller {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if let Err(e) = self.handle_resumed(el) {
            eprintln!("Application error during resume: {}", e);
            el.exit();
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if let Err(e) = self.handle_window_event(el, id, event) {
            eprintln!("Window_event error during resume: {}", e);
            el.exit();
        }
    }

    // 特にイベントがない時に何をするかを決定
    // メインウィンドウが表示されている間は常に最新のマウス位置を追いかける
    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        let core = guard_opt!(self.core.as_mut());
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
            let cfg = core.cfg.read();
            match cfg.active_role {
                WindowRole::Floating => {
                    el.set_control_flow(winit::event_loop::ControlFlow::Poll);
                    unsafe {
                        let _ = WaitForSingleObject(core.renderer.waitable_object, 1000);
                    };
                    let (x, y) = utils::set_predicted_position(
                        core.mw.hwnd,
                        self.state.mx,
                        self.state.my,
                        core.mw.window.scale_factor(),
                    );
                    (self.state.mx, self.state.my) = (x, y);
                }
                WindowRole::Fixed => {
                    el.set_control_flow(winit::event_loop::ControlFlow::Wait);
                    if let Ok((x, y)) = utils::calc_fixed_position(
                        core.mw.l_size.width,
                        core.mw.l_size.height,
                        &cfg.fixed.position,
                        cfg.fixed.margin,
                    ) {
                        let _ = win32::set_window_position(core.mw.hwnd, x, y);
                        (self.state.wx, self.state.wy) = (x, y);
                    }
                }
            }
            core.mw.window.request_redraw();
        } else {
            el.set_control_flow(winit::event_loop::ControlFlow::Wait);
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
                        core.mw.l_size = new_size;
                        let _ = core.mw.window.request_inner_size(new_size);
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
                    println!("config updated!");
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

        let cfg = self.cfg.as_ref().context("Config is loaded at startup")?;

        let core = AppCore::new(el, cfg.clone(), self.state.mode)?;
        println!("AppCore initialized!");

        // ウィンドウを描画
        core.renderer.set_opacity(0.0)?;
        win32::set_window_style(core.mw.hwnd)?;

        let _ = core.mw.window.request_inner_size(core.mw.l_size);
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
                let (w, h) = (core.mw.l_size.width, core.mw.l_size.height);
                let (opacity, is_animating) = core.mw.show_state.update(
                    Duration::from_millis(160),
                    self.state.displayed,
                    style.opacity,
                );

                if self.state.displayed {
                    core.renderer.set_opacity(opacity)?;
                    core.renderer.draw(self.state.mode, w, h, style.padding)?;

                    // アニメーション中のみ再描画を予約
                    if is_animating {
                        core.mw.window.request_redraw();
                    }
                } else {
                    // 非表示なら画面外に飛ばし透明に
                    core.renderer.set_opacity(0.0)?;
                    win32::set_window_position(core.mw.hwnd, -10000, -10000)?;
                }
            }
            WindowEvent::CloseRequested => {
                el.exit();
            }
            WindowEvent::Resized(p_size) => {
                // OSが確実にサイズ変更を完了したタイミング
                // ここで p_size を使って IDXGISwapChain::ResizeBuffers を呼ぶ
                core.renderer.resize(p_size.width, p_size.height)?;
                core.mw.window.request_redraw();
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
            let final_size = LogicalSize::new((w + p * 2.0).ceil(), (h + p * 2.0).ceil());

            core.mw.l_size = final_size;

            let _ = core.mw.window.request_inner_size(final_size);
        }

        core.mw.window.request_redraw();

        Ok(())
    }
}
