use anyhow::{Context, Result, anyhow};
use gpui::*;
use gpui_component_assets::*;
use input_mode_viewer::*;

// TODO:
// ツリー探索は重たいため、IUIAutomation::ElementFromHandleを使い、IMEインジケーターのRuntimeIdをキャッシュ
// IUIAutomaitonPropertyChangedEventHandlerを実装したstructを作り、タスクバーのIME要素にAddPropertyChangedEventHandlerを登録
// モードの表示は入力状態移行時と、無操作状態が指定秒数経過後のみ。
// ウィンドウの切り替えだけでなく、要素を選択した時にも状態更新
// フェードアウトとマウス追従を実装
// 現在の自動消去はバグがあるため修正
// クールダウンタイムを実装
// ウィンドウを半透明に
// 追従は最初に画面座標を取得し、GPUIの座標を上書きする？

fn main() -> Result<()> {
    let application = Application::new().with_assets(Assets);

    application.run(move |app| {
        gpui_component::init(app);

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

    Ok(())
}
