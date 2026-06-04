use crate::{
    common::{
        self, AppConfig, AutoHide, D2d1ColorExt, DisplayStyle, GpuiColorExt, TextFormat, WindowRole,
    },
    ui::{ColorPickerSettingItem, SettingsWindow, components},
};
use gpui::*;
use gpui_component::{
    ActiveTheme,
    color_picker::{ColorPickerEvent, ColorPickerState},
    input::{InputEvent, InputState, NumberInputEvent, StepAction},
    setting::{NumberFieldOptions, SettingField, SettingItem},
};

pub struct Floating {
    pub bg_color: Entity<ColorPickerState>,
    pub bg_selected_color: Option<Hsla>,

    pub font_color: Entity<ColorPickerState>,
    pub font_selected_color: Option<Hsla>,

    pub number_input_value: f32,
    pub number_input: Entity<InputState>,

    #[allow(dead_code)]
    pub subscriptions: Vec<Subscription>,
}

impl Floating {
    pub fn new(window: &mut Window, cx: &mut Context<SettingsWindow>) -> Self {
        let default_style = AppConfig::default().fixed.style;

        let bg_selected_color = Some(AppConfig::global(cx).floating.style.bg_color.to_hsla());
        let bg_color = cx.new(|cx| {
            ColorPickerState::new(window, cx).default_value(default_style.bg_color.to_hsla())
        });

        let font_selected_color = Some(AppConfig::global(cx).floating.style.font_color.to_hsla());
        let font_color = cx.new(|cx| {
            ColorPickerState::new(window, cx).default_value(default_style.font_color.to_hsla())
        });

        let number_input_value = AppConfig::global(cx).floating.auto_hide.time;
        let number_input =
            cx.new(|cx| InputState::new(window, cx).default_value(number_input_value.to_string()));

        let subscriptions = vec![
            cx.subscribe(&bg_color, |this, _, ev, cx| match ev {
                ColorPickerEvent::Change(color) => {
                    AppConfig::global_mut(cx).floating.style.bg_color =
                        color.unwrap_or_default().to_d2d1_color();
                    this.floating.bg_selected_color = *color;
                    common::request_config_save(AppConfig::global(cx).clone());
                }
            }),
            cx.subscribe(&font_color, |this, _, ev, cx| match ev {
                ColorPickerEvent::Change(color) => {
                    AppConfig::global_mut(cx).floating.style.font_color =
                        color.unwrap_or_default().to_d2d1_color();
                    this.floating.font_selected_color = *color;
                    common::request_config_save(AppConfig::global(cx).clone());
                }
            }),
            cx.subscribe_in(
                &number_input,
                window,
                |this, state, e: &InputEvent, _, cx| match e {
                    InputEvent::Change => {
                        let text = state.read(cx).value();
                        let value = text
                            .parse::<f32>()
                            .unwrap_or(AppConfig::default().floating.auto_hide.time);
                        this.floating.number_input_value = value;

                        AppConfig::global_mut(cx).floating.auto_hide.time = value;
                        common::request_config_save(AppConfig::global(cx).clone());
                    }
                    InputEvent::Focus => {}
                    _ => {}
                },
            ),
            cx.subscribe_in(
                &number_input,
                window,
                |this, state, e: &NumberInputEvent, window, cx| match e {
                    NumberInputEvent::Step(step_action) => match step_action {
                        StepAction::Decrement => {
                            this.floating.number_input_value =
                                (this.floating.number_input_value - 1.0).max(0.0);

                            AppConfig::global_mut(cx).floating.auto_hide.time =
                                this.floating.number_input_value;
                            common::request_config_save(AppConfig::global(cx).clone());

                            state.update(cx, |input, cx| {
                                input.set_value(
                                    this.floating.number_input_value.to_string(),
                                    window,
                                    cx,
                                );
                            });
                        }
                        StepAction::Increment => {
                            this.floating.number_input_value += 1.0;

                            AppConfig::global_mut(cx).floating.auto_hide.time =
                                this.floating.number_input_value;
                            common::request_config_save(AppConfig::global(cx).clone());

                            state.update(cx, |input, cx| {
                                input.set_value(
                                    this.floating.number_input_value.to_string(),
                                    window,
                                    cx,
                                );
                            });
                        }
                    },
                },
            ),
        ];

        Self {
            bg_color,
            bg_selected_color,
            font_color,
            font_selected_color,
            number_input_value,
            number_input,
            subscriptions,
        }
    }

    pub fn floating(&mut self) -> Vec<SettingItem> {
        let number_input = self.number_input.clone();
        vec![
            SettingItem::new(
                "Font Size",
                SettingField::number_input(
                    NumberFieldOptions {
                        min: 5.0,
                        max: 100.0,
                        step: 1.0,
                        precision: None,
                    },
                    |cx: &App| AppConfig::global(cx).floating.style.font_size.into(),
                    |val: f64, cx: &mut App| {
                        AppConfig::global_mut(cx).floating.style.font_size = val as f32;
                        common::request_config_save(AppConfig::global(cx).clone());
                    },
                )
                .default_value(AppConfig::default().floating.style.font_size),
            )
            .description("Adjust the text size between 5 and 100. (Default: 14)"),
            SettingItem::new(
                "Font Color",
                SettingField::element(ColorPickerSettingItem::new(
                    self.font_color.clone(),
                    self.font_selected_color,
                )),
            )
            .description("Font Color: Default #F2F2F2"),
            SettingItem::new(
                "Text Format",
                SettingField::dropdown(
                    vec![
                        (TextFormat::Full.as_ref().into(), "Full".into()),
                        (TextFormat::Compact.as_ref().into(),"Compact".into()),
                    ],
                    |cx: &App| {
                        AppConfig::global(cx)
                            .floating
                            .style
                            .text_format
                            .as_ref()
                            .to_string()
                            .into()
                    },
                    |val: SharedString, cx: &mut App| {
                        let style = val
                            .as_str()
                            .parse::<TextFormat>()
                            .unwrap_or(TextFormat::Full);
                        AppConfig::global_mut(cx).floating.style.text_format = style;
                        common::request_config_save(AppConfig::global(cx).clone());
                    },
                )
                .default_value(AppConfig::default().floating.style.text_format.as_ref().to_string()),
            )
            .description("Full (Default): Show all text.\nCompact: Show essential text only."),
            SettingItem::new(
                "Background Color",
                SettingField::element(ColorPickerSettingItem::new(
                    self.bg_color.clone(),
                    self.bg_selected_color,
                )),
            )
            .description("Set the background color of the overlay. (Default: #333333)"),
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
                        AppConfig::global_mut(cx).floating.style.padding = val as f32;
                        common::request_config_save(AppConfig::global(cx).clone());
                    },
                )
                .default_value(AppConfig::default().floating.style.padding),
            )
            .description("Adjust the internal spacing between the text and the window edge. (Default: 5)"),
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
                        AppConfig::global_mut(cx).floating.style.opacity = (val / 100.0) as f32;

                        common::request_config_save(AppConfig::global(cx).clone());
                    },
                )
                .default_value(AppConfig::default().floating.style.opacity * 100.0),
            )
            .description("Adjust the window transparency from 1% to 100%. (Default: 50%)"),
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
                        AppConfig::global_mut(cx).floating.offset.x = val as i32;
                        common::request_config_save(AppConfig::global(cx).clone());
                    },
                )
                .default_value(AppConfig::default().floating.offset.x),
            )
            .description("Horizontal distance from the mouse cursor. (Default: 20)\nPositive values move the window to the right, negative to the left."),
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
                        AppConfig::global_mut(cx).floating.offset.y = val as i32;
                        common::request_config_save(AppConfig::global(cx).clone());
                    },
                )
                .default_value(AppConfig::default().floating.offset.y),
            )
            .description("Vertical distance from the mouse cursor. (Default: 20)\nPositive values move the window to the top, negative to the bottom."),
            SettingItem::new(
                "Display Style",
                SettingField::dropdown(
                    vec![
                        (DisplayStyle::Smart(AutoHide::default()).as_ref().into(), "Smart".into()),
                        (DisplayStyle::Always.as_ref().into(), "Always".into()),
                    ],
                    |cx: &App| {
                        AppConfig::global(cx)
                            .floating
                            .display_style
                            .as_ref()
                            .to_string()
                            .into()
                    },
                    |val: SharedString, cx: &mut App| {
                        let s = val
                            .as_str()
                            .parse::<DisplayStyle>()
                            .unwrap_or(DisplayStyle::Smart(AutoHide::default()));
                        AppConfig::global_mut(cx).floating.display_style = s;
                        common::request_config_save(AppConfig::global(cx).clone());
                    },
                )
                .default_value(AppConfig::default().floating.display_style.as_ref().to_string()),
            )
            .description("Smart (Default): Automatically shows or hides the overlay based on activity.\nAlways: The overlay is always visible."),
            SettingItem::new("Auto Hide", SettingField::render(move |_, _, cx| {
                let is_enable = AppConfig::global(cx).floating.auto_hide.enabled;
                let is_disabled = AppConfig::global(cx).floating.display_style == DisplayStyle::Always;
                let text_color = cx.theme().muted_foreground;
                let border_color = cx.theme().border;

                components::auto_hide(text_color, border_color, is_enable, is_disabled, number_input.clone(), WindowRole::Floating)
            })).description("Set the time (in seconds) before the window hides automatically. (Default 3)\nNote: This setting only applies when Visibility Mode is set to \"Smart\"."),
        ]
    }
}
