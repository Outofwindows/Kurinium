use winapi::um::securitybaseapi::GetTokenInformation;
use winapi::um::winnt::{TokenElevation, TOKEN_ELEVATION};
use crate::utils::syscall::{self, access, token_access};

pub fn is_admin() -> bool {
    let current_process = syscall::nt_current_process();    
    let Some(token_handle) = syscall::nt_open_process_token(current_process, token_access::TOKEN_QUERY) else {
        return false;
    };
    
    let is_elevated = check_token_elevation(token_handle as *mut std::ffi::c_void);
    syscall::nt_close(token_handle);
    
    is_elevated
}

pub fn is_process_elevated(pid: u32) -> bool {
    let Some(proc_handle) = syscall::nt_open_process(pid, access::PROCESS_QUERY_INFORMATION) else {
        return false;
    };
    
    let Some(token_handle) = syscall::nt_open_process_token(proc_handle, token_access::TOKEN_QUERY) else {
        syscall::nt_close(proc_handle);
        return false;
    };
    
    let is_elevated = check_token_elevation(token_handle as *mut std::ffi::c_void);
    
    syscall::nt_close(token_handle);
    syscall::nt_close(proc_handle);
    
    is_elevated
}

fn check_token_elevation(token_handle: *mut std::ffi::c_void) -> bool {
    unsafe {
        let mut elevation: TOKEN_ELEVATION = std::mem::zeroed();
        let mut size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
        
        let result = GetTokenInformation(
            token_handle,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            size,
            &mut size,
        );
        
        result != 0 && elevation.TokenIsElevated != 0
    }
}
