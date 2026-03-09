use anyhow::Result;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::mpsc::Sender;
use std::sync::*;
use std::thread;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::LRESULT;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

// Dropガード
struct HookGuard(HWINEVENTHOOK, HHOOK, HHOOK);
impl Drop for HookGuard {
    fn drop(&mut self) {
        dbg!("Unhook drop");
        unsafe {
            let _ = UnhookWinEvent(self.0);
            let _ = UnhookWindowsHookEx(self.1);
            let _ = UnhookWindowsHookEx(self.2);
        }
    }
}

// グローバルな送信機
// コールバック関数からメインスレッドへ合図を送るため
static EVENT_SENDER: OnceLock<Mutex<Sender<AppEvent>>> = OnceLock::new();

#[derive(Debug, Clone)]
pub enum AppEvent {
    CheckRequest,
}

// フォーカス切り替えフック
unsafe extern "system" fn win_event_proc(
    _h_win_event_hook: HWINEVENTHOOK,
    event: u32,
    _hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dw_ms_event_time: u32,
) {
    match event {
        EVENT_OBJECT_FOCUS => {
            println!("EVENT_OBJECT_FOCUS");
        }
        _ => {}
    }
}

// キー入力フック
unsafe extern "system" fn keyboard_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    unsafe {
        if n_code >= 0 && w_param.0 == WM_KEYUP as usize {
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
        if ncode >= 0 && w_param.0 == WM_LBUTTONUP as usize {
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

pub fn win_hooks() -> mpsc::Receiver<AppEvent> {
    // チャンネル作成
    let (tx, rx) = mpsc::channel::<AppEvent>();

    // 送信機をセット
    EVENT_SENDER.set(Mutex::new(tx)).unwrap();

    // フック監視用の別スレッドを機動
    thread::spawn(|| -> Result<()> {
        unsafe {
            // ウィンドウフック
            let win_hook = SetWinEventHook(
                EVENT_OBJECT_FOCUS,
                EVENT_OBJECT_FOCUS,
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
                Some(HINSTANCE::default()),
                0,
            )?;
            // クリックフック
            let mouse_hook =
                SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), Some(HINSTANCE::default()), 0)?;

            let _guard = HookGuard(win_hook, kbd_hook, mouse_hook);

            // メッセージループ
            let mut msg = MSG::default();
            // 0 (false) -1 (エラー) それ以外 (true)
            while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
                DispatchMessageW(&msg);
            }
        }
        Ok(())
    });
    rx
}
