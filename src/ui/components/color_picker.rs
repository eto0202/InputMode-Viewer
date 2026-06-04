use gpui::{prelude::FluentBuilder, *};
use gpui_component::{ActiveTheme, Colorize, Sizable, color_picker::{ColorPicker, ColorPickerState}, h_flex, setting::{RenderOptions, SettingFieldElement}};



pub struct ColorPickerSettingItem {
    color: Entity<ColorPickerState>,
    selected_color: Option<Hsla>,
}

impl ColorPickerSettingItem {
    pub fn new(color: Entity<ColorPickerState>, selected_color: Option<Hsla>) -> Self {
        Self { color, selected_color }
    }
}

impl SettingFieldElement for ColorPickerSettingItem {
    type Element = Div;

    fn render_field(&self, _: &RenderOptions, _: &mut Window, cx: &mut App) -> Self::Element {
        h_flex()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .py_1()
            .px_2()
            .child(
                h_flex()
                    .justify_around()
                    .child(ColorPicker::new(&self.color).small())
                    .when_some(self.selected_color, |this, color| {
                        this.w_24().child(color.to_hex())
                    }),
            )
    }
}
