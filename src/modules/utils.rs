use gpui::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ffi::c_void;
use windows::Win32::Foundation::*;
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::WindowsAndMessaging::*;

// RawViewWalkerを使って子孫を走査し、条件に合う要素の名前を返す
pub fn find_ime_char_recursive(
    walker: &IUIAutomationTreeWalker,
    element: &IUIAutomationElement,
) -> Option<String> {
    unsafe {
        if let Ok(id) = element.CurrentAutomationId() {
            if id.to_string() == "InnerTextBlock" {
                // かつ、表示中であること
                if !element.CurrentIsOffscreen().unwrap().as_bool() {
                    let name = element.CurrentName().unwrap_or_default().to_string();
                    match name.as_str() {
                        "\u{e986}" | // ひらがな (あ)
                        "\u{e97e}" | // 半角英数 (A)
                        "\u{e987}" | // 全角カタカナ (カ)
                        "\u{e981}" | // 全角英数 (Ａ)
                        "\u{e988}" | // 半角カタカナ (ｶ)
                        "\u{e982}"   // 半角英数 (A - 別バリエーション)
                        => {
                            println!("CurrentName: {:?}", name);
                            return Some(name)
                        },
                        _ => {} // 他のアイコン（Wi-Fi等）は無視
                    }
                }
            }
        }
        // 子要素
        if let Ok(mut child) = walker.GetFirstChildElement(element) {
            loop {
                if let Some(res) = find_ime_char_recursive(walker, &child) {
                    return Some(res);
                }
                // 次の兄弟要素
                match walker.GetNextSiblingElement(&child) {
                    Ok(next) => child = next,
                    Err(_) => break,
                }
            }
        }
        None
    }
}

// グリフの文字コードから、IMEがONか,表示用文字列のタプルを返す
pub fn get_ime_status(char_code: char) -> (bool, &'static str) {
    match char_code as u32 {
        0xE986 => (true, "ひらがな (あ)"),
        0xE987 => (true, "全角カタカナ (カ)"),
        0xE981 => (true, "全角英数 (Ａ)"),
        0xE988 => (true, "半角カタカナ (ｶ)"),

        0xE971 | 0xE97E | 0xE982 => (false, "半角英数 (A)"),

        0xE989 => (false, "IME無効 (×)"),
        _ => (false, "不明なモード"),
    }
}

// メモリ上のバイト列から画像をデコードしアイコンを生成
// アプリケーション内に画像が保存される
pub fn load_icon(to_include_bytes: &[u8]) -> tray_icon::Icon {
    let img = image::load_from_memory(to_include_bytes)
        .unwrap()
        .into_rgba8();
    let (width, height) = img.dimensions();
    let rgba = img.into_raw();

    tray_icon::Icon::from_rgba(rgba, width, height).unwrap()
}

// ウィンドウの可視化
pub fn set_window_visibility(window: &Window, visible: bool) {
    unsafe {
        let hwnd = convert_window_handle(window);

        if visible {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        } else {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }
}

// ウィンドウの位置指定
pub fn set_window_position(window: &Window) {
    unsafe {
        let hwnd = convert_window_handle(window);

        let uflags = SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER;

        let mut point = POINT { x: 0, y: 0 };

        if GetCursorPos(&mut point).is_ok() {
            SetWindowPos(hwnd, None, point.x + 20, point.y + 20, 0, 0, uflags).unwrap();
        };
    }
}

// 指定されたwindowの最前面固定を設定
pub fn set_always_on_top(window: &Window, enabled: bool) {
    let hwnd = convert_window_handle(window);
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
        SetWindowPos(hwnd, Some(insert_after), 0, 0, 0, 0, uflags).unwrap();
    }
}

// クリック透過
pub fn set_click_through(window: &Window) {
    let hwnd = convert_window_handle(window);
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
    }
}

pub fn convert_window_handle(window: &Window) -> HWND {
    // window_handle()はgpuiとraw_window_handleにそれぞれ存在している
    // HasWindowHandle::window_handleでraw_window_handle側のメソッドを呼び、windowHandleを取得
    let window_handle = HasWindowHandle::window_handle(window).unwrap();
    let raw_window_handle = window_handle.as_ref();
    // Win32か判定
    let RawWindowHandle::Win32(handle) = *raw_window_handle else {
        unreachable!();
    };
    // rawWindowHandleから整数を取得し、voidポインタにキャスト
    // windowsクレートはポインタとして扱われている
    HWND(handle.hwnd.get() as *mut c_void)
}
