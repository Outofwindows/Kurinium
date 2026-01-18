use crate::prelude::*;
use winapi::um::winuser::{ShowWindow, SW_HIDE, SW_SHOW, IsWindowVisible};
use winapi::um::wincon::GetConsoleWindow;
use winapi::um::fileapi::{GetFileAttributesW, SetFileAttributesW};
use winapi::um::winnt::FILE_ATTRIBUTE_HIDDEN;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

#[poise::command(prefix_command)]
pub async fn visible(
    ctx: PoiseContext<'_>,
    #[description = "target (console/file)"] target: Option<String>,
    #[description = "args"] #[rest] args: Option<String>,
) -> Result<(), Error> {
    let target = match target {
        Some(t) if !t.trim().is_empty() => t.to_lowercase(),
        _ => {
            ctx.say("ERROR: Missing target\nUsage: `.visible <console|file> <args>`").await?;
            return Ok(());
        }
    };
    let args = args.unwrap_or_default();
    
    if target == "file" {
        let parts: Vec<&str> = args.split_whitespace().collect();

        let Some(last_part) = parts.last() else {
            ctx.say("Usage: `.visible file <path> <show/hide/status>`").await?;
            return Ok(());
        };

        let action = last_part.to_lowercase();
        let path_str = if parts.len() > 1 {
            args.rfind(last_part)
                .map(|pos| args[..pos].trim().to_string())
                .unwrap_or_else(|| args.trim().to_string())
        } else {
            parts[0].to_string()
        };
        
        let path_clean = path_str.trim_matches('"');
        let path_wide: Vec<u16> = OsStr::new(path_clean).encode_wide().chain(Some(0)).collect();
        let path_display = path_clean.to_string();

        let result = tokio::task::spawn_blocking(move || {
            let attrs = unsafe { GetFileAttributesW(path_wide.as_ptr()) };
            if attrs == winapi::um::fileapi::INVALID_FILE_ATTRIBUTES {
                return format!("File not found or access denied: `{}`", path_display);
            }

            match action.as_str() {
                "show" | "unhide" => {
                    let new_attrs = attrs & !FILE_ATTRIBUTE_HIDDEN;
                    if unsafe { SetFileAttributesW(path_wide.as_ptr(), new_attrs) } != 0 {
                        format!("File unhidden: `{}`", path_display)
                    } else {
                        format!("Failed to unhide file: `{}`", path_display)
                    }
                },
                "hide" => {
                    let new_attrs = attrs | FILE_ATTRIBUTE_HIDDEN;
                    if unsafe { SetFileAttributesW(path_wide.as_ptr(), new_attrs) } != 0 {
                        format!("File hidden: `{}`", path_display)
                    } else {
                        format!("Failed to hide file: `{}`", path_display)
                    }
                },
                "status" | "check" => {
                    if (attrs & FILE_ATTRIBUTE_HIDDEN) != 0 {
                        format!("File is **HIDDEN**: `{}`", path_display)
                    } else {
                        format!("File is **VISIBLE**: `{}`", path_display)
                    }
                },
                _ => {
                    format!("Usage: `.visible file <path> <show/hide/status>` (Action was: {})", action)
                }
            }
        }).await?;
        
        ctx.say(result).await?;

    } else {
        let action = if target == "console" {
            args.clone().to_lowercase()
        } else {
            target
        };
        
        let result = tokio::task::spawn_blocking(move || {
            let hwnd = unsafe { GetConsoleWindow() };
            if hwnd.is_null() {
                return "No console window found.".to_string();
            }

            match action.as_str() {
                "show" | "on" => {
                    unsafe { ShowWindow(hwnd, SW_SHOW); }
                    "Console shown.".to_string()
                },
                "hide" | "off" => {
                    unsafe { ShowWindow(hwnd, SW_HIDE); }
                    "Console hidden.".to_string()
                },
                "status" | "get" => {
                    let is_visible = unsafe { IsWindowVisible(hwnd) != 0 };
                    if is_visible {
                        "Console is currently **Visible**.".to_string()
                    } else {
                        "Console is currently **Hidden**.".to_string()
                    }
                },
                _ => "Usage: `.visible [console] <on/off/status>` or `.visible file <path> <show/hide>`".to_string()
            }
        }).await?;

        ctx.say(result).await?;
    }
    
    Ok(())
}
