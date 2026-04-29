#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use input_mode_viewer::{core::logic, ui::settings};

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
