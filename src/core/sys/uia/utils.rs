use crate::{core::sys::uia::text::InputMode, skip_err};
use anyhow::Context;
use std::sync::{Mutex, OnceLock};
use windows::Win32::{
    System::Com::{CLSCTX_ALL, CoCreateInstance},
    UI::Accessibility::*,
};

// アプリ全体で共有されるUIA初期化用の鍵
//  UIAの初期化は、複数のスレッドで同時に行うとクラッシュしたりバグったりするらしい
pub static UIA_INIT_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn get_uia_lock() -> &'static Mutex<()> {
    // 初めて呼ばれた時にMutexを中に入れて、次からはそれを返す
    UIA_INIT_LOCK.get_or_init(|| Mutex::new(()))
}

pub fn uia_init() -> anyhow::Result<(IUIAutomation, IUIAutomationCacheRequest)> {
    let _lock = get_uia_lock().lock().unwrap();
    // uia取得
    let uia: IUIAutomation = unsafe {
        CoCreateInstance(&CUIAutomation8, None, CLSCTX_ALL).context("UIA取得に失敗")?
    };
    // キャッシュリクエスト
    let cache = create_cache_request(&uia).context("キャッシュリクエスト作成に失敗")?;

    Ok((uia, cache))
}

fn create_cache_request(uia: &IUIAutomation) -> anyhow::Result<IUIAutomationCacheRequest> {
    unsafe {
        let cache_req = uia
            .CreateCacheRequest()
            .context("Failed CreateCacheRequest")?;

        // RawViewに設定し、すべての要素を無視せず表示
        // これを設定しないとInnerTextBlockが無視される
        cache_req
            .SetTreeFilter(&uia.RawViewCondition()?)
            .context("Failed SetTreeFilter")?;

        // 取得したいプロパティ
        // 探索用
        cache_req.AddProperty(UIA_NamePropertyId)?;
        cache_req.AddProperty(UIA_AutomationIdPropertyId)?;
        cache_req.AddProperty(UIA_IsOffscreenPropertyId)?;
        cache_req.AddProperty(UIA_RuntimeIdPropertyId)?;
        // 入力判定用
        cache_req.AddProperty(UIA_IsEnabledPropertyId)?;
        cache_req.AddProperty(UIA_ControlTypePropertyId)?;
        cache_req.AddPattern(UIA_TextPatternId)?;
        cache_req.AddPattern(UIA_TextEditPatternId)?;
        cache_req.AddPattern(UIA_ValuePatternId)?;

        // 検索範囲
        cache_req
            .SetTreeScope(TreeScope_Element)
            .context("Failed SetTreeScope")?;

        Ok(cache_req)
    }
}

// キャッシュされたUIA要素リストから指定のIDのIUIAutomationElementを取得
pub fn find_element(
    array: &IUIAutomationElementArray,
    id: &'static str,
) -> anyhow::Result<IUIAutomationElement> {
    unsafe {
        // 早期リターン
        let len = array.Length()?;
        for i in 0..len {
            // 早期リターン
            let el = array.GetElement(i)?;

            if skip_err!(el.CachedAutomationId()) != id {
                continue;
            }

            if crate::skip_err!(el.CachedIsOffscreen()).as_bool() {
                continue;
            }

            let name = crate::skip_err!(el.CachedName()).to_string();
            if matches!(InputMode::from_glyph(&name), InputMode::Unknown) {
                continue;
            }

            return Ok(el);
        }

        Err(anyhow::anyhow!("Element Not Available"))
    }
}
