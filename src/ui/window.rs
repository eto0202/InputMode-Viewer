use gpui::*;

use crate::common::{app_config, config};

pub struct SettingsWindow {
    pub config: app_config::AppConfig,
}

impl SettingsWindow {
    pub fn new(_: &mut Context<Self>) -> Self {
        Self {
            config: config::load_config(),
        }
    }
}

impl Render for SettingsWindow {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let con = self.config.clone();
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x2e2e2e))
            .p_4()
            .child(div().child("設定").mb_4())
            .child(div().child(format!(
                "現在のフォントサイズ: {}",
                con.fixed.style.font_size
            )))
            .child(
                button("save")
                    .child("保存して適用")
                    .on_click(move |_, _, cx| {
                        let _ = config::save_config(&con);
                        cx.quit();
                    }),
            )
    }
}

fn button(id: impl Into<ElementId>) -> Stateful<gpui::Div> {
    div()
        .id(id)
        .bg(rgb(0x4a4a4a))
        .hover(|s| s.bg(rgb(0x5a5a5a)))
        .p_2()
        .cursor_pointer()
}
