use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::Input::Ime::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::Interface;

#[derive(Debug, PartialEq, Eq)]
pub enum InputCapability {
    // 入力欄である(UIAで確認済み、またはキャレットがある)
    Yes,
    // 入力欄ではない(ボタン、背景、読み取り専用など)
    No,
    // 判別不能
    Unknown,
}

// 外部ウィンドウのテキスト入力可能性を確認
pub fn text_input_capability(uia: &IUIAutomation) -> InputCapability {
    unsafe {
        // フォーカス要素の取得
        let Ok(focused_element) = uia.GetFocusedElement() else {
            println!("フォーカス要素の取得に失敗");
            return check_win32_input_capability();
        };
        // println!("フォーカス要素: {:?}", focused_element);
        // 要素が無効化されていないかチェック
        if let Ok(enabled) = focused_element.CurrentIsEnabled() {
            if !enabled.as_bool() {
                println!("要素が無効化されている");
                return InputCapability::No;
            }
        }

        // TextPatternかTextEditPatternが存在する
        if focused_element.GetCurrentPattern(UIA_TextPatternId).is_ok()
            || focused_element
                .GetCurrentPattern(UIA_TextEditPatternId)
                .is_ok()
        {
            if !is_read_only(&focused_element) {
                return InputCapability::Yes;
            }
            return InputCapability::No;
        }
        println!("TextPatternかTextEditPatternが存在しない");

        // ControlTypeのチェック
        let Ok(control_type) = focused_element.CurrentControlType() else {
            println!("ControlTypeが見つからない");
            return InputCapability::No;
        };
        println!("ControlType: {:?}", control_type);

        #[allow(non_upper_case_globals)]
        match control_type {
            UIA_EditControlTypeId | UIA_DocumentControlTypeId => {
                if !is_read_only(&focused_element) {
                    return InputCapability::Yes;
                }
            }
            UIA_CustomControlTypeId | UIA_WindowControlTypeId => {
                // 入力可能かつカーソルがIビーム（テキスト要素）
                if !is_read_only(&focused_element) && is_cursor_ibeam() {
                    return InputCapability::Yes;
                }
            }
            _ => {
                // その他のIDのうち、カーソルがIビームならテキスト要素として判別する
                if is_cursor_ibeam() {
                    return InputCapability::Unknown;
                } else {
                    return InputCapability::No;
                }
            }
        }

        // 判別不可能な場合
        println!("UIA 判別不可能");
        check_win32_input_capability()
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
        if let Ok(pattern_unk) = element.GetCurrentPattern(UIA_ValuePatternId) {
            // パターンを持っていればキャストを試みる
            if let Ok(value_pattern) = pattern_unk.cast::<IUIAutomationValuePattern>() {
                // ReadOnlyかチェック
                if let Ok(read_only) = value_pattern.CurrentIsReadOnly() {
                    // println!("読み取り専用: {:?}", read_only.as_bool());
                    return read_only.as_bool();
                }
            }
        }
        println!("GetCurrentPatternが存在しない");
        false
    }
}

// UIAで判定できない場合
fn check_win32_input_capability() -> InputCapability {
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
