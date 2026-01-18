use thiserror::Error;

#[derive(Error, Debug)]
pub enum KuriniumError {
    // ============ IO Errors ============
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },

    #[error("Directory not found: {path}")]
    DirectoryNotFound { path: String },

    // ============ Network Errors ============
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Connection timeout after {timeout_ms}ms")]
    ConnectionTimeout { timeout_ms: u64 },

    #[error("Download failed: {reason}")]
    DownloadFailed { reason: String },

    #[error("Upload failed: {reason}")]
    UploadFailed { reason: String },

    // ============ Discord Errors ============
    #[error("Discord API error: {0}")]
    Discord(#[from] serenity::Error),

    #[error("Channel not found: {channel_id}")]
    ChannelNotFound { channel_id: u64 },

    #[error("Guild not found: {guild_id}")]
    GuildNotFound { guild_id: u64 },

    #[error("Failed to send message: {reason}")]
    MessageSendFailed { reason: String },

    // ============ Crypto Errors ============
    #[error("Encryption failed: {reason}")]
    Encryption { reason: String },

    #[error("Decryption failed: {reason}")]
    Decryption { reason: String },

    #[error("Invalid key length: expected {expected}, got {actual}")]
    InvalidKeyLength { expected: usize, actual: usize },

    // ============ Process Errors ============
    #[error("Process execution failed: {0}")]
    ProcessExecution(String),

    #[error("Process not found: {name}")]
    ProcessNotFound { name: String },

    #[error("Command timed out after {timeout_secs} seconds")]
    CommandTimeout { timeout_secs: u64 },

    // ============ Validation Errors ============
    #[error("Invalid argument: {message}")]
    InvalidArgument { message: String },

    #[error("Missing required argument: {name}")]
    MissingArgument { name: String },

    #[error("Invalid path '{path}': {reason}")]
    InvalidPath { path: String, reason: String },

    // ============ Config Errors ============
    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("Auth manager not initialized")]
    AuthNotInitialized,

    #[error("Auth manager already initialized")]
    AuthAlreadyInitialized,

    // ============ System Errors ============
    #[error("Windows API error in {function}: code {code}")]
    WindowsApi { function: String, code: i32 },

    #[error("Registry error: {0}")]
    Registry(String),

    #[error("Screenshot capture failed: {reason}")]
    Screenshot { reason: String },

    #[error("Webcam capture failed: {reason}")]
    Webcam { reason: String },

    #[error("System info error: {reason}")]
    SystemInfo { reason: String },

    // ============ Zip/Archive Errors ============
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    // ============ JSON Errors ============
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    // ============ Migration/Generic Errors ============
    #[error("{0}")]
    Other(String),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

impl KuriniumError {
    pub fn invalid_arg(msg: impl Into<String>) -> Self {
        Self::InvalidArgument {
            message: msg.into(),
        }
    }

    pub fn missing_arg(name: impl Into<String>) -> Self {
        Self::MissingArgument { name: name.into() }
    }

    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config {
            message: msg.into(),
        }
    }

    pub fn process(msg: impl Into<String>) -> Self {
        Self::ProcessExecution(msg.into())
    }

    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }

    pub fn file_not_found(path: impl Into<String>) -> Self {
        Self::FileNotFound { path: path.into() }
    }

    pub fn dir_not_found(path: impl Into<String>) -> Self {
        Self::DirectoryNotFound { path: path.into() }
    }
}
