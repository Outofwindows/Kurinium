#![allow(static_mut_refs)]

use std::collections::HashSet;
use std::ffi::c_void;
use std::ptr;
use std::sync::OnceLock;

use dinvk::module::{get_module_address, get_proc_address};
use dinvk::syscall;
use dinvk::types::HANDLE;
use dinvk::winapis::NT_SUCCESS;

const PAGE_EXECUTE_READWRITE: u32 = 0x40;

// Obfuscated module names
fn mod_kernelbase() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(|| obfstr::obfstr!("kernelbase.dll").to_string())
}
fn mod_kernel32() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(|| obfstr::obfstr!("kernel32.dll").to_string())
}
fn mod_ntdll() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(|| obfstr::obfstr!("ntdll.dll").to_string())
}
fn mod_mscoree() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(|| obfstr::obfstr!("mscoree.dll").to_string())
}

// Obfuscated function names
fn fn_terminate_process() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(|| obfstr::obfstr!("TerminateProcess").to_string())
}
fn fn_exit_process() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(|| obfstr::obfstr!("ExitProcess").to_string())
}
fn fn_cor_exit_process() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(|| obfstr::obfstr!("CorExitProcess").to_string())
}
fn fn_nt_terminate_process() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(|| obfstr::obfstr!("NtTerminateProcess").to_string())
}
fn fn_rtl_exit_user_process() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(|| obfstr::obfstr!("RtlExitUserProcess").to_string())
}
fn fn_exit_thread() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(|| obfstr::obfstr!("ExitThread").to_string())
}

struct ExitFunction {
    module_fn: fn() -> &'static str,
    function_fn: fn() -> &'static str,
    address: Option<*mut c_void>,
    original_bytes: [u8; 32],
    is_patched: bool,
}

impl ExitFunction {
    const fn new(module_fn: fn() -> &'static str, function_fn: fn() -> &'static str) -> Self {
        Self {
            module_fn,
            function_fn,
            address: None,
            original_bytes: [0u8; 32],
            is_patched: false,
        }
    }

    fn module(&self) -> &'static str { (self.module_fn)() }
    fn function(&self) -> &'static str { (self.function_fn)() }
}

static mut EXIT_FUNCTIONS: [ExitFunction; 7] = [
    ExitFunction::new(mod_kernelbase, fn_terminate_process),
    ExitFunction::new(mod_kernel32,   fn_terminate_process),
    ExitFunction::new(mod_kernelbase, fn_exit_process),
    ExitFunction::new(mod_kernel32,   fn_exit_process),
    ExitFunction::new(mod_mscoree,    fn_cor_exit_process),
    ExitFunction::new(mod_ntdll,      fn_nt_terminate_process),
    ExitFunction::new(mod_ntdll,      fn_rtl_exit_user_process),
];

#[cfg(target_arch = "x86_64")]
const SHELLCODE_SIZE: usize = 19;

#[cfg(target_arch = "x86")]
const SHELLCODE_SIZE: usize = 12;

pub fn patch_exit() -> bool {
    let exit_thread_addr = match get_exit_thread_address() {
        Some(addr) => addr,
        None => return false,
    };

    let shellcode = generate_shellcode(exit_thread_addr);

    let mut patched_addresses: HashSet<usize> = HashSet::new();
    let mut patch_count = 0;

    unsafe {
        for func in EXIT_FUNCTIONS.iter_mut() {
            let addr = get_function_address(func.module(), func.function());
            if addr.is_none() {
                continue;
            }

            let func_addr = addr.unwrap();
            let addr_usize = func_addr as usize;

            if patched_addresses.contains(&addr_usize) {
                continue;
            }

            func.address = addr;

            if !read_memory(func_addr, &mut func.original_bytes[..SHELLCODE_SIZE]) {
                continue;
            }

            if !write_memory_syscall(func_addr, &shellcode) {
                continue;
            }

            func.is_patched = true;
            patched_addresses.insert(addr_usize);
            patch_count += 1;
        }
    }

    patch_count > 0
}

pub fn reset_exit_functions() {
    let mut reset_addresses: HashSet<usize> = HashSet::new();

    unsafe {
        for func in EXIT_FUNCTIONS.iter_mut() {
            if !func.is_patched {
                continue;
            }

            if let Some(addr) = func.address {
                let addr_usize = addr as usize;

                if reset_addresses.contains(&addr_usize) {
                    func.is_patched = false;
                    continue;
                }

                if write_memory_syscall(addr, &func.original_bytes[..SHELLCODE_SIZE]) {
                    func.is_patched = false;
                    reset_addresses.insert(addr_usize);
                }
            }
        }
    }
}

pub fn is_patched() -> bool {
    unsafe { EXIT_FUNCTIONS.iter().any(|f| f.is_patched) }
}

pub fn get_patched_functions() -> Vec<String> {
    let mut result = Vec::new();

    unsafe {
        for func in EXIT_FUNCTIONS.iter() {
            if func.is_patched {
                result.push(format!("{}!{}", func.module(), func.function()));
            }
        }
    }

    result
}

pub fn safe_exit(code: i32) -> ! {
    reset_exit_functions();
    std::process::exit(code);
}

fn get_exit_thread_address() -> Option<*mut c_void> {
    let kernelbase = get_module_address(mod_kernelbase(), None);
    
    if !kernelbase.is_null() {
        let addr = get_proc_address(kernelbase, fn_exit_thread(), None);
        if !addr.is_null() {
            return Some(addr as *mut c_void);
        }
    }

    let kernel32 = get_module_address(mod_kernel32(), None);
    
    if !kernel32.is_null() {
        let addr = get_proc_address(kernel32, fn_exit_thread(), None);
        if !addr.is_null() {
            return Some(addr as *mut c_void);
        }
    }

    None
}

fn get_function_address(module: &str, function: &str) -> Option<*mut c_void> {
    let module_base = get_module_address(module, None);
    
    if module_base.is_null() { 
        return None;
    }

    let addr = get_proc_address(module_base, function, None);
    if addr.is_null() {
        return None;
    }

    Some(addr as *mut c_void)
}

#[cfg(target_arch = "x86_64")]
fn generate_shellcode(exit_thread_addr: *mut c_void) -> [u8; SHELLCODE_SIZE] {
    let addr_bytes = (exit_thread_addr as u64).to_le_bytes();

    [
        0x48, 0xC7, 0xC1, 0x00, 0x00, 0x00, 0x00, // MOV RCX, 0 (exit code = 0)
        0x48, 0xB8, // MOV RAX, imm64
        addr_bytes[0],
        addr_bytes[1],
        addr_bytes[2],
        addr_bytes[3],
        addr_bytes[4],
        addr_bytes[5],
        addr_bytes[6],
        addr_bytes[7],
        0xFF, 0xE0, // JMP RAX
    ]
}

#[cfg(target_arch = "x86")]
fn generate_shellcode(exit_thread_addr: *mut c_void) -> [u8; SHELLCODE_SIZE] {
    let addr_bytes = (exit_thread_addr as u32).to_le_bytes();

    [
        0xB9, 0x00, 0x00, 0x00, 0x00, // MOV ECX, 0 (exit code = 0)
        0xB8, // MOV EAX, imm32
        addr_bytes[0],
        addr_bytes[1],
        addr_bytes[2],
        addr_bytes[3],
        0xFF, 0xE0, // JMP EAX
    ]
}

fn read_memory(addr: *mut c_void, buffer: &mut [u8]) -> bool {
    unsafe {
        ptr::copy_nonoverlapping(addr as *const u8, buffer.as_mut_ptr(), buffer.len());
    }
    true
}

fn write_memory_syscall(addr: *mut c_void, data: &[u8]) -> bool {
    unsafe {
        let process_handle: HANDLE = -1isize as HANDLE;
        let mut base_address = addr;
        let mut region_size = data.len();
        let mut old_protect: u32 = 0;

        let status = syscall!(
            obfstr::obfstr!("NtProtectVirtualMemory"),
            process_handle,
            &mut base_address as *mut *mut c_void,
            &mut region_size,
            PAGE_EXECUTE_READWRITE,
            &mut old_protect
        );

        let status = match status {
            Some(s) => s,
            None => return false,
        };

        if !NT_SUCCESS(status) {
            return false;
        }

        ptr::copy_nonoverlapping(data.as_ptr(), addr as *mut u8, data.len());

        let mut base_address = addr;
        let mut region_size = data.len();
        let mut _dummy: u32 = 0;

        let _ = syscall!(
            obfstr::obfstr!("NtProtectVirtualMemory"),
            process_handle,
            &mut base_address as *mut *mut c_void,
            &mut region_size,
            old_protect,
            &mut _dummy
        );

        true
    }
}

#[allow(dead_code)]
pub fn patch_exit_with_handler(handler: extern "system" fn(u32)) -> bool {
    let handler_addr = handler as *mut c_void;
    let shellcode = generate_shellcode(handler_addr);

    let mut patched_addresses: HashSet<usize> = HashSet::new();
    let mut patch_count = 0;

    unsafe {
        for func in EXIT_FUNCTIONS.iter_mut() {
            let addr = get_function_address(func.module(), func.function());
            if addr.is_none() {
                continue;
            }

            let func_addr = addr.unwrap();
            let addr_usize = func_addr as usize;

            if patched_addresses.contains(&addr_usize) {
                continue;
            }

            func.address = addr;

            if !read_memory(func_addr, &mut func.original_bytes[..SHELLCODE_SIZE]) {
                continue;
            }

            if !write_memory_syscall(func_addr, &shellcode) {
                continue;
            }

            func.is_patched = true;
            patched_addresses.insert(addr_usize);
            patch_count += 1;
        }
    }

    patch_count > 0
}

#[allow(dead_code)]
pub fn get_patch_info() -> Vec<(String, usize, bool)> {
    let mut info = Vec::new();

    unsafe {
        for func in EXIT_FUNCTIONS.iter() {
            let name = format!("{}!{}", func.module(), func.function());
            let addr = func.address.map(|a| a as usize).unwrap_or(0);
            info.push((name, addr, func.is_patched));
        }
    }

    info
}