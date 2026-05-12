use crate::{
    common::{app_config::AppConfig, config},
    ui::components::{
        alert_dialog::restart_alert_dialog, fixed::Fixed, floating::Floating, general::general,
        list_components::process_list::ProcessList,
    },
};
use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants},
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
    pub process_list: ProcessList,
    pub is_restart: bool,
    pub is_later: bool,
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
            process_list: ProcessList::new(window, cx),
            is_restart: false,
            is_later: false,
        }
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity().downgrade();

        div()
            .size_full()
            .child(
                div().relative().size_full().child(
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
                                        .items(general(window, cx)),
                                    SettingGroup::new()
                                        .title("Fixed")
                                        .items(Fixed::fixed(&mut self.fixed)),
                                    SettingGroup::new()
                                        .title("Floating")
                                        .items(Floating::floating(&mut self.floating)),
                                ]),
                            SettingPage::new("ProcessManager").groups(vec![
                                SettingGroup::new().items(ProcessList::render_list(
                                    self.process_list.search_input.clone(),
                                    self.process_list.p_state.clone(),
                                    self.process_list.c_state.clone(),
                                    cx,
                                )),
                            ]),
                        ]),
                ),
            )
            .child({
                if self.is_restart && !self.is_later {
                    div().child(restart_alert_dialog(cx, entity))
                } else {
                    div().size_0().invisible()
                }
            })
            .child({
                if self.is_later {
                    div().absolute().top_3().right_16().child(
                        Button::new("sidebar-restart")
                            .label("Restart Now")
                            .primary()
                            .on_click(|_, _, _| {}),
                    )
                } else {
                    div().size_0().invisible()
                }
            })
    }
}
