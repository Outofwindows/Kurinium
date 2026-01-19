use std::env;
use std::sync::OnceLock;
use crate::utils::syscall::{self, access};
use crate::config::Config;
use crate::core::exit_patcher::safe_exit;
use crate::utils::admin::is_process_elevated;
use crate::log_debug;

static SINGLETON_MUTEX: OnceLock<isize> = OnceLock::new();

pub fn singleton_process(current_is_admin: bool) {
    use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS};
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::core::PCWSTR;

    let mutex_name = format!("Global\\{}", obfstr::obfstr!("kurinium_singleton_mutex"));
    let mut mutex_name_wide: Vec<u16> = mutex_name.encode_utf16().collect();
    mutex_name_wide.push(0);

    unsafe {
        let mutex_handle = CreateMutexW(
            None,
            true,
            PCWSTR(mutex_name_wide.as_ptr()),
        );

        if let Ok(handle) = mutex_handle {
            if handle.is_invalid() {
                log_debug!("[Singleton] Failed to create mutex");
                fallback_process_check(current_is_admin);
                return;
            }

            let last_error = windows::Win32::Foundation::GetLastError();
            if last_error == ERROR_ALREADY_EXISTS {
                log_debug!("[Singleton] Another instance detected via mutex");
                let _ = CloseHandle(handle);

                if !current_is_admin {
                    safe_exit(0);
                } else {
                    fallback_process_check(current_is_admin);
                    return;
                }
            }

            SINGLETON_MUTEX.set(handle.0 as isize).ok();
            log_debug!("[Singleton] Mutex created successfully");
        } else {
            log_debug!("[Singleton] Mutex creation failed, using fallback");
            fallback_process_check(current_is_admin);
        }
    }
}

fn fallback_process_check(current_is_admin: bool) {
    let Ok(current_exe_path) = env::current_exe() else {
        return;
    };
    let current_pid = std::process::id();
    let mut names_to_check = std::collections::HashSet::new();

    if let Some(name) = current_exe_path.file_name().and_then(|n| n.to_str()) {
        if !name.is_empty() && name.is_ascii() {
            names_to_check.insert(name.to_lowercase());
        }
    }

    let config_name = Config::get_exe_name();
    if !config_name.is_empty() && config_name.is_ascii() {
        names_to_check.insert(config_name.to_lowercase());
    }

    if names_to_check.is_empty() {
        return;
    }

    let processes = syscall::get_process_list();
    for (pid, proc_name) in processes {
        if pid == current_pid {
            continue;
        }

        let proc_lower = proc_name.to_lowercase();
        if !names_to_check.iter().any(|n| proc_lower.contains(n)) {
            continue;
        }

        let other_is_admin = is_process_elevated(pid);
        if current_is_admin {
            if !terminate_process(pid) {
                log_debug!("[Singleton] Failed to terminate process PID {}", pid);
            }
        } else {
            if other_is_admin {
                safe_exit(0);
            } else {
                if !terminate_process(pid) {
                    log_debug!("[Singleton] Failed to terminate process PID {}", pid);
                }
            }
        }
    }
}

fn terminate_process(pid: u32) -> bool {
    if let Some(handle) = syscall::nt_open_process(pid, access::PROCESS_TERMINATE) {
        let success = syscall::nt_terminate_process(handle, 1);
        syscall::nt_close(handle);
        success
    } else {
        false
    }
}
