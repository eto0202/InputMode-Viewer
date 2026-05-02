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
    let uia: IUIAutomation = unsafe { CoCreateInstance(&CUIAutomation8, None, CLSCTX_ALL) }
        .context("Failed to initialize IUIAutomation")?;
    // キャッシュリクエスト
    let cache = create_cache_request(&uia).context("Failed to create cache request")?;

    log::info!("IUIAutomation initialization successful");
    Ok((uia, cache))
}

fn create_cache_request(uia: &IUIAutomation) -> anyhow::Result<IUIAutomationCacheRequest> {
    let cache_req =
        unsafe { uia.CreateCacheRequest() }.context("Failed to create cache request")?;

    // RawViewに設定し、すべての要素を無視せず表示
    // これを設定しないとInnerTextBlockが無視される
    let filter = unsafe { uia.RawViewCondition() }?;
    unsafe { cache_req.SetTreeFilter(&filter) }.context("Failed to SetTreeFilter")?;

    // 取得したいプロパティ
    // 探索用
    [
        UIA_NamePropertyId,
        UIA_AutomationIdPropertyId,
        UIA_IsOffscreenPropertyId,
        UIA_RuntimeIdPropertyId,
        UIA_IsEnabledPropertyId,
        UIA_ControlTypePropertyId,
    ]
    .into_iter()
    .try_for_each(|id| unsafe { cache_req.AddProperty(id) })?;

    [
        UIA_TextPatternId,
        UIA_TextEditPatternId,
        UIA_ValuePatternId,
    ]
    .into_iter()
    .try_for_each(|id| unsafe { cache_req.AddPattern(id) })?;

    // 検索範囲
    unsafe { cache_req.SetTreeScope(TreeScope_Element) }.context("Failed SetTreeScope")?;

    Ok(cache_req)
}

// キャッシュされたUIA要素リストから指定のIDのIUIAutomationElementを取得
pub fn find_element(
    array: &IUIAutomationElementArray,
    id: &'static str,
) -> anyhow::Result<IUIAutomationElement> {
    let len = unsafe { array.Length() }?;

    for i in 0..len {
        // 早期リターン
        let el = unsafe { array.GetElement(i) }?;

        if skip_err!(unsafe { el.CachedAutomationId() }) != id {
            continue;
        }

        if skip_err!(unsafe { el.CachedIsOffscreen() }).as_bool() {
            continue;
        }

        let glyph = skip_err!(unsafe { el.CachedName() }).to_string();

        if matches!(InputMode::from_glyph(&glyph), InputMode::Unknown) {
            continue;
        }

        return Ok(el);
    }

    Err(anyhow::anyhow!("Element Not Available"))
}
