mod modules;
use modules::*;
use std::time::Duration;
use windows::Win32::System::Com::*;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::*;

// TODO:
// モードの表示は入力状態移行時と、無操作状態が指定秒数経過後のみ。
// ウィンドウの切り替えだけでなく、要素を選択した時にも状態更新

fn main() -> windows::core::Result<()> {
    // メインスレッドでUIAロジックを動かす
    unsafe {
        let rx = hooks::event_loop();

        // 初期化処理
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
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
            let event = rx.recv_timeout(Duration::from_secs(5));
            // イベントを検知した瞬間にrx.recv_timeoutが解除されチェック処理が走る

            // 定期チェック
            // イベントが来ないまま時間経過
            // Err(Timeout)が返ってくる
            // 下の行へ進み、IMEの状態を取得して表示を更新

            // 即時反応
            // 別スレッドのフック関数が動き、チャネルにAppEvent::FocusChangedを送信
            // recv_timeoutは1秒経っていなくても即座にOk(AppEvent)を返して待機を終了
            // 下の行へ進み、IMEの状態を取得して表示を更新
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
                        let has_input_capability = input_capability::text_input_capability(&uia);

                        let should_show = match has_input_capability {
                            input_capability::InputCapability::Yes => true, // 入力欄ならIME状態問わず表示
                            input_capability::InputCapability::No => false, // 入力不可なら絶対に出さない
                            input_capability::InputCapability::Unknown => is_ime_active, // 判別不能ならONの時だけ救済表示
                        };

                        if should_show {
                            // 状態に変化があったときのみ更新
                            if current_glyph != last_mode_char {
                                last_mode_char = current_glyph;
                                println!("IME: {:?} (Cap: {:?})", input_mode, has_input_capability);
                            } else {
                                println!("IME: {:?} (Cap: {:?})", input_mode, has_input_capability);
                            }
                        } else {
                            println!("入力不可能 (Cap: {:?})", has_input_capability);
                        }
                    }
                    None => {
                        println!("IMEがオフ もしくは英語入力");
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
