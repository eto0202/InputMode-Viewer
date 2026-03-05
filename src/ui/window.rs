use crate::sys::uia::input_mode::*;
use gpui::*;

pub struct MainWindow {
    // テキストデータを保持
    pub input_mode: InputMode,
    // 自動消去用のID
    pub display_id: u64,
    
    
}

impl MainWindow {
    // 起動時にテキストデータを受け取る
    pub fn new(input_mode: InputMode, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            input_mode,
            display_id: 0,
        }
    }
}

impl Render for MainWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            // ダークグレー(rgb(39, 39, 39))
            .bg(rgb(0x0027_2727))
            .flex()
            .items_center()
            .justify_center()
            .text_color(rgb(0x00ffffff))
            .child(self.input_mode.as_str())
    }
}
