use anyhow::Context;
use std::path::PathBuf;
use std::{env, process};
use tracing::instrument;
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

use crate::common::AppConfig;
use crate::engine::ComGuard;
use crate::{common, engine, ui};

#[instrument]
pub fn app_run() -> anyhow::Result<()> {
    let mut cfg = common::load_config();

    set_startup(&cfg).context("An error occurred during startup.")?;

    let args: Vec<String> = std::env::args().collect();
    let is_ui_mode = args.get(1).map(|s| s.as_str()) == Some("--ui");

    if is_ui_mode {
        // --parent-pid の値を探す
        let parent_pid = args
            .iter()
            .position(|arg| arg == "--parent-pid")
            .and_then(|pos| args.get(pos + 1))
            .and_then(|s| s.parse::<u32>().ok());

        tracing::info!(parent_pid, "Launch in UI mode");
        ui::run(parent_pid).context("Error executing the settings screen")?;
        return Ok(());
    } else {
        restart_as_admin(&mut cfg).context("Error during privilege escalation check")?;

        tracing::info!("Start the main logic");
        engine::run().context("Main logic execution error")?;
    }

    Ok(())
}

#[instrument(skip(cfg))]
fn set_startup(cfg: &AppConfig) -> anyhow::Result<()> {
    // COMの初期化
    // メインスレッドではwinitがCOINIT_APARTMENTTHREADEDで起動する
    let _guard =
        ComGuard::new(COINIT_APARTMENTTHREADED).context("Failed to initialize COM (STA)")?;

    if cfg.startup {
        if let Err(e) = register_startup_task(cfg.administrator) {
            tracing::warn!(cause = ?e, "Startup registration failed, but continue processing");
        }
    } else if let Err(e) = unregister_startup_task() {
        tracing::warn!(cause = ?e, "Failed to disable startup");
    }

    Ok(())
}

pub fn register_startup_task(admin_required: bool) -> anyhow::Result<()> {
    unsafe {
        // タスクサービスへの接続
        let service = set_service().context("Failed to connect to the task service")?;
        // 新しいタスク定義を作成
        let task_definition = service
            .NewTask(0)
            .context("Failed to create a new task definition")?;
        // プリンシパルの設定 (権限レベル)
        let principal = task_definition
            .Principal()
            .context("Failed to obtain Principal")?;
        set_principal(principal, admin_required).context("Failed permission settings")?;
        // トリガーの設定 (ログイン時に実行)
        let triggers = task_definition
            .Triggers()
            .context("Failed to retrieve the trigger collection")?;
        set_trigger(triggers).context("Failed to configure the trigger (at login)")?;
        // アクションの設定 (実行するプログラム)
        let exe_path = std::env::current_exe().context("Failed to obtain my own pass")?;
        let actions = task_definition
            .Actions()
            .context("Failed to retrieve action collection")?;
        set_action(actions, exe_path.clone()).context("Failed to configure the action")?;
        // 詳細設定
        let settings = task_definition
            .Settings()
            .context("Failed to configure task details")?;
        set_settings(settings).context("Failed to update the settings object")?;
        // 登録
        let root_folder = service
            .GetFolder(&BSTR::from(r"\"))
            .context("Failed to retrieve the task root folder")?;

        root_folder
            .RegisterTaskDefinition(
                &BSTR::from("InputModeViewer_Startup"), // タスク名
                &task_definition,
                TASK_CREATE_OR_UPDATE.0,
                &VARIANT::default(), // ユーザーID
                &VARIANT::default(), // パスワード
                TASK_LOGON_INTERACTIVE_TOKEN,
                &VARIANT::default(), // SDDL
            )
            .context("Failed to save the task")?;

        tracing::info!(?exe_path, admin_required, "Register a startup task");
    }
    Ok(())
}

#[instrument]
pub fn unregister_startup_task() -> anyhow::Result<()> {
    let service = set_service().context("Failed to connect to the task service")?;
    unsafe {
        let root_folder = service
            .GetFolder(&BSTR::from(r"\"))
            .context("Failed to retrieve the task root folder")?;
        // タスク名が一致するものを削除
        match root_folder.DeleteTask(&BSTR::from("InputModeViewer_Startup"), 0) {
            Ok(_) => tracing::info!("Delete startup tasks"),
            Err(e) if e.code() == HRESULT(0x80070002u32 as i32) => {
                tracing::info!("No startup tasks were registered (Skip deletion)");
            }
            Err(e) => {
                return Err(anyhow::anyhow!(e))
                    .context("An unexpected error occurred while deleting a startup task");
            }
        }
    }
    Ok(())
}

#[instrument]
fn set_service() -> anyhow::Result<ITaskService> {
    unsafe {
        let service: ITaskService = CoCreateInstance(&TaskScheduler, None, CLSCTX_ALL)
            .context("Failed to create the Task Scheduler component")?;

        service
            .Connect(
                &VARIANT::default(),
                &VARIANT::default(),
                &VARIANT::default(),
                &VARIANT::default(),
            )
            .context("Failed to connect to the Task Scheduler service")?;
        Ok(service)
    }
}

#[instrument(skip(principal))] // principalオブジェクトの中身は見えないのでskip
fn set_principal(principal: IPrincipal, admin_required: bool) -> anyhow::Result<()> {
    unsafe {
        principal
            .SetRunLevel(if admin_required {
                TASK_RUNLEVEL_HIGHEST // 管理者権限
            } else {
                TASK_RUNLEVEL_LUA // 標準権限
            })
            .with_context(|| {
                format!(
                    "Failed to set the task execution permission level ({})",
                    if admin_required { "HIGHEST" } else { "LUA" }
                )
            })?;
        // 現在のユーザーで実行するように設定
        principal
            .SetLogonType(TASK_LOGON_INTERACTIVE_TOKEN)
            .context("Failed to configure the task's logon type (interactive token)")?;
    }
    Ok(())
}

#[instrument(skip(triggers))]
fn set_trigger(triggers: ITriggerCollection) -> anyhow::Result<()> {
    unsafe {
        let trigger = triggers
            .Create(TASK_TRIGGER_LOGON)
            .context("Failed to create a logon trigger")?;
        let logon_trigger: ILogonTrigger = trigger
            .cast()
            .context("Unable to convert the trigger object to a logon type")?;
        // 特定のユーザーを指定せず「誰かがログインしたら」にするのが一般的
        logon_trigger
            .SetUserId(&BSTR::from(""))
            .context("Failed to set the trigger's executing user ID")?;
    }
    Ok(())
}

#[instrument(skip(actions))]
fn set_action(actions: IActionCollection, exe_path: PathBuf) -> anyhow::Result<()> {
    unsafe {
        let action = actions
            .Create(TASK_ACTION_EXEC)
            .context("Failed to create the action")?;
        let exec_action: IExecAction = action
            .cast()
            .context("The action object could not be converted to an executable type")?;
        let path_str = exe_path
            .to_str()
            .context("Failed to convert the execution path string")?;
        exec_action
            .SetPath(&BSTR::from(path_str))
            .with_context(|| {
                format!(
                    "Failed to set the path to the executable program ({})",
                    path_str
                )
            })?;
        // 作業ディレクトリをexeのある場所に設定）
        let work_dir = exe_path
            .parent()
            .and_then(|p| p.to_str())
            .context("Failed to retrieve the path to the working directory")?;
        exec_action
            .SetWorkingDirectory(&BSTR::from(work_dir))
            .with_context(|| format!("Failed to set the working directory ({})", work_dir))?;
    }
    Ok(())
}

#[instrument(skip(settings))]
fn set_settings(settings: ITaskSettings) -> anyhow::Result<()> {
    unsafe {
        settings
            .SetEnabled(VARIANT_TRUE)
            .context("Failed to enable the task")?;
        settings
            .SetStartWhenAvailable(VARIANT_TRUE)
            .context("Failed to configure ‘StartWhenAvailable’")?;
        settings
            .SetHidden(VARIANT_FALSE)
            .context("Failed to set the ‘Hidden’ attribute")?;
        // AC電源のみの制限を解除
        settings
            .SetDisallowStartIfOnBatteries(VARIANT_FALSE)
            .context("Failed to remove the AC power restriction")?;
        settings
            .SetStopIfGoingOnBatteries(VARIANT_FALSE)
            .context("Failed to cancel the pause setting during battery switch")?;
        // 実行時間に制限を設けない
        settings
            .SetExecutionTimeLimit(&BSTR::from("PT0S"))
            .context("Failed to set the execution time limit (unlimited)")?;
    }
    Ok(())
}

#[instrument(skip(cfg))]
fn restart_as_admin(cfg: &mut AppConfig) -> anyhow::Result<()> {
    // 権限状態の同期と昇格チェック
    if engine::elevated_check() {
        // 現在管理者なら設定を同期
        if !cfg.administrator {
            cfg.administrator = true;
            common::save_config(cfg)
                .context("Failed to synchronize administrator status in config")?;
            tracing::info!("Already running as admin. Config state synchronized.");
        } else {
            tracing::info!("Running with administrative privileges.");
        }
    } else if cfg.administrator {
        // 現在一般権限で、設定では管理者として実行となっている場合
        tracing::info!("Attempting to elevate privileges (UAC prompt)...");

        // 自らの実行ファイルパスを取得
        let exe_path = env::current_exe()
            .context("Failed to retrieve current executable path for elevation")?;
        let exe_path_str = HSTRING::from(exe_path.as_os_str());

        let result =
            unsafe { ShellExecuteW(None, w!("runas"), &exe_path_str, None, None, SW_SHOW) };

        if result.0 as usize > 32 {
            tracing::info!("Elevation process launched successfully. Exiting current instance.");
            process::exit(0);
        } else {
            // ユーザーが「いいえ」を押した場合などはここ
            tracing::warn!(
                res = result.0 as usize,
                "Elevation failed or was cancelled by user. Falling back to normal user mode."
            );
            cfg.administrator = false;
            if let Err(e) = common::save_config(cfg) {
                tracing::warn!(error = ?e, "Failed to save fallback configuration");
            }
        }
    } else {
        tracing::info!("Running as normal user.");
    }

    Ok(())
}
