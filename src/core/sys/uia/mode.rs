use crate::core::{
    app::controller::Message,
    sys::{
        hooks::AppEvent,
        uia::{com, init::uia_init, text::*, *},
    },
};
use anyhow::Context;
use std::{sync::*, thread};
use windows::Win32::{
    System::{Com::COINIT_MULTITHREADED, Variant::VARIANT},
    UI::Accessibility::*,
};
use winit::event_loop::EventLoopProxy;

pub fn mode_thread(proxy: EventLoopProxy<Message>, rx: mpsc::Receiver<AppEvent>) {
    thread::spawn(move || {
        let _guard = com::ComGuard::new(COINIT_MULTITHREADED);

        // エラーが起きている間はリトライし続ける
        while let Err(e) = run_monitor_loop(&proxy, &rx) {
            log::warn!("IME Monitor Error: {:?}. Restarting...", e);
            thread::sleep(std::time::Duration::from_secs(3));
        }
    });
}

// 監視のメインロジック
fn run_monitor_loop(
    proxy: &EventLoopProxy<Message>,
    rx: &mpsc::Receiver<AppEvent>,
) -> anyhow::Result<()> {
    let mut ime = ImeMonitor::new()?;
    log::info!("Run ImeMonitor successful");

    let mut processed = std::time::Instant::now();
    let mut mode = InputMode::new();

    loop {
        // イベント受信
        // 送信側がいなくなったらスレッドを終了
        let event = rx.recv()?;
        match event {
            AppEvent::CheckRequest => {
                // デバウンス処理
                if processed.elapsed() < std::time::Duration::from_millis(200) {
                    continue;
                }
                log::debug!("uia_thread: Event Received");

                // ゲームなど起きるIME変更の遅延の対策
                for i in 0..3 {
                    // IMEの状態を取得
                    let cur_mode = ime.fetch_current_mode().unwrap_or_default();

                    // 前回と違うモードが取れたら、即座に送信して終了
                    if cur_mode != mode {
                        proxy.send_event(Message::Mode(cur_mode))?;
                        mode = cur_mode;
                        break;
                    }

                    // 3回目の試行なら、同じ値でもキー入力があった事実として送る
                    if i == 2 {
                        proxy.send_event(Message::Mode(mode))?;
                    }

                    std::thread::sleep(std::time::Duration::from_millis(10));
                }

                processed = std::time::Instant::now();
            }
        }
    }
}

struct ImeMonitor {
    #[allow(dead_code)]
    uia: IUIAutomation,
    root: IUIAutomationElement,
    cache_req: IUIAutomationCacheRequest,
    tray_wnd: IUIAutomationCondition,
    text_block: IUIAutomationCondition,
    cached: Option<IUIAutomationElement>,
}

impl ImeMonitor {
    fn new() -> anyhow::Result<Self> {
        let (uia, cache_req) = uia_init().context("Failed to initialize UIA")?;

        let (root, tray_wnd, text_block) = unsafe {
            let root = uia
                .GetRootElement()
                .context("Failed to load IUIAutomationElement")?;

            // タスクバーウィンドウを特定
            let tray_wnd = uia
                .CreatePropertyCondition(UIA_ClassNamePropertyId, &VARIANT::from("Shell_TrayWnd"))
                .context("Failed to create Shell_TrayWnd condition")?;

            // SystemTrayIcon内のテキストを特定
            let text_block = uia
                .CreatePropertyCondition(
                    UIA_AutomationIdPropertyId,
                    &VARIANT::from("InnerTextBlock"),
                )
                .context("Failed to create InnerTextBlock condition")?;

            (root, tray_wnd, text_block)
        };

        Ok(Self {
            uia,
            root,
            cache_req,
            tray_wnd,
            text_block,
            cached: None,
        })
    }

    fn fetch_current_mode(&mut self) -> anyhow::Result<InputMode> {
        // 生存確認
        let needs_refresh = self
            .cached
            .as_ref()
            .map(|el| unsafe { el.CurrentProcessId().is_err() })
            .unwrap_or(true); // None ならリフレッシュ必要

        if needs_refresh {
            self.cached = unsafe { self.root.FindFirst(TreeScope_Children, &self.tray_wnd) }.ok();
        }

        let uia_el = self.cached.as_ref().context("Element not found")?;

        let els = unsafe {
            uia_el.FindAllBuildCache(TreeScope_Descendants, &self.text_block, &self.cache_req)
        }?;

        let el = init::find_element(&els, "InnerTextBlock")?;
        let name = unsafe { el.CachedName() }?;

        Ok(InputMode::from_glyph(&name.to_string()))
    }
}
