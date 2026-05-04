use winit::dpi::PhysicalPosition;

use crate::core::app::{calc::VirtualScreen, prelude::*};

pub struct AppCore {
    pub cfg: Arc<RwLock<AppConfig>>,
    pub tray_icon: TrayIcon,
    pub mw: MainWindow,
    pub renderer: DCompRenderer,
}

impl AppCore {
    pub fn new(
        el: &ActiveEventLoop,
        cfg: Arc<RwLock<AppConfig>>,
        mode: InputMode,
        v_screen: VirtualScreen,
    ) -> anyhow::Result<Self> {
        log::info!("Create ProxyWindow successful");

        let style = AppCore::get_style(&cfg, cfg.read().active_role)?;

        let p_pos = PhysicalPosition::new(v_screen.x as f32, v_screen.y as f32);
        let p_size = PhysicalSize::new(v_screen.cx as f32, v_screen.cy as f32);

        let mw = MainWindow::new(el, cfg.read().active_role, p_pos, p_size)?;
        log::info!("Create ManagedWindow successful");

        win32::set_window_style(mw.hwnd)?;
        log::info!("Set window style successful");

        let (renderer, _w, _h) = DCompRenderer::new(mw.hwnd, mode, &style, mw.window.scale_factor())
            .context("DCompRenderer Initialize Failed")?;
        log::info!("Create DCompRenderer successful");

        // トレイアイコン
        let tray_icon = tray::tray_icon()?;
        log::info!("Create tray icon successful");

        Ok(Self { cfg, tray_icon, mw, renderer })
    }

    // モードが変化した時に、ウィンドウサイズを再計算
    pub fn try_resize(
        cfg: &Arc<RwLock<AppConfig>>,
        renderer: &DCompRenderer,
        mode: InputMode,
        role: WindowRole,
    ) -> anyhow::Result<PhysicalSize<f32>> {
        let style = AppCore::get_style(cfg, role)?;
        let metrics = renderer.calc_metrics(mode)?;

        let p = style.padding;
        let final_size = PhysicalSize::new(
            (metrics.width + p * 2.0).ceil(),
            (metrics.height + p * 2.0).ceil(),
        );

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
