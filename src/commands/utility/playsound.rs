// todo: rework it, download mp3 and play it silently
use crate::prelude::*;

#[poise::command(prefix_command, aliases("play"))]
pub async fn playsound(
    ctx: PoiseContext<'_>,
    #[description = "YouTube/Audio URL"] url: String,
) -> Result<(), Error> {
    if url.is_empty() {
        ctx.say("Usage: `.playsound <url>`").await?;
        return Ok(());
    }
    
    if !url.starts_with("http://") && !url.starts_with("https://") {
        ctx.say("Invalid URL.").await?;
        return Ok(());
    }

    let reply = ctx.say(format!("Playing audio from `{}`...", url)).await?;
    // todo: test for music
    let script = format!(
        "Start-Process msedge -ArgumentList '{}' -WindowStyle Hidden", 
        url
    );
    
    use powershell_script::PsScriptBuilder;
    let result = tokio::task::spawn_blocking(move || {
        PsScriptBuilder::new()
            .no_profile(true)
            .non_interactive(true)
            .hidden(true)
            .print_commands(false)
            .build()
            .run(&script)
    }).await?;
    
    match result {
        Ok(_) => {
            reply.edit(ctx, poise::CreateReply::default().content("Audio started in background (Edge Hidden).")).await?;
        },
        Err(e) => {
            reply.edit(ctx, poise::CreateReply::default().content(format!("Failed to start audio: {}", e))).await?;
        }
    }

    Ok(())
}
