use crate::{
    common::{
        app_config::{AppConfig, D2d1ColorExt, GpuiColorExt},
        config,
    },
    ui::{components::color_picker::ColorPickerSettingItem, window::SettingsWindow},
};
use gpui::*;
use gpui_component::{
    color_picker::{ColorPickerEvent, ColorPickerState},
    setting::{NumberFieldOptions, SettingField, SettingItem},
};

pub struct Floating {
    pub bg_state: Entity<ColorPickerState>,
    pub bg_color: Option<Hsla>,

    pub font_state: Entity<ColorPickerState>,
    pub font_color: Option<Hsla>,

    pub subscriptions: Vec<Subscription>,
}

impl Floating {
    pub fn new(window: &mut Window, cx: &mut Context<SettingsWindow>) -> Self {
        let bg_color = AppConfig::global(cx).floating.style.bg_color.to_hsla();
        let font_color = AppConfig::global(cx).floating.style.font_color.to_hsla();

        let bg_state = cx.new(|cx| {
            ColorPickerState::new(window, cx)
                .default_value(bg_color)
        });
        let font_state = cx.new(|cx| {
            ColorPickerState::new(window, cx)
                .default_value(font_color)
        });

        let subscriptions = vec![
            cx.subscribe(&bg_state, |this, _, ev, cx| match ev {
                ColorPickerEvent::Change(color) => {
                    AppConfig::global_mut(cx).floating.style.bg_color =
                        color.unwrap_or_default().to_d2d1_color();
                    this.floating.bg_color = *color;
                    let _ = config::save_config(AppConfig::global(cx));
                }
            }),
            cx.subscribe(&font_state, |this, _, ev, cx| match ev {
                ColorPickerEvent::Change(color) => {
                    AppConfig::global_mut(cx).floating.style.font_color =
                        color.unwrap_or_default().to_d2d1_color();
                    this.floating.font_color = *color;
                    let _ = config::save_config(AppConfig::global(cx));
                }
            }),
        ];

        Self {
            bg_state,
            bg_color: Some(bg_color),
            font_state,
            font_color: Some(font_color),
            subscriptions,
        }
    }

    pub fn floating(&mut self) -> Vec<SettingItem> {
        vec![
            SettingItem::new(
                "Font Size",
                SettingField::number_input(
                    NumberFieldOptions {
                        min: 0.0,
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
                "Font Color",
                SettingField::element(ColorPickerSettingItem::new(
                    self.font_state.clone(),
                    self.font_color,
                )),
            )
            .description("Font Color: Default #F2F2F2"),
            SettingItem::new(
                "Background Color",
                SettingField::element(ColorPickerSettingItem::new(
                    self.bg_state.clone(),
                    self.bg_color,
                )),
            )
            .description("Background Color: Default #333333"),
            SettingItem::new(
                "Padding",
                SettingField::number_input(
                    NumberFieldOptions {
                        min: 0.0,
                        max: 100.0,
                        step: 1.0,
                    },
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
                    NumberFieldOptions {
                        min: 0.0,
                        max: 100.0,
                        step: 1.0,
                    },
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
                    NumberFieldOptions {
                        min: -50.0,
                        max: 50.0,
                        step: 1.0,
                    },
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
                    NumberFieldOptions {
                        min: -50.0,
                        max: 50.0,
                        step: 1.0,
                    },
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
}
