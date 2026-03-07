use gpui::*;
use input_mode_viewer::*;

// TODO:
// モードの表示は入力状態移行時と、無操作状態が指定秒数経過後のみ。
// ウィンドウの切り替えだけでなく、要素を選択した時にも状態更新
// フェードアウトとマウス追従を実装
// 現在の自動消去はバグがあるため修正
// クールダウンタイムを実装
// ウィンドウを半透明に

fn main() {
    let application = Application::new();

    application.run(move |app| {

        // メインウィンドウを画面端にサイズ0で表示してユーザーから見えないように
        let options = WindowOptions {
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

        let _ = app.open_window(options, |_, app| app.new(app::controller::Controller::new));
    });
}
