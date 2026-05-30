#![windows_subsystem = "windows"]
use anyhow::Context;
use input_mode_viewer::{
    core::{app::mouse_tracking::RoGuard, sys::new_renderer::init_dispatcher_queue, utils},
    run::app_run,
};
use windows::Win32::{
    System::WinRT::RO_INIT_SINGLETHREADED,
    UI::{
        HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext},
        WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW},
    },
};
use windows_core::{HSTRING, w};

// TODO
// キャレット座標は管理者権限なら取得できるかも
// IUIAutomationTextPattern2::GetCaretRangeで取得できるかも
// 表示時にタスクバーの反応が悪い？
// 最前面に不具合
// モニターサイズからトレイメニュー付近を指定
// 最終位置を記憶
// positon_threadがエラー落ちした時、core.renderer.get_controller() から作り直した新しいコントローラーを渡す
// メインスレッドも自動復旧するように
// リスタート時や復旧時に通知を出す
// on_app_quit が動いていないっぽい

fn main() -> anyhow::Result<()> {
    let _log_guard = utils::init_logger()?;
    // 起動時の環境情報を入れる
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        os = std::env::consts::OS,
        "Application starting..."
    );
    set_panic_hook();

    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    // COMの初期化 (WinRT用)
    let _guard =
        RoGuard::new(RO_INIT_SINGLETHREADED).context("Failed to initialize COM (WinRT)")?;
    // DispatcherQueueの初期化
    let _queue_controller =
        init_dispatcher_queue().context("Failed to initialize DispatcherQueue")?;

    tracing::info!("Environment initialized successfully");

    if let Err(e) = app_run() {
        tracing::error!(error = ?e, "Application terminated with error:\n{:#?}", e);

        // ユーザー通知用メッセージ
        let user_error_msg = format!("{:#?}", e);
        unsafe {
            MessageBoxW(
                None,
                &HSTRING::from(&user_error_msg),
                w!("Application Error"),
                MB_OK | MB_ICONERROR,
            );
        }
        std::process::exit(1);
    }
    tracing::info!("Application exited gracefully");
    Ok(())
}

// パニックが起きた時に、自動的に tracing::error! に流す
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

        // 現在のスパン（どの #[instrument] 関数の中にいるか）を取得
        let span_trace = tracing_error::SpanTrace::capture();

        // tracingのエラーとして記録
        tracing::error!(
            panic_location = %location,
            panic_message = %message,
            span_trace = %span_trace,
            "A panic occurred"
        );
    }));
}
