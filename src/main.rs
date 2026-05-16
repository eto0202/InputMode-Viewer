#![windows_subsystem = "windows"]
use input_mode_viewer::{core::utils, run::app_run};
use windows::Win32::UI::{
    HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext},
    WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW},
};
use windows_core::{HSTRING, w};

// TODO
// デバック画面を実装し、未設定のグリフ、ログファイルを表示
// キャレット座標は管理者権限なら取得できるかも
// IUIAutomationTextPattern2::GetCaretRangeで取得できるかも
// フェードアウト
// Compactモード時に余白が大きすぎる問題と画面からはみ出る問題
// 表示時にタスクバーの反応が悪い？
// 最前面に不具合
// モニターサイズからトレイメニュー付近を指定
// 最終位置を記憶
// コミット間隔をフレームレートに合わせて負荷を軽減
// フェードインが動作していないっぽい

fn main() -> anyhow::Result<()> {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    utils::init_logger()?;
    log::info!("Logger initialized successful");

    set_panic_hook();
    log::info!("Set panic hook successful");

    if let Err(e) = app_run() {
        let error_msg = format!("{:?}", e);
        log::error!("Fatal error: {}", error_msg);

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

// パニックが起きた時に、自動的に log::error! に流す
fn set_panic_hook() {
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
}
