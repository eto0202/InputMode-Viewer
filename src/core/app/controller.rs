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
    pub is_visible: bool,
    pub show_state: ShowState,
    pub last_raw_mouse_x: i32,
    pub last_raw_mouse_y: i32,
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            state: AppState {
                last_cap: InputCapability::Unknown,
                last_mode: InputMode::Unknown,
                is_visible: false,
                show_state: ShowState::Hidden,
                last_raw_mouse_x: 0,
                last_raw_mouse_y: 0,
            },
            core: None,
            config: None,
        }
    }
}

impl ApplicationHandler<Message> for Controller {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(e) = self.handle_resumed(event_loop) {
            eprintln!("Application error during resume: {}", e);
            event_loop.exit();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if let Err(e) = self.handle_window_event(event_loop, id, event) {
            eprintln!("Window_event error during resume: {}", e);
            event_loop.exit();
        }
    }

    // 特にイベントがない時に何をするかを決定
    // メインウィンドウが表示されている間は常に最新のマウス位置を追いかける
    // 非表示の時は何か起きるまで寝て待つ設定にしてCPU消費を抑える
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(core) = self.core.as_mut() else {
            return;
        };
        // タスクトレイイベント
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id.as_ref() {
                tray::ID_QUIT => event_loop.exit(),
                tray::ID_SETTING => {
                    let _ = ui::spawn::spawn_settings_ui();
                }
                _ => {}
            }
        }

        if self.state.is_visible {
            let floating = core.find_by_role(WindowRole::Floating);
            let window_date = if let Ok(mw) = floating {
                let hwnd = mw.hwnd;
                let scale = mw.window.scale_factor();
                mw.window.request_redraw();
                Some((hwnd, scale))
            } else {
                None
            };

            if let Some((hwnd, scale)) = window_date {
                event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
                let (current_x, current_y) = utils::set_predicted_position(
                    hwnd,
                    self.state.last_raw_mouse_x,
                    self.state.last_raw_mouse_y,
                    scale,
                );
                self.state.last_raw_mouse_x = current_x;
                self.state.last_raw_mouse_y = current_y;
            }
        } else {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, msg: Message) {
        match msg {
            Message::Cap(cap) => {
                self.state.last_cap = cap;
            }
            Message::Mode(mode) => {
                let old_mode = self.state.last_mode;
                self.state.last_mode = mode;

                if self.state.last_cap != InputCapability::No {
                    self.state.is_visible = true;
                    println!("last_cap: {:?}", self.state.last_cap);
                    println!("last_mode: {:?}", self.state.last_mode);
                }

                // モードが変化した時に、ウィンドウサイズを再計算してリサイズ要求
                if old_mode != mode {
                    let (Some(core), Some(cfg)) = (self.core.as_mut(), self.config.as_ref()) else {
                        return;
                    };

                    for mw in core.windows.values_mut() {
                        if let Some(renderer) = &mw.render_stack {
                            if let Ok(new_size) = AppCore::try_resize(cfg, renderer, mode, mw.role)
                            {
                                mw.current_size = new_size;
                                let _ = mw.window.request_inner_size(new_size);
                            }
                        }
                    }
                }
            }
            Message::ConfigUpdated => {
                let new_data = config::load_config();

                if let Some(cfg) = &self.config {
                    // 書き込みロックを取得
                    let mut lock = cfg.write();
                    // デリファレンスして中身を丸ごと差し替える
                    *lock = new_data;
                    println!("config updated!");
                    // その後、この最新設定を使ってウィンドウを更新する
                }

                let _ = self.apply_config_to_all();
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

        let core = AppCore::new(el, cfg.clone(), self.state.last_mode)?;
        self.core = Some(core);
        println!("AppCore initialized!");

        let Some(core) = self.core.as_mut() else {
            return Ok(());
        };
        // 各ウィンドウを描画
        for mw in core.windows.values_mut() {
            let renderer = mw
                .render_stack
                .as_mut()
                .context("Renderer not found on ManagedWindow")?;

            renderer.set_opacity(0.0)?;

            // 設定から有効・無効を取得
            let is_enabled = {
                let cfg = cfg.read();
                match mw.role {
                    WindowRole::Floating => cfg.floating.enabled,
                    WindowRole::Fixed => cfg.fixed.enabled,
                }
            };
            mw.window.set_visible(is_enabled);

            win32::set_window_style(mw.hwnd)?;

            let _ = mw.window.request_inner_size(mw.current_size);
            mw.window.request_redraw();
        }

        Ok(())
    }

    fn handle_window_event(
        &mut self,
        el: &ActiveEventLoop,
        id: WindowId,
        event: WindowEvent,
    ) -> anyhow::Result<()> {
        let (Some(core), Some(cfg)) = (self.core.as_mut(), self.config.as_ref()) else {
            return Ok(());
        };

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
        let renderer = mw
            .render_stack
            .as_mut()
            .context("Renderer not found on ManagedWindow")?;

        // 表示判定
        match event {
            WindowEvent::RedrawRequested => {
                let should_show = match self.state.last_cap {
                    InputCapability::No => false,
                    InputCapability::Yes => self.state.last_mode != InputMode::Unknown,
                    InputCapability::Unknown => self.state.last_mode.is_on(), // 不明の場合はONの時だけ表示
                };

                // ウィンドウのロールに合わせたスタイル取得
                let style = AppCore::get_style(&cfg, mw.role)?;
                let (p, target_opacity) = (style.padding, style.opacity);
                // 現在保持しているサイズを取得
                let width = mw.current_size.width;
                let height = mw.current_size.height;
                // フェードイン時間
                let fade_duration = Duration::from_millis(160);

                // フェードイン設定
                let (current_opacity, is_animating) =
                    self.state
                        .show_state
                        .update(fade_duration, should_show, target_opacity);

                if should_show {
                    // 描画と表示
                    renderer.set_opacity(current_opacity)?;
                    renderer.draw(self.state.last_mode, width, height, p)?;

                    // アニメーション中のみ再描画を予約
                    if is_animating {
                        mw.window.request_redraw();
                    }
                } else {
                    self.state.is_visible = false;
                    // 非表示なら画面外に飛ばし透明に
                    win32::set_window_position(mw.hwnd, -10000, -10000)?;
                }
            }
            WindowEvent::CloseRequested => {
                el.exit();
            }
            WindowEvent::Resized(physical_size) => {
                // OSが確実にサイズ変更を完了したタイミング
                // ここで physical_size を使って IDXGISwapChain::ResizeBuffers を呼ぶ
                renderer.resize(physical_size.width, physical_size.height)?;
                mw.window.request_redraw();
            }
            _ => (),
        }

        Ok(())
    }

    // 再描画を伝播
    pub fn apply_config_to_all(&mut self) -> anyhow::Result<()> {
        let core = self.core.as_mut().context("AppCore missing")?;
        let cfg = self.config.as_ref().context("Config missing")?;
        let new_data = config::load_config();

        for mw in core.windows.values_mut() {
            let renderer = mw
                .render_stack
                .as_mut()
                .context("Renderer not found on ManagedWindow")?;
            let style = AppCore::get_style(&cfg, mw.role)?;

            // ここに各設定
            let is_enabled = {
                match mw.role {
                    WindowRole::Floating => new_data.floating.enabled,
                    WindowRole::Fixed => new_data.fixed.enabled,
                }
            };
            mw.window.set_visible(is_enabled);

            // Rendererのリソース（色、フォント）を更新
            renderer.update_config(&style)?;
            // サイズの再計算とリサイズ
            if let Ok((w, h)) = renderer.calc_metrics(self.state.last_mode) {
                let padding = style.padding;
                let final_size = LogicalSize::new(
                    (w + padding * 2.0).ceil() as u32,
                    (h + padding * 2.0).ceil() as u32,
                );
                let _ = mw.window.request_inner_size(final_size);
            }

            mw.window.request_redraw();
        }

        Ok(())
    }
}
