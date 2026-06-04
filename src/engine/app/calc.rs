use anyhow::Context;
use windows::Win32::{
    Foundation::POINT,
    Graphics::{
        DirectWrite::DWRITE_TEXT_METRICS,
        Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint},
    },
    UI::{
        HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
        WindowsAndMessaging::{
            GetCursorPos, GetSystemMetrics, SM_CXSCREEN, SM_CXVIRTUALSCREEN, SM_CYSCREEN,
            SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
        },
    },
};

use crate::common::WindowPos;

// 座標計算
#[allow(dead_code)]
fn calc_predicted_potision(
    current: POINT,
    mouse_x: i32,
    mouse_y: i32,
    px: i32,
    f: f32,
) -> (i32, i32) {
    // 前回からの移動量(速度)を計算
    let dx = current.x - mouse_x;
    let dy = current.y - mouse_y;

    // 予測係数の設定
    // 移動距離が指定px以下なら無視
    let dist_sq = dx * dx + dy * dy; // // 三平方の定理のルート取る前。ルート計算は重いので2乗のまま比較
    // 移動距離が2ピクセル未満（2の2乗で4未満）なら、マウスが止まっているか手が震えているだけなので予測を0.0
    let k = if dist_sq < px * px { 0.0 } else { f }; // fフレーム先

    // 予測座標を計算
    // 今の場所に、速度 × フレーム数を足す
    let predicted_x = current.x + (dx as f32 * k) as i32;
    let predicted_y = current.y + (dy as f32 * k) as i32;

    (predicted_x, predicted_y)
}

// マウス位置の予測
#[allow(dead_code)]
pub fn set_predicted_position(
    mouse_x: i32,
    mouse_y: i32,
    scale: f64,
    offset: POINT,
) -> (i32, i32, i32, i32) {
    // 出力引数
    let mut current = POINT::default();
    let _ = unsafe { GetCursorPos(&mut current) }; // 現在のマウス座標

    // 保存しておいた前回からの移動量(速度)を計算
    // TODO: 予測係数は設定変更出来るように
    let (predicted_x, predicted_y) = calc_predicted_potision(current, mouse_x, mouse_y, 2, 1.6);

    // マウスから少しずらす
    let offset_x = offset.x * scale as i32;
    let offset_y = offset.y * scale as i32;

    // 現在のマウス座標とマージンを足した予測座標
    (
        current.x,
        current.y,
        predicted_x + offset_x,
        predicted_y + offset_y,
    )
}

// モニターサイズを取得
pub fn monitor_info(pt: POINT) -> anyhow::Result<(MONITORINFO, f64)> {
    // モニターのワークエリアを取得
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };

    unsafe {
        let h = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        if h.is_invalid() {
            anyhow::bail!(
                "Failed to get monitor handle from point ({}, {})",
                pt.x,
                pt.y
            );
        }

        GetMonitorInfoW(h, &mut info)
            .ok()
            .context("Failed to get monitor information")?;

        // モニターのDPIスケールを取得
        let mut dpi_x = 0;
        let mut dpi_y = 0;

        let scale = if GetDpiForMonitor(h, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y).is_ok() {
            dpi_x as f64 / 96.0
        } else {
            tracing::warn!("Failed to get DPI for monitor, falling back to 1.0");
            1.0
        };

        Ok((info, scale))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VirtualScreen {
    pub x: i32,
    pub y: i32,
    pub cx: i32,
    pub cy: i32,
}
impl VirtualScreen {
    pub fn new() -> Self {
        unsafe {
            // 仮想画面全体のサイズを取得
            let mut x = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let mut y = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let mut cx = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let mut cy = GetSystemMetrics(SM_CYVIRTUALSCREEN);

            // 幅や高さが0ならプライマリモニターの値で上書き
            if cx == 0 || cy == 0 {
                tracing::warn!(
                    "Virtual screen metrics are invalid (cx: {}, cy: {}). Falling back to primary monitor.",
                    cx,
                    cy
                );
                x = 0;
                y = 0;
                cx = GetSystemMetrics(SM_CXSCREEN);
                cy = GetSystemMetrics(SM_CYSCREEN);
            }

            // それでも0なら最低限の値を設定
            if cx == 0 || cy == 0 {
                tracing::error!(
                    "Primary monitor metrics also failed. Using 1920x1080 as a hard fallback."
                );
                // フルHD
                cx = 1920;
                cy = 1080;
            }

            let result = Self { x, y, cx, cy };

            tracing::info!(
                virtual_screen = ?result,
                "Virtual screen metrics loaded"
            );

            result
        }
    }
}
impl Default for VirtualScreen {
    fn default() -> Self {
        Self::new()
    }
}

// Fixedウィンドウの物理座標を計算して返す
pub fn fixed_position(
    metrics: DWRITE_TEXT_METRICS,
    pos: &WindowPos,
    m: i32,
    p: f32,
    info: MONITORINFO,
    s: f64,
) -> anyhow::Result<POINT> {
    let work_area = info.rcWork;

    // visualの物理サイズ
    let p_width = ((metrics.width as f64 + p as f64 * 2.0) * s).ceil() as i32;
    let p_height = ((metrics.height as f64 + p as f64 * 2.0) * s).ceil() as i32;

    let margin = (m as f64 * s).ceil() as i32;

    // 座標計算
    let wa_width = work_area.right - work_area.left;
    let wa_height = work_area.bottom - work_area.top;

    let x = match pos {
        // 左側（マージン分右へ）
        WindowPos::TopLeft | WindowPos::BottomLeft | WindowPos::CenterLeft => {
            work_area.left + margin
        }
        // 右側（右端から幅とマージン分左へ）
        WindowPos::TopRight | WindowPos::BottomRight | WindowPos::CenterRight => {
            work_area.right - p_width - margin
        }
        // 中央（ワークエリア中央から幅の半分左へ）
        WindowPos::Top | WindowPos::Bottom | WindowPos::Center => {
            work_area.left + (wa_width - p_width) / 2
        }
    };

    let y = match pos {
        // 上側（マージン分下へ）
        WindowPos::Top | WindowPos::TopLeft | WindowPos::TopRight => work_area.top + margin,
        // 下側（下端から高さとマージン分上へ）
        WindowPos::Bottom | WindowPos::BottomLeft | WindowPos::BottomRight => {
            work_area.bottom - p_height - margin
        }
        // 中央（ワークエリア中央から高さの半分上へ）
        WindowPos::Center | WindowPos::CenterLeft | WindowPos::CenterRight => {
            work_area.top + (wa_height - p_height) / 2
        }
    };

    let result = POINT { x, y };

    tracing::debug!(
        ?pos,
        calculated_point = ?result,
        monitor_rect = ?info.rcWork,
        "Fixed position calculated"
    );

    Ok(result)
}
