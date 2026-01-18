use crate::prelude::*;
use std::env;
use std::os::windows::process::CommandExt;
use std::process::Command;

#[poise::command(prefix_command, aliases("upd"))]
pub async fn update(
    ctx: PoiseContext<'_>,
    #[description = "URL to new binary or attach file"]
    #[rest]
    url: Option<String>,
) -> Result<(), Error> {
    let download_url = if let poise::Context::Prefix(p_ctx) = ctx {
        if let Some(att) = p_ctx.msg.attachments.first() {
            att.url.clone()
        } else {
            url.unwrap_or_default()
        }
    } else {
        url.unwrap_or_default()
    };

    if download_url.is_empty() {
        ctx.say("Usage: `.update <url>` or attach a file").await?;
        return Ok(());
    }
    let reply = ctx.say(format!("Downloading update from: `{}`...", download_url)).await?;

    let resp = reqwest::get(&download_url).await?;
    if !resp.status().is_success() {
        reply.edit(ctx, poise::CreateReply::default()
            .content(format!("Download failed: HTTP {}", resp.status()))).await?;
        return Ok(());
    }
    let bytes = resp.bytes().await?;

    let temp_dir = env::temp_dir();
    let temp_path = temp_dir.join("kurinium_update.exe");
    tokio::fs::write(&temp_path, &bytes).await?;

    reply.edit(ctx, poise::CreateReply::default()
        .content("Replacing binary...")).await?;

    let current_exe = std::env::current_exe()?;

    let temp_path_clone = temp_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        self_replace::self_replace(&temp_path_clone)
    }).await?;

    if let Err(e) = result {
        reply.edit(ctx, poise::CreateReply::default()
            .content(format!("Update failed: {}", e))).await?;
        let _ = tokio::fs::remove_file(&temp_path).await;
        return Ok(());
    }

    reply.edit(ctx, poise::CreateReply::default()
        .content("Update successful! Restarting...")).await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    Command::new(&current_exe)
        .creation_flags(0x08000000)
        .spawn()
        .ok();
    std::process::exit(0);
}
