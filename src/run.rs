use crate::{
    common::{app_config::AppConfig, config},
    core::{
        logic,
        sys::uia::com::ComGuard,
        utils::{self},
    },
    ui::settings,
};

use anyhow::Context;
use std::path::PathBuf;
use std::{env, process};
use windows::Win32::{
    Foundation::{VARIANT_FALSE, VARIANT_TRUE},
    System::{
        Com::{CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance},
        TaskScheduler::{
            IActionCollection, IExecAction, ILogonTrigger, IPrincipal, ITaskService, ITaskSettings,
            ITriggerCollection, TASK_ACTION_EXEC, TASK_CREATE_OR_UPDATE,
            TASK_LOGON_INTERACTIVE_TOKEN, TASK_RUNLEVEL_HIGHEST, TASK_RUNLEVEL_LUA,
            TASK_TRIGGER_LOGON, TaskScheduler,
        },
        Variant::VARIANT,
    },
    UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOW},
};
use windows_core::{BSTR, HRESULT, HSTRING, Interface, w};

pub fn app_run() -> anyhow::Result<()> {
    let mut cfg = config::load_config();

    set_startup(&cfg)?;
    log::info!("Setting startup successfully");

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
        restart_as_admin(&mut cfg)?;

        logic::run()?;
        log::info!("Main logic started successfully");
    }

    Ok(())
}

fn set_startup(cfg: &AppConfig) -> anyhow::Result<()> {
    // COMの初期化
    // メインスレッドではwinitがCOINIT_APARTMENTTHREADEDで起動する
    let _guard = ComGuard::new(COINIT_APARTMENTTHREADED)?;

    if cfg.startup {
        log::info!("Startup task registered/updated");
        if let Err(e) = register_startup_task(cfg.administrator) {
            log::warn!("Failed to register startup task: {}", e);
        }
        log::info!(
            "Change task run level: {:?}",
            if cfg.administrator { "HIGHEST" } else { "LUA" }
        );
    } else {
        unregister_startup_task()?;
        log::info!("Startup task removed");
    }
    Ok(())
}

pub fn register_startup_task(admin_required: bool) -> anyhow::Result<()> {
    unsafe {
        // タスクサービスへの接続
        let service = set_service()?;
        // 新しいタスク定義を作成
        let task_definition = service.NewTask(0)?;
        // プリンシパルの設定 (権限レベル)
        let principal = task_definition.Principal()?;
        set_principal(principal, admin_required)?;
        // トリガーの設定 (ログイン時に実行)
        let triggers = task_definition.Triggers()?;
        set_trigger(triggers)?;
        // アクションの設定 (実行するプログラム)
        let exe_path = std::env::current_exe()?;
        let actions = task_definition.Actions()?;
        set_action(actions, exe_path)?;
        // 詳細設定
        let settings = task_definition.Settings()?;
        set_settings(settings)?;
        // 登録
        let root_folder = service.GetFolder(&BSTR::from(r"\"))?;
        root_folder.RegisterTaskDefinition(
            &BSTR::from("InputModeViewer_Startup"), // タスク名
            &task_definition,
            TASK_CREATE_OR_UPDATE.0,
            &VARIANT::default(), // ユーザーID
            &VARIANT::default(), // パスワード
            TASK_LOGON_INTERACTIVE_TOKEN,
            &VARIANT::default(), // SDDL
        )?;
        log::info!("Register startup task");
    }
    Ok(())
}

pub fn unregister_startup_task() -> anyhow::Result<()> {
    let service = set_service()?;
    unsafe {
        let root_folder = service.GetFolder(&BSTR::from(r"\"))?;
        // タスク名が一致するものを削除
        match root_folder.DeleteTask(&BSTR::from("InputModeViewer_Startup"), 0) {
            Ok(_) => log::info!("Successfully deleted startup task."),
            Err(e) if e.code() == HRESULT(0x80070002u32 as i32) => {
                log::info!("Startup task not found, nothing to delete.");
            }
            Err(e) => return Err(anyhow::anyhow!("Failed to delete task: {}", e)),
        }
        log::info!("Unregister startup task");
    }
    Ok(())
}

fn set_service() -> anyhow::Result<ITaskService> {
    unsafe {
        let service: ITaskService = CoCreateInstance(&TaskScheduler, None, CLSCTX_ALL)?;

        service.Connect(
            &VARIANT::default(),
            &VARIANT::default(),
            &VARIANT::default(),
            &VARIANT::default(),
        )?;
        Ok(service)
    }
}

fn set_principal(principal: IPrincipal, admin_required: bool) -> anyhow::Result<()> {
    unsafe {
        principal.SetRunLevel(if admin_required {
            TASK_RUNLEVEL_HIGHEST // 管理者権限
        } else {
            TASK_RUNLEVEL_LUA // 標準権限
        })?;
        // 現在のユーザーで実行するように設定
        principal.SetLogonType(TASK_LOGON_INTERACTIVE_TOKEN)?;
    }
    Ok(())
}

fn set_trigger(triggers: ITriggerCollection) -> anyhow::Result<()> {
    unsafe {
        let trigger = triggers.Create(TASK_TRIGGER_LOGON)?;
        let logon_trigger: ILogonTrigger = trigger.cast()?;
        // 特定のユーザーを指定せず「誰かがログインしたら」にするのが一般的
        logon_trigger.SetUserId(&BSTR::from(""))?;
    }
    Ok(())
}

fn set_action(actions: IActionCollection, exe_path: PathBuf) -> anyhow::Result<()> {
    unsafe {
        let action = actions.Create(TASK_ACTION_EXEC)?;
        let exec_action: IExecAction = action.cast()?;
        exec_action.SetPath(&BSTR::from(exe_path.to_str().unwrap()))?;
        // 作業ディレクトリをexeのある場所に設定）
        let work_dir = exe_path.parent().unwrap().to_str().unwrap();
        exec_action.SetWorkingDirectory(&BSTR::from(work_dir))?;
    }
    Ok(())
}

fn set_settings(settings: ITaskSettings) -> anyhow::Result<()> {
    unsafe {
        settings.SetEnabled(VARIANT_TRUE)?;
        settings.SetStartWhenAvailable(VARIANT_TRUE)?;
        settings.SetHidden(VARIANT_FALSE)?;
        // AC電源のみの制限を解除
        settings.SetDisallowStartIfOnBatteries(VARIANT_FALSE)?;
        settings.SetStopIfGoingOnBatteries(VARIANT_FALSE)?;
        // 実行時間に制限を設けない
        settings.SetExecutionTimeLimit(&BSTR::from("PT0S"))?;
    }
    Ok(())
}

fn restart_as_admin(cfg: &mut AppConfig) -> anyhow::Result<()> {
    // 権限状態の同期と昇格チェック
    if utils::elevated_check() {
        // 現在管理者なら設定を同期
        if !cfg.administrator {
            cfg.administrator = true;
            config::save_config(cfg)?;
        }
        log::info!("Running as administrator.");
    } else {
        // 現在一般権限で、設定では管理者として実行となっている場合
        if cfg.administrator {
            log::info!("Attempting to elevate privileges...");

            // 自らの実行ファイルパスを取得
            let exe_path = env::current_exe().context("Failed to retrieve the execution path")?;
            let exe_path_str = HSTRING::from(exe_path.as_os_str());

            let result = unsafe {
                ShellExecuteW(
                    None,
                    w!("runas"), // 昇格
                    &exe_path_str,
                    None, // 引数を渡す
                    None,
                    SW_SHOW,
                )
            };

            if result.0 as usize > 32 {
                process::exit(0);
            } else {
                log::warn!("Failed to elevate. Falling back to normal user.");
                cfg.administrator = false;
                config::save_config(cfg)?;
            }
        }
        log::info!("Not running as administrator.");
    }
    Ok(())
}
