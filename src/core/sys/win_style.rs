use anyhow::*;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::{
    Foundation::*,
    Graphics::Dwm::{
        DWMWA_TRANSITIONS_FORCEDISABLED, DwmExtendFrameIntoClientArea, DwmSetWindowAttribute,
    },
    UI::{Controls::MARGINS, WindowsAndMessaging::*},
};
use windows_core::BOOL;

// 指定されたwindowの最前面固定を設定
pub fn set_always_on_top(hwnd: HWND, enabled: bool) -> anyhow::Result<()> {
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

    unsafe { SetWindowPos(hwnd, Some(insert_after), 0, 0, 0, 0, uflags) }?;

    Ok(())
}

pub fn set_window_style(hwnd: HWND) -> anyhow::Result<()> {
    // 基本スタイル
    unsafe {
        let style = WS_POPUP | WS_VISIBLE;
        SetWindowLongPtrW(hwnd, GWL_STYLE, style.0 as isize);
    }

    // 拡張スタイル
    unsafe {
        let ex_style = WS_EX_LAYERED
            | WS_EX_TRANSPARENT
            | WS_EX_NOACTIVATE
            | WS_EX_TOOLWINDOW
            | WS_EX_TOPMOST
            | WS_EX_NOREDIRECTIONBITMAP;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style.0 as isize);
    };

    // 背景ブラシとDWM設定
    unsafe {
        SetClassLongPtrW(hwnd, GCLP_HBRBACKGROUND, 0);
        let _ = DwmExtendFrameIntoClientArea(
            hwnd,
            &MARGINS {
                cxLeftWidth: -1,
                cxRightWidth: -1,
                cyTopHeight: -1,
                cyBottomHeight: -1,
            },
        );
    }

    // アニメーション無効
    let disable_anim = BOOL(1);
    let _ = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_TRANSITIONS_FORCEDISABLED,
            &disable_anim as *const _ as _,
            4,
        )
    };

    // 最前面 位置指定
    unsafe {
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOREDRAW | SWP_FRAMECHANGED,
        )
    }?;

    Ok(())
}

pub fn get_hwnd(has_handle: &impl HasWindowHandle) -> anyhow::Result<HWND> {
    match has_handle.window_handle()?.as_raw() {
        RawWindowHandle::Win32(h) => Ok(HWND(h.hwnd.get() as _)),
        _ => Err(anyhow!("Not window_handle")),
    }
}
