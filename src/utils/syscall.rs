use dinvk::syscall;
use dinvk::types::HANDLE;
use std::ffi::c_void;
use std::ptr::null_mut;

// OBJECT_ATTRIBUTES for NT functions
#[repr(C)]
pub struct ObjectAttributes {
    pub length: u32,
    pub root_directory: *mut c_void,
    pub object_name: *mut c_void,
    pub attributes: u32,
    pub security_descriptor: *mut c_void,
    pub security_qos: *mut c_void,
}

impl Default for ObjectAttributes {
    fn default() -> Self {
        Self {
            length: std::mem::size_of::<ObjectAttributes>() as u32,
            root_directory: null_mut(),
            object_name: null_mut(),
            attributes: 0,
            security_descriptor: null_mut(),
            security_qos: null_mut(),
        }
    }
}

// CLIENT_ID for NT functions
#[repr(C)]
pub struct ClientId {
    pub unique_process: *mut c_void,
    pub unique_thread: *mut c_void,
}

// Returns process handle or null on failure
pub fn nt_open_process(pid: u32, desired_access: u32) -> Option<HANDLE> {
    let mut handle: HANDLE = null_mut();    
    let mut obj_attr = ObjectAttributes::default();
    let mut client_id = ClientId {
        unique_process: pid as *mut c_void,
        unique_thread: null_mut(),
    };
    
    let status = syscall!(
        "NtOpenProcess",
        &mut handle,
        desired_access,
        &mut obj_attr,
        &mut client_id
    )?;
    
    if status >= 0 {
        Some(handle)
    } else { None }
}

// NtTerminateProcess
pub fn nt_terminate_process(handle: HANDLE, exit_code: u32) -> bool {
    if let Some(status) = syscall!(obfstr::obfstr!("NtTerminateProcess"), handle, exit_code) {
        status >= 0
    } else {
        false
    }
}

// NtClose  
pub fn nt_close(handle: HANDLE) -> bool {
    if let Some(status) = syscall!(obfstr::obfstr!("NtClose"), handle) {
        status >= 0
    } else {
        false
    }
}

// NtOpenProcessToken
pub fn nt_open_process_token(process_handle: HANDLE, desired_access: u32) -> Option<HANDLE> {
    let mut token_handle: HANDLE = null_mut();
    
    let status = syscall!(
        "NtOpenProcessToken",
        process_handle,
        desired_access,
        &mut token_handle
    )?;
    
    if status >= 0 {
        Some(token_handle)
    } else {
        None
    }
}

pub fn nt_current_process() -> HANDLE {
    -1isize as HANDLE
}

// Process access rights constants
pub mod access {
    pub const PROCESS_TERMINATE: u32 = 0x0001;
    pub const PROCESS_QUERY_INFORMATION: u32 = 0x0400;
    pub const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    pub const PROCESS_VM_READ: u32 = 0x0010;
}

// Token access rights constants
pub mod token_access {
    pub const TOKEN_QUERY: u32 = 0x0008;
    pub const TOKEN_ADJUST_PRIVILEGES: u32 = 0x0020;
}

// NtQuerySystemInformation
// Process enumeration without handles

// UNICODE_STRING structure
#[repr(C)]
#[derive(Clone, Copy)]
pub struct UnicodeString {
    pub length: u16,
    pub maximum_length: u16,
    pub buffer: *mut u16,
}

// SYSTEM_PROCESS_INFORMATION
#[repr(C)]
pub struct SystemProcessInformation {
    pub next_entry_offset: u32,
    pub number_of_threads: u32,
    pub working_set_private_size: i64,
    pub hard_fault_count: u32,
    pub number_of_threads_high_watermark: u32,
    pub cycle_time: u64,
    pub create_time: i64,
    pub user_time: i64,
    pub kernel_time: i64,
    pub image_name: UnicodeString,
    pub base_priority: i32,
    pub unique_process_id: *mut c_void,
    pub inherited_from_unique_process_id: *mut c_void,
    pub handle_count: u32,
    pub session_id: u32,
    pub unique_process_key: usize,
    pub peak_virtual_size: usize,
    pub virtual_size: usize,
    pub page_fault_count: u32,
    pub peak_working_set_size: usize,
    pub working_set_size: usize,
    // i think theres more but we dont need them
}

const SYSTEM_PROCESS_INFORMATION_CLASS: u32 = 5;

// NtQuerySystemInformation
// will NOT open process handles, avoiding LSASS detection
pub fn get_process_list() -> Vec<(u32, String)> {
    let mut processes = Vec::new();
    
    // Start with 256KB buffer, grow if needed
    let mut buffer_size: u32 = 256 * 1024;
    let mut buffer: Vec<u8>;
    
    loop {
        buffer = vec![0u8; buffer_size as usize];
        let mut return_length: u32 = 0;
        
        let status = syscall!(
            obfstr::obfstr!("NtQuerySystemInformation"),
            SYSTEM_PROCESS_INFORMATION_CLASS,
            buffer.as_mut_ptr() as *mut c_void,
            buffer_size,
            &mut return_length
        );
        
        match status {
            Some(s) if s >= 0 => break,
            Some(s) if s == -0x3FFFFFFC => {  // STATUS_INFO_LENGTH_MISMATCH (0xC0000004)
                buffer_size = return_length + 0x10000;
                continue;
            }
            _ => return processes,
        }
    }
    
    let mut offset: usize = 0;
    loop {
        let info = unsafe {
            &*(buffer.as_ptr().add(offset) as *const SystemProcessInformation)
        };
        
        let pid = info.unique_process_id as u32;
        let name = if !info.image_name.buffer.is_null() && info.image_name.length > 0 {
            let chars = info.image_name.length as usize / 2;
            let slice = unsafe { std::slice::from_raw_parts(info.image_name.buffer, chars) };
            String::from_utf16_lossy(slice)
        } else {
            String::from("[System Process]")
        };
        
        processes.push((pid, name));
        
        if info.next_entry_offset == 0 {
            break;
        }
        offset += info.next_entry_offset as usize;
    }
    
    processes
}

#[derive(Debug, Clone)]
pub struct ProcessDetails {
    pub pid: u32,
    pub name: String,
    pub parent_pid: u32,
    pub memory_usage: u64, // working_set_private_size
    pub start_time: u64,   // create_time (Windows FILETIME format)
    pub user_time: u64,
    pub kernel_time: u64,
}

// NtQuerySystemInformation
pub fn get_detailed_process_list() -> Vec<ProcessDetails> {
    let mut processes = Vec::new();    
    let mut buffer_size: u32 = 256 * 1024;
    let mut buffer: Vec<u8>;
    
    loop {
        buffer = vec![0u8; buffer_size as usize];
        let mut return_length: u32 = 0;
        
        let status = syscall!(
            obfstr::obfstr!("NtQuerySystemInformation"),
            SYSTEM_PROCESS_INFORMATION_CLASS,
            buffer.as_mut_ptr() as *mut c_void,
            buffer_size,
            &mut return_length
        );
        
        match status {
            Some(s) if s >= 0 => break,
            Some(s) if s == -0x3FFFFFFC => { 
                buffer_size = return_length + 0x10000;
                continue;
            }
            _ => return processes,
        }
    }
    
    let mut offset: usize = 0;
    loop {
        let info = unsafe {
            &*(buffer.as_ptr().add(offset) as *const SystemProcessInformation)
        };
        
        let pid = info.unique_process_id as u32;
        let parent_pid = info.inherited_from_unique_process_id as u32;
        
        let name = if !info.image_name.buffer.is_null() && info.image_name.length > 0 {
            let chars = info.image_name.length as usize / 2;
            let slice = unsafe { std::slice::from_raw_parts(info.image_name.buffer, chars) };
            String::from_utf16_lossy(slice)
        } else {
            String::from("[System Process]")
        };
        
        processes.push(ProcessDetails {
            pid,
            name,
            parent_pid,
            memory_usage: info.working_set_private_size as u64,
            start_time: info.create_time as u64,
            user_time: info.user_time as u64,
            kernel_time: info.kernel_time as u64,
        });
        
        if info.next_entry_offset == 0 {
            break;
        }
        offset += info.next_entry_offset as usize;
    }
    
    processes
}


pub fn get_filtered_process_names() -> Vec<String> {
    get_process_list()
        .into_iter()
        .filter_map(|(_, name)| {
            let lower = name.to_lowercase();
            if lower.contains("lsass") 
                || lower.contains("csrss") 
                || lower.contains("smss")
                || lower.contains("[system") {
                None
            } else {
                Some(name)
            }
        })
        .collect()
}
