use windows::Win32::{
    Foundation::{HWND, POINT},
    UI::WindowsAndMessaging::GetCursorPos,
};

use crate::core::sys::win32;

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
