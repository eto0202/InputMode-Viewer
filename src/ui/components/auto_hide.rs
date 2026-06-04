use gpui::*;
use gpui_component::{
    Disableable, checkbox::Checkbox, h_flex, input::{InputState, NumberInput}, v_flex
};

use crate::common::{self, AppConfig, WindowRole};

pub fn auto_hide(
    text_color: Hsla,
    border_color: Hsla,
    is_enable: bool,
    is_disabled: bool,
    number_input: Entity<InputState>,
    role: WindowRole,
) -> impl IntoElement {
    let checkbox = Checkbox::new(format!("hide-enable-{}", role.as_ref()))
        .checked(is_enable)
        .disabled(is_disabled)
        .on_click(move |val, _, cx: &mut App| {
            match role {
                WindowRole::Fixed => {
                    AppConfig::global_mut(cx).fixed.auto_hide.enabled = *val;
                }
                WindowRole::Floating => {
                    AppConfig::global_mut(cx).floating.auto_hide.enabled = *val;
                }
            }

            common::request_config_save(AppConfig::global(cx).clone());
        });

    let number_input = NumberInput::new(&number_input)
        .disabled(is_disabled)
        .items_start();

    h_flex()
        .gap_4()
        .child(
            v_flex()
                .border_r_1()
                .border_color(border_color)
                .border_dashed()
                .pr_4()
                .items_center()
                .justify_start()
                .gap_2()
                .h(px(70.0))
                .child(div().text_color(text_color).child("Enable"))
                .child(checkbox),
        )
        .child(
            v_flex()
                .justify_start()
                .gap_2()
                .h(px(70.0))
                .w(px(120.0))
                .child(div().text_color(text_color).child("Hide Time"))
                .child(number_input),
        )
}
