use gpui::{App};
use gpui_component::setting::{NumberFieldOptions, SettingField, SettingItem};
use crate::common::{app_config::AppConfig, config};

pub fn floating() -> Vec<SettingItem> {
    vec![
        SettingItem::new(
            "Font Size",
            SettingField::number_input(
                NumberFieldOptions {
                    min: 5.0, // バリデーションの不具合のため0.0に
                    max: 100.0,
                    step: 1.0,
                },
                |cx: &App| AppConfig::global(cx).floating.style.font_size.into(),
                |val: f64, cx: &mut App| {
                    let size = if val < 5.0 { 5.0 } else { val };
                    AppConfig::global_mut(cx).floating.style.font_size = size as f32;
                    let _ = config::save_config(AppConfig::global(cx));
                },
            )
            .default_value(AppConfig::default().floating.style.font_size),
        )
        .description("Font size: Min 5, Max 100, Default 14"),
        SettingItem::new(
            "Padding",
            SettingField::number_input(
                NumberFieldOptions { min: 0.0, max: 100.0, step: 1.0 },
                |cx: &App| AppConfig::global(cx).floating.style.padding.into(),
                |val: f64, cx: &mut App| {
                    let p = if val < 0.0 { 0.0 } else { val };
                    AppConfig::global_mut(cx).floating.style.padding = p as f32;
                    let _ = config::save_config(AppConfig::global(cx));
                },
            )
            .default_value(AppConfig::default().floating.style.padding),
        )
        .description("Padding size: Min 0, Max 100, Default 5"),
        SettingItem::new(
            "Opacity",
            SettingField::number_input(
                NumberFieldOptions { min: 1.0, max: 100.0, step: 1.0 },
                |cx: &App| (AppConfig::global(cx).floating.style.opacity * 100.0) as f64,
                |val: f64, cx: &mut App| {
                    let o = if val < 1.0 { 1.0 } else { val };
                    AppConfig::global_mut(cx).floating.style.opacity = (o / 100.0) as f32;

                    let _ = config::save_config(AppConfig::global(cx));
                },
            )
            .default_value(AppConfig::default().floating.style.opacity * 100.0),
        )
        .description("Opacity (%): Min 1, Max 100, Default 50"),
        SettingItem::new(
            "Offset X",
            SettingField::number_input(
                NumberFieldOptions { min: -50.0, max: 50.0, step: 1.0 },
                |cx: &App| AppConfig::global(cx).floating.offset.x.into(),
                |val: f64, cx: &mut App| {
                    let x = if val < -50.0 { -50.0 } else { val };
                    AppConfig::global_mut(cx).floating.offset.x = x as i32;
                    let _ = config::save_config(AppConfig::global(cx));
                },
            )
            .default_value(AppConfig::default().floating.offset.x),
        )
        .description("Distance from the mouse:\nMin -50, Max 50, Default 20"),
        SettingItem::new(
            "Offset Y",
            SettingField::number_input(
                NumberFieldOptions { min: -50.0, max: 50.0, step: 1.0 },
                |cx: &App| AppConfig::global(cx).floating.offset.y.into(),
                |val: f64, cx: &mut App| {
                    let y = if val < -50.0 { -50.0 } else { val };
                    AppConfig::global_mut(cx).floating.offset.y = y as i32;
                    let _ = config::save_config(AppConfig::global(cx));
                },
            )
            .default_value(AppConfig::default().floating.offset.y),
        )
        .description("Distance from the mouse:\nMin -50, Max 50, Default 20"),
    ]
}
