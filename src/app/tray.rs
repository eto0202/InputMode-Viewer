use crate::*;
use gpui::*;
use tray_icon::{
    TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuId, MenuItem},
};

const MENU_ID_QUIT: &str = "quit";

pub fn tray_event(async_app: AsyncApp) {
    let menu_receiver = MenuEvent::receiver();
    let _tray_receiver = TrayIconEvent::receiver();

    // タスクトレイイベントの監視
    if let Ok(event) = menu_receiver.try_recv()
        && event.id == MenuId::new(MENU_ID_QUIT)
    {
        async_app
            .update(|cx| {
                cx.quit();
            })
            .ok();
    }
}

pub fn create_tray_icon() -> TrayIcon {
    // イベント監視用にIDを固定して作成する
    // IDは定数を使用
    let quit_item = MenuItem::with_id(MenuId::new(MENU_ID_QUIT), "Quit", true, None);

    let tray_menu = Menu::new();

    // コンパイル時に画像をバイナリに取り込む
    let img_bytes = include_bytes!("../icon.png");
    let icon = app::utils::load_icon(img_bytes);

    tray_menu.append(&quit_item).unwrap();

    TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("input_mode_viewer")
        .with_icon(icon)
        .with_menu_on_left_click(false)
        .build()
        .unwrap()
}
