use windows::Win32::{
    Foundation::{HWND, POINT},
    Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint},
    UI::{
        HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
        WindowsAndMessaging::GetCursorPos,
    },
};

use crate::{common::app_config::WindowPos, core::sys::win32};

// メモリ上のバイト列から画像をデコードしアイコンを生成
// アプリケーション内に画像が保存される
pub fn load_icon(to_include_bytes: &[u8]) -> tray_icon::Icon {
    let img = image::load_from_memory(to_include_bytes)
        .unwrap()
        .into_rgba8();
    let (width, height) = img.dimensions();
    let rgba = img.into_raw();

    tray_icon::Icon::from_rgba(rgba, width, height).unwrap()
}

// 座標計算
pub fn calc_predicted_potision(
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
pub fn set_predicted_position(hwnd: HWND, mouse_x: i32, mouse_y: i32, scale: f64) -> (i32, i32) {
    // 出力引数
    let mut current = POINT::default();
    let _ = unsafe { GetCursorPos(&mut current) }; // 現在のマウス座標

    // 保存しておいた前回からの移動量(速度)を計算
    let (predicted_x, predicted_y) = calc_predicted_potision(current, mouse_x, mouse_y, 2, 1.6);

    // マウスから少しずらす
    let offset = 20 * scale as i32;

    let _ = win32::set_window_position(hwnd, predicted_x + offset, predicted_y + offset);

    // 現在のマウス座標を保存
    (current.x, current.y)
}

/// マウス位置のモニターを判定し、Fixedウィンドウの物理座標を計算して返す
pub fn calc_fixed_position(
    logical_width: f32,
    logical_height: f32,
    position: &WindowPos,
    margin_logical: i32,
) -> anyhow::Result<(i32, i32)> {
    // 1. 現在のマウス位置を取得
    let mut pt = POINT::default();
    unsafe {
        GetCursorPos(&mut pt).ok();
    }

    // 2. マウス位置のモニターハンドルを取得
    let hmonitor = unsafe { MonitorFromPoint(pt, MONITOR_DEFAULTTOPRIMARY) };

    // 3. モニターのワークエリアを取得
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(hmonitor, &mut info) }.as_bool() {
        anyhow::bail!("Failed to get monitor info");
    }
    let work_area = info.rcWork;

    // 4. モニターのDPIスケールを取得
    let mut dpi_x = 0;
    let mut dpi_y = 0;
    let scale = if unsafe { GetDpiForMonitor(hmonitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) }
        .is_ok()
    {
        dpi_x as f64 / 96.0
    } else {
        1.0
    };

    // 5. 論理サイズから物理サイズ・マージンへ変換
    let p_width = (logical_width as f64 * scale).ceil() as i32;
    let p_height = (logical_height as f64 * scale).ceil() as i32;
    let margin = (margin_logical as f64 * scale).ceil() as i32;

    // 6. 座標計算
    let wa_width = work_area.right - work_area.left;
    let wa_height = work_area.bottom - work_area.top;

    let x = match position {
        WindowPos::Left => work_area.left + margin,
        WindowPos::Right => work_area.right - p_width - margin,
        WindowPos::Top | WindowPos::Bottom => work_area.left + (wa_width - p_width) / 2,
    };

    let y = match position {
        WindowPos::Top => work_area.top + margin,
        WindowPos::Bottom => work_area.bottom - p_height - margin,
        WindowPos::Left | WindowPos::Right => work_area.top + (wa_height - p_height) / 2,
    };

    Ok((x, y))
}
