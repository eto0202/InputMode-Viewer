use crate::modules::*;
use gpui::*;
use std::sync::mpsc;
use std::{thread, time::Duration};
use tray_icon::{
    TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuId, MenuItem},
};

const MENU_ID_QUIT: &str = "quit";

pub struct Controller {
    _tray_icon: TrayIcon,
}

impl Controller {
    pub fn new(ctrl_cx: &mut Context<Self>) -> Self {
        let tray_icon = Self::create_tray_icon();

        // 1. 文字列をやり取りするチャネルを作成
        let (tx, rx) = mpsc::channel::<String>();

        open_main_window(ctrl_cx, "".to_string());

        // OSスレッドに写したことで、recv_timeoutが描画処理を止めない
        thread::spawn(move || {
            if let Err(e) = ime_event_loop::ime_event_loop(tx) {
                eprintln!("ime_event_loop Error: {:?}", e);
            }
        });

        // GUI更新
        ctrl_cx
            .spawn(|_, cx: &mut AsyncApp| {
                let async_cx = cx.clone();
                async move {
                    loop {
                        while let Ok(new_text) = rx.try_recv() {
                            if new_text.is_empty() {
                                Self::handle_close_window(&async_cx);
                            } else {
                                Self::handle_update_window(&async_cx, new_text);
                            }
                        }
                        async_cx
                            .background_executor()
                            .timer(Duration::from_millis(50))
                            .await;
                    }
                }
            })
            .detach();

        // タスクトレイイベント
        ctrl_cx
            .spawn(async move |_, async_cx| {
                let async_cx = async_cx.clone();
                Self::task_event_loop(async_cx).await;
            })
            .detach();

        Self {
            _tray_icon: tray_icon,
        }
    }

    async fn task_event_loop(async_cx: AsyncApp) {
        let menu_receiver = MenuEvent::receiver();
        let _tray_receiver = TrayIconEvent::receiver();

        // タスクトレイイベントの監視
        if let Ok(event) = menu_receiver.try_recv()
            && event.id == MenuId::new(MENU_ID_QUIT)
        {
            async_cx
                .update(|cx| {
                    cx.quit();
                })
                .ok();
        }
    }

    fn create_tray_icon() -> TrayIcon {
        // イベント監視用にIDを固定して作成する
        // IDは定数を使用
        let quit_item = MenuItem::with_id(MenuId::new(MENU_ID_QUIT), "Quit", true, None);

        let tray_menu = Menu::new();

        // コンパイル時に画像をバイナリに取り込む
        let img_bytes = include_bytes!("../icon.png");
        let icon = utils::load_icon(img_bytes);

        tray_menu.append(&quit_item).unwrap();

        TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("input_mode_viewer")
            .with_icon(icon)
            .with_menu_on_left_click(false)
            .build()
            .unwrap()
    }

    fn handle_update_window(async_cx: &AsyncApp, new_text: String) {
        async_cx
            .update(|app_cx| {
                // 更新対象のハンドルを特定
                let target_handle = app_cx
                    .windows()
                    .iter()
                    .find_map(|w| w.downcast::<main_window::MainWindow>());

                // 指定のハンドルに対してのみupdate
                if let Some(handle) = target_handle {
                    handle
                        .update(app_cx, |view, window, cx| {
                            // テキスト更新
                            if view.text != new_text {
                                view.text = new_text.clone();
                                // 更新のたびにカウントアップ
                                view.display_id += 1;
                                cx.notify();
                            }
                            win32_utils::set_window_position(window);
                            win32_utils::set_window_visibility(window, true);

                            // 自動消去タスク
                            let current_id = view.display_id;
                            cx.spawn(async move |_, cx| {
                                cx.background_executor().timer(Duration::from_secs(2)).await;

                                handle.update(cx, |view, window, _| {
                                    // 待機中にIDが変わっていなければ、ユーザーは沈黙していると判断
                                    if view.display_id == current_id {
                                        win32_utils::set_window_visibility(window, false);
                                    }
                                })
                            })
                            .detach();
                        })
                        .ok();
                } else {
                    open_main_window(app_cx, new_text);
                }
            })
            .ok();
    }

    fn handle_close_window(async_cx: &AsyncApp) {
        async_cx
            .update(|app_cx| {
                // 更新対象のハンドルを特定
                let target_handle = app_cx
                    .windows()
                    .iter()
                    .find_map(|w| w.downcast::<main_window::MainWindow>());

                // 指定のハンドルに対してのみupdate
                if let Some(handle) = target_handle {
                    handle
                        .update(app_cx, |_, window, _| {
                            win32_utils::set_window_visibility(window, false);
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

fn open_main_window(app_cx: &mut App, text: String) {
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

    let handle_ref = app_cx.open_window(window_options, |window, app_cx| {
        app_cx.new(|cx| main_window::MainWindow::new(text, window, cx))
    });

    if let Ok(window_handle) = handle_ref {
        app_cx
            .update_window(window_handle.into(), |_, window, _| {
                win32_utils::set_always_on_top(window, true);
                win32_utils::set_click_through(window);
                win32_utils::set_window_visibility(window, false);
            })
            .ok();
    }
}
