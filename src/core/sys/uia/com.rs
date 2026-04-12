use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};
use windows_core::HRESULT;

// スレッドを抜ける時に自動でCoUninitializeを呼ぶためのガード
pub struct ComGuard;
impl ComGuard {
    pub fn new() -> anyhow::Result<Self> {
        unsafe {
            let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
            hr.ok()?;
            // S_FALSE : すでに初期化済み
            if hr == HRESULT(1) {
                println!("COM already initialized");
            }
        }
        println!("COM Initialize");
        Ok(ComGuard)
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        };
        println!("COM Drop");
    }
}
