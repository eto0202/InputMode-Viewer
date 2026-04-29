use gpui::*;
use gpui_component::{
    setting::{SettingGroup, SettingPage, Settings},
    *,
};
use crate::{
    common::{app_config::AppConfig, config},
    ui::components::{appearance::appearance, fixed::fixed, floating::floating},
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

pub struct SettingsWindow {}

impl SettingsWindow {
    pub fn new(_window: &mut Window, cx: &mut App) -> Self {
        let cfg = config::load_config();

        if !cx.has_global::<AppConfig>() {
            cx.set_global(cfg);
        }
        Self {}
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, _: &mut gpui::Window, _: &mut Context<Self>) -> impl IntoElement {
        Settings::new("app-config")
            .with_group_variant(group_box::GroupBoxVariant::Outline)
            .sidebar_width(px(180.0))
            .pages(vec![
                // ページ（左側のサイドバーメニュー）
                SettingPage::new("General")
                    .default_open(true)
                    .groups(vec![
                        // グループ（メイン領域のセクション）
                        SettingGroup::new().title("Appearance").items(appearance()),
                        SettingGroup::new().title("Fixed").items(fixed()),
                        SettingGroup::new().title("Floating").items(floating()),
                    ]),
            ])
    }
}
