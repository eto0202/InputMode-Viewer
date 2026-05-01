use crate::{
    common::{
        app_config::{AppConfig, WindowRole},
        config,
    },
    ui::window::SettingsWindow,
};
use gpui::{App, Context, SharedString, *};
use gpui_component::{
    ActiveTheme, Theme, ThemeMode,
    setting::{SettingField, SettingItem},
};

pub fn appearance(_: &mut Window, _: &mut Context<SettingsWindow>) -> Vec<SettingItem> {
    vec![
        SettingItem::new(
            "Dark Mode",
            SettingField::switch(
                |cx: &App| cx.theme().mode.is_dark(),
                |val: bool, cx: &mut App| {
                    let mode = if val {
                        ThemeMode::Dark
                    } else {
                        ThemeMode::Light
                    };
                    Theme::global_mut(cx).mode = mode;
                    Theme::change(mode, None, cx);
                    let _ = config::save_config(AppConfig::global(cx));
                },
            ),
        )
        .description("Enable dark mode"),
        SettingItem::new(
            "Auto Switch Theme",
            SettingField::checkbox(
                |cx: &App| AppConfig::global(cx).auto_switch_theme,
                |val: bool, cx: &mut App| {
                    AppConfig::global_mut(cx).auto_switch_theme = val;
                    let _ = config::save_config(AppConfig::global(cx));
                },
            )
            .default_value(AppConfig::default().auto_switch_theme),
        )
        .description("Automatically switch theme based on system settings."),
        SettingItem::new(
            "Start Up",
            SettingField::checkbox(
                |cx: &App| AppConfig::global(cx).startup,
                |val: bool, cx: &mut App| {
                    AppConfig::global_mut(cx).startup = val;
                    let _ = config::save_config(AppConfig::global(cx));
                },
            ),
        )
        .description("Enable start up"),
        SettingItem::new(
            "Window Type",
            SettingField::dropdown(
                vec![
                    (WindowRole::Fixed.as_ref().into(), "Fixed".into()),
                    (WindowRole::Floating.as_ref().into(), "Floating".into()),
                ],
                |cx: &App| {
                    AppConfig::global(cx)
                        .active_role
                        .as_ref()
                        .to_string()
                        .into()
                },
                |val: SharedString, cx: &mut App| {
                    let role = val
                        .as_str()
                        .parse::<WindowRole>()
                        .unwrap_or(WindowRole::Fixed);
                    AppConfig::global_mut(cx).active_role = role;
                    let _ = config::save_config(AppConfig::global(cx));
                },
            )
            .default_value(AppConfig::default().active_role.as_ref().to_string()),
        )
        .description("Enter window type: Default Fixed"),
    ]
}
