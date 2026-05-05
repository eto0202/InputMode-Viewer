

use crate::{
    common::config,
    core::{logic, utils},
    ui::settings,
};

pub fn app_run() -> anyhow::Result<()> {
    let mut cfg = config::load_config();

    if cfg.startup {
        log::info!("Startup task registered/updated");
        if let Err(e) = utils::register_startup_task(cfg.administrator) {
            log::warn!("Failed to register startup task: {}", e);
        }
        log::info!(
            "Change task run level: {:?}",
            if cfg.administrator { "HIGHEST" } else { "LUA" }
        );
    } else {
        // TODO: タスク削除
        utils::unregister_startup_task()?;
        log::info!("Startup task removed");
    }

    let args: Vec<String> = std::env::args().collect();
    let is_ui_mode = args.get(1).map(|s| s.as_str()) == Some("--ui");
    if is_ui_mode {
        // --parent-pid の値を探す
        let parent_pid = args
            .iter()
            .position(|arg| arg == "--parent-pid")
            .and_then(|pos| args.get(pos + 1))
            .and_then(|s| s.parse::<u32>().ok());

        settings::run(parent_pid)?;
        log::info!("Setting process started successfully");
        return Ok(());
    } else {
        utils::restart_as_admin(&mut cfg)?;

        logic::run()?;
        log::info!("Main logic started successfully");
    }

    Ok(())
}
