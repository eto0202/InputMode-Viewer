use windows::Win32::UI::Accessibility::*;

// RawViewWalkerを使って子孫を走査し、条件に合う要素の名前を返す
pub fn find_ime_char(
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
                if let Some(res) = find_ime_char(walker, &child) {
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
