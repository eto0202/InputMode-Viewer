use crate::core::app::prelude::*;

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Cap(InputCapability),
    Mode(InputMode),
    ConfigUpdated,
}

// 全ての部品
pub struct Controller {
    pub state: AppState,
    pub core: Option<AppCore>,
    pub config: Option<Arc<RwLock<AppConfig>>>,
}

pub struct AppState {
    pub last_cap: InputCapability,
    pub last_mode: InputMode,
    pub displayed: bool,
    pub mx: i32,
    pub my: i32,
    pub wx: i32,
    pub wy: i32,
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            state: AppState {
                last_cap: InputCapability::Unknown,
                last_mode: InputMode::Unknown,
                displayed: false,
                mx: 0,
                my: 0,
                wx: 0,
                wy: 0,
            },
            core: None,
            config: None,
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
    // 非表示の時は何か起きるまで寝て待つ設定にしてCPU消費を抑える
    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        let Some(core) = self.core.as_mut() else {
            return;
        };
        // タスクトレイイベント
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id.as_ref() {
                tray::ID_QUIT => el.exit(),
                tray::ID_SETTING => {
                    let _ = ui::spawn::spawn_settings_ui();
                }
                _ => {}
            }
        }

        if self.state.displayed {
            el.set_control_flow(winit::event_loop::ControlFlow::Poll);

            for mw in core.windows.values() {
                if !mw.config_enabled {
                    continue;
                }

                match mw.role {
                    WindowRole::Floating => {
                        let (current_x, current_y) = utils::set_predicted_position(
                            mw.hwnd,
                            self.state.mx,
                            self.state.my,
                            mw.window.scale_factor(),
                        );
                        (self.state.mx, self.state.my) = (current_x, current_y);
                        mw.window.request_redraw();
                    }
                    WindowRole::Fixed => {
                        if let Ok(work_area) = utils::get_work_area(self.state.mx, self.state.my) {
                            let (current_x, current_y) = utils::calc_fixed_position(
                                work_area,
                                mw.l_size.width as u32,
                                mw.l_size.height as u32,
                                &core.config.read().fixed.position,
                                20,
                            );
                            (self.state.wx, self.state.wy) = (current_x, current_y);
                            mw.window.request_redraw();
                        };
                    }
                }
            }
        } else {
            el.set_control_flow(winit::event_loop::ControlFlow::Wait);
        }
    }

    fn user_event(&mut self, _el: &ActiveEventLoop, msg: Message) {
        match msg {
            Message::Cap(cap) => {
                self.state.last_cap = cap;
            }
            Message::Mode(mode) => {
                let old_mode = self.state.last_mode;
                self.state.last_mode = mode;

                self.state.displayed = match self.state.last_cap {
                    InputCapability::No => false,
                    InputCapability::Yes => self.state.last_mode != InputMode::Unknown,
                    InputCapability::Unknown => self.state.last_mode.is_on(), // 不明の場合はONの時だけ表示
                };

                // モードが変化した時に、ウィンドウサイズを再計算してリサイズ要求
                if old_mode != mode {
                    let (Some(core), Some(cfg)) = (self.core.as_mut(), self.config.as_ref()) else {
                        return;
                    };

                    for mw in core.windows.values_mut() {
                        let Some(renderer) = &mw.render_stack else {
                            continue;
                        };
                        if let Ok(new_size) = AppCore::try_resize(cfg, renderer, mode, mw.role) {
                            mw.l_size = new_size;
                            let _ = mw.window.request_inner_size(new_size);
                        }
                    }
                }
            }
            Message::ConfigUpdated => {
                let new_data = config::load_config();

                if let Some(cfg) = self.config.clone() {
                    // 書き込みロックを取得
                    let mut lock = cfg.write();
                    // デリファレンスして中身を丸ごと差し替える
                    *lock = new_data;
                    println!("config updated!");
                    // その後、この最新設定を使ってウィンドウを更新する
                    let _ = self.apply_config_to_all();
                }
            }
        }
    }
}

impl Controller {
    fn handle_resumed(&mut self, el: &ActiveEventLoop) -> anyhow::Result<()> {
        if self.core.is_some() {
            return Ok(());
        }

        let cfg = self
            .config
            .as_ref()
            .context("Config should be loaded at startup")?;

        let mut core = AppCore::new(el, cfg.clone(), self.state.last_mode)?;
        println!("AppCore initialized!");

        // 各ウィンドウを描画
        for mw in core.windows.values_mut() {
            let renderer = mw.render_stack.as_mut().context("Renderer not found")?;

            renderer.set_opacity(0.0)?;
            win32::set_window_style(mw.hwnd)?;

            let _ = mw.window.request_inner_size(mw.l_size);
            mw.window.request_redraw();
        }

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
        let cfg = self.config.as_ref().context("Config missing")?;

        // プロキシウィンドウのイベントなら無視
        if id == core.proxy_window.id() {
            if let WindowEvent::CloseRequested = event {
                el.exit();
            }
            return Ok(());
        }

        // IDから対象のウィンドウを引く
        let mw = match core.windows.get_mut(&id) {
            Some(w) => w,
            None => return Ok(()), // 予期せぬウィンドウIDなら無視
        };
        // ウィンドウが renderer を持っていない場合はスキップ
        let renderer = mw.render_stack.as_mut().context("Renderer not found")?;

        // 表示判定
        match event {
            WindowEvent::RedrawRequested => {
                let style = AppCore::get_style(&cfg, mw.role)?;
                let (width, height) = (mw.l_size.width, mw.l_size.height);
                let (current_opacity, is_animating) = mw.show_state.update(
                    Duration::from_millis(160),
                    self.state.displayed,
                    style.opacity,
                );

                let should_draw = self.state.displayed && mw.config_enabled;
                if should_draw {
                    renderer.set_opacity(current_opacity)?;
                    renderer.draw(self.state.last_mode, width, height, style.padding)?;

                    // アニメーション中のみ再描画を予約
                    if is_animating {
                        mw.window.request_redraw();
                    }
                } else {
                    // 非表示なら画面外に飛ばし透明に
                    renderer.set_opacity(0.0)?;
                    win32::set_window_position(mw.hwnd, -10000, -10000)?;
                }
            }
            WindowEvent::CloseRequested => {
                el.exit();
            }
            WindowEvent::Resized(p_size) => {
                // OSが確実にサイズ変更を完了したタイミング
                // ここで p_size を使って IDXGISwapChain::ResizeBuffers を呼ぶ
                renderer.resize(p_size.width, p_size.height)?;
                mw.window.request_redraw();
            }
            _ => (),
        }

        Ok(())
    }

    // 再描画を伝播
    pub fn apply_config_to_all(&mut self) -> anyhow::Result<()> {
        let core = self.core.as_mut().context("AppCore missing")?;
        let app_cfg = self.config.as_ref().context("Config missing")?;
        let cfg_lock = app_cfg.read();

        for mw in core.windows.values_mut() {
            let renderer = mw.render_stack.as_mut().context("Renderer not found")?;
            let style = AppCore::get_style(&app_cfg, mw.role)?;

            // ここに各設定
            // Rendererのリソース（色、フォント）を更新
            renderer.update_config(&style)?;
            // サイズの再計算とリサイズ
            if let Ok((w, h)) = renderer.calc_metrics(self.state.last_mode) {
                let p = style.padding;
                let final_size = LogicalSize::new((w + p * 2.0).ceil(), (h + p * 2.0).ceil());

                mw.l_size = final_size;

                mw.config_enabled = match mw.role {
                    WindowRole::Floating => cfg_lock.floating.enabled,
                    WindowRole::Fixed => cfg_lock.fixed.enabled,
                };

                let _ = mw.window.request_inner_size(final_size);
                mw.window.request_redraw();
            }
        }

        Ok(())
    }
}
