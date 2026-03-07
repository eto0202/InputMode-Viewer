use crate::app::controller::Message;
use crate::sys::hooks::AppEvent;
use crate::sys::uia::input_mode::*;
use crate::sys::uia::utils;
use anyhow::{Context, Result};
use std::sync::*;
use std::thread;
use windows::Win32::System::Com::*;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::*;

// スレッドを抜ける時に自動でCoUninitializeを呼ぶためのガード
struct ComGuard;
impl Drop for ComGuard {
    fn drop(&mut self) {
        println!("uia_event COM Drop");
        unsafe {
            CoUninitialize();
        }
    }
}

pub fn uia_thread(tx: mpsc::Sender<Message>, rx: mpsc::Receiver<AppEvent>) {
    unsafe {
        thread::spawn(move || {
            loop {
                let result = || -> Result<()> {
                    println!("--- uia_thread start ---");
                    // COMの初期化
                    CoInitializeEx(None, COINIT_APARTMENTTHREADED)
                        .ok()
                        .context("UIAのインスタンス作成に失敗: uia_thread")?;

                    let _guard = ComGuard;

                    // uia取得
                    let uia: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)
                        .context("UIA取得に失敗: uia_thread")?;

                    let root = uia
                        .GetRootElement()
                        .context("root要素の取得に失敗: uia_thread")?;

                    // 検索用のキャッシュリクエスト
                    let cache_request = utils::create_cache_request(&uia)
                        .context("キャッシュリクエスト作成に失敗: uia_thread")?;

                    // タスクバーウィンドウを特定
                    let tray_condition = uia
                        .CreatePropertyCondition(
                            UIA_ClassNamePropertyId,
                            &VARIANT::from("Shell_TrayWnd"),
                        )
                        .context("Condition作成に失敗: uia_thread")?;

                    // SystemTrayIcon内のテキストを特定
                    let btn_condition = uia
                        .CreatePropertyCondition(
                            UIA_AutomationIdPropertyId,
                            &VARIANT::from("InnerTextBlock"),
                        )
                        .context("Condition作成に失敗: uia_thread")?;

                    // 目的の要素を保持する変数
                    let mut cached_tray: Option<IUIAutomationElement> = None;
                    let mut last_sent_mode = InputMode::Unknown;

                    loop {
                        // イベント受信とタイムアウト
                        let event = rx.recv_timeout(std::time::Duration::from_millis(5000));
                        match event {
                            Ok(AppEvent::CheckRequest) => {
                                println!("Event Received: uia_thread");
                            }
                            Err(mpsc::RecvTimeoutError::Disconnected) => {
                                println!("Disconnected: uia_thread");
                                break;
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
                                .FindAllBuildCache(
                                    TreeScope_Descendants,
                                    &btn_condition,
                                    &cache_request,
                                )
                                .context("FindAllBuildCacheに失敗: uia_thread")?;

                            if let Some(el) = utils::find_element(&elements_array, "InnerTextBlock")
                            {
                                // NameプロパティからInputModeを取得
                                match el.CachedName() {
                                    Ok(name) => {
                                        let current_mode = InputMode::from_glyph(name.to_string());
                                        // もし前回と変わっていたら値を更新して送信
                                        if current_mode != last_sent_mode {
                                            println!(
                                                "[ Change mode: {:?} -> {:?} ]",
                                                last_sent_mode, current_mode
                                            );
                                            tx.send(Message::Mode(current_mode))?;
                                            last_sent_mode = current_mode;
                                        } else {
                                            tx.send(Message::Mode(last_sent_mode))?;
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
                    Ok(())
                }();

                if let Err(e) = result {
                    eprintln!("uia_thread fatal error. restarting in 3 seconds: {:?}", e);
                    std::thread::sleep(std::time::Duration::from_secs(3));
                } else {
                    // エラーなしで戻ってきた場合はスレッドを完全に終了
                    break;
                }
            }
        });
    }
}
