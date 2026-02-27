use crate::modules::*;
use std::error::Error;
use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::mpsc::*;
use std::thread;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::LRESULT;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

// グローバルな送信機
// コールバック関数からメインスレッドへ合図を送るため
static EVENT_SENDER: OnceLock<Mutex<Sender<AppEvent>>> = OnceLock::new();

pub enum AppEvent {
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
unsafe extern "system" fn keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
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

// クリックイベント
unsafe extern "system" fn mouse_proc(ncode: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    unsafe {
        if ncode >= 0 && w_param.0 == WM_LBUTTONDOWN as usize {
            send_event();
        }
        CallNextHookEx(None, ncode, w_param, l_param)
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

pub fn event_loop() -> Receiver<hooks::AppEvent> {
    // チャンネル作成
    let (tx, rx) = channel::<hooks::AppEvent>();

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
        // クリックフック
        let mouse_hook = SetWindowsHookExW(
            WH_MOUSE_LL,
            Some(mouse_proc),
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
        let _ = UnhookWindowsHookEx(mouse_hook);
    });

    rx
}
