use crate::core::app::prelude::*;

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Cap(InputCapability),
    Mode(InputMode),
    ConfigUpdated,
}

// 「隠す」「フェードイン中」「表示中」の3つの状態で管理し、アニメーションを実装
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ShowState {
    Hidden,
    FadeIn {
        start_at: Instant,
        duration: Duration,
    },
    Visible,
}

// 全ての部品
pub struct Controller {
    pub tray_icon: Option<TrayIcon>,
    pub proxy_window: Option<Window>,
    pub windows: Option<HashMap<WindowId, ManagedWindow>>,
    pub config: Option<Arc<RwLock<AppConfig>>>,
    pub renderer: Option<DCompRenderer>,
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
            tray_icon: None,
            proxy_window: None,
            windows: None,
            config: None,
            renderer: None,
            last_cap: InputCapability::Unknown,
            last_mode: InputMode::Unknown,
            is_visible: false,
            show_state: ShowState::Hidden,
            last_raw_mouse_x: 0,
            last_raw_mouse_y: 0,
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

        if self.is_visible {
            let managed = self.find_by_role(WindowRole::Floating);
            let window_date = if let Ok(window) = managed {
                let hwnd = window.hwnd;
                let scale = window.window.scale_factor();
                window.window.request_redraw();
                Some((hwnd, scale))
            } else {
                None
            };

            if let Some((hwnd, scale)) = window_date {
                event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
                let (current_x, current_y) = utils::set_predicted_position(
                    hwnd,
                    self.last_raw_mouse_x,
                    self.last_raw_mouse_y,
                    scale,
                );
                self.last_raw_mouse_x = current_x;
                self.last_raw_mouse_y = current_y;
            }
        } else {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, msg: Message) {
        match msg {
            Message::Cap(cap) => {
                self.last_cap = cap;
            }
            Message::Mode(mode) => {
                let old_mode = self.last_mode;
                self.last_mode = mode;

                if self.last_cap != InputCapability::No {
                    self.is_visible = true;
                    println!("last_cap: {:?}", self.last_cap);
                    println!("last_mode: {:?}", self.last_mode);
                }

                // モードが変化した時に、ウィンドウサイズを再計算してリサイズ要求
                if old_mode != mode {
                    if let Some(renderer) = &self.renderer {
                        if let Err(e) = self.try_resize(renderer, mode, WindowRole::Floating) {
                            eprintln!("Resize failed: {}", e);
                        }
                    }
                }
            }
            Message::ConfigUpdated => {
                let new_data = config::load_config();

                if let Some(con) = &self.config {
                    // 書き込みロックを取得
                    let mut lock = con.write();
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
        if self.proxy_window.is_none() {
            // プロキシウィンドウを作成
            // メインウィンドウを消した時にアプリ自体が終了してしまうことがあるため
            // イベントを受け取るための身代わりとして用意
            let attr = WindowAttributes::default()
                .with_visible(false)
                .with_active(false)
                .with_skip_taskbar(true)
                .with_decorations(false)
                .with_max_inner_size(LogicalSize::new(0, 0))
                .with_position(LogicalPosition::new(0, 0));

            let window = el.create_window(attr)?;
            self.proxy_window = Some(window);

            self.windows = Some(HashMap::new());
            let _id = self.create_window(el, WindowRole::Floating)?;
            let mw = self.find_by_role(WindowRole::Floating)?;

            win32::set_window_style(mw.hwnd)?;

            // DCompRendererを作成
            // todo: fixedウィンドウを不可視で作成
            let style = get_style(self.config.as_ref().unwrap(), WindowRole::Floating)?;
            let (renderer, w, h) =
                DCompRenderer::new(mw.hwnd, self.last_mode, &style, mw.window.scale_factor())?;
            renderer.set_visibility(0.0)?;

            // 描画更新
            let _size = mw.window.request_inner_size(LogicalSize::new(w, h));
            mw.window.request_redraw();

            self.renderer = Some(renderer);

            // トレイアイコン
            self.tray_icon = Some(tray::tray_icon()?);
        }
        Ok(())
    }

    fn handle_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) -> anyhow::Result<()> {
        let (mw, renderer) = {
            let renderer = self.renderer.as_ref().context("renderer is missing")?;
            let windows = self.windows.as_mut().context("windows is missing")?;
            let mw = windows
                .values_mut()
                .find(|w| w.role == WindowRole::Floating)
                .context("Not found Floating")?;
            (mw, renderer)
        };

        // 表示判定
        match event {
            WindowEvent::RedrawRequested => {
                let should_show = match self.last_cap {
                    InputCapability::No => false,
                    InputCapability::Yes => self.last_mode != InputMode::Unknown,
                    InputCapability::Unknown => self.last_mode.is_on(), // 不明の場合はONの時だけ表示
                };

                let style = get_style(self.config.as_ref().unwrap(), WindowRole::Floating)?;
                let p = style.padding;
                let target_opacity = style.opacity;
                // フェードイン時間
                let fade_duration = Duration::from_millis(160);

                // 現在のウィンドウの「論理サイズ」を取得して描画
                let scale = mw.window.scale_factor();
                let logical_size = mw.window.inner_size().to_logical::<f32>(scale);
                let width = logical_size.width;
                let height = logical_size.height;

                // フェードイン設定
                match (should_show, self.show_state) {
                    // 非表示なら画面外に飛ばし透明に
                    (false, _) => {
                        renderer.draw(self.last_mode, width, height, p)?;
                        win32::set_window_position(mw.hwnd, -10000, -10000)?;
                        renderer.set_visibility(0.0)?;

                        self.show_state = ShowState::Hidden;
                        self.is_visible = false;
                    }
                    // 以降がフェードイン部分
                    (true, ShowState::Hidden) => {
                        renderer.draw(self.last_mode, width, height, p)?;
                        self.show_state = ShowState::FadeIn {
                            start_at: Instant::now(),
                            duration: fade_duration,
                        };
                        mw.window.request_redraw();
                    }
                    (true, ShowState::FadeIn { start_at, duration }) => {
                        renderer.draw(self.last_mode, width, height, p)?;

                        let elapsed = start_at.elapsed();
                        // 経過時間 ÷ 160ミリ秒 で進捗率（0.0〜1.0）を出す。1.0までループし徐々に濃く。
                        let progress = (elapsed.as_secs_f32() / duration.as_secs_f32()).min(1.0);

                        let current_opacity = progress * target_opacity;
                        renderer.set_visibility(current_opacity)?;

                        if progress < 1.0 {
                            mw.window.request_redraw();
                        } else {
                            self.show_state = ShowState::Visible;
                        }
                    }
                    (true, ShowState::Visible) => {
                        renderer.draw(self.last_mode, width, height, p)?;
                        mw.window.request_redraw();
                    }
                }
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
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

    // ウィンドウ作成と登録
    pub fn create_window(
        &mut self,
        el: &ActiveEventLoop,
        role: WindowRole,
    ) -> anyhow::Result<WindowId> {
        let managed = {
            // 設定から該当するスタイルを取得
            let cfg = self.config.clone().context("Config is missing")?;
            let style = get_style(&cfg, role)?;
            ManagedWindow::new(el, role, &*style)?
        };

        let id = managed.window.id();
        let windows = self.windows.as_mut().context("Windows is missing")?;

        windows.insert(id, managed);

        Ok(id)
    }

    // ロール検索 参照
    pub fn find_by_role(&self, role: WindowRole) -> anyhow::Result<&ManagedWindow> {
        let windows = self.windows.as_ref().context("Windows is missing")?;

        windows
            .values()
            .find(|w| w.role == role)
            .context("Not find role")
    }

    // ロール検索 可変参照
    pub fn find_by_role_mut(&mut self, role: WindowRole) -> anyhow::Result<&mut ManagedWindow> {
        let windows = self.windows.as_mut().context("Windows is missing")?;

        windows
            .values_mut()
            .find(|w| w.role == role)
            .context("Not find role")
    }

    // 再描画を伝播
    pub fn apply_config_to_all(&mut self) -> anyhow::Result<()> {
        let new_data = config::load_config();
        let windows = self.windows.as_mut().context("Windows is missing")?;

        for mw in windows.values_mut() {
            let cfg = self.config.as_ref().context("Config is missing")?;
            let style = get_style(cfg, mw.role)?;

            // ここに各設定
            let _enabled = match mw.role {
                WindowRole::Floating => new_data.floating.enabled,
                WindowRole::Fixed => new_data.fixed.enabled,
            };

            // Rendererのリソース（色、フォント）を更新
            if let Some(renderer) = &mut self.renderer {
                renderer.update_config(&style)?;

                // サイズの再計算とリサイズ
                if let Ok((w, h)) = renderer.calc_metrics(self.last_mode) {
                    let padding = style.padding;
                    let final_size = LogicalSize::new(
                        (w + padding * 2.0).ceil() as u32,
                        (h + padding * 2.0).ceil() as u32,
                    );
                    let _ = mw.window.request_inner_size(final_size);
                }
            }

            mw.window.request_redraw();
        }

        Ok(())
    }

    // モードが変化した時に、ウィンドウサイズを再計算してリサイズ要求
    fn try_resize(
        &self,
        renderer: &DCompRenderer,
        mode: InputMode,
        role: WindowRole,
    ) -> anyhow::Result<()> {
        let cfg = self.config.as_ref().context("Config is missing")?;
        let style = get_style(cfg, WindowRole::Floating).context("No style")?;
        let (w, h) = renderer.calc_metrics(mode).context("Calc metrics failed")?;
        let mw = self.find_by_role(role).context("Windows is missing")?;

        let p = style.padding;
        let final_size = LogicalSize::new((w + p * 2.0).ceil(), (h + p * 2.0).ceil());

        let _ = mw.window.request_inner_size(final_size);
        Ok(())
    }
}

// スタイルの取得
pub fn get_style(
    cfg: &Arc<RwLock<AppConfig>>,
    role: WindowRole,
) -> anyhow::Result<MappedRwLockReadGuard<'_, WindowStyle>> {
    // ロックを取得
    let guard = cfg.read();

    // ガードをWindowStyleだけに絞り込む
    let style = RwLockReadGuard::map(guard, |cfg| match role {
        WindowRole::Floating => &cfg.floating.style,
        WindowRole::Fixed => &cfg.fixed.style,
    });

    Ok(style)
}
