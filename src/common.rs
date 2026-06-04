use anyhow::Context;
use directories::ProjectDirs;
use gpui::App;
use gpui_component::{Theme, ThemeMode};
use palette::{FromColor, Srgba};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    sync::{
        LazyLock,
        mpsc::{self, Sender},
    },
    time::Duration,
};
use strum_macros::{AsRefStr, EnumIter, EnumString};
use tracing::instrument;
use windows::Win32::{Foundation::POINT, Graphics::Direct2D::Common::D2D1_COLOR_F};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AppConfig {
    pub startup: bool, // タスクスケジューラへの登録(管理者権限の要求)
    pub administrator: bool,
    pub transparent: bool, // 透過
    pub quality: RenderingQuality,
    pub cfg_theme: ConfigTheme,
    pub floating: FloatingWindow, // マウス追従ウィンドウ
    pub fixed: FixedWindow,       // 固定ウィンドウ
    pub active_role: WindowRole,
    pub process_cfg: ProcessConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FloatingWindow {
    pub role: WindowRole,
    #[serde(with = "PointDef")]
    pub offset: POINT, // マウスからどれくらい離すか
    pub style: WindowStyle,
    pub display_style: DisplayStyle,
    pub auto_hide: AutoHide,
}

impl Default for FloatingWindow {
    fn default() -> Self {
        Self {
            role: WindowRole::Floating,
            offset: POINT { x: 20, y: 20 },
            style: WindowStyle::default(),
            display_style: DisplayStyle::default(),
            auto_hide: AutoHide::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FixedWindow {
    pub role: WindowRole,
    pub pos: WindowPos, // 表示位置
    pub margin: i32,
    pub style: WindowStyle, // ウィンドウスタイル
    pub display_style: DisplayStyle,
    pub auto_hide: AutoHide,
}

impl Default for FixedWindow {
    fn default() -> Self {
        Self {
            role: WindowRole::Fixed,
            pos: WindowPos::default(),
            margin: 20,
            style: WindowStyle::default(),
            display_style: DisplayStyle::default(),
            auto_hide: AutoHide::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, AsRefStr, EnumString)]
pub enum DisplayStyle {
    Always,
    Smart(AutoHide),
}

impl Default for DisplayStyle {
    fn default() -> Self {
        Self::Smart(AutoHide::default())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct AutoHide {
    pub enabled: bool,
    pub time: f32,
}

impl Default for AutoHide {
    fn default() -> Self {
        Self { enabled: false, time: 3.0 }
    }
}

// POINTと同じ構造を持つ定義用の型
#[derive(Serialize, Deserialize)]
#[serde(remote = "POINT")]
struct PointDef {
    pub x: i32,
    pub y: i32,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "D2D1_COLOR_F")]
struct D2d1ColorFDef {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

#[derive(Default, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, AsRefStr, EnumString)]
pub enum WindowPos {
    #[default]
    Top,
    TopLeft,
    TopRight,
    Center,
    CenterLeft,
    CenterRight,
    Bottom,
    BottomLeft,
    BottomRight,
}

#[derive(Default, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, AsRefStr, EnumString)]
pub enum TextFormat {
    Compact,
    #[default]
    Full,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WindowStyle {
    pub padding: f32,   // 余白 最終的なウィンドウサイズは実際の文字列のMetrics + padding
    pub opacity: f32,   // ウィンドウの透明度
    pub font_size: f32, // フォントサイズ
    #[serde(with = "D2d1ColorFDef")]
    pub font_color: D2D1_COLOR_F, //
    pub text_format: TextFormat,
    #[serde(with = "D2d1ColorFDef")]
    pub bg_color: D2D1_COLOR_F, // 背景色
}

impl Default for WindowStyle {
    fn default() -> Self {
        Self {
            padding: 5.0,
            opacity: 0.5,
            font_size: 14.0,
            font_color: D2D1_COLOR_F {
                r: 0.95,
                g: 0.95,
                b: 0.95,
                a: 1.0,
            },
            text_format: TextFormat::default(),
            bg_color: D2D1_COLOR_F { r: 0.2, g: 0.2, b: 0.2, a: 1.0 },
        }
    }
}

pub trait GpuiColorExt {
    fn to_d2d1_color(&self) -> D2D1_COLOR_F;
    fn to_palette(&self) -> palette::Hsla;
}

impl GpuiColorExt for gpui::Hsla {
    fn to_d2d1_color(&self) -> D2D1_COLOR_F {
        let srgba = palette::Srgba::from_color(self.to_palette());
        D2D1_COLOR_F {
            r: srgba.red,
            g: srgba.green,
            b: srgba.blue,
            a: srgba.alpha,
        }
    }

    fn to_palette(&self) -> palette::Hsla {
        palette::Hsla::new(self.h * 360.0, self.s, self.l, self.a)
    }
}

pub trait D2d1ColorExt {
    fn to_hsla(&self) -> gpui::Hsla;
}

impl D2d1ColorExt for D2D1_COLOR_F {
    fn to_hsla(&self) -> gpui::Hsla {
        let srgba = Srgba::new(self.r, self.g, self.b, self.a);
        let hsla = palette::Hsla::from_color(srgba);
        hsla.to_gpui()
    }
}

pub trait PaletteColorExt {
    fn to_gpui(&self) -> gpui::Hsla;
}

impl PaletteColorExt for palette::Hsla {
    fn to_gpui(&self) -> gpui::Hsla {
        gpui::Hsla {
            h: self.color.hue.into_degrees() / 360.0,
            s: self.color.saturation,
            l: self.color.lightness,
            a: self.alpha,
        }
    }
}

#[derive(
    Default,
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    EnumIter,
    AsRefStr,
    EnumString,
)]
pub enum WindowRole {
    #[default]
    Fixed,
    Floating,
}

#[derive(
    Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone, Copy, AsRefStr, EnumString,
)]
pub enum ConfigTheme {
    #[default]
    System,
    Dark,
    Light,
}

impl ConfigTheme {
    pub fn theme_change(&self, cx: &mut App) {
        match self {
            ConfigTheme::System => Theme::sync_system_appearance(None, cx),
            ConfigTheme::Dark => Theme::change(ThemeMode::Dark, None, cx),
            ConfigTheme::Light => Theme::change(ThemeMode::Light, None, cx),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default)]
pub struct ProcessSet {
    pub processes: Vec<String>,
}

impl ProcessSet {
    pub fn contains(&self, proc_name: &str) -> bool {
        self.processes.iter().any(|p| p == proc_name)
    }

    // 重複チェックをしながら追加
    pub fn insert(&mut self, proc_name: &str) {
        if !self.contains(proc_name) {
            self.processes.push(proc_name.to_string());
        }
    }

    pub fn remove(&mut self, proc_name: &str) {
        self.processes.retain(|p| p != proc_name);
    }
}

#[derive(
    Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone, Copy, AsRefStr, EnumString,
)]
pub enum PolicyMode {
    #[default]
    BlackList,
    WhiteList,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default)]
pub struct ProcessConfig {
    pub mode: PolicyMode,      // 現在の動作モード
    pub blacklist: ProcessSet, // ブラックリストのデータ
    pub whitelist: ProcessSet, // ホワイトリストのデータ
}

#[derive(
    Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone, Copy, AsRefStr, EnumString,
)]
pub enum RenderingQuality {
    Performance,
    #[default]
    Balanced,
    HighQuality,
    Ultra,
}

pub fn get_config_path() -> anyhow::Result<PathBuf> {
    let project_dirs = ProjectDirs::from("com", "", "InputMode-Viewer")
        .context("Failed to get AppData directory")?;

    let config_dirs = project_dirs.config_dir();

    // ディレクトリが無ければ作成
    std::fs::create_dir_all(config_dirs).context("Failed to create log directory")?;

    Ok(config_dirs.join("config.toml"))
}

pub fn load_config() -> AppConfig {
    // パスの取得
    let path = match get_config_path() {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(
                error = ?e,
                fallback = "AppConfig::default()",
                "Could not determine config path"
            );
            return AppConfig::default();
        }
    };
    if !path.exists() {
        let default_config = AppConfig::default();
        if let Err(e) = save_config(&default_config) {
            tracing::error!(
                error = ?e,
                "Failed to save config.toml."
            );
        }
        return default_config;
    }

    // 読み込みとパース
    let res = fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!(e).context("File read failed"))
        .and_then(|content| {
            toml::from_str(&content).map_err(|e| anyhow::anyhow!(e).context("Parse failed"))
        });

    match res {
        Ok(config) => config,
        Err(e) => {
            tracing::warn!(
                error = ?e,
                path = %path.display(), // Display トレイトを使って出力する
                fallback = "AppConfig::default()",
                "Failed to load config, using default values:\n{:#?}", e
            );
            AppConfig::default()
        }
    }
}

#[instrument(skip(config))] // 設定内容は巨大になる可能性があるためログ引数からは外す
pub fn save_config(config: &AppConfig) -> anyhow::Result<()> {
    let path = get_config_path().context("Failed to get config path")?;
    let tmp_path = path.with_extension("toml.tmp");

    // シリアライズ
    let contents = toml::to_string_pretty(config).context("Failed to serialize config to TOML")?;

    // 一時ファイルに書き込み
    // 確実に書き込むためにスコープを分ける
    {
        let mut file = fs::File::create(&tmp_path)
            .with_context(|| format!("Failed to create temp file: {:?}", tmp_path))?;

        file.write_all(contents.as_bytes())
            .context("Failed to write to temp file")?;

        file.sync_all()
            .context("Failed to sync temp file to disk")?; // OSのバッファを物理ディスクに強制フラッシュ
    }

    // リネーム
    // 同じドライブ内であればこの操作はアトミック
    fs::rename(&tmp_path, &path)
        .with_context(|| format!("Failed to rename temp file to {:?}", path))?;

    // どこに保存したか残しておく
    tracing::info!(path = %path.display(), "Configuration saved successfully");
    Ok(())
}

// グローバルな送信機を定義（LazyLockで初回アクセス時にスレッド起動）
static CONFIG_SAVE_TX: LazyLock<Sender<AppConfig>> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel::<AppConfig>();

    std::thread::spawn(move || {
        let mut pending_config: Option<AppConfig> = None;
        let debounce_timeout = Duration::from_millis(1000); // 1秒待機

        loop {
            // 保留中のデータがある場合はタイムアウト付きで待機、ない場合は無限に待機
            let wait_timeout = if pending_config.is_some() {
                debounce_timeout
            } else {
                Duration::MAX
            };

            match rx.recv_timeout(wait_timeout) {
                Ok(new_config) => {
                    // 新しい変更が届いたので、保留してタイマーをリセット
                    pending_config = Some(new_config);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // 1秒間次の変更が来なかったので、ディスクに書き込む
                    if let Some(config) = pending_config.take() {
                        tracing::info!("Debounce timeout: Saving configuration to disk...");
                        if let Err(e) = save_config(&config) {
                            tracing::error!(error = ?e, "Background save failed: {:#?}", e);
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    tracing::info!("Config saver thread shutting down");
                    break;
                }
            }
        }
    });

    tx
});

// 外部から呼ぶためのインターフェース
pub fn request_config_save(config: AppConfig) {
    if let Err(e) = CONFIG_SAVE_TX.send(config) {
        tracing::warn!(error = %e, "Failed to send save request to background thread");
    }
}
