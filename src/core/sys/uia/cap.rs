use crate::core::app::controller::Message;
use crate::core::sys::hooks::AppEvent;
use crate::core::sys::uia::com;
use crate::core::sys::uia::utils::uia_init;
use anyhow::Context;
use std::sync::mpsc;
use std::thread;
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::Input::Ime::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::Interface;
use winit::event_loop::EventLoopProxy;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum InputCapability {
    // 入力欄である(UIAで確認済み、またはキャレットがある)
    Yes,
    // 入力欄ではない(ボタン、背景、読み取り専用など)
    No,
    // 判別不能
    #[default]
    Unknown,
}

pub fn cap_thread(proxy: EventLoopProxy<Message>, rx: mpsc::Receiver<AppEvent>) {
    thread::spawn(move || {
        let _guard = com::ComGuard::new();

        loop {
            if let Err(e) = run_cap_monitor(&proxy, &rx) {
                eprintln!("Cap Monitor Error: {:?}. Restarting...", e);
                thread::sleep(std::time::Duration::from_secs(3));
            } else {
                // エラーなしで戻ってきた場合はスレッドを完全に終了
                break;
            }
        }
    });
}

fn run_cap_monitor(
    proxy: &EventLoopProxy<Message>,
    rx: &mpsc::Receiver<AppEvent>,
) -> anyhow::Result<()> {
    let (uia, cache_request) = uia_init().context("UIA初期化に失敗")?;
    let mut last_cap = InputCapability::Unknown;
    let mut last_processed = std::time::Instant::now();

    loop {
        // イベント受信
        // 送信側がいなくなったらスレッドを終了
        let event = rx.recv()?;
        match event {
            AppEvent::CheckRequest => {
                // デバウンス処理
                if last_processed.elapsed() < std::time::Duration::from_millis(200) {
                    continue;
                }
                println!("cap_thread: Event Received");
                // 入力可能性を取得
                let current_cap = input_capability(&uia, &cache_request)?;
                // 前回と違う場合のみ通知
                if last_cap != current_cap {
                    proxy.send_event(Message::Cap(current_cap))?;
                }

                last_cap = current_cap;
                last_processed = std::time::Instant::now();
            }
        }
    }
}

// 外部ウィンドウのテキスト入力可能性を確認
fn input_capability(
    uia: &IUIAutomation,
    cache: &IUIAutomationCacheRequest,
) -> anyhow::Result<InputCapability> {
    println!("-- Input_capability Check --");

    if win32_input_capability() == InputCapability::Yes {
        return Ok(InputCapability::Yes);
    }

    // フォーカス要素の取得
    let res = unsafe { uia.GetFocusedElementBuildCache(cache) };
    let Ok(el) = res else {
        return Ok(win32_input_capability());
    };

    // 要素が無効化されていないかチェック
    if let Some(enabled) = unsafe { el.CachedIsEnabled() }.ok() {
        if !enabled.as_bool() {
            return Ok(InputCapability::No);
        }
    }

    // TextPatternかTextEditPatternが存在する
    let has_text_pattern = unsafe {
        el.GetCachedPattern(UIA_TextPatternId).is_ok()
            || el.GetCachedPattern(UIA_TextEditPatternId).is_ok()
    };

    // ControlTypeのチェック
    let Some(control_type) = unsafe { el.CachedControlType() }.ok() else {
        return Ok(InputCapability::No);
    };

    println!("control type: {:?}", control_type);

    #[allow(non_upper_case_globals)]
    let cap = match control_type {
        UIA_EditControlTypeId => {
            if !is_read_only(&el) {
                InputCapability::Yes
            } else {
                InputCapability::No
            }
        }
        UIA_PaneControlTypeId | UIA_CustomControlTypeId | UIA_WindowControlTypeId => {
            if !is_read_only(&el) && (is_cursor_ibeam() || has_text_pattern) {
                InputCapability::Yes
            } else {
                InputCapability::Unknown
            }
        }
        _ => {
            if is_cursor_ibeam() {
                InputCapability::Unknown
            } else {
                InputCapability::No
            }
        }
    };

    // 判別不可能な場合
    if cap == InputCapability::Unknown {
        return Ok(win32_input_capability());
    }

    Ok(cap)
}

// マウスカーソルの形状判定
fn is_cursor_ibeam() -> bool {
    let mut info = CURSORINFO::default();
    info.cbSize = std::mem::size_of::<CURSORINFO>() as u32;

    if unsafe { GetCursorInfo(&mut info) }.is_ok() {
        // カーソルが表示されているかチェック
        if (info.flags.0 & CURSOR_SHOWING.0) != 0 {
            // 標準のIビームカーソルをロード
            let beam_cursor = unsafe { LoadCursorW(None, IDC_IBEAM) }.unwrap_or_default();
            // ハンドルを比較
            // println!("カーソルがIビーム: {:?}", info.hCursor == beam_cursor);
            return info.hCursor == beam_cursor;
        }
    }

    false
}

// 読み取り専用チェック
fn is_read_only(element: &IUIAutomationElement) -> bool {
    unsafe {
        // IUnknownを返すのでIUIAutomationValuePatternにキャストする
        if let Ok(pattern_unk) = element.GetCachedPattern(UIA_ValuePatternId) {
            // パターンを持っていればキャストを試みる
            if let Ok(value_pattern) = pattern_unk.cast::<IUIAutomationValuePattern>() {
                // ReadOnlyかチェック
                if let Ok(read_only) = value_pattern.CachedIsReadOnly() {
                    // println!("読み取り専用: {:?}", read_only.as_bool());
                    return read_only.as_bool();
                }
            }
        }

        false
    }
}

// UIAで判定できない場合
fn win32_input_capability() -> InputCapability {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return InputCapability::No;
    }

    let thread_id = unsafe { GetWindowThreadProcessId(hwnd, None) };

    // キャレットが存在し、点滅しているか

    let mut info = GUITHREADINFO::default();
    info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;

    let is_input_active = (info.flags.0 & (GUI_CARETBLINKING.0 | GUI_INMENUMODE.0)) != 0;

    if unsafe { GetGUIThreadInfo(thread_id, &mut info).is_ok() } {
        // キャレットが見えていて点滅中
        if is_input_active || !info.hwndCaret.0.is_null() {
            // println!("キャレットが見えていて点滅中");
            return InputCapability::Yes;
        }
    }
    // IMEコンテキストが有効か
    let himc = unsafe { ImmGetContext(hwnd) };
    if !himc.0.is_null() {
        let _ = unsafe { ImmReleaseContext(hwnd, himc) };
        // println!("IMMコンテキストが有効");
        return InputCapability::Yes;
    }

    InputCapability::Unknown
}
