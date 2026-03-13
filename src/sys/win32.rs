use anyhow::Result;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

// ウィンドウの可視化
pub fn set_window_visibility(hwnd: HWND, visible: bool) -> Result<()> {
    if visible {
        set_window_position(hwnd)?;
        set_window_opacity(hwnd, 180)?;
        unsafe {
            ShowWindow(hwnd, SW_SHOWNOACTIVATE).ok()?;
        }
    } else {
        set_window_opacity(hwnd, 0)?;
    }
    Ok(())
}

// ウィンドウの位置指定
pub fn set_window_position(hwnd: HWND) -> Result<()> {
    let uflags = SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER | SWP_ASYNCWINDOWPOS | SWP_NOCOPYBITS;

    let mut point = POINT { x: 0, y: 0 };

    unsafe {
        if GetCursorPos(&mut point).is_ok() {
            SetWindowPos(hwnd, None, point.x + 20, point.y + 20, 0, 0, uflags)?;
        };
    }

    Ok(())
}

// 指定されたwindowの最前面固定を設定
pub fn set_always_on_top(hwnd: HWND, enabled: bool) -> Result<()> {
    // 最前面を切り替える
    let insert_after = if enabled {
        // 最前面レイヤー
        HWND_TOPMOST
    } else {
        // 通常レイヤー
        HWND_NOTOPMOST
    };
    // 位置とサイズを変更しない
    let uflags = SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW;

    unsafe {
        SetWindowPos(hwnd, Some(insert_after), 0, 0, 0, 0, uflags)?;
    }

    Ok(())
}

// クリック透過
pub fn set_click_through(hwnd: HWND) -> Result<()> {
    unsafe {
        let current_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
        SetWindowLongW(
            hwnd,
            GWL_EXSTYLE,
            // WS_EX_NOACTIVATE: クリックしてもフォーカスを移さない
            // WS_EX_TRANSPARENT: マウス入力を透過
            // WS_EX_LAYERED: 透過ウィンドウ化
            current_style | (WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE).0 as i32,
        );
        Ok(())
    }
}

// ウィンドウの透明度
// SW_HIDEはウィンドウを冬眠させ、gpuiの描画が止まってしまう可能性がある。透明度を変更することで可視と不可視を切り替える。
pub fn set_window_opacity(hwnd: HWND, opacity: u8) -> Result<()> {
    unsafe {
        // opacity: 0 (透明) - 255 (不透明)
        SetLayeredWindowAttributes(hwnd, COLORREF(0x00000000), opacity, LWA_ALPHA)?;
        Ok(())
    }
}
