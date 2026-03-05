use crate::app::controller::Message;
use crate::sys::hooks::AppEvent;
use anyhow::Result;
use std::sync::mpsc;
use std::thread;
use windows::Win32::System::Com::*;
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::Input::Ime::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::Interface;

// スレッドを抜ける時に自動でCoUninitializeを呼ぶためのガード
struct ComGuard;
impl Drop for ComGuard {
    fn drop(&mut self) {
        println!("input COM drop");
        unsafe {
            CoUninitialize();
        }
    }
}

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

pub fn input_thread(tx: mpsc::Sender<Message>, rx: mpsc::Receiver<AppEvent>) {
    thread::spawn(move || -> Result<()> {
        unsafe {
            // COMの初期化
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;

            let _guard = ComGuard;

            // uia取得
            let uia: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)?;
            // キャッシュリクエスト
            let cache = uia.CreateCacheRequest()?;
            // RawViewに設定し、すべての要素を無視せず表示
            // これを設定しないとInnerTextBlockが無視される
            cache.SetTreeFilter(&uia.RawViewCondition()?)?;

            // 取得したいプロパティ
            cache.AddProperty(UIA_IsEnabledPropertyId)?;
            cache.AddProperty(UIA_ControlTypePropertyId)?;
            cache.AddPattern(UIA_TextPatternId)?;
            cache.AddPattern(UIA_TextEditPatternId)?;
            cache.AddPattern(UIA_ValuePatternId)?;

            // 検索範囲
            cache.SetTreeScope(TreeScope_Element)?;

            // hooksからの通知を待機
            loop {
                println!("--- input_thread ---");
                let event = rx.recv_timeout(std::time::Duration::from_millis(5000));
                match event {
                    Ok(AppEvent::CheckRequest) => {
                        tx.send(Message::Cap(
                            input_capability(&uia, &cache).unwrap_or_default(),
                        ))?;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        break;
                    }
                    Err(_) => {}
                }
            }
            Ok(())
        }
    });
}

// 外部ウィンドウのテキスト入力可能性を確認
pub fn input_capability(
    uia: &IUIAutomation,
    cache: &IUIAutomationCacheRequest,
) -> Result<InputCapability> {
    unsafe {
        println!("--- Input_capability check ---");
        // フォーカス要素の取得
        let Ok(el) = uia.GetFocusedElementBuildCache(cache) else {
            return Ok(win32_input_capability());
        };
        // println!("フォーカス要素: {:?}", el);
        // 要素が無効化されていないかチェック
        if let Some(enabled) = el.CachedIsEnabled().ok() {
            if !enabled.as_bool() {
                return Ok(InputCapability::No);
            }
        }

        // TextPatternかTextEditPatternが存在する
        let has_text_pattern = el.GetCachedPattern(UIA_TextPatternId).is_ok()
            || el.GetCachedPattern(UIA_TextEditPatternId).is_ok();
        if has_text_pattern {
            return Ok(if is_read_only(&el) {
                InputCapability::No
            } else {
                InputCapability::Yes
            });
        }

        // ControlTypeのチェック
        let Some(control_type) = el.CachedControlType().ok() else {
            return Ok(InputCapability::No);
        };

        #[allow(non_upper_case_globals)]
        let cap = match control_type {
            UIA_EditControlTypeId | UIA_DocumentControlTypeId => {
                if is_read_only(&el) {
                    InputCapability::No
                } else {
                    InputCapability::Yes
                }
            }
            UIA_CustomControlTypeId | UIA_WindowControlTypeId => {
                // 入力可能かつカーソルがIビーム（テキスト要素）
                if !is_read_only(&el) && is_cursor_ibeam() {
                    InputCapability::Yes
                } else {
                    InputCapability::No
                }
            }
            _ => {
                // その他のIDのうち、カーソルがIビームならテキスト要素として判別する
                if is_cursor_ibeam() {
                    InputCapability::Unknown
                } else {
                    InputCapability::No
                }
            }
        };

        // 判別不可能な場合
        println!("UIA 判別不可能");
        if cap == InputCapability::No {
            return Ok(win32_input_capability());
        }

        Ok(cap)
    }
}

// マウスカーソルの形状判定
fn is_cursor_ibeam() -> bool {
    unsafe {
        let mut info = CURSORINFO::default();
        info.cbSize = std::mem::size_of::<CURSORINFO>() as u32;

        if GetCursorInfo(&mut info).is_ok() {
            // カーソルが表示されているかチェック
            if (info.flags.0 & CURSOR_SHOWING.0) != 0 {
                // 標準のIビームカーソルをロード
                let beam_cursor = LoadCursorW(None, IDC_IBEAM).unwrap_or_default();
                // ハンドルを比較
                // println!("カーソルがIビーム: {:?}", info.hCursor == beam_cursor);
                return info.hCursor == beam_cursor;
            }
        }
        println!("カーソルの形状判定に失敗");
        false
    }
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
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return InputCapability::No;
        }

        let thread_id = GetWindowThreadProcessId(hwnd, None);

        // キャレットが存在し、点滅しているか
        let mut info = GUITHREADINFO::default();
        info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;

        if GetGUIThreadInfo(thread_id, &mut info).is_ok() {
            // キャレットが見えていて点滅中
            if (info.flags & GUI_CARETBLINKING).0 != 0 {
                // println!("キャレットが見えていて点滅中");
                return InputCapability::Yes;
            }
        }
        // IMEコンテキストが有効か
        let himc = ImmGetContext(hwnd);
        if !himc.0.is_null() {
            let _ = ImmReleaseContext(hwnd, himc);
            // println!("IMMコンテキストが有効");
            return InputCapability::Yes;
        }
        println!("win32 判別不能");
        InputCapability::Unknown
    }
}
