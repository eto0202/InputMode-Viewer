use anyhow::Result;
use windows::Win32::UI::Accessibility::*;

use crate::sys::uia::input_mode::InputMode;

// あらかじめ情報をキャッシュに乗せることで、find_ime_char内での通信コストをゼロに
pub fn create_ime_cache_request(uia: &IUIAutomation) -> Result<IUIAutomationCacheRequest> {
    println!("--- Create ime cache ---");
    unsafe {
        let cache_request = uia.CreateCacheRequest()?;

        // RawViewに設定し、すべての要素を無視せず表示
        // これを設定しないとInnerTextBlockが無視される
        cache_request.SetTreeFilter(&uia.RawViewCondition()?)?;

        // 取得したいプロパティ
        cache_request.AddProperty(UIA_NamePropertyId)?;
        cache_request.AddProperty(UIA_AutomationIdPropertyId)?;
        cache_request.AddProperty(UIA_IsOffscreenPropertyId)?;
        cache_request.AddProperty(UIA_RuntimeIdPropertyId)?;

        // 検索範囲
        cache_request.SetTreeScope(TreeScope_Element)?;

        Ok(cache_request)
    }
}

// キャッシュされたUIA要素リストから指定のIDのIUIAutomationElementを取得
pub fn find_element(
    array: IUIAutomationElementArray,
    target_id: &'static str,
) -> Option<IUIAutomationElement> {
    unsafe {
        (0..array.Length().unwrap_or(0))
            .filter_map(|i| array.GetElement(i).ok())
            .find_map(|el| {
                // IDチェック
                let id = el.CachedAutomationId().ok()?.to_string();
                if id != target_id {
                    return None;
                }

                // 表示状態チェック
                let is_visible_true = el.CachedIsOffscreen().ok()?.as_bool();
                if is_visible_true {
                    return None;
                }

                // "\u{e971}"はIME用では無い可能性がある
                let name = el.CachedName().ok()?.to_string();
                if matches!(InputMode::from_glyph(name), InputMode::Unknown) {
                    return None;
                }

                return Some(el);
            })
    }
}
