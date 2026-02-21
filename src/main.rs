use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::mpsc::Sender;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::LRESULT;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::System::Com::*;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

// TODO:
// ただし、モードの表示は入力状態移行時と、無操作状態が指定秒数経過後のみ。

// グローバルな送信機
// コールバック関数からメインスレッドへ合図を送るため
static EVENT_SENDER: OnceLock<Mutex<Sender<AppEvent>>> = OnceLock::new();

enum AppEvent {
    CheckRequest,
}

// ウィンドウフォーカス切り替えフック
unsafe extern "system" fn win_event_proc(
    _h_win_event_hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dw_ms_event_time: u32,
) {
    // フォーカスが切り替わったら通知を送る
    if event == EVENT_SYSTEM_FOREGROUND && hwnd.0.is_null() != true {
        send_event();
    }
}

// キー入力フック
unsafe extern "system" fn keyboard_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    unsafe {
        if n_code >= 0 && w_param.0 == WM_KEYDOWN as usize {
            let kbd = &*(l_param.0 as *const KBDLLHOOKSTRUCT);

            match VIRTUAL_KEY(kbd.vkCode as u16) {
                VK_KANJI | VK_OEM_AUTO | VK_OEM_ENLW | VK_CAPITAL | VK_CONVERT | VK_NONCONVERT
                | VK_MODECHANGE | VK_IME_OFF | VK_IME_ON => {
                    send_event();
                }
                _ => {}
            }
        }
        // 次のフックへ流す。これがないとキー入力が出来なくなる。
        CallNextHookEx(None, n_code, w_param, l_param)
    }
}
// 通知送信用
fn send_event() {
    if let Some(sender_mutex) = EVENT_SENDER.get() {
        if let Ok(tx) = sender_mutex.lock() {
            let _ = tx.send(AppEvent::CheckRequest);
        }
    }
}

fn main() -> windows::core::Result<()> {
    // チャンネル作成
    let (tx, rx) = channel::<AppEvent>();

    // 送信機をセット
    EVENT_SENDER.set(Mutex::new(tx)).ok();

    // フック監視用の別スレッドを機動
    thread::spawn(|| unsafe {
        // ウィンドウフック
        let win_hook = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(win_event_proc), // コールバック関数を指定
            0,
            0,
            WINEVENT_OUTOFCONTEXT, // 外部プロセスとして監視
        );
        // キー入力フック
        let kbd_hook = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(keyboard_proc),
            Some(HINSTANCE(0 as *mut c_void)),
            0,
        )
        .unwrap();

        // メッセージループ
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            DispatchMessageW(&msg);
        }

        let _ = UnhookWinEvent(win_hook);
        let _ = UnhookWindowsHookEx(kbd_hook);
    });

    // メインスレッドでUIAロジックを動かす
    unsafe {
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
            let event = rx.recv_timeout(Duration::from_secs(3));
            // ウィンドウを切り替えた瞬間にrx.recv_timeoutが解除されチェック処理が走る

            // 定期チェック
            // ウィンドウの切り替え通知が来ないまま時間経過
            // Err(Timeout)が返ってくる
            // 下の行へ進み、IMEの状態を取得して表示を更新

            // 即時反応
            // 別スレッドのフック関数が動き、チャネルにAppEvent::FocusChangedを送信
            // recv_timeoutは1秒経っていなくても即座にOk(AppEvent)を返して待機を終了
            // 下の行へ進み、IMEの状態を取得して表示を更新
            match event {
                Ok(AppEvent::CheckRequest) => {
                    println!("Active Window Changed - IME Check");
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
                match find_ime_char_recursive(&walker, tray) {
                    Some(current_mode_char) => {
                        // 変化が合った時だけ表示して状態を更新
                        if current_mode_char != last_mode_char {
                            println!("IME Mode: {}", convert_str(&current_mode_char));
                            // 状態更新
                            last_mode_char = current_mode_char;
                        } else {
                            println!("IME Mode: {}", convert_str(&current_mode_char));
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

// RawViewWalkerを使って子孫を走査し、条件に合う要素の名前を返す
fn find_ime_char_recursive(
    walker: &IUIAutomationTreeWalker,
    element: &IUIAutomationElement,
) -> Option<String> {
    unsafe {
        if let Ok(id) = element.CurrentAutomationId() {
            if id.to_string() == "InnerTextBlock" {
                // かつ、表示中であること
                if !element.CurrentIsOffscreen().unwrap().as_bool() {
                    let name = element.CurrentName().unwrap_or_default().to_string();
                    match name.as_str() {
                        "\u{e986}" | // ひらがな (あ)
                        "\u{e97e}" | // 半角英数 (A)
                        "\u{e987}" | // 全角カタカナ (カ)
                        "\u{e981}" | // 全角英数 (Ａ)
                        "\u{e988}" | // 半角カタカナ (ｶ)
                        "\u{e982}"   // 半角英数 (A - 別バリエーション)
                        => return Some(name),
                        _ => {} // 他のアイコン（Wi-Fi等）は無視
                    }
                }
            }
        }
        // 子要素
        if let Ok(mut child) = walker.GetFirstChildElement(element) {
            loop {
                if let Some(res) = find_ime_char_recursive(walker, &child) {
                    return Some(res);
                }
                // 次の兄弟要素
                match walker.GetNextSiblingElement(&child) {
                    Ok(next) => child = next,
                    Err(_) => break,
                }
            }
        }
        None
    }
}

fn convert_str(string: &String) -> &'static str {
    let char = string.chars().next().unwrap_or_default();
    let hex_code = format!("U+{:04X}", char as u32);
    match hex_code.as_str() {
        "U+E986" => "ひらがな (あ)",
        "U+E971" => "半角英数 (A)",
        "U+E97E" => "半角英数 (A)",
        "U+E987" => "全角カタカナ (カ)",
        "U+E981" => "全角英数 (Ａ)",
        "U+E989" => "半角カタカナ (ｶ)",
        _ => "その他のモード",
    }
}
