use gpui::{App, SharedString};
use gpui_component::setting::{SettingField, SettingItem};
use crate::common::{
    app_config::{AppConfig, WindowRole},
    config,
};

pub fn appearance() -> Vec<SettingItem> {
    vec![
        SettingItem::new(
            "Start Up",
            SettingField::switch(
                // 【Getter】現在の設定を読み取る
                |cx: &App| AppConfig::global(cx).startup,
                // 【Setter】UI操作時に設定を更新する
                |val: bool, cx: &mut App| {
                    AppConfig::global_mut(cx).startup = val;
                    let _ = config::save_config(AppConfig::global(cx));
                },
            ),
        )
        .description("Enable start up"),
        SettingItem::new(
            "Window Type",
            SettingField::dropdown(
                vec![
                    (WindowRole::Fixed.as_ref().into(), "Fixed".into()),
                    (WindowRole::Floating.as_ref().into(), "Floating".into()),
                ],
                |cx: &App| AppConfig::global(cx).active_role.as_ref().to_string().into(),
                |val: SharedString, cx: &mut App| {
                    let role = val.as_str().parse::<WindowRole>().unwrap_or(WindowRole::Fixed);
                    AppConfig::global_mut(cx).active_role = role;
                    let _ = config::save_config(AppConfig::global(cx));
                },
            )
            .default_value(AppConfig::default().active_role.as_ref().to_string()),
        )
        .description("Enter window type: Default Fixed"),
    ]
}
