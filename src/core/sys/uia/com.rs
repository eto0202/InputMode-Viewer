use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize};
use windows_core::HRESULT;

// スレッドを抜ける時に自動でCoUninitializeを呼ぶためのガード
pub struct ComGuard;
impl ComGuard {
    pub fn new(mode: windows::Win32::System::Com::COINIT) -> anyhow::Result<Self> {
        let hr = unsafe { CoInitializeEx(None, mode) };
        hr.ok()?;
        // S_FALSE : すでに初期化済み
        if hr == HRESULT(1) {
            log::debug!("COM already initialized");
        }

        log::info!("COM Initialize");
        Ok(ComGuard)
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        };
        log::debug!("COM Drop");
    }
}
