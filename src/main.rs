use std::thread;
use std::time::Duration;
use windows::Win32::System::Com::*;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::*;

// TODO:
// InputMode取得は、アクティブウィンドウ切り替え時とモード切り替え時のみ。内部で状態を保持。
// ただし、モードの表示は入力状態移行時と、無操作状態が指定秒数経過後のみ。

fn main() -> windows::core::Result<()> {
    unsafe {
        // 初期化処理
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
        let uia: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL)?;

        let root = uia.GetRootElement()?;

        // タスクバーウィンドウを特定
        let tray_condition =
            uia.CreatePropertyCondition(UIA_ClassNamePropertyId, &VARIANT::from("Shell_TrayWnd"))?;

        let walker = uia.RawViewWalker()?;

        // IUIAutomationElementを保持して処理を軽減
        let mut cached_tray: Option<IUIAutomationElement> = None;

        loop {
            // キャッシュが無い場合のみ検索
            if cached_tray.is_none() {
                if let Ok(tray) = root.FindFirst(TreeScope_Children, &tray_condition) {
                    cached_tray = Some(tray);
                }
            }

            // キャッシュがある場合
            if let Some(ref tray) = cached_tray {
                match find_ime_char_recursive(&walker, tray) {
                    Some(mode_char) => {
                        let char = mode_char.chars().next().unwrap_or_default();
                        println!("IME Mode: {}", convert_str(char));
                    }
                    None => {
                        println!("IMEがオフ もしくは英語入力");
                    }
                }

                // cached_trayがエクスプローラー再起動などでエラーを返した場合
                if tray.CurrentName().is_err() {
                    cached_tray = None;
                }
            }
            thread::sleep(Duration::from_millis(1000));
        }
    }
}

// RawViewWalkerを使って子孫を走査し、条件に合う要素の名前を返す
fn find_ime_char_recursive(
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
                        => return Some(name),
                        _ => {} // 他のアイコン（Wi-Fi等）は無視
                    }
                }
            }
        }
        // 子要素へ進む
        if let Ok(mut child) = walker.GetFirstChildElement(element) {
            loop {
                if let Some(res) = find_ime_char_recursive(walker, &child) {
                    return Some(res);
                }
                // 次の兄弟要素へ
                match walker.GetNextSiblingElement(&child) {
                    Ok(next) => child = next,
                    Err(_) => break,
                }
            }
        }
        None
    }
}

fn convert_str(char: char) -> &'static str {
    let hex_code = format!("U+{:04X}", char as u32);
    match hex_code.as_str() {
        "U+E986" => "ひらがな (あ)",
        "U+E971" => "半角英数 (A)",
        "U+E97E" => "半角英数 (A)",
        "U+E987" => "全角カタカナ (カ)",
        "U+E981" => "全角英数 (Ａ)",
        "U+E989" => "半角カタカナ (ｶ)",
        _ => "その他のモード",
    }
}
