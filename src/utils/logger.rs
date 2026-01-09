use std::fs::{OpenOptions, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use chrono::Local;

static LOG_FILE: Lazy<Mutex<Option<File>>> = Lazy::new(|| Mutex::new(None));

pub fn init_logger() {
    let log_path = get_log_path();
    
    if let Ok(file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        if let Ok(mut guard) = LOG_FILE.lock() {
            *guard = Some(file);
        }
    }
}

fn get_log_path() -> PathBuf {
    let exe_path = std::env::current_exe().unwrap_or_default();
    let exe_dir = exe_path.parent().unwrap_or(std::path::Path::new("."));
    exe_dir.join("debug.log")
}

pub fn log(message: &str) {
    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(ref mut file) = *guard {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            let _ = writeln!(file, "[{}] {}", timestamp, message);
            let _ = file.flush();
        }
    }
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        if $crate::config::Config::SHOW_CONSOLE {
            $crate::utils::logger::log(&format!($($arg)*));
        }
    };
}
