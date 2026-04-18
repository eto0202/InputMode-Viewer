use windows::Win32::Foundation::POINT;

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
