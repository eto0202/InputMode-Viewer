use std::{
    sync::{Mutex, OnceLock, mpsc::Sender, *},
    thread,
};
use windows::Win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
    UI::{Accessibility::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

use crate::{guard_opt, guard_res};

// Dropガード
struct HookGuard(HWINEVENTHOOK, HHOOK, HHOOK);
impl Drop for HookGuard {
    fn drop(&mut self) {
        log::debug!("Unhook drop");
        unsafe {
            let _ = UnhookWinEvent(self.0);
            let _ = UnhookWindowsHookEx(self.1);
            let _ = UnhookWindowsHookEx(self.2);
        }
    }
}

// グローバルな送信機
// コールバック関数からメインスレッドへ合図を送るため
// std::sync::mpsc::Senderは、そのままでは複数のスレッドで同時に共有して使うことが出来ない
static EVENT_SENDER: OnceLock<Mutex<Sender<AppEvent>>> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
pub enum AppEvent {
    CheckRequest,
}

// 通知送信用
pub fn send_event() {
    let sender_mutex = guard_opt!(EVENT_SENDER.get());
    let tx = guard_res!(sender_mutex.lock());
    let _ = tx.send(AppEvent::CheckRequest);
}

// フォーカス切り替え
unsafe extern "system" fn win_event_proc(
    _h_win_event_hook: HWINEVENTHOOK,
    event: u32,
    _hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dw_ms_event_time: u32,
) {
    if event == EVENT_OBJECT_FOCUS {
        send_event();
    }
}

// キー入力フック
unsafe extern "system" fn keyboard_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if n_code >= 0 && w_param.0 == WM_KEYUP as usize {
        let kbd = unsafe { &*(l_param.0 as *const KBDLLHOOKSTRUCT) };
        match VIRTUAL_KEY(kbd.vkCode as u16) {
            VK_KANJI | VK_OEM_AUTO | VK_OEM_ENLW | VK_CAPITAL | VK_CONVERT | VK_NONCONVERT
            | VK_MODECHANGE | VK_IME_OFF | VK_IME_ON => {
                send_event();
            }
            _ => {}
        }
    }
    // 次のフックへ流す。これがないとキー入力が出来なくなる。
    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}

// クリック
unsafe extern "system" fn mouse_proc(ncode: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if ncode >= 0 && w_param.0 == WM_LBUTTONUP as usize {
        send_event();
    }
    unsafe { CallNextHookEx(None, ncode, w_param, l_param) }
}

pub fn win_hooks() -> mpsc::Receiver<AppEvent> {
    // チャンネル作成
    let (tx, rx) = mpsc::channel::<AppEvent>();

    // 送信機をセット
    EVENT_SENDER.set(Mutex::new(tx)).unwrap();

    // フック監視用の別スレッドを機動
    thread::spawn(|| -> anyhow::Result<()> {
        let (win_hook, kbd_hook, mouse_hook) = unsafe {
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

            log::info!("Set hook successful");
            (win_hook, kbd_hook, mouse_hook)
        };

        let _guard = HookGuard(win_hook, kbd_hook, mouse_hook);

        // メッセージループ
        let mut msg = MSG::default();
        // 0 (false) -1 (エラー) それ以外 (true)
        unsafe {
            while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
                DispatchMessageW(&msg);
            }
        };
        Ok(())
    });
    rx
}
