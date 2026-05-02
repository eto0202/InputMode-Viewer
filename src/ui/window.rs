use crate::{
    common::{app_config::AppConfig, config},
    ui::components::{fixed::Fixed, floating::Floating, general::appearance},
};
use gpui::*;
use gpui_component::{
    setting::{SettingGroup, SettingPage, Settings},
    *,
};

impl Global for AppConfig {}

impl AppConfig {
    pub fn global(cx: &App) -> &AppConfig {
        cx.global::<AppConfig>()
    }

    pub fn global_mut(cx: &mut App) -> &mut AppConfig {
        cx.global_mut::<AppConfig>()
    }
}

pub struct SettingsWindow {
    pub fixed: Fixed,
    pub floating: Floating,
}

impl SettingsWindow {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let cfg = config::load_config();

        if !cx.has_global::<AppConfig>() {
            cx.set_global(cfg);
        }

        Self {
            fixed: Fixed::new(window, cx),
            floating: Floating::new(window, cx),
        }
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) -> impl IntoElement {
        Settings::new("app-config")
            .with_group_variant(group_box::GroupBoxVariant::Outline)
            .sidebar_width(px(180.0))
            .pages(vec![
                // ページ（左側のサイドバーメニュー）
                SettingPage::new("Application")
                    .default_open(true)
                    .groups(vec![
                        // グループ（メイン領域のセクション）
                        SettingGroup::new()
                            .title("General")
                            .items(appearance(window, cx)),
                        SettingGroup::new()
                            .title("Fixed")
                            .items(Fixed::fixed(&mut self.fixed)),
                        SettingGroup::new()
                            .title("Floating")
                            .items(Floating::floating(&mut self.floating)),
                    ]),
            ])
    }
}
