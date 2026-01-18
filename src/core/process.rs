use anyhow::Result;
use sysinfo::System;
use crate::utils::syscall::{self, access};

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cmdline: String,
    pub parent_pid: Option<u32>,
    pub memory_usage: u64,
    pub cpu_usage: f32,
    pub start_time: u64,
    pub status: String,
}

pub struct ProcessManager {
    system: System,
}

impl ProcessManager {
    pub fn new() -> Self {
        let mut system = System::new();
        system.refresh_memory();
        system.refresh_cpu_all();
        Self { system }
    }

    pub fn refresh(&mut self) {
        self.system.refresh_memory();
        self.system.refresh_cpu_all();
    }

    pub fn list_processes(&mut self) -> Vec<ProcessInfo> {
        self.refresh();
        let mut processes = Vec::new();
        let sys_procs = syscall::get_detailed_process_list();
        for proc in sys_procs {
            let info = ProcessInfo {
                pid: proc.pid,
                name: proc.name,
                cmdline: String::new(),
                parent_pid: Some(proc.parent_pid),
                memory_usage: proc.memory_usage, // Bytes
                cpu_usage: 0.0,
                start_time: proc.start_time,
                status: "Run".to_string(),
            };
            processes.push(info);
        }

        // Sort by PID
        processes.sort_by_key(|p| p.pid);
        processes
    }

    pub fn get_process_by_pid(&mut self, pid: u32) -> Option<ProcessInfo> {
        self.list_processes().into_iter().find(|p| p.pid == pid)
    }

    pub fn find_processes_by_name(&mut self, name: &str) -> Vec<ProcessInfo> {
        let name_lower = name.to_lowercase();
        self.list_processes()
            .into_iter()
            .filter(|p| p.name.to_lowercase().contains(&name_lower))
            .collect()
    }

    pub fn kill_process(&mut self, pid: u32) -> Result<()> {
        let handle = syscall::nt_open_process(pid, access::PROCESS_TERMINATE | access::PROCESS_QUERY_INFORMATION)
            .ok_or_else(|| anyhow::anyhow!("Failed to open process: {}", pid))?;

        let success = syscall::nt_terminate_process(handle, 1);
        syscall::nt_close(handle);

        if success {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to terminate process: {}", pid))
        }
    }

    pub fn get_system_info(&mut self) -> SystemInfo {
        self.refresh();
        let proc_count = syscall::get_process_list().len();
        SystemInfo {
            process_count: proc_count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub process_count: usize,
}

pub fn format_memory_size(bytes: u64) -> String {
 crate::utils::formatting::format_memory_size(bytes)
}

pub fn format_cpu_usage(usage: f32) -> String {
    crate::utils::formatting::format_cpu_usage(usage)
}
