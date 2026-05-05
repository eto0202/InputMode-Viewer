use crate::{
    common::{app_config::AppConfig, config},
    core::sys::uia::com::ComGuard,
};
use anyhow::Context;
use check_elevation::is_elevated;
use directories::ProjectDirs;
use flexi_logger::{Cleanup, Criterion, FileSpec, Logger, Naming};
use std::{env, process};
use windows::Win32::{
    Foundation::{VARIANT_FALSE, VARIANT_TRUE},
    System::{
        Com::{CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance},
        TaskScheduler::{
            IExecAction, ILogonTrigger, ITaskService, TASK_ACTION_EXEC, TASK_CREATE_OR_UPDATE,
            TASK_LOGON_INTERACTIVE_TOKEN, TASK_RUNLEVEL_HIGHEST, TASK_RUNLEVEL_LUA,
            TASK_TRIGGER_LOGON, TaskScheduler,
        },
        Variant::VARIANT,
    },
    UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOW},
};
use windows_core::{BSTR, HRESULT, HSTRING, Interface, w};

pub fn register_startup_task(admin_required: bool) -> anyhow::Result<()> {
    unsafe {
        // COMの初期化
        // メインスレッドではwinitがCOINIT_APARTMENTTHREADEDで起動するため
        let _guard = ComGuard::new(COINIT_APARTMENTTHREADED)?;

        // タスクサービスへの接続
        let service: ITaskService = CoCreateInstance(&TaskScheduler, None, CLSCTX_ALL)?;
        service.Connect(
            &VARIANT::default(),
            &VARIANT::default(),
            &VARIANT::default(),
            &VARIANT::default(),
        )?;

        // 新しいタスク定義を作成
        let task_definition = service.NewTask(0)?;

        // プリンシパルの設定 (権限レベル)
        let principal = task_definition.Principal()?;
        principal.SetRunLevel(if admin_required {
            TASK_RUNLEVEL_HIGHEST // 管理者権限
        } else {
            TASK_RUNLEVEL_LUA // 標準権限
        })?;
        // 現在のユーザーで実行するように設定
        principal.SetLogonType(TASK_LOGON_INTERACTIVE_TOKEN)?;

        // トリガーの設定 (ログイン時に実行)
        let triggers = task_definition.Triggers()?;
        let trigger = triggers.Create(TASK_TRIGGER_LOGON)?;
        let logon_trigger: ILogonTrigger = trigger.cast()?;
        // 特定のユーザーを指定せず「誰かがログインしたら」にするのが一般的
        logon_trigger.SetUserId(&BSTR::from(""))?;

        // アクションの設定 (実行するプログラム)
        let exe_path = std::env::current_exe()?;
        let actions = task_definition.Actions()?;
        let action = actions.Create(TASK_ACTION_EXEC)?;
        let exec_action: IExecAction = action.cast()?;
        exec_action.SetPath(&BSTR::from(exe_path.to_str().unwrap()))?;
        // 作業ディレクトリをexeのある場所に設定）
        let work_dir = exe_path.parent().unwrap().to_str().unwrap();
        exec_action.SetWorkingDirectory(&BSTR::from(work_dir))?;

        // 詳細設定
        let settings = task_definition.Settings()?;
        settings.SetEnabled(VARIANT_TRUE)?;
        settings.SetStartWhenAvailable(VARIANT_TRUE)?;
        settings.SetHidden(VARIANT_FALSE)?;
        // ノートPC向け：AC電源のみの制限を解除
        settings.SetDisallowStartIfOnBatteries(VARIANT_FALSE)?;
        settings.SetStopIfGoingOnBatteries(VARIANT_FALSE)?;
        // 実行時間に制限を設けない（デフォルト3日の制限を解除）
        settings.SetExecutionTimeLimit(&BSTR::from("PT0S"))?;

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
    }
    Ok(())
}

pub fn unregister_startup_task() -> anyhow::Result<()> {
    unsafe {
        let _guard = ComGuard::new(COINIT_APARTMENTTHREADED)?;
        let service: ITaskService = CoCreateInstance(&TaskScheduler, None, CLSCTX_ALL)?;
        log::info!("unregister_startup_task");
        service.Connect(
            &VARIANT::default(),
            &VARIANT::default(),
            &VARIANT::default(),
            &VARIANT::default(),
        )?;

        let root_folder = service.GetFolder(&BSTR::from(r"\"))?;
        // タスク名が一致するものを削除
        match root_folder.DeleteTask(&BSTR::from("InputModeViewer_Startup"), 0) {
            Ok(_) => log::info!("Successfully deleted startup task."),
            Err(e) if e.code() == HRESULT(0x80070002u32 as i32) => {
                log::info!("Startup task not found, nothing to delete.");
            }
            Err(e) => return Err(anyhow::anyhow!("Failed to delete task: {}", e)),
        }
    }
    Ok(())
}

pub fn restart_as_admin(cfg: &mut AppConfig) -> anyhow::Result<()> {
    // 権限状態の同期と昇格チェック
    if elevated_check() {
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

pub fn init_logger() -> anyhow::Result<()> {
    // 保存先
    let proj_dirs = ProjectDirs::from("com", "", "input_mode_viewer")
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
    let log_dir = proj_dirs.data_local_dir().join("logs");

    // ロガーの初期化
    Logger::try_with_str("info")?
        .log_to_file(FileSpec::default().directory(log_dir).basename("app"))
        .rotate(
            Criterion::Size(10 * 1024 * 1024), // 10MBごとに新しいファイルへ
            Naming::Timestamps,
            Cleanup::KeepLogFiles(5), // 最新の3つだけ残して古いのは消す
        )
        .start()?;

    Ok(())
}

// メモリ上のバイト列から画像をデコードしアイコンを生成
// アプリケーション内に画像が保存される
pub fn load_icon(to_include_bytes: &[u8]) -> tray_icon::Icon {
    let img = image::load_from_memory(to_include_bytes)
        .unwrap()
        .into_rgba8();
    let (width, height) = img.dimensions();
    let rgba = img.into_raw();

    tray_icon::Icon::from_rgba(rgba, width, height).unwrap()
}

pub fn restart_application(dropping_privileges: bool) {
    // 自らの実行ファイルパスを取得
    let exe_path = std::env::current_exe().expect("Failed to get exe path");

    let result = if dropping_privileges {
        let quoted_path = format!("\"{}\"", exe_path.display());
        let args_str = HSTRING::from(quoted_path);
        unsafe {
            ShellExecuteW(
                None,
                w!("open"),
                w!("explorer.exe"), // 実行ファイルはエクスプローラー
                &args_str,
                None,
                SW_SHOW,
            )
        }
    } else {
        let exe_path_str = HSTRING::from(exe_path.as_os_str());
        let args_str = HSTRING::from("");
        unsafe { ShellExecuteW(None, None, &exe_path_str, &args_str, None, SW_SHOW) }
    };

    if result.0 as usize > 32 {
        log::info!("Restart process spawned successfully. Exiting current process.");
        std::process::exit(0);
    } else {
        log::error!(
            "Failed to restart application via ShellExecuteW: {:?}",
            result
        );
    }
}

pub fn elevated_check() -> bool {
    let current_is_elevated = is_elevated().unwrap_or(false);
    log::info!(
        "Administrator: {:?}",
        if current_is_elevated { "TRUE" } else { "FALSE" }
    );

    current_is_elevated
}
