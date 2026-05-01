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

// ウィンドウの位置指定
pub fn set_window_position(hwnd: HWND, x: i32, y: i32) -> anyhow::Result<()> {
    // SWP_NOACTIVATE: フォーカスを奪わない
    // SWP_NOSIZE: サイズは変えない
    // SWP_ASYNCWINDOWPOS: スレッドをブロックせずに座標を送る
    // SWP_NOCOPYBITS: 描画バッファのコピーをスキップ（DCompなので不要）
    let uflags = SWP_NOSIZE | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS | SWP_NOCOPYBITS;

    unsafe { SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, 0, 0, uflags) }?;
    Ok(())
}

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
        let style = GetWindowLongW(hwnd, GWL_STYLE);
        let new_style = (style as u32 & !(WS_OVERLAPPED | WS_CAPTION | WS_THICKFRAME).0)
            | WS_POPUP.0
            | WS_VISIBLE.0;

        SetWindowLongW(hwnd, GWL_STYLE, new_style as i32);
    }

    // 拡張スタイル
    unsafe {
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
        SetWindowLongW(
            hwnd,
            GWL_EXSTYLE,
            ex_style
                | (WS_EX_LAYERED
                    | WS_EX_TRANSPARENT
                    | WS_EX_NOACTIVATE
                    | WS_EX_TOOLWINDOW
                    | WS_EX_TOPMOST)
                    .0 as i32
                | 0x00200000, // WS_EX_NOREDIRECTIONBITMAP
        );
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
            -10000,
            -10000,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED | SWP_NOREDRAW,
        )
    }?;
    Ok(())
}

// ウィンドウの透明度
pub fn set_window_opacity(hwnd: HWND, opacity: u8) -> anyhow::Result<()> {
    // opacity: 0 (透明) - 255 (不透明)
    unsafe { SetLayeredWindowAttributes(hwnd, COLORREF(0), opacity, LWA_ALPHA) }?;
    Ok(())
}

// WS_POPUP に書き換え、枠線やタイトルバーに関連するフラグをすべて除去
pub fn set_window_popup(hwnd: HWND) -> anyhow::Result<()> {
    let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) };

    let new_style = (style as u32
        & !(WS_OVERLAPPED | WS_CAPTION | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX).0)
        | WS_POPUP.0;

    unsafe {
        SetWindowLongW(hwnd, GWL_STYLE, new_style as i32);
    };

    Ok(())
}

pub fn get_hwnd(has_handle: &impl HasWindowHandle) -> anyhow::Result<HWND> {
    match has_handle.window_handle()?.as_raw() {
        RawWindowHandle::Win32(h) => Ok(HWND(h.hwnd.get() as _)),
        _ => Err(anyhow!("Not Win32")),
    }
}
