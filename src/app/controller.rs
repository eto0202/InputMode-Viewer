use crate::sys::uia::cap::InputCapability;
use crate::sys::uia::text::InputMode;
use crate::sys::win32;
use crate::{sys::renderer::DCompRenderer, ui::popup_window::MainWindow};
use std::time::{Duration, Instant};
use tray_icon::TrayIcon;
use tray_icon::{
    TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::WindowEvent;
use winit::window::{Window, WindowAttributes, WindowId};
use winit::{event_loop::ActiveEventLoop, platform::windows::WindowAttributesExtWindows};

const ID_QUIT: &str = "Quit";

// 外部から届くIMEの変更通知
#[derive(Debug, Clone, Copy)]
pub enum Message {
    Cap(InputCapability),
    Mode(InputMode),
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
    pub main_window: Option<MainWindow>,
    pub main_window_id: Option<WindowId>,
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
            main_window: None,
            main_window_id: None,
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
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id.as_ref() {
                ID_QUIT => event_loop.exit(),
                _ => {}
            }
        }

        if self.is_visible {
            let window_info = self
                .main_window
                .as_ref()
                .map(|mw| (mw.hwnd, mw.window.scale_factor()));

            if let Some((hwnd, scale)) = window_info {
                event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
                self.set_predicted_window_position(hwnd, scale);

                if let Some(mw) = &mut self.main_window {
                    mw.window.request_redraw();
                }
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
        }
    }
}

impl Controller {
    fn handle_resumed(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
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

            let window = event_loop.create_window(attr)?;
            self.proxy_window = Some(window);

            let mw = MainWindow::new(event_loop)?;
            self.main_window_id = Some(mw.id);

            win32::set_window_style(mw.hwnd)?;
            let (width, height) = mw.size();

            // DCompRendererを作成
            // 最初は透明
            let renderer = DCompRenderer::new(mw.hwnd, width, height, mw.window.scale_factor())?;
            renderer.set_visibility(0.0)?;

            // 描画更新
            mw.window.request_redraw();

            self.main_window = Some(mw);
            self.renderer = Some(renderer);

            // タスクトレイに「Quit」メニュー付きのアイコンを出す
            let tray_menu = Menu::new();
            let quit_item = MenuItem::with_id(ID_QUIT, "Quit", true, None);
            tray_menu.append(&quit_item)?;

            let tray_icon = TrayIconBuilder::new()
                .with_menu(Box::new(tray_menu))
                .with_tooltip("Input Mode Viewer")
                .build()?;

            self.tray_icon = Some(tray_icon);
        }
        Ok(())
    }

    fn handle_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: WindowId,
        event: WindowEvent,
    ) -> anyhow::Result<()> {
        if Some(id) != self.main_window_id {
            return Ok(());
        }

        let (Some(mw), Some(renderer)) = (&mut self.main_window, &self.renderer) else {
            return Ok(());
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
        let offset = (20.0 * scale) as i32;

        let _ = win32::set_window_position(hwnd, predicted_x + offset, predicted_y + offset);

        // 現在のマウス座標を保存
        self.last_raw_mouse_x = current.x;
        self.last_raw_mouse_y = current.y;
    }
}
