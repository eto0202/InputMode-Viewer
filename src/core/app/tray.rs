use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuItem},
};

use crate::core::utils;

pub const ID_QUIT: &str = "Quit";
pub const ID_SETTING: &str = "Setting";

pub fn tray_icon() -> anyhow::Result<TrayIcon> {
    let menu = Menu::new();

    let settings = MenuItem::with_id(ID_SETTING, "Setting", true, None);
    let quit = MenuItem::with_id(ID_QUIT, "Quit", true, None);

    menu.append(&settings)?;
    menu.append(&quit)?;

    // コンパイル時に画像をバイナリに取り込む
    let img_bytes = include_bytes!("../../icon.png");
    let icon = utils::load_icon(img_bytes);

    let tray_icon = TrayIconBuilder::new()
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .with_tooltip("Input Mode Viewer")
        .build()?;

    Ok(tray_icon)
}
