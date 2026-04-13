use anyhow::Context;
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tray_icon::TrayIcon;
use tray_icon::menu::MenuEvent;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalSize};
use winit::event::WindowEvent;
use winit::window::{Window, WindowAttributes, WindowId};
use winit::{event_loop::ActiveEventLoop, platform::windows::WindowAttributesExtWindows};

use crate::common::app_config::{AppConfig, WindowRole, WindowStyle};
use crate::common::config;
use crate::core::app::tray;
use crate::core::sys::renderer::DCompRenderer;
use crate::core::sys::uia::cap::InputCapability;
use crate::core::sys::uia::text::InputMode;
use crate::core::sys::win32;
use crate::core::window::window::ManagedWindow;
use crate::ui;

// 外部から届くカスタムイベント
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
                // 必要な値だけコピー
                let hwnd = window.hwnd;
                let scale = window.window.scale_factor();
                window.window.request_redraw();
                Some((hwnd, scale))
            } else {
                None
            };

            if let Some((hwnd, scale)) = window_date {
                event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
                self.set_predicted_window_position(hwnd, scale);
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
                self.last_mode = mode;

                if self.last_cap != InputCapability::No {
                    self.is_visible = true;
                    println!("last_cap: {:?}", self.last_cap);
                    println!("last_mode: {:?}", self.last_mode);
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
            let _id = self.create_window(el, WindowRole::Floating).unwrap();
            let mw = self.find_by_role(WindowRole::Floating)?;

            win32::set_window_style(mw.hwnd)?;
            let (width, height) = mw.size();

            // DCompRendererを作成
            // 最初は透明
            let renderer = DCompRenderer::new(mw.hwnd, width, height, mw.window.scale_factor())?;
            renderer.set_visibility(0.0)?;

            // 描画更新
            let _size = mw.window.request_inner_size(PhysicalSize::new(100, 40));
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
        id: WindowId,
        event: WindowEvent,
    ) -> anyhow::Result<()> {
        let (mw, renderer) = {
            let renderer = self.renderer.as_ref().context("renderer is missing")?;
            let windows = self.windows.as_mut().context("windows is missing")?;

            let mw = windows
                .values_mut()
                .find(|w| w.role == WindowRole::Floating)
                .context("Not found role")?;

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

                let (width, height) = mw.size();
                let target_opacity = 0.5;
                // フェードイン時間
                let fade_duration = Duration::from_millis(160);

                // フェードイン設定
                match (should_show, self.show_state) {
                    // 非表示なら画面外に飛ばし透明に
                    (false, _) => {
                        renderer.draw(self.last_mode, width, height)?;
                        win32::set_window_position(mw.hwnd, -10000, -10000)?;
                        renderer.set_visibility(0.0)?;

                        self.show_state = ShowState::Hidden;
                        self.is_visible = false;
                    }
                    // 以降がフェードイン部分
                    (true, ShowState::Hidden) => {
                        renderer.draw(self.last_mode, width, height)?;
                        self.show_state = ShowState::FadeIn {
                            start_at: Instant::now(),
                            duration: fade_duration,
                        };
                        mw.window.request_redraw();
                    }
                    (true, ShowState::FadeIn { start_at, duration }) => {
                        renderer.draw(self.last_mode, width, height)?;

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
                        renderer.draw(self.last_mode, width, height)?;
                        mw.window.request_redraw();
                    }
                }
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => (),
        }

        Ok(())
    }

    // マウス位置の予測
    fn set_predicted_window_position(&mut self, hwnd: HWND, scale: f64) {
        // 出力引数
        let mut current = POINT::default();
        let _ = unsafe { GetCursorPos(&mut current) }; // 現在のマウス座標
        // 保存しておいた前回からの移動量(速度)を計算
        let dx = current.x - self.last_raw_mouse_x;
        let dy = current.y - self.last_raw_mouse_y;

        // 予測係数の設定
        // 移動距離が1px以下なら無視
        let dist_sq = dx * dx + dy * dy; // // 三平方の定理のルート取る前。ルート計算は重いので2乗のまま比較
        // 移動距離が2ピクセル未満（2の2乗で4未満）なら、マウスが止まっているか手が震えているだけなので予測を0.0
        let k = if dist_sq < 4 { 0.0 } else { 1.6 }; // 1.6フレーム先

        // 予測座標を計算
        // 今の場所に、速度 × フレーム数を足す
        let predicted_x = current.x + (dx as f32 * k) as i32;
        let predicted_y = current.y + (dy as f32 * k) as i32;

        // マウスから少しずらす
        let offset = 20 * scale as i32;

        let _ = win32::set_window_position(hwnd, predicted_x + offset, predicted_y + offset);

        // 現在のマウス座標を保存
        self.last_raw_mouse_x = current.x;
        self.last_raw_mouse_y = current.y;
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

    // 全ウィンドウに再描画を伝播
    pub fn apply_config_to_all(&mut self) -> anyhow::Result<()> {
        let windows = self.windows.as_mut().context("Windows is missing")?;

        for managed in windows.values_mut() {
            let cfg = self.config.as_ref().context("Config is missing")?;
            let _style = get_style(cfg, managed.role);

            // ここに各設定

            managed.window.request_redraw();
        }

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
