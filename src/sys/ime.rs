use crate::*;
use std::sync::mpsc;
use std::time::Duration;
use windows::Win32::System::Com::*;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::*;

pub fn ime_event(tx: mpsc::Sender<String>) -> windows::core::Result<()> {
    unsafe {
        let rx = sys::hooks::event_loop();
        // 初期化処理
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED).is_ok();
        let uia: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)?;

        let root = uia.GetRootElement()?;
        // タスクバーウィンドウを特定
        let tray_condition =
            uia.CreatePropertyCondition(UIA_ClassNamePropertyId, &VARIANT::from("Shell_TrayWnd"))?;

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
                Ok(sys::hooks::AppEvent::CheckRequest) => {
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
                match sys::utils::find_ime_char(&walker, tray) {
                    Some(current_glyph) => {
                        let char_code = current_glyph.chars().next().unwrap_or_default();
                        let (is_ime_active, input_mode) = sys::utils::get_ime_status(char_code);
                        // 判定結果
                        let has_input_capability = sys::input::input_capability(&uia);

                        let should_show = match has_input_capability {
                            sys::input::InputCapability::Yes => true, // 入力欄ならIME状態問わず表示
                            sys::input::InputCapability::No => false, // 入力不可なら絶対に出さない
                            sys::input::InputCapability::Unknown => is_ime_active, // 判別不能ならONの時だけ救済表示
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
