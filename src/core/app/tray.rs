use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuItem},
};

pub const ID_QUIT: &str = "Quit";
pub const ID_SETTING: &str = "Setting";

pub fn tray_icon() -> anyhow::Result<TrayIcon> {
    let tray_menu = Menu::new();

    let settings_item = MenuItem::with_id(ID_SETTING, "Setting", true, None);
    let quit_item = MenuItem::with_id(ID_QUIT, "Quit", true, None);

    tray_menu.append(&settings_item)?;
    tray_menu.append(&quit_item)?;

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("Input Mode Viewer")
        .build()?;

    Ok(tray_icon)
}
