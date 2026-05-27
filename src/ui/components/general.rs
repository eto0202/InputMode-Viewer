use crate::{common::app_config::RenderingQuality, ui::prelude::*};

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
            .description("Enable this to run the application with administrative privileges. If this doesn't work in some apps, please enable it.\nRestart required to apply changes."),
            SettingItem::new(
                "Launch at Startup",
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
            .description("Register the application to the Task Scheduler to start automatically. The privilege level (Highest / Standard) will be determined by your current settings.\nRestart required to apply changes."),
            SettingItem::new(
                "Transparent",
                SettingField::checkbox(
                    |cx: &App| AppConfig::global(cx).transparent,
                    |val: bool, cx: &mut App| {
                        AppConfig::global_mut(cx).transparent = val;
                        let _ = config::save_config(AppConfig::global(cx));
                    },
                ).default_value(AppConfig::default().transparent),
            )
            .description("Enable transparency to smooth out dark edges."),
            SettingItem::new(
                "Rendering Quality",
                SettingField::dropdown(
                    vec![
                        (RenderingQuality::Performance.as_ref().into(), "Performance".into()),
                        (RenderingQuality::Balanced.as_ref().into(), "Balanced".into()),
                        (RenderingQuality::HighQuality.as_ref().into(), "HighQuality".into()),
                        (RenderingQuality::Ultra.as_ref().into(), "Ultra".into()),
                    ],
                    |cx: &App| AppConfig::global(cx).quality.as_ref().to_string().into(),
                    |val: SharedString, cx: &mut App| {
                        let q = val
                            .as_str()
                            .parse::<RenderingQuality>()
                            .unwrap_or(RenderingQuality::Balanced);
                        AppConfig::global_mut(cx).quality = q;
                        let _ = config::save_config(AppConfig::global(cx));
                    },
                )
                .default_value(AppConfig::default().quality.as_ref().to_string()),
            )
            .description("Performance: Reduces resource usage by limiting animations.\nBalanced (Default): Uses standard animations to balance resource usage and visual experience.\nHighQuality: Maximizes animation fluidity for a smooth visual experience.\nUltra: Maximizes tracking speed for next-gen high-refresh-rate displays."),
            SettingItem::new(
                "UI Theme",
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
            .description("Choose the application's appearance. System (Default), Light, Dark"),
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
                        log::debug!("Current val: {:?}", val);
                        let role = val
                            .as_str()
                            .parse::<WindowRole>()
                            .unwrap_or(WindowRole::Fixed);
                        log::debug!("Current Role: {:?}", role);
                        AppConfig::global_mut(cx).active_role = role;
                        let _ = config::save_config(AppConfig::global(cx));
                    },
                )
                .default_value(AppConfig::default().active_role.as_ref().to_string()),
            )
            .description("Fixed (Default): The window stays in a fixed position.\nFloating: The window follows the mouse cursor."),
        ]
}
