#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use input_mode_viewer::{core::utils, run::app_run};
use windows::Win32::UI::{
    HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext},
    WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW},
};
use windows_core::{HSTRING, w};

// TODO: デバック画面を実装し、未設定のグリフ、ログファイルを表示

fn main() -> anyhow::Result<()> {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    utils::init_logger()?;
    log::info!("Logger initialized successful");

    // パニックが起きた時に、自動的に log::error! に流す設定
    std::panic::set_hook(Box::new(|panic_info| {
        // パニックメッセージの取得を試みる
        let payload = panic_info.payload();
        let message = if let Some(s) = payload.downcast_ref::<&str>() {
            *s
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.as_str()
        } else {
            "Unknown panic message"
        };

        // パニック発生場所を取得
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        // ログファイルにエラーとして書き込む
        log::error!("PANIC occurred at {}: {}", location, message);
    }));

    if let Err(e) = app_run() {
        let error_msg = format!("{:?}", e);
        log::error!("Fatal error: {}", error_msg);

        // Windowsのメッセージボックスを表示
        unsafe {
            MessageBoxW(
                None,
                &HSTRING::from(&error_msg),
                w!("Application Error"),
                MB_OK | MB_ICONERROR,
            );
        }
        std::process::exit(1);
    }
    log::info!("Main process started successfully");
    Ok(())
}
