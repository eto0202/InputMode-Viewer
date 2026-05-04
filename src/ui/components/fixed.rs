use crate::{
    common::{
        app_config::{self, AppConfig, D2d1ColorExt, GpuiColorExt, WindowPos},
        config,
    },
    ui::{components::color_picker::ColorPickerSettingItem, window::SettingsWindow},
};
use gpui::{App, SharedString, *};
use gpui_component::{
    color_picker::{ColorPickerEvent, ColorPickerState},
    setting::{NumberFieldOptions, SettingField, SettingItem},
};

pub struct Fixed {
    pub bg_state: Entity<ColorPickerState>,
    pub bg_color: Option<Hsla>,

    pub font_state: Entity<ColorPickerState>,
    pub font_color: Option<Hsla>,

    pub subscriptions: Vec<Subscription>,
}

impl Fixed {
    pub fn new(window: &mut Window, cx: &mut Context<SettingsWindow>) -> Self {
        let bg_color = AppConfig::global(cx).fixed.style.bg_color.to_hsla();
        let font_color = AppConfig::global(cx).fixed.style.font_color.to_hsla();

        let bg_state = cx.new(|cx| ColorPickerState::new(window, cx).default_value(bg_color));
        let font_state = cx.new(|cx| ColorPickerState::new(window, cx).default_value(font_color));

        let subscriptions = vec![
            cx.subscribe(&bg_state, |this, _, ev, cx| match ev {
                ColorPickerEvent::Change(color) => {
                    AppConfig::global_mut(cx).fixed.style.bg_color =
                        color.unwrap_or_default().to_d2d1_color();
                    this.fixed.bg_color = *color;
                    let _ = config::save_config(AppConfig::global(cx));
                }
            }),
            cx.subscribe(&font_state, |this, _, ev, cx| match ev {
                ColorPickerEvent::Change(color) => {
                    AppConfig::global_mut(cx).fixed.style.font_color =
                        color.unwrap_or_default().to_d2d1_color();
                    this.fixed.font_color = *color;
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

    pub fn fixed(&mut self) -> Vec<SettingItem> {
        vec![
            SettingItem::new(
                "Font Size",
                SettingField::number_input(
                    NumberFieldOptions {
                        min: 0.0,
                        max: 100.0,
                        step: 1.0,
                    },
                    |cx: &App| AppConfig::global(cx).fixed.style.font_size.into(),
                    |val: f64, cx: &mut App| {
                        let size = if val < 5.0 { 5.0 } else { val };
                        AppConfig::global_mut(cx).fixed.style.font_size = size as f32;
                        let _ = config::save_config(AppConfig::global(cx));
                    },
                )
                .default_value(AppConfig::default().fixed.style.font_size),
            )
            .description("Font Size: Min 5, Max 100, Default 14"),
            SettingItem::new(
                "Font Color",
                SettingField::element(ColorPickerSettingItem::new(
                    self.font_state.clone(),
                    self.font_color,
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
                            .fixed
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
                        AppConfig::global_mut(cx).fixed.style.text_style = style;
                        let _ = config::save_config(AppConfig::global(cx));
                    },
                )
                .default_value(AppConfig::default().active_role.as_ref().to_string()),
            )
            .description("Text Style: Default Full"),
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
                    |cx: &App| AppConfig::global(cx).fixed.style.padding.into(),
                    |val: f64, cx: &mut App| {
                        let p = if val < 0.0 { 0.0 } else { val };
                        AppConfig::global_mut(cx).fixed.style.padding = p as f32;
                        let _ = config::save_config(AppConfig::global(cx));
                    },
                )
                .default_value(AppConfig::default().fixed.style.padding),
            )
            .description("Padding: Min 0, Max 100, Default 5"),
            SettingItem::new(
                "Opacity",
                SettingField::number_input(
                    NumberFieldOptions {
                        min: 0.0,
                        max: 100.0,
                        step: 1.0,
                    },
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
                        (WindowPos::TopLeft.as_ref().into(), "TopLeft".into()),
                        (WindowPos::TopRight.as_ref().into(), "TopRight".into()),
                        (WindowPos::Center.as_ref().into(), "Center".into()),
                        (WindowPos::CenterLeft.as_ref().into(), "CenterLeft".into()),
                        (WindowPos::CenterRight.as_ref().into(), "CenterRight".into()),
                        (WindowPos::Bottom.as_ref().into(), "Bottom".into()),
                        (WindowPos::BottomLeft.as_ref().into(), "BottomLeft".into()),
                        (WindowPos::BottomRight.as_ref().into(), "BottomRight".into()),
                    ],
                    |cx: &App| {
                        AppConfig::global(cx)
                            .fixed
                            .pos
                            .as_ref()
                            .to_string()
                            .into()
                    },
                    |val: SharedString, cx: &mut App| {
                        let pos = val.as_str().parse::<WindowPos>().unwrap_or(WindowPos::Top);
                        AppConfig::global_mut(cx).fixed.pos = pos;
                        let _ = config::save_config(AppConfig::global(cx));
                    },
                )
                .default_value(AppConfig::default().fixed.pos.as_ref().to_string()),
            )
            .description("Window Position: Default Top"),
            SettingItem::new(
                "Margin",
                SettingField::number_input(
                    NumberFieldOptions {
                        min: 0.0,
                        max: 500.0,
                        step: 1.0,
                    },
                    |cx: &App| (AppConfig::global(cx).fixed.margin) as f64,
                    |val: f64, cx: &mut App| {
                        let m = if val < 0.0 { 0.0 } else { val };
                        AppConfig::global_mut(cx).fixed.margin = m as i32;
                        let _ = config::save_config(AppConfig::global(cx));
                    },
                )
                .default_value(AppConfig::default().fixed.margin),
            )
            .description("Margin : Min 0, Max 500, Default 20"),
        ]
    }
}
