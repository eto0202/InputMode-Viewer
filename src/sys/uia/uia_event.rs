use crate::app::controller::Message;
use crate::sys::hooks::AppEvent;
use crate::sys::uia::input_mode::*;
use crate::sys::uia::utils;
use std::sync::*;
use std::thread;
use windows::Win32::System::Variant::VARIANT;

use windows::Win32::System::Com::*;

use windows::Win32::UI::Accessibility::*;

pub fn uia_thread(tx: mpsc::Sender<Message>, rx: mpsc::Receiver<AppEvent>) {
    unsafe {
        thread::spawn(move || {
            println!("--- uia_thread ---");
            // COMの初期化
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED).unwrap();
            // uia取得
            let uia: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL).unwrap();
            let root = uia.GetRootElement().unwrap();

            // 検索用のキャッシュリクエスト
            let cache_request = utils::create_ime_cache_request(&uia).unwrap();

            // タスクバーウィンドウを特定
            let tray_condition = uia
                .CreatePropertyCondition(UIA_ClassNamePropertyId, &VARIANT::from("Shell_TrayWnd"))
                .unwrap();

            // SystemTrayIcon内のテキストを特定
            let btn_condition = uia
                .CreatePropertyCondition(
                    UIA_AutomationIdPropertyId,
                    &VARIANT::from("InnerTextBlock"),
                )
                .unwrap();

            // 目的の要素を保持する変数
            // IUIAutomationElementを保持して処理を軽減
            let mut cached_tray: Option<IUIAutomationElement> = None;
            let mut last_sent_mode = InputMode::Unknown;

            loop {
                // イベント受信とタイムアウト
                let event = rx.recv_timeout(std::time::Duration::from_millis(5000));
                match event {
                    Ok(AppEvent::CheckRequest) => {
                        println!("--- IME check start ---");
                        tx.send(Message::Mode(last_sent_mode)).unwrap();
                    }
                    Err(_) => {}
                }

                // 要素を持ってなければ検索して取得
                // キャッシュが無い場合のみ検索
                if cached_tray.is_none() {
                    println!("--- Nothing cached_tray ---");
                    // タスクバー本体(Shell_TrayWnd)を見つける
                    if let Ok(tray) = root.FindFirst(TreeScope_Children, &tray_condition) {
                        cached_tray = Some(tray);
                    }
                }

                // 要素を持っているなら最新の状態を取得して変化を確認
                if let Some(ref tray) = cached_tray {
                    let elements_array = tray
                        .FindAllBuildCache(TreeScope_Descendants, &btn_condition, &cache_request)
                        .unwrap();

                    if let Some(el) = utils::find_element(elements_array, "InnerTextBlock") {
                        // NameプロパティからInputModeを取得
                        match el.CachedName() {
                            Ok(name) => {
                                let current_mode = InputMode::from_glyph(name.to_string());
                                println!(
                                    "Current: {:?} - LastSent: {:?}",
                                    current_mode, last_sent_mode
                                );
                                // もし前回と変わっていたら送信して値を更新
                                if current_mode != last_sent_mode {
                                    println!("--- Change mode ---");
                                    tx.send(Message::Mode(current_mode)).unwrap();
                                    last_sent_mode = current_mode;
                                }
                            }
                            Err(_) => {
                                // COMオブジェクトが無効になった時
                                println!("--- Element Error ---");
                                last_sent_mode = InputMode::Unknown;
                            }
                        }
                        // キャッシュの生存確認
                        if tray.CurrentProcessId().is_err() {
                            println!("--- Cache is dead ---");
                            cached_tray = None;
                        }
                    }
                }
            }
        });
    }
}
