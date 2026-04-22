use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;
use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AppConfig {
    pub startup: bool,            // タスクスケジューラへの登録(管理者権限の要求)
    pub floating: FloatingWindow, // マウス追従ウィンドウ
    pub fixed: FixedWindow,       // 固定ウィンドウ
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FloatingWindow {
    pub enabled: bool,
    pub role: WindowRole,
    #[serde(with = "PointDef")]
    pub offset: POINT, // マウスからどれくらい離すか
    pub style: WindowStyle,
}

impl Default for FloatingWindow {
    fn default() -> Self {
        Self {
            enabled: true,
            role: WindowRole::Floating,
            offset: POINT { x: 20, y: 20 },
            style: WindowStyle::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FixedWindow {
    pub enabled: bool, // 表示切り替え
    pub role: WindowRole,
    pub position: WindowPos, // 表示位置
    pub style: WindowStyle,  // ウィンドウスタイル
}

impl Default for FixedWindow {
    fn default() -> Self {
        Self {
            enabled: true, // 確認用にtrue
            role: WindowRole::Fixed,
            position: WindowPos::Top,
            style: WindowStyle::default(),
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum WindowPos {
    Top,
    Bottom,
    Left,
    Right,
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
            bg_color: D2D1_COLOR_F {
                r: 0.2,
                g: 0.2,
                b: 0.2,
                a: 1.0,
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
pub enum WindowRole {
    Fixed,
    Floating,
}
