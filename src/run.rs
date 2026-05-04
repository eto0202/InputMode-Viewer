

use crate::{
    common::config,
    core::{logic, utils},
    ui::settings,
};

pub fn app_run() -> anyhow::Result<()> {
    let mut cfg = config::load_config();
    utils::restart_as_admin(&mut cfg)?;

    let args: Vec<String> = std::env::args().collect();

    if args.get(1).map(|s| s.as_str()) == Some("--ui") {
        // --parent-pid の値を探す
        let parent_pid = args
            .iter()
            .position(|arg| arg == "--parent-pid")
            .and_then(|pos| args.get(pos + 1))
            .and_then(|s| s.parse::<u32>().ok());

        settings::run(parent_pid)?;
        log::info!("Setting process started successfully");
        return Ok(());
    }

    logic::run()?;
    log::info!("Main logic started successfully");
    Ok(())
}
