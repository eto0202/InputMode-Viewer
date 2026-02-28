use gpui::*;
use gpui_component_assets::*;
use input_mode_viewer::modules::*;

// TODO:
// モードの表示は入力状態移行時と、無操作状態が指定秒数経過後のみ。
// ウィンドウの切り替えだけでなく、要素を選択した時にも状態更新

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |app_cx| {
        gpui_component::init(app_cx);

        // メインウィンドウを画面端にサイズ0で表示してユーザーから見えないように
        let controller_options = WindowOptions {
            // PopUpにすることでタスクバーにアイコンが表示されない
            kind: WindowKind::PopUp,
            titlebar: None,
            focus: false,
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: Point::new(px(0.), px(0.)),
                size: size(px(0.), px(0.)),
            })),
            ..Default::default()
        };

        app_cx
            .open_window(controller_options, |_, app_cx| {
                app_cx.new(controller::Controller::new)
            })
            .unwrap();
    });
}
