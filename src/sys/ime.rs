use crate::sys::config::*;
use crate::*;
use std::sync::mpsc;
use std::time::Duration;
use windows::Win32::System::Com::*;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::*;

pub fn ime_event(tx: mpsc::Sender<InputMode>) -> windows::core::Result<()> {
    unsafe {
        let rx = sys::hooks::event_loop();
        // 初期化処理
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED).is_ok();
        let uia: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)?;

        let root = uia.GetRootElement()?;

        // 指定したID情報のキャッシュ
        let cache_request = sys::utils::create_ime_cache_request(&uia)?;

        // タスクバーウィンドウを特定
        let tray_condition =
            uia.CreatePropertyCondition(UIA_ClassNamePropertyId, &VARIANT::from("Shell_TrayWnd"))?;

        // SystemTrayIcon内のテキストを特定
        let btn_condition = uia.CreatePropertyCondition(
            UIA_AutomationIdPropertyId,
            &VARIANT::from("InnerTextBlock"),
        )?;

        // IUIAutomationElementを保持して処理を軽減
        let mut cached_tray: Option<IUIAutomationElement> = None;

        // IME状態保持用
        let mut last_mode: InputMode = InputMode::Unknown;

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
                    println!("Active Window Changed - IME Check");
                }
                Err(_) => {}
            }

            // キャッシュが無い場合のみ検索
            if cached_tray.is_none() {
                // タスクバー本体(Shell_TrayWnd)を見つける
                if let Ok(tray) = root.FindFirst(TreeScope_Children, &tray_condition) {
                    cached_tray = Some(tray);
                }
            }

            // キャッシュがある場合
            if let Some(ref tray) = cached_tray {
                let elements_array =
                    tray.FindAllBuildCache(TreeScope_Descendants, &btn_condition, &cache_request)?;

                // NameプロパティからInputModeを取得
                let input_mode = sys::utils::find_id(elements_array, "InnerTextBlock");
                
                // InputModeからIMEのオンオフを取得
                let is_ime_active = InputMode::is_on(&input_mode);
                

                // 現在入力状態かどうか
                let has_input_capability = sys::input::input_capability(&uia);
                let should_show = match has_input_capability {
                    sys::input::InputCapability::Yes => true, // 入力欄ならIME状態問わず表示
                    sys::input::InputCapability::No => false, // 入力不可なら絶対に出さない
                    sys::input::InputCapability::Unknown => is_ime_active, // 判別不能ならONの時だけ救済表示
                };
                if should_show {
                    // 前回のモードと違う、あるいは前回は非表示だった場合
                    if input_mode != last_mode || is_interaction {
                        // println!("IME: {:?} (Cap: {:?})",input_mode, has_input_capability);
                        let _ = tx.send(input_mode);
                        last_mode = input_mode;
                    }
                } else {
                    // 表示すべきでない状況で前回まで表示していた場合
                    if last_mode != InputMode::Unknown {
                        let _ = tx.send(InputMode::Unknown);
                        // println!("入力不可能 (Cap: {:?})", has_input_capability);
                        last_mode = InputMode::Unknown;
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
