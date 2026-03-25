// #![windows_subsystem = "windows"]
use input_mode_viewer::{
    app::controller::{self, Message},
    sys::{
        hooks,
        uia::{cap, mode},
    },
};
use std::sync::mpsc;
use winit::event_loop::{ControlFlow, EventLoop};

// TODO:
// モードの表示は入力状態移行時と、無操作状態が指定秒数経過後のみ。
// フェードアウト
// 現在の自動消去はバグがあるため修正
// クールダウンタイムを実装
// 表示位置指定(追従、全画面)
// 追従の可変ポーリングのユーザー設定
// 入力状態時にhCursorが変わっている可能性

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::<Message>::with_user_event().build()?;

    let proxy = event_loop.create_proxy();

    let (tx_uia, rx_uia) = mpsc::channel();
    let (tx_input, rx_input) = mpsc::channel();
    let rx_hooks = hooks::win_hooks();

    // ディスパッチャー
    std::thread::spawn(move || -> anyhow::Result<()> {
        while let Ok(event) = rx_hooks.recv() {
            tx_uia.send(event.clone())?;
            tx_input.send(event.clone())?;
        }
        Ok(())
    });

    let proxy_uia = proxy.clone();
    let proxy_input = proxy.clone();

    mode::mode_thread(proxy_uia, rx_uia);
    cap::cap_thread(proxy_input, rx_input);

    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = controller::Controller {
        ..Default::default()
    };

    event_loop.run_app(&mut app).unwrap();
    Ok(())
}
