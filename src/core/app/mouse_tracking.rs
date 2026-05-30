use crate::core::app::prelude::*;

// 通知用メッセージ
pub enum ControlMessage {
    ResetPosition,
    Terminate,
    Refresh,
}

pub struct PositionController {
    tx: Sender<ControlMessage>,
    wake_event: SendHandle,
}

impl PositionController {
    pub fn send(&self, msg: ControlMessage) -> anyhow::Result<()> {
        self.tx.send(msg)?;
        unsafe {
            SetEvent(self.wake_event.0)?;
        }
        Ok(())
    }
}

impl Drop for PositionController {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.wake_event.0);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)] // メモリレイアウトを元の型と同じにする
pub struct SendHandle(pub HANDLE);
// Send と Syncを許可
unsafe impl Send for SendHandle {}
unsafe impl Sync for SendHandle {}

pub fn spawn_position_thread(
    core: &AppCore,
    state: &AppState,
) -> anyhow::Result<PositionController> {
    let shared = Arc::clone(&state.shared);
    let cfg = Arc::clone(&core.cfg);
    let renderer = core.renderer.get_controller();

    // 自動リセットイベント
    let h = unsafe { CreateEventW(None, false, false, None)? };
    let wake_event = SendHandle(h);

    // スレッドに渡すためにコピー
    let thread_wake_event = wake_event;

    // メインスレッドからの通知用
    let (tx, rx) = mpsc::channel::<ControlMessage>();

    std::thread::spawn(move || {
        let _guard = RoGuard::new(RO_INIT_MULTITHREADED);

        // 自スレッドのハンドルを取得し優先度を上げる
        unsafe {
            let thread_handle = GetCurrentThread();
            // THREAD_PRIORITY_HIGHEST: 標準より2段階高い
            // THREAD_PRIORITY_ABOVE_NORMAL: 標準より1段階高い
            if let Err(e) = SetThreadPriority(thread_handle, THREAD_PRIORITY_HIGHEST) {
                log::warn!("Failed to set thread priority: {:?}", e);
            }
        }

        // エラーが起きている間はリトライし続ける
        while let Err(e) = run_positon_thread(
            shared.clone(),
            cfg.clone(),
            renderer.clone(),
            &rx,
            thread_wake_event,
        ) {
            log::warn!("Position thread Error: {:?}. Restarting...", e);
            std::thread::sleep(Duration::from_secs(3));
        }
    });

    Ok(PositionController { tx, wake_event })
}

fn run_positon_thread(
    shared: Arc<SharedState>,
    cfg_swap: Arc<ArcSwap<AppConfig>>,
    renderer: RendererController,
    rx: &Receiver<ControlMessage>,
    wake_event: SendHandle,
) -> anyhow::Result<()> {
    const INTERVAL_60HZ: i64 = -166_666; // 100ns単位 (16.6ms)
    const INTERVAL_120HZ: i64 = -83_333;
    const INTERVAL_240HZ: i64 = -41_666;
    const INTERVAL_480HZ: i64 = -20_833;

    let timer = WaitableTimer::new().context("Faild to create timer")?;
    let mut is_tracking = false;
    let mut last_move_time = Instant::now();
    let mut last_pt = POINT::default();

    // 待ち受けハンドルの配列
    let handles_idle = [wake_event.0];
    let handles_tracking = [wake_event.0, timer.handle];

    register_rawinput_devices()?;

    // 初回起動時に一度だけ位置合わせ
    let init_cfg = cfg_swap.load();
    let mut current_pt = POINT::default();
    unsafe {
        GetCursorPos(&mut current_pt)?;
    }
    update_position(&shared, &init_cfg, &renderer, current_pt)?;

    loop {
        let cfg = cfg_swap.load();
        let displayed = shared.displayed.load(Ordering::Relaxed);

        while let Ok(ctrl) = rx.try_recv() {
            match ctrl {
                ControlMessage::ResetPosition | ControlMessage::Refresh => {
                    log::info!("Position reset triggered by config change.");
                    // アニメーションが動いているプロパティに対して直接値をセットすると
                    // 現在実行中のアニメーションが強制的に切断され、静的な値が優先される
                    // renderer の mouse_tracking を呼び出し、PropertySet の値は更新されているが
                    // sprite_visual の Offset プロパティは固定モードの時にアニメーションとの接続が切れたまま
                    // PropertySet と Offset を繋ぐ ExpressionAnimation を再接続させる
                    renderer.mouse_expr_start()?;

                    let mut current_pt = POINT::default();
                    unsafe {
                        GetCursorPos(&mut current_pt)?;
                    }
                    update_position(&shared, &cfg, &renderer, current_pt)?;

                    is_tracking = true;
                }
                ControlMessage::Terminate => return Ok(()),
            }
        }

        // 待機処理
        if !displayed {
            is_tracking = false;
            unsafe {
                MsgWaitForMultipleObjectsEx(
                    Some(&handles_idle),
                    INFINITE,
                    QS_POSTMESSAGE,
                    MWMO_INPUTAVAILABLE,
                );
            }
            // 起きたらWM_INPUTなどを一掃
            let mut msg = MSG::default();
            unsafe { while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {} }
            continue;
        } else if !is_tracking {
            unsafe {
                MsgWaitForMultipleObjectsEx(
                    Some(&handles_idle),
                    INFINITE,
                    QS_RAWINPUT | QS_POSTMESSAGE,
                    MWMO_INPUTAVAILABLE,
                );
            }
        } else {
            unsafe {
                MsgWaitForMultipleObjectsEx(
                    Some(&handles_tracking),
                    100,
                    QS_POSTMESSAGE,
                    MWMO_INPUTAVAILABLE,
                );
            }
        }

        // メッセージの一掃
        let mut msg = MSG::default();
        let mut raw_notified = false;
        unsafe {
            // フィルタを 0, 0 にして全メッセージをキューから抜かないとビジーループになる
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_INPUT {
                    raw_notified = true;
                }
                // WM_INPUT以外も Dispatch しておかないとシステムが詰まる場合がある
                DispatchMessageW(&msg);
            }
        }

        if !displayed {
            continue;
        }

        // マウスが動いたフラグの更新
        if raw_notified {
            is_tracking = true;
            last_move_time = Instant::now();
        }

        let mut current_pt = POINT::default();
        unsafe {
            GetCursorPos(&mut current_pt)?;
        }

        if current_pt != last_pt {
            update_position(&shared, &cfg, &renderer, current_pt)?;
            last_pt = current_pt;
            last_move_time = Instant::now();
            is_tracking = true;
        } else if is_tracking && last_move_time.elapsed() > Duration::from_secs(2) {
            // 2秒間変化がなければ追従終了
            is_tracking = false;
        }

        //  追従中のみ次回のタイマーを予約
        if is_tracking {
            let interval = if displayed && cfg.active_role != WindowRole::Fixed {
                match cfg.quality {
                    RenderingQuality::Performance => INTERVAL_60HZ,
                    RenderingQuality::Balanced => INTERVAL_120HZ,
                    RenderingQuality::HighQuality => INTERVAL_240HZ,
                    RenderingQuality::Ultra => INTERVAL_480HZ,
                }
            } else {
                INTERVAL_60HZ
            };

            timer.set_timer(interval)?;
        }
    }
}

fn update_position(
    shared: &SharedState,
    cfg: &AppConfig,
    renderer: &RendererController,
    current_pt: POINT,
) -> anyhow::Result<()> {
    let v_screen = shared.v_screen.load();
    let metrics = shared.metrics.load();

    match cfg.active_role {
        WindowRole::Floating => {
            renderer.mouse_tracking(
                current_pt.x - v_screen.x + cfg.floating.offset.x,
                current_pt.y - v_screen.y + cfg.floating.offset.y,
            )?;
            {
                let mut lock = shared.floating.lock();
                *lock = current_pt;
            }
        }
        WindowRole::Fixed => {
            // Fixedモードの計算
            // めんどくさいので毎回座標を取得
            let (info, s) = calc::monitor_info(current_pt)?;
            let m = cfg.fixed.margin;
            let p = cfg.fixed.style.padding;
            // **metrics: DWRITE_TEXT_METRICS
            let pos = calc::fixed_position(**metrics, &cfg.fixed.pos, m, p, info, s)?;
            renderer.set_position((pos.x - v_screen.x) as f32, (pos.y - v_screen.y) as f32)?;
            {
                let mut lock = shared.fixed.lock();
                *lock = pos;
            }
        }
    }
    Ok(())
}

pub struct RoGuard;

impl RoGuard {
    pub fn new(init_type: RO_INIT_TYPE) -> windows::core::Result<Self> {
        unsafe {
            // RoInitialize は成功すると S_OK (0) か S_FALSE (1) を返す
            RoInitialize(init_type)?;
        }
        Ok(RoGuard)
    }
}

impl Drop for RoGuard {
    fn drop(&mut self) {
        unsafe {
            RoUninitialize();
        }
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn register_rawinput_devices() -> anyhow::Result<()> {
    unsafe {
        let instance = GetModuleHandleW(None)?;
        let window_class = w!("RawInputWindow");

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            lpszClassName: window_class,
            ..Default::default()
        };

        RegisterClassExW(&wc);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            window_class,
            w!(""),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE), // メッセージ専用
            None,
            Some(HINSTANCE(instance.0)),
            None,
        )?;

        let rid = RAWINPUTDEVICE {
            usUsagePage: 0x01,                          // HID_USAGE_PAGE_GENERIC
            usUsage: 0x02,                              // HID_USAGE_GENERIC_MOUSE
            dwFlags: RIDEV_INPUTSINK | RIDEV_DEVNOTIFY, // バックグラウンドでも受信
            hwndTarget: hwnd,
        };

        RegisterRawInputDevices(&[rid], std::mem::size_of::<RAWINPUTDEVICE>() as u32)?;
    }
    Ok(())
}

struct WaitableTimer {
    handle: HANDLE,
}

impl WaitableTimer {
    fn new() -> Option<Self> {
        let handle = unsafe {
            CreateWaitableTimerExW(
                None,
                None,
                CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
                TIMER_ALL_ACCESS.0,
            )
        };

        match handle {
            Err(e) => {
                log::warn!("Failed to get WaitableTimer HANDLE: {}", e);
                None
            }
            Ok(handle) => Some(Self { handle }),
        }
    }

    fn set_timer(&self, interval: i64) -> windows_core::Result<()> {
        unsafe {
            SetWaitableTimer(self.handle, &interval, 0, None, None, false)?;
        }
        Ok(())
    }

    fn _wait_timer(&self) {
        unsafe {
            WaitForSingleObject(self.handle, INFINITE);
        }
    }
}

impl Drop for WaitableTimer {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            unsafe {
                log::info!("WaitableTimer Closing handle...");
                let _ = CloseHandle(self.handle);
            }
        }
    }
}
