use anyhow::Result;
use gpui::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ffi::c_void;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

// ウィンドウの可視化
pub fn set_window_visibility(window: &Window, visible: bool) -> Result<()> {
    let hwnd = convert_window_handle(window)?;

    if visible {
        set_window_position(window)?;
        set_window_opacity(window, 255)?;
        unsafe {
            ShowWindow(hwnd, SW_SHOWNOACTIVATE).ok()?;
        }
    } else {
        set_window_opacity(window, 0)?;
    }
    Ok(())
}

// ウィンドウの位置指定
pub fn set_window_position(window: &Window) -> Result<()> {
    let hwnd = convert_window_handle(window)?;

    let uflags = SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER;

    let mut point = POINT { x: 0, y: 0 };

    unsafe {
        if GetCursorPos(&mut point).is_ok() {
            SetWindowPos(hwnd, None, point.x + 20, point.y + 20, 0, 0, uflags)?;
        };
    }

    Ok(())
}

// 指定されたwindowの最前面固定を設定
pub fn set_always_on_top(window: &Window, enabled: bool) -> Result<()> {
    let hwnd = convert_window_handle(window)?;
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
pub fn set_click_through(window: &Window) -> Result<()> {
    let hwnd = convert_window_handle(window)?;
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
pub fn set_window_opacity(window: &Window, opacity: u8) -> Result<()> {
    let hwnd = convert_window_handle(window)?;
    unsafe {
        // opacity: 0 (透明) - 255 (不透明)
        SetLayeredWindowAttributes(hwnd, COLORREF(0x00000000), opacity, LWA_ALPHA)?;
        Ok(())
    }
}

pub fn convert_window_handle(window: &Window) -> Result<HWND> {
    // window_handle()はgpuiとraw_window_handleにそれぞれ存在している
    // HasWindowHandle::window_handleでraw_window_handle側のメソッドを呼び、windowHandleを取得
    let window_handle = HasWindowHandle::window_handle(window)
        .map_err(|e| anyhow::anyhow!("Window handle error: {:?}", e))?;
    let raw_window_handle = window_handle.as_ref();
    // Win32か判定
    let RawWindowHandle::Win32(handle) = *raw_window_handle else {
        unreachable!();
    };
    // rawWindowHandleから整数を取得し、voidポインタにキャスト
    // windowsクレートはポインタとして扱われている
    Ok(HWND(handle.hwnd.get() as *mut c_void))
}
