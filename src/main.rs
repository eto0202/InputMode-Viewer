#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use input_mode_viewer::core::logic;
use input_mode_viewer::ui::settings;

// TODO:
// モードの表示は入力状態移行時と、無操作状態が指定秒数経過後のみ。
// フェードアウト
// 現在の自動消去はバグがあるため修正
// クールダウンタイムを実装
// 追従の可変ポーリングのユーザー設定
// 設定読み込み
// 設定反映
// 更新

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.get(1).map(|s| s.as_str()) == Some("--ui") {
        // --parent-pid の値を探す
        let parent_pid = args
            .iter()
            .position(|arg| arg == "--parent-pid")
            .and_then(|pos| args.get(pos + 1))
            .and_then(|s| s.parse::<u32>().ok());

        settings::run(parent_pid)?;
        return Ok(());
    }

    logic::run()?;

    Ok(())
}
