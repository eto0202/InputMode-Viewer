use crate::core::{
    app::controller::Message,
    sys::{
        hooks::AppEvent,
        uia::{com, text::*, utils::uia_init, *},
    },
};
use anyhow::Context;
use std::{sync::*, thread};
use windows::Win32::{System::Variant::VARIANT, UI::Accessibility::*};
use winit::event_loop::EventLoopProxy;

pub fn mode_thread(proxy: EventLoopProxy<Message>, rx: mpsc::Receiver<AppEvent>) {
    thread::spawn(move || {
        let _guard = com::ComGuard::new();

        // エラーが起きている間はリトライし続ける
        while let Err(e) = run_monitor_loop(&proxy, &rx) {
            eprintln!("IME Monitor Error: {:?}. Restarting...", e);
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
    let mut processed = std::time::Instant::now();
    let mut mode = InputMode::Unknown;

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
                println!("uia_thread: Event Received");

                // ゲームなど起きるIME変更の遅延の対策
                for i in 0..3 {
                    // IMEの状態を取得
                    let cur_mode = ime.fetch_current_mode().unwrap_or(InputMode::Unknown);

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
        let (uia, cache_req) = uia_init().context("UIA初期化に失敗")?;
        let root = unsafe { uia.GetRootElement().context("UIA取得に失敗: uia_thread")? };

        // タスクバーウィンドウを特定
        let tray_wnd = unsafe {
            uia.CreatePropertyCondition(UIA_ClassNamePropertyId, &VARIANT::from("Shell_TrayWnd"))
                .context("Condition作成に失敗: uia_thread")?
        };

        // SystemTrayIcon内のテキストを特定
        let text_block = unsafe {
            uia.CreatePropertyCondition(
                UIA_AutomationIdPropertyId,
                &VARIANT::from("InnerTextBlock"),
            )
            .context("Condition作成に失敗: uia_thread")?
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
        unsafe {
            // トレイ要素が無い、死んでいる場合にのみ探す
            if self.cached.is_none() || self.cached.as_ref().unwrap().CurrentProcessId().is_err() {
                self.cached = self.root.FindFirst(TreeScope_Children, &self.tray_wnd).ok();
            }

            let tray = self.cached.as_ref().context("Tray not found")?;

            // InnerTextBlockを探す
            let els =
                tray.FindAllBuildCache(TreeScope_Descendants, &self.text_block, &self.cache_req)?;

            // 要素を特定
            let el = utils::find_element(&els, "InnerTextBlock")?;
            let name = el.CachedName()?;

            Ok(InputMode::from_glyph(&name.to_string()))
        }
    }
}
