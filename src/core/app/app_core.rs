use arc_swap::ArcSwap;
use winit::dpi::PhysicalPosition;

use crate::core::app::{calc::VirtualScreen, prelude::*};

pub struct AppCore {
    pub cfg: Arc<ArcSwap<AppConfig>>,
    pub tray_icon: TrayIcon,
    pub mw: MainWindow,
    pub renderer: DCompRenderer,
}

impl AppCore {
    #[instrument(skip(el, cfg, v_screen))]
    pub fn new(
        el: &ActiveEventLoop,
        cfg: Arc<ArcSwap<AppConfig>>,
        mode: InputMode,
        v_screen: Guard<Arc<VirtualScreen>>,
    ) -> anyhow::Result<Self> {
        let guard = cfg.load();
        let role = guard.active_role;

        let style = AppCore::get_style(&cfg, role)
            .context("Failed to extract window style from configuration")?;

        let p_pos = PhysicalPosition::new(v_screen.x as f32, v_screen.y as f32);
        let p_size = PhysicalSize::new(v_screen.cx as f32, v_screen.cy as f32);

        // コンパイル時に画像をバイナリに取り込む
        let bytes = include_bytes!("../../icon.png");

        let mw = MainWindow::new(el, role, p_pos, p_size, bytes)
            .context("Failed to create MainWindow instance")?;
        tracing::info!(?role, ?p_pos, ?p_size, "MainWindow created successfully");

        win_style::set_window_style(mw.hwnd)
            .with_context(|| format!("Failed to apply Win32 styles to HWND: {:?}", mw.hwnd))?;
        tracing::debug!("Applied Win32 window styles (transparency, click-through, etc.)");

        let scale_factor = mw.window.scale_factor();
        let renderer = DCompRenderer::new(mw.hwnd, mode, &style, scale_factor, guard.transparent)
            .context("DCompRenderer initialization failed")?;
        tracing::info!(
            scale_factor,
            transparent = guard.transparent,
            "DirectComposition renderer initialized"
        );

        // トレイアイコン
        let tray_icon = tray::tray_icon(bytes).context("Failed to initialize system tray icon")?;
        tracing::debug!("System tray icon initialized");

        Ok(Self { cfg, tray_icon, mw, renderer })
    }

    // モードが変化した時に、ウィンドウサイズを再計算
    #[instrument(skip(cfg, renderer))]
    pub fn try_resize(
        cfg: &Arc<ArcSwap<AppConfig>>,
        renderer: &DCompRenderer,
        mode: InputMode,
        role: WindowRole,
    ) -> anyhow::Result<PhysicalSize<f32>> {
        let style = AppCore::get_style(cfg, role).context("Failed to get style for resizing")?;
        let metrics = renderer
            .calc_metrics(mode.clone(), style.text_format)
            .context("Failed to calculate text metrics for resizing")?;

        let p = style.padding;
        let final_size = PhysicalSize::new(
            (metrics.width + p * 2.0).ceil(),
            (metrics.height + p * 2.0).ceil(),
        );

        tracing::debug!(?mode, ?final_size, "Calculated new window size");
        Ok(final_size)
    }

    // スタイルの取得
    pub fn get_style(
        cfg: &Arc<ArcSwap<AppConfig>>,
        role: WindowRole,
    ) -> anyhow::Result<WindowStyle> {
        let lock = cfg.load();
        // ガードをWindowStyleだけに絞り込む
        let style = match role {
            WindowRole::Floating => &lock.floating.style,
            WindowRole::Fixed => &lock.fixed.style,
        };

        Ok(style.clone())
    }
}
