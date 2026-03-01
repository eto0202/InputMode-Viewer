use crate::*;
use gpui::*;
use std::sync::mpsc;
use std::{thread, time::Duration};
use tray_icon::TrayIcon;

pub struct Controller {
    _tray_icon: TrayIcon,
}

impl Controller {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let tray_icon = app::tray::create_tray_icon();

        // 1. 文字列をやり取りするチャネルを作成
        let (tx, rx) = mpsc::channel::<String>();

        open_main_window(cx, "".to_string());

        // OSスレッドに写したことで、recv_timeoutが描画処理を止めない
        thread::spawn(move || {
            if let Err(e) = sys::ime::ime_event(tx) {
                eprintln!("ime_event Error: {:?}", e);
            }
        });

        // GUI更新
        cx.spawn(async move |_, async_app| {
            let async_app = async_app.clone();
            loop {
                while let Ok(new_text) = rx.try_recv() {
                    if new_text.is_empty() {
                        Self::handle_close_window(&async_app);
                    } else {
                        Self::handle_update_window(&async_app, new_text);
                    }
                }
                async_app
                    .background_executor()
                    .timer(Duration::from_millis(50))
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
            _tray_icon: tray_icon,
        }
    }

    fn handle_update_window(async_app: &AsyncApp, new_text: String) {
        async_app
            .update(|app| {
                // 更新対象のハンドルを特定
                let target_handle = app
                    .windows()
                    .iter()
                    .find_map(|w| w.downcast::<ui::window::MainWindow>());

                // 指定のハンドルに対してのみupdate
                if let Some(handle) = target_handle {
                    handle
                        .update(app, |view, window, cx| {
                            // テキスト更新
                            if view.text != new_text {
                                view.text = new_text.clone();
                                // 更新のたびにカウントアップ
                                view.display_id += 1;
                                cx.notify();
                            }
                            sys::win32::set_window_position(window);
                            sys::win32::set_window_visibility(window, true);

                            // 自動消去タスク
                            let current_id = view.display_id;
                            cx.spawn(async move |_, async_app| {
                                async_app
                                    .background_executor()
                                    .timer(Duration::from_secs(2))
                                    .await;

                                handle.update(async_app, |view, window, _| {
                                    // 待機中にIDが変わっていなければ、ユーザーは沈黙していると判断
                                    if view.display_id == current_id {
                                        sys::win32::set_window_visibility(window, false);
                                    }
                                })
                            })
                            .detach();
                        })
                        .ok();
                } else {
                    open_main_window(app, new_text);
                }
            })
            .ok();
    }

    fn handle_close_window(async_app: &AsyncApp) {
        async_app
            .update(|app| {
                // 更新対象のハンドルを特定
                let target_handle = app
                    .windows()
                    .iter()
                    .find_map(|w| w.downcast::<ui::window::MainWindow>());

                // 指定のハンドルに対してのみupdate
                if let Some(handle) = target_handle {
                    handle
                        .update(app, |_, window, _| {
                            sys::win32::set_window_visibility(window, false);
                        })
                        .ok();
                }
            })
            .ok();
    }
}

impl Render for Controller {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

fn open_main_window(app: &mut App, text: String) {
    let window_options = WindowOptions {
        kind: WindowKind::PopUp,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            // 画面外で生成
            origin: Point::new(px(0.), px(0.)),
            size: size(px(120.), px(40.)),
        })),
        ..Default::default()
    };

    let handle_ref = app.open_window(window_options, |window, app| {
        app.new(|cx| ui::window::MainWindow::new(text, window, cx))
    });

    if let Ok(window_handle) = handle_ref {
        app.update_window(window_handle.into(), |_, window, _| {
            sys::win32::set_always_on_top(window, true);
            sys::win32::set_click_through(window);
            sys::win32::set_window_visibility(window, false);
        })
        .ok();
    }
}
