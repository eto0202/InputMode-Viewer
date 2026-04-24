use strum::IntoEnumIterator;

use crate::core::app::prelude::*;

pub struct AppCore {
    pub config: Arc<RwLock<AppConfig>>,
    pub tray_icon: TrayIcon,
    pub proxy_window: Window,
    pub windows: HashMap<WindowId, ManagedWindow>,
}

impl AppCore {
    pub fn new(
        el: &ActiveEventLoop,
        config: Arc<RwLock<AppConfig>>,
        last_mode: InputMode,
    ) -> anyhow::Result<Self> {
        // プロキシウィンドウを作成
        // メインウィンドウを消した時にアプリ自体が終了してしまうことがあるため
        // イベントを受け取るための身代わりとして用意
        let attr = WindowAttributes::default()
            .with_visible(false)
            .with_active(false)
            .with_skip_taskbar(true)
            .with_decorations(false)
            .with_max_inner_size(LogicalSize::new(0, 0))
            .with_position(LogicalPosition::new(0, 0));
        let proxy_window = el.create_window(attr)?;

        let mut windows = HashMap::new();

        for role in WindowRole::iter() {
            let style = AppCore::get_style(&config, role)?;
            let mut mw = ManagedWindow::new(el, role)?;
            let id = mw.window.id();

            let (r, w, h) =
                DCompRenderer::new(mw.hwnd, last_mode, &style, mw.window.scale_factor())?;
            (mw.render_stack, mw.l_size) = (Some(r), LogicalSize::new(w, h));

            let is_enabled = match role {
                WindowRole::Floating => config.read().floating.enabled,
                WindowRole::Fixed => config.read().fixed.enabled,
            };

            mw.config_enabled = is_enabled;

            windows.insert(id, mw);
        }

        // トレイアイコン
        let tray_icon = tray::tray_icon()?;

        Ok(Self {
            config,
            tray_icon,
            proxy_window,
            windows,
        })
    }

    // ロール検索 参照
    pub fn find_by_role(&self, role: WindowRole) -> anyhow::Result<&ManagedWindow> {
        self.windows
            .values()
            .find(|w| w.role == role)
            .context("Not find role")
    }

    // ロール検索 可変参照
    pub fn find_by_role_mut(&mut self, role: WindowRole) -> anyhow::Result<&mut ManagedWindow> {
        self.windows
            .values_mut()
            .find(|w| w.role == role)
            .context("Not find role")
    }

    // モードが変化した時に、ウィンドウサイズを再計算
    pub fn try_resize(
        cfg: &Arc<RwLock<AppConfig>>,
        renderer: &DCompRenderer,
        mode: InputMode,
        role: WindowRole,
    ) -> anyhow::Result<LogicalSize<f32>> {
        let style = AppCore::get_style(cfg, role).context("No style")?;
        let (w, h) = renderer.calc_metrics(mode).context("Calc metrics failed")?;

        let p = style.padding;
        let final_size = LogicalSize::new((w + p * 2.0).ceil(), (h + p * 2.0).ceil());

        Ok(final_size)
    }

    // スタイルの取得
    pub fn get_style(
        cfg: &Arc<RwLock<AppConfig>>,
        role: WindowRole,
    ) -> anyhow::Result<WindowStyle> {
        // ロックを取得
        let guard = cfg.read();
        // ガードをWindowStyleだけに絞り込む
        let style = RwLockReadGuard::map(guard, |cfg| match role {
            WindowRole::Floating => &cfg.floating.style,
            WindowRole::Fixed => &cfg.fixed.style,
        });

        Ok(style.clone())
    }
}
