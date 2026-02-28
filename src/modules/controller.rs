use crate::modules::*;
use gpui::*;
use std::sync::mpsc;
use std::{thread, time::Duration};
use tray_icon::{
    TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuId, MenuItem},
};
use windows::Win32::System::Com::*;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::*;

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
            if let Err(e) = Self::ime_event_loop(tx) {
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

    fn ime_event_loop(tx: mpsc::Sender<String>) -> windows::core::Result<()> {
        unsafe {
            let rx = hooks::event_loop();
            // 初期化処理
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED).is_ok();
            let uia: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)?;

            let root = uia.GetRootElement()?;
            // タスクバーウィンドウを特定
            let tray_condition = uia.CreatePropertyCondition(
                UIA_ClassNamePropertyId,
                &VARIANT::from("Shell_TrayWnd"),
            )?;

            let walker = uia.RawViewWalker()?;
            // IUIAutomationElementを保持して処理を軽減
            let mut cached_tray: Option<IUIAutomationElement> = None;
            // IME状態保持用
            let mut last_mode_char: String = String::new();

            loop {
                // 制限時間付きの待機
                let event = rx.recv_timeout(Duration::from_millis(5000));
                // 何らかの操作が行われた
                let is_interaction = event.is_ok();
                // イベントを検知した瞬間にrx.recv_timeoutが解除されチェック処理が走る

                // 定期チェック
                // イベントが来ないまま時間経過
                // Err(Timeout)が返ってくる
                // 下の行へ

                // 即時反応
                // 別スレッドのフック関数が動き、チャネルにAppEvent::FocusChangedを送信
                // recv_timeoutは1秒経っていなくても即座にOk(AppEvent)を返して待機を終了
                // 下の行へ

                match event {
                    Ok(hooks::AppEvent::CheckRequest) => {
                        // println!("Active Window Changed - IME Check");
                    }
                    Err(_) => {}
                }

                // キャッシュが無い場合のみ検索
                if cached_tray.is_none() {
                    if let Ok(tray) = root.FindFirst(TreeScope_Children, &tray_condition) {
                        cached_tray = Some(tray);
                    }
                }

                // キャッシュがある場合
                if let Some(ref tray) = cached_tray {
                    match utils::find_ime_char_recursive(&walker, tray) {
                        Some(current_glyph) => {
                            let char_code = current_glyph.chars().next().unwrap_or_default();
                            let (is_ime_active, input_mode) = utils::get_ime_status(char_code);
                            // 判定結果
                            let has_input_capability =
                                input_capability::text_input_capability(&uia);

                            let should_show = match has_input_capability {
                                input_capability::InputCapability::Yes => true, // 入力欄ならIME状態問わず表示
                                input_capability::InputCapability::No => false, // 入力不可なら絶対に出さない
                                input_capability::InputCapability::Unknown => is_ime_active, // 判別不能ならONの時だけ救済表示
                            };

                            if should_show {
                                // 状態に変化があったときのみ更新
                                // ウィンドウ表示位置を変更するために、ユーザーイベントの有無を判定する
                                if current_glyph != last_mode_char || is_interaction {
                                    // println!("IME: {:?} (Cap: {:?})",input_mode, has_input_capability);
                                    let _ = tx.send(input_mode.to_string());
                                    last_mode_char = current_glyph;
                                }
                            } else {
                                if !last_mode_char.is_empty() {
                                    let _ = tx.send(String::new());
                                    // println!("入力不可能 (Cap: {:?})", has_input_capability);
                                    last_mode_char.clear();
                                }
                            }
                        }
                        None => {
                            // println!("IMEがオフ もしくは英語入力");
                        }
                    }
                    // キャッシュの生存確認
                    if tray.CurrentName().is_err() {
                        cached_tray = None;
                    }
                }
            }
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
                            utils::set_window_position(window);
                            utils::set_window_visibility(window, true);

                            // 自動消去タスク
                            let current_id = view.display_id;
                            cx.spawn(async move |_, cx| {
                                cx.background_executor().timer(Duration::from_secs(2)).await;

                                handle.update(cx, |view, window, _| {
                                    // 待機中にIDが変わっていなければ、ユーザーは沈黙していると判断
                                    if view.display_id == current_id {
                                        utils::set_window_visibility(window, false);
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
                            utils::set_window_visibility(window, false);
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
                utils::set_always_on_top(window, true);
                utils::set_click_through(window);
                utils::set_window_visibility(window, false);
            })
            .ok();
    }
}
