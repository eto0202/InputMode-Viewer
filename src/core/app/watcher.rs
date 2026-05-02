use crate::core::app::prelude::*;

pub fn spawn_config_watcher(
    proxy: EventLoopProxy<controller::Message>,
) -> anyhow::Result<impl Watcher> {
    let path = config::get_config_path()?;
    let parent_dir = path.parent().context("Invalid config path")?.to_path_buf();

    // 親ディレクトリは確実に作成
    std::fs::create_dir_all(&parent_dir)?;

    let mut watcher = notify::recommended_watcher(move |res: Result<Event, Error>| match res {
        Ok(e) => {
            // config.tomlが含まれているかチェック
            if e.paths.iter().any(|p| p.ends_with("config.toml")) {
                match e.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {
                        let _ = proxy.send_event(Message::ConfigUpdated);
                    }
                    _ => {}
                }
            }
        }
        Err(e) => log::error!("notify recommended_watcher Error: {:?}", e),
    })?;

    watcher.watch(&parent_dir, notify::RecursiveMode::NonRecursive)?;

    Ok(watcher)
}
