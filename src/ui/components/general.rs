use crate::{
    common::{
        app_config::{AppConfig, ConfigTheme, WindowRole},
        config,
    },
    ui::window::SettingsWindow,
};
use gpui::{App, Context, SharedString, *};
use gpui_component::setting::{SettingField, SettingItem};

pub fn general(
    _: &mut Window,
    cx: &mut Context<SettingsWindow>,
    cur_admin: bool,
    cur_start: bool,
) -> Vec<SettingItem> {
    let entity1 = cx.entity().downgrade();
    let entity2 = cx.entity().downgrade();
    vec![
            SettingItem::new(
                "Run as Administrator",
                SettingField::checkbox(
                    move |_: &App| cur_admin,
                    move |val: bool, cx: &mut App| {
                        let _ = entity1.update(cx, |this, cx| {
                            this.is_restart = true;
                            this.cur_admin = val;
                            cx.notify();
                        });
                        // let _ = config::save_config(AppConfig::global(cx));
                    },
                ),
            )
            .description("If this doesn't work in some apps, please enable it."),
            SettingItem::new(
                "Start Up",
                SettingField::checkbox(
                    move |_: &App| cur_start,
                    move |val: bool, cx: &mut App| {
                        let _ = entity2.update(cx, |this, cx| {
                            this.is_restart = true;
                            this.cur_start = val;
                            
                            cx.notify();
                        });
                        // let _ = config::save_config(AppConfig::global(cx));
                    },
                ),
            )
            .description("Run automatically when the PC starts up. Based on current permissions.\nAdministrator: HIGHEST, Standard User: LUA"),
            SettingItem::new(
                "Theme Mode",
                SettingField::dropdown(
                    vec![
                        (ConfigTheme::System.as_ref().into(), "System".into()),
                        (ConfigTheme::Dark.as_ref().into(), "Dark".into()),
                        (ConfigTheme::Light.as_ref().into(), "Light".into()),
                    ],
                    |cx: &App| AppConfig::global(cx).cfg_theme.as_ref().to_string().into(),
                    |val: SharedString, cx: &mut App| {
                        let mode = val
                            .as_str()
                            .parse::<ConfigTheme>()
                            .unwrap_or(ConfigTheme::System);
                        mode.theme_change(cx);
                        AppConfig::global_mut(cx).cfg_theme = mode;
                        let _ = config::save_config(AppConfig::global(cx));
                    },
                )
                .default_value(AppConfig::default().cfg_theme.as_ref().to_string()),
            )
            .description("Theme Mode: Default System"),
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
            .description("Window type: Default Fixed"),
        ]
}
