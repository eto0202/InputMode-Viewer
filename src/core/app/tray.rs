use crate::core::app::prelude::*;

pub const ID_QUIT: &str = "Quit";
pub const ID_SETTING: &str = "Setting";
pub const ID_RESTART: &str = "Restart";

#[instrument(skip(bytes))]
pub fn tray_icon(bytes: &[u8]) -> anyhow::Result<TrayIcon> {
    let menu = Menu::new();

    let settings = MenuItem::with_id(ID_SETTING, "Setting", true, None);
    let restart = MenuItem::with_id(ID_RESTART, "Restart", true, None);
    let quit = MenuItem::with_id(ID_QUIT, "Quit", true, None);

    menu.append(&settings)
        .context("Failed to append 'Setting' to tray menu")?;
    menu.append(&restart)
        .context("Failed to append 'Restart' to tray menu")?;
    menu.append(&quit)
        .context("Failed to append 'Quit' to tray menu")?;

    tracing::debug!("Tray menu structure constructed");

    let raw =
        utils::decode_to_rgba(bytes).context("Failed to decode tray icon image (icon.png)")?;
    let icon = utils::to_tray_icon(raw).context("Failed to create tray icon")?;

    let tray_icon = TrayIconBuilder::new()
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .with_tooltip("Input Mode Viewer")
        .build()
        .context("Failed to register tray icon with the system shell")?;

    tracing::info!("System tray icon initialized successfully");
    Ok(tray_icon)
}
