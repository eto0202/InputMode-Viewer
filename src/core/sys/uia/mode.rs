use crate::core::app::controller::Message;
use crate::core::sys::hooks::AppEvent;
use crate::core::sys::uia::com;
use crate::core::sys::uia::text::*;
use crate::core::sys::uia::utils::uia_init;
use crate::core::sys::uia::*;
use anyhow::Context;
use std::sync::*;
use std::thread;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::*;
use winit::event_loop::EventLoopProxy;

pub fn mode_thread(proxy: EventLoopProxy<Message>, rx: mpsc::Receiver<AppEvent>) {
    thread::spawn(move || {
        let _guard = com::ComGuard::new();

        loop {
            if let Err(e) = run_monitor_loop(&proxy, &rx) {
                eprintln!("IME Monitor Error: {:?}. Restarting...", e);
                thread::sleep(std::time::Duration::from_secs(3));
            } else {
                // エラーなしで戻ってきた場合はスレッドを完全に終了
                break;
            }
        }
    });
}

// 監視のメインロジック
fn run_monitor_loop(
    proxy: &EventLoopProxy<Message>,
    rx: &mpsc::Receiver<AppEvent>,
) -> anyhow::Result<()> {
    let mut ime = ImeMonitor::new()?;
    let mut last_processed = std::time::Instant::now();
    let mut last_sent_mode = InputMode::Unknown;

    loop {
        // イベント受信
        // 送信側がいなくなったらスレッドを終了
        let event = rx.recv()?;
        match event {
            AppEvent::CheckRequest => {
                // デバウンス処理
                if last_processed.elapsed() < std::time::Duration::from_millis(200) {
                    continue;
                }
                println!("uia_thread: Event Received");

                // ゲームなど起きるIME変更の遅延の対策
                for i in 0..3 {
                    // IMEの状態を取得
                    let current_mode = ime.fetch_current_mode().unwrap_or(InputMode::Unknown);

                    // 前回と違うモードが取れたら、即座に送信して終了
                    if current_mode != last_sent_mode {
                        proxy.send_event(Message::Mode(current_mode))?;
                        last_sent_mode = current_mode;
                        break;
                    }

                    // 3回目の試行なら、同じ値でもキー入力があった事実として送る
                    if i == 2 {
                        proxy.send_event(Message::Mode(last_sent_mode))?;
                    }

                    std::thread::sleep(std::time::Duration::from_millis(10));
                }

                last_processed = std::time::Instant::now();
            }
        }
    }
}

struct ImeMonitor {
    #[allow(dead_code)]
    uia: IUIAutomation,
    root: IUIAutomationElement,
    cache_request: IUIAutomationCacheRequest,
    tray_cond: IUIAutomationCondition,
    text_cond: IUIAutomationCondition,
    cached_tray: Option<IUIAutomationElement>,
}

impl ImeMonitor {
    fn new() -> anyhow::Result<Self> {
        let (uia, cache_request) = uia_init().context("UIA初期化に失敗")?;
        let root = unsafe { uia.GetRootElement().context("UIA取得に失敗: uia_thread")? };

        // タスクバーウィンドウを特定
        let tray_cond = unsafe {
            uia.CreatePropertyCondition(UIA_ClassNamePropertyId, &VARIANT::from("Shell_TrayWnd"))
                .context("Condition作成に失敗: uia_thread")?
        };

        // SystemTrayIcon内のテキストを特定
        let text_cond = unsafe {
            uia.CreatePropertyCondition(
                UIA_AutomationIdPropertyId,
                &VARIANT::from("InnerTextBlock"),
            )
            .context("Condition作成に失敗: uia_thread")?
        };

        let cached_tray = None;

        Ok(Self {
            uia,
            root,
            cache_request,
            tray_cond,
            text_cond,
            cached_tray,
        })
    }

    fn fetch_current_mode(&mut self) -> anyhow::Result<InputMode> {
        unsafe {
            // トレイ要素が無い、死んでいる場合にのみ探す
            if self.cached_tray.is_none()
                || self
                    .cached_tray
                    .as_ref()
                    .unwrap()
                    .CurrentProcessId()
                    .is_err()
            {
                self.cached_tray = self
                    .root
                    .FindFirst(TreeScope_Children, &self.tray_cond)
                    .ok();
            }

            let tray = self.cached_tray.as_ref().context("Tray not found")?;

            // InnerTextBlockを探す
            let elements = tray.FindAllBuildCache(
                TreeScope_Descendants,
                &self.text_cond,
                &self.cache_request,
            )?;

            // 要素を特定
            let el = utils::find_element(&elements, "InnerTextBlock")?;
            let name = el.CachedName()?;

            Ok(InputMode::from_glyph(&name.to_string()))
        }
    }
}
