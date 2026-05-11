use crate::{
    common::{
        app_config::{self, AppConfig, D2d1ColorExt, GpuiColorExt},
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
    pub bg_color: Entity<ColorPickerState>,
    pub bg_selected_color: Option<Hsla>,

    pub font_color: Entity<ColorPickerState>,
    pub font_selected_color: Option<Hsla>,

    pub subscriptions: Vec<Subscription>,
}

impl Floating {
    pub fn new(window: &mut Window, cx: &mut Context<SettingsWindow>) -> Self {
        let default_style = AppConfig::default().fixed.style;

        let bg_selected_color = AppConfig::global(cx).floating.style.bg_color.to_hsla();
        let bg_color = cx.new(|cx| {
            ColorPickerState::new(window, cx).default_value(default_style.bg_color.to_hsla())
        });

        let font_selected_color = AppConfig::global(cx).floating.style.font_color.to_hsla();
        let font_color = cx.new(|cx| {
            ColorPickerState::new(window, cx).default_value(default_style.font_color.to_hsla())
        });

        let subscriptions = vec![
            cx.subscribe(&bg_color, |this, _, ev, cx| match ev {
                ColorPickerEvent::Change(color) => {
                    AppConfig::global_mut(cx).floating.style.bg_color =
                        color.unwrap_or_default().to_d2d1_color();
                    this.floating.bg_selected_color = *color;
                    let _ = config::save_config(AppConfig::global(cx));
                }
            }),
            cx.subscribe(&font_color, |this, _, ev, cx| match ev {
                ColorPickerEvent::Change(color) => {
                    AppConfig::global_mut(cx).floating.style.font_color =
                        color.unwrap_or_default().to_d2d1_color();
                    this.floating.font_selected_color = *color;
                    let _ = config::save_config(AppConfig::global(cx));
                }
            }),
        ];

        Self {
            bg_color,
            bg_selected_color: Some(bg_selected_color),
            font_color,
            font_selected_color: Some(font_selected_color),
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
                        precision: None,
                    },
                    |cx: &App| AppConfig::global(cx).floating.style.font_size.into(),
                    |val: f64, cx: &mut App| {
                        let size = val.clamp(5.0, 5.0);
                        AppConfig::global_mut(cx).floating.style.font_size = size as f32;
                        let _ = config::save_config(AppConfig::global(cx));
                    },
                )
                .default_value(AppConfig::default().floating.style.font_size),
            )
            .description("Font Size: Min 5, Max 100, Default 14"),
            SettingItem::new(
                "Font Color",
                SettingField::element(ColorPickerSettingItem::new(
                    self.font_color.clone(),
                    self.font_selected_color,
                )),
            )
            .description("Font Color: Default #F2F2F2"),
            SettingItem::new(
                "Text Style",
                SettingField::dropdown(
                    vec![
                        (app_config::TextStyle::Full.as_ref().into(), "Full".into()),
                        (
                            app_config::TextStyle::Compact.as_ref().into(),
                            "Compact".into(),
                        ),
                    ],
                    |cx: &App| {
                        AppConfig::global(cx)
                            .floating
                            .style
                            .text_style
                            .as_ref()
                            .to_string()
                            .into()
                    },
                    |val: SharedString, cx: &mut App| {
                        let style = val
                            .as_str()
                            .parse::<app_config::TextStyle>()
                            .unwrap_or(app_config::TextStyle::Full);
                        AppConfig::global_mut(cx).floating.style.text_style = style;
                        let _ = config::save_config(AppConfig::global(cx));
                    },
                )
                .default_value(AppConfig::default().active_role.as_ref().to_string()),
            )
            .description("Text Style: Default Full"),
            SettingItem::new(
                "Background Color",
                SettingField::element(ColorPickerSettingItem::new(
                    self.bg_color.clone(),
                    self.bg_selected_color,
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
                        precision: None,
                    },
                    |cx: &App| AppConfig::global(cx).floating.style.padding.into(),
                    |val: f64, cx: &mut App| {
                        let p = val.clamp(0.0, 0.0);
                        AppConfig::global_mut(cx).floating.style.padding = p as f32;
                        let _ = config::save_config(AppConfig::global(cx));
                    },
                )
                .default_value(AppConfig::default().floating.style.padding),
            )
            .description("Padding: Min 0, Max 100, Default 5"),
            SettingItem::new(
                "Opacity",
                SettingField::number_input(
                    NumberFieldOptions {
                        min: 0.0,
                        max: 100.0,
                        step: 1.0,
                        precision: None,
                    },
                    |cx: &App| (AppConfig::global(cx).floating.style.opacity * 100.0) as f64,
                    |val: f64, cx: &mut App| {
                        let o = val.clamp(1.0, 1.0);
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
                        precision: None,
                    },
                    |cx: &App| AppConfig::global(cx).floating.offset.x.into(),
                    |val: f64, cx: &mut App| {
                        let x = val.clamp(-50.0, -50.0);
                        AppConfig::global_mut(cx).floating.offset.x = x as i32;
                        let _ = config::save_config(AppConfig::global(cx));
                    },
                )
                .default_value(AppConfig::default().floating.offset.x),
            )
            .description("Distance from the mouse X:\nMin -50, Max 50, Default 20"),
            SettingItem::new(
                "Offset Y",
                SettingField::number_input(
                    NumberFieldOptions {
                        min: -50.0,
                        max: 50.0,
                        step: 1.0,
                        precision: None,
                    },
                    |cx: &App| AppConfig::global(cx).floating.offset.y.into(),
                    |val: f64, cx: &mut App| {
                        let y = val.clamp(-50.0, -50.0);
                        AppConfig::global_mut(cx).floating.offset.y = y as i32;
                        let _ = config::save_config(AppConfig::global(cx));
                    },
                )
                .default_value(AppConfig::default().floating.offset.y),
            )
            .description("Distance from the mouse Y:\nMin -50, Max 50, Default 20"),
            SettingItem::new(
                "Tracking Frequency",
                SettingField::number_input(
                        NumberFieldOptions {
                            min: 0.01,
                            max: 0.1,
                            step: 0.01,
                            precision: Some(2),
                        },
                    |cx: &App| AppConfig::global(cx).floating.frequency as f64,
                    |val: f64, cx: &mut App| {
                        AppConfig::global_mut(cx).floating.frequency = val as f32;
                        let _ = config::save_config(AppConfig::global(cx));
                    },
                )
                .default_value(AppConfig::default().floating.frequency as f64),
            )
            .description("Mouse tracking frequency:\nMin 0.01, Max 0.1, Default 0.05\nThe lower the value, the smoother the tracking."),
        ]
    }
}
