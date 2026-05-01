use crate::core::app::prelude::*;

pub struct AppCore {
    pub cfg: Arc<RwLock<AppConfig>>,
    pub tray_icon: TrayIcon,
    pub proxy_window: Window,
    pub mw: ManagedWindow,
    pub renderer: DCompRenderer,
}

impl AppCore {
    pub fn new(
        el: &ActiveEventLoop,
        cfg: Arc<RwLock<AppConfig>>,
        mode: InputMode,
    ) -> anyhow::Result<Self> {
        // プロキシウィンドウを作成
        // メインウィンドウを消した時にアプリ自体が終了してしまうことがあるため
        // イベントを受け取るための身代わりとして用意
        let attr = WindowAttributes::default()
            .with_visible(false)
            .with_active(false)
            .with_skip_taskbar(true)
            .with_decorations(false)
            .with_max_inner_size(LogicalSize::new(1, 1))
            .with_position(LogicalPosition::new(0, 0));
        let proxy_window = el.create_window(attr)?;

        let style = AppCore::get_style(&cfg, cfg.read().active_role)?;

        let (info, _scale) = utils::monitor_info()?;
        let mut mw = ManagedWindow::new(el, cfg.read().active_role, info)?;

        win32::set_window_style(mw.hwnd)?;

        let (renderer, w, h) = DCompRenderer::new(mw.hwnd, mode, &style, mw.window.scale_factor())
            .expect("DCompRenderer Failed");

        mw.l_size = LogicalSize::new(w, h);

        // トレイアイコン
        let tray_icon = tray::tray_icon()?;

        Ok(Self {
            cfg,
            tray_icon,
            proxy_window,
            mw,
            renderer,
        })
    }

    // モードが変化した時に、ウィンドウサイズを再計算
    pub fn try_resize(
        cfg: &Arc<RwLock<AppConfig>>,
        renderer: &DCompRenderer,
        mode: InputMode,
        role: WindowRole,
    ) -> anyhow::Result<PhysicalSize<f32>> {
        let style = AppCore::get_style(cfg, role).context("No style")?;
        let (w, h) = renderer.calc_metrics(mode).context("Calc metrics failed")?;

        let p = style.padding;
        let final_size = PhysicalSize::new((w + p * 2.0).ceil(), (h + p * 2.0).ceil());

        Ok(final_size)
    }

    // スタイルの取得
    pub fn get_style(
        cfg: &Arc<RwLock<AppConfig>>,
        role: WindowRole,
    ) -> anyhow::Result<WindowStyle> {
        // ロックを取得
        let lock = cfg.read();
        // ガードをWindowStyleだけに絞り込む
        let style = RwLockReadGuard::map(lock, |cfg| match role {
            WindowRole::Floating => &cfg.floating.style,
            WindowRole::Fixed => &cfg.fixed.style,
        });

        Ok(style.clone())
    }
}
