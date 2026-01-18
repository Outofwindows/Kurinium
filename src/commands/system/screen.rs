use crate::prelude::*;
use windows::Win32::UI::WindowsAndMessaging::{SendMessageW, WM_SYSCOMMAND, SC_MONITORPOWER};
use windows::Win32::Foundation::{HWND, WPARAM, LPARAM};
use powershell_script::PsScriptBuilder;

#[poise::command(prefix_command, aliases("display"))]
pub async fn screen(
    ctx: PoiseContext<'_>,
    #[description = "b <0-100> | m <on/off>"]
    #[rest]
    args: Option<String>,
) -> Result<(), Error> {
    let args = match args {
        Some(a) => a,
        None => {
            ctx.say("Usage: `.screen b <0-100>` (Brightness) | `.screen m <on|off>` (Monitor)").await?;
            return Ok(());
        }
    };

    let mut parts = args.split_whitespace();
    let mode = match parts.next() {
        Some(m) => m.to_lowercase(),
        None => {
            ctx.say("Usage: `.screen b <0-100>` (Brightness) | `.screen m <on|off>` (Monitor)").await?;
            return Ok(());
        }
    };
    
    let value = match parts.next() {
        Some(v) => v.to_lowercase(),
        None => {
            ctx.say("Usage: `.screen b <0-100>` (Brightness) | `.screen m <on|off>` (Monitor)").await?;
            return Ok(());
        }
    };

    match mode.as_str() {
        "b" | "brightness" => {
            if let Ok(level) = value.parse::<u32>() {
                let level = level.min(100);
                
                let script = format!(
                    "try {{ (Get-WmiObject -Namespace root/wmi -Class WmiMonitorBrightnessMethods).WmiSetBrightness(1, {}) }} catch {{ Write-Error $_ }}",
                    level
                );
                
                let result = tokio::task::spawn_blocking(move || {
                    PsScriptBuilder::new()
                        .no_profile(true)
                        .non_interactive(true)
                        .hidden(true)
                        .build()
                        .run(&script)
                }).await?;

                if result.is_ok() {
                    ctx.say(format!("Brightness set to {}%", level)).await?;
                } else {
                    ctx.say("Failed to set brightness (WMI error or not supported)").await?;
                }
            } else {
                ctx.say("Usage: `.screen b <0-100>`").await?;
            }
        },
        "m" | "monitor" => {
            let lparam: isize = match value.as_str() {
                "on" | "1" => -1,
                "off" | "0" => 2,
                "standby" => 1,
                _ => {
                    ctx.say("Usage: `.screen m <on|off|standby>`").await?;
                    return Ok(());
                }
            };
            
            tokio::task::spawn_blocking(move || {
                unsafe {
                    let hwnd_broadcast = HWND(0xFFFF as *mut _);
                    SendMessageW(
                        hwnd_broadcast,
                        WM_SYSCOMMAND,
                        Some(WPARAM(SC_MONITORPOWER as usize)),
                        Some(LPARAM(lparam))
                    );
                }
            }).await?;
            
            ctx.say(format!("Monitor power set to: {}", value)).await?;
        },
        _ => {
            ctx.say("Usage: `.screen b <0-100>` (Brightness) | `.screen m <on|off>` (Monitor)").await?;
        }
    }

    Ok(())
}