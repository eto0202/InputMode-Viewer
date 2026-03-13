use std::sync::mpsc;

use anyhow::Result;
use input_mode_viewer::{
    app::controller::{self, Message},
    sys::{hooks, input, uia::uia_event},
};
use winit::event_loop::{ControlFlow, EventLoop};

// TODO:
// モードの表示は入力状態移行時と、無操作状態が指定秒数経過後のみ。
// ウィンドウの切り替えだけでなく、要素を選択した時にも状態更新
// フェードアウトとマウス追従を実装
// 現在の自動消去はバグがあるため修正
// クールダウンタイムを実装
// ウィンドウを半透明に

fn main() -> Result<()> {
    let event_loop = EventLoop::<Message>::with_user_event().build()?;

    let proxy = event_loop.create_proxy();

    let (tx_uia, rx_uia) = mpsc::channel();
    let (tx_input, rx_input) = mpsc::channel();
    let rx_hooks = hooks::win_hooks();

    // ディスパッチャー
    std::thread::spawn(move || -> Result<()> {
        while let Ok(event) = rx_hooks.recv() {
            let _ = tx_uia.send(event.clone());
            let _ = tx_input.send(event.clone());
        }
        Ok(())
    });

    let proxy_uia = proxy.clone();
    let proxy_input = proxy.clone();

    uia_event::uia_thread(proxy_uia, rx_uia);
    input::input_thread(proxy_input, rx_input);

    // イベントループを「制御を戻さない」モードに設定
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = controller::Controller {
        ..Default::default()
    };

    event_loop.run_app(&mut app).unwrap();
    Ok(())
}
