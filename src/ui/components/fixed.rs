use gpui::{App, SharedString};
use gpui_component::setting::{NumberFieldOptions, SettingField, SettingItem};
use crate::common::{
    app_config::{AppConfig, WindowPos},
    config,
};

pub fn fixed() -> Vec<SettingItem> {
    vec![
        SettingItem::new(
            "Font Size",
            SettingField::number_input(
                NumberFieldOptions { min: 5.0, max: 100.0, step: 1.0 },
                |cx: &App| AppConfig::global(cx).fixed.style.font_size.into(),
                |val: f64, cx: &mut App| {
                    let size = if val < 5.0 { 5.0 } else { val };
                    AppConfig::global_mut(cx).fixed.style.font_size = size as f32;
                    let _ = config::save_config(AppConfig::global(cx));
                },
            )
            .default_value(AppConfig::default().fixed.style.font_size),
        )
        .description("Font size: Min 5, Max 100, Default 14"),
        SettingItem::new(
            "Padding",
            SettingField::number_input(
                NumberFieldOptions { min: 0.0, max: 100.0, step: 1.0 },
                |cx: &App| AppConfig::global(cx).fixed.style.padding.into(),
                |val: f64, cx: &mut App| {
                    let p = if val < 0.0 { 0.0 } else { val };
                    AppConfig::global_mut(cx).fixed.style.padding = p as f32;
                    let _ = config::save_config(AppConfig::global(cx));
                },
            )
            .default_value(AppConfig::default().fixed.style.padding),
        )
        .description("Padding size: Min 0, Max 100, Default 5"),
        SettingItem::new(
            "Opacity",
            SettingField::number_input(
                NumberFieldOptions { min: 1.0, max: 100.0, step: 1.0 },
                |cx: &App| (AppConfig::global(cx).fixed.style.opacity * 100.0) as f64,
                |val: f64, cx: &mut App| {
                    let o = if val < 1.0 { 1.0 } else { val };
                    AppConfig::global_mut(cx).fixed.style.opacity = (o / 100.0) as f32;

                    let _ = config::save_config(AppConfig::global(cx));
                },
            )
            .default_value(AppConfig::default().fixed.style.opacity * 100.0),
        )
        .description("Opacity (%): Min 1, Max 100, Default 50"),
        SettingItem::new(
            "Window Position",
            SettingField::dropdown(
                vec![
                    (WindowPos::Top.as_ref().into(), "Top".into()),
                    (WindowPos::Left.as_ref().into(), "Left".into()),
                    (WindowPos::Bottom.as_ref().into(), "Bottom".into()),
                    (WindowPos::Right.as_ref().into(), "Right".into()),
                ],
                |cx: &App| AppConfig::global(cx).fixed.position.as_ref().to_string().into(),
                |val: SharedString, cx: &mut App| {
                    let pos = val.as_str().parse::<WindowPos>().unwrap_or(WindowPos::Top);
                    AppConfig::global_mut(cx).fixed.position = pos;
                    let _ = config::save_config(AppConfig::global(cx));
                },
            )
            .default_value(AppConfig::default().fixed.position.as_ref().to_string()),
        )
        .description("Enter window position: Default Top"),
    ]
}
