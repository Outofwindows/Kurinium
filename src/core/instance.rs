use std::env;
use crate::utils::syscall::{self, access};
use crate::config::Config;
use crate::core::exit_patcher::safe_exit;
use crate::utils::admin::is_process_elevated;

pub fn singleton_prcess(current_is_admin: bool) {
    let Ok(current_exe_path) = env::current_exe() else {
        return;
    };
    let current_pid = std::process::id();
    let mut names_to_check = std::collections::HashSet::new();
    if let Some(name) = current_exe_path.file_name().and_then(|n| n.to_str()) {
        names_to_check.insert(name.to_lowercase());
    }
    names_to_check.insert(Config::get_exe_name().to_lowercase());

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
            terminate_process(pid);
        } else {
            if other_is_admin {
                safe_exit(0);
            } else { terminate_process(pid); }
        }
    }
}

fn terminate_process(pid: u32) {
    if let Some(handle) = syscall::nt_open_process(pid, access::PROCESS_TERMINATE) {
        syscall::nt_terminate_process(handle, 1);
        syscall::nt_close(handle);
    }
}
