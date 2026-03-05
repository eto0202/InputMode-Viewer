use crate::sys::hooks::AppEvent;
use crate::sys::input::*;
use crate::sys::uia::input_mode::*;
use crate::sys::uia::*;
use crate::sys::win32::convert_window_handle;
use crate::sys::*;
use crate::*;
use anyhow::Result;
use gpui::*;
use std::sync::mpsc;
use tray_icon::TrayIcon;
use windows::Win32::UI::WindowsAndMessaging::{SW_SHOWNOACTIVATE, ShowWindow};

pub enum Message {
    Mode(InputMode),
    Cap(InputCapability),
}

pub struct Controller {
    _tray_icon: TrayIcon,
}

impl Controller {
    #[allow(unused_assignments)]
    pub fn new(cx: &mut Context<Self>) -> Self {
        // チャンネル作成
        let (tx, rx) = mpsc::channel::<Message>();
        let (tx_uia, rx_uia) = mpsc::channel::<AppEvent>();
        let (tx_input, rx_input) = mpsc::channel::<AppEvent>();
        // OSイベント
        let rx_hooks = hooks::win_hooks();
        // ディスパッチャー
        std::thread::spawn(move || -> Result<()> {
            while let Ok(event) = rx_hooks.recv() {
                tx_uia.send(event.clone())?;
                tx_input.send(event.clone())?;
            }
            Ok(())
        });
        // ワーカー
        uia_event::uia_thread(tx.clone(), rx_uia);
        input::input_thread(tx.clone(), rx_input);

        // GUI更新
        cx.spawn(async move |_, async_app| {
            let async_app = async_app.clone();

            // 最新の状態を保持する変数
            let mut last_cap = InputCapability::Unknown;
            let mut last_mode = InputMode::Unknown;

            loop {
                let mut has_new_msg = false;
                // メッセージを処理
                while let Ok(msg) = rx.try_recv() {
                    match msg {
                        Message::Mode(mode) => {
                            last_mode = mode;
                            has_new_msg = true;
                        }
                        Message::Cap(cap) => {
                            last_cap = cap;
                        }
                    }
                }

                // 新しい情報が届いた場合のみ判定
                if has_new_msg {
                    let should_show = match last_cap {
                        InputCapability::No => false,
                        InputCapability::Yes => last_mode != InputMode::Unknown,
                        InputCapability::Unknown => last_mode.is_on(),
                    };

                    async_app
                        .update(|app| {
                            // 描画更新
                            if should_show {
                                Self::handle_update_window(app, last_mode);
                            } else {
                                Self::handle_close_window(app);
                            }
                        })
                        .ok();
                }

                async_app
                    .background_executor()
                    .timer(std::time::Duration::from_millis(50))
                    .await;
            }
        })
        .detach();

        // タスクトレイイベント
        cx.spawn(async move |_, async_app| {
            let async_app = async_app.clone();
            app::tray::tray_event(async_app);
        })
        .detach();

        Self {
            _tray_icon: app::tray::create_tray_icon(),
        }
    }

    fn handle_update_window(app: &mut App, input_mode: InputMode) {
        // 更新対象のハンドルを特定
        let target_handle = app
            .windows()
            .iter()
            .find_map(|w| w.downcast::<ui::window::MainWindow>());

        // 指定のハンドルに対してのみupdate
        if let Some(handle) = target_handle {
            handle
                .update(app, |view, window, cx| -> Result<()> {
                    // テキスト更新
                    if view.input_mode != input_mode {
                        view.input_mode = input_mode;
                        // 更新のたびにカウントアップ
                        view.display_id += 1;
                        cx.notify();

                        sys::win32::set_window_visibility(window, true)?;
                    }
                    sys::win32::set_window_visibility(window, true)?;
                    Ok(())
                })
                .ok();
        } else {
            open_main_window(app, input_mode);
        }
    }

    fn handle_close_window(app: &mut App) {
        // 更新対象のハンドルを特定
        let target_handle = app
            .windows()
            .iter()
            .find_map(|w| w.downcast::<ui::window::MainWindow>());

        // 指定のハンドルに対してのみupdate
        if let Some(handle) = target_handle {
            handle
                .update(app, |_, window, _| -> Result<()> {
                    sys::win32::set_window_visibility(window, false)?;
                    Ok(())
                })
                .ok();
        }
    }
}

impl Render for Controller {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

fn open_main_window(app: &mut App, input_mode: InputMode) {
    let window_options = WindowOptions {
        kind: WindowKind::PopUp,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            // 画面外で生成
            origin: Point::new(px(0.), px(0.)),
            size: size(px(100.), px(30.)),
        })),
        ..Default::default()
    };

    let handle_ref = app.open_window(window_options, |window, app| {
        app.new(|cx| ui::window::MainWindow::new(input_mode, window, cx))
    });

    if let Ok(window_handle) = handle_ref {
        app.update_window(window_handle.into(), |_, window, _| -> Result<()> {
            sys::win32::set_always_on_top(window, true)?;
            sys::win32::set_click_through(window)?;
            sys::win32::set_window_opacity(window, 0)?;
            unsafe {
                ShowWindow(convert_window_handle(window)?, SW_SHOWNOACTIVATE).ok()?;
            }
            Ok(())
        })
        .ok();
    }
}
