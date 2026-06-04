use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::group_box::GroupBoxVariant;
use gpui_component::setting::{SettingGroup, SettingPage, Settings};

use crate::common::{self, AppConfig};
use crate::ui::{Fixed, Floating, ProcessList, components};

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
    pub cur_admin: bool,
    pub cur_start: bool,
}

impl SettingsWindow {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let cfg = common::load_config();

        if !cx.has_global::<AppConfig>() {
            cx.set_global(cfg.clone());
        }

        Self {
            fixed: Fixed::new(window, cx),
            floating: Floating::new(window, cx),
            process_list: ProcessList::new(window, cx),
            is_restart: false,
            is_later: false,
            cur_admin: cfg.administrator,
            cur_start: cfg.startup,
        }
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity().downgrade();
        let cur_admin = self.cur_admin;
        let cur_start = self.cur_start;

        div()
            .size_full()
            .child(
                div().relative().size_full().child(
                    Settings::new("app-config")
                        .with_group_variant(GroupBoxVariant::Outline)
                        .sidebar_width(px(180.0))
                        .pages(vec![
                            // ページ（左側のサイドバーメニュー）
                            SettingPage::new("Application")
                                .default_open(true)
                                .groups(vec![
                                    // グループ（メイン領域のセクション）
                                    SettingGroup::new().title("General").items(
                                        components::general(
                                            window,
                                            cx,
                                            self.cur_admin,
                                            self.cur_start,
                                        ),
                                    ),
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
                    div().child(components::restart_alert_dialog(cx, entity))
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
                            .on_click(move |_, _, cx| {
                                AppConfig::global_mut(cx).administrator = cur_admin;
                                AppConfig::global_mut(cx).startup = cur_start;

                                common::request_config_save(AppConfig::global(cx).clone());
                            }),
                    )
                } else {
                    div().size_0().invisible()
                }
            })
    }
}
