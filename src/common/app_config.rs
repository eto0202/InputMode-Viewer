use gpui_component::ThemeMode;
use palette::{FromColor, Srgba};
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumIter, EnumString};
use windows::Win32::{Foundation::POINT, Graphics::Direct2D::Common::D2D1_COLOR_F};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AppConfig {
    pub startup: bool, // タスクスケジューラへの登録(管理者権限の要求)
    pub auto_switch_theme: bool,
    pub theme_mode: ThemeMode,
    pub floating: FloatingWindow, // マウス追従ウィンドウ
    pub fixed: FixedWindow,       // 固定ウィンドウ
    pub active_role: WindowRole,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FloatingWindow {
    pub role: WindowRole,
    #[serde(with = "PointDef")]
    pub offset: POINT, // マウスからどれくらい離すか
    pub style: WindowStyle,
    pub text_style: TextStyle,
}

impl Default for FloatingWindow {
    fn default() -> Self {
        Self {
            role: WindowRole::Floating,
            offset: POINT { x: 20, y: 20 },
            style: WindowStyle::default(),
            text_style: TextStyle::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FixedWindow {
    pub role: WindowRole,
    pub position: WindowPos, // 表示位置
    pub margin: i32,
    pub style: WindowStyle, // ウィンドウスタイル
    pub text_style: TextStyle,
}

impl Default for FixedWindow {
    fn default() -> Self {
        Self {
            role: WindowRole::Fixed,
            position: WindowPos::Top,
            margin: 20,
            style: WindowStyle::default(),
            text_style: TextStyle::default(),
        }
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

#[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq, AsRefStr, EnumString)]
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

#[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq, AsRefStr, EnumString)]
pub enum TextStyle {
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
    pub font_color: D2D1_COLOR_F, // フォントカラー
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
