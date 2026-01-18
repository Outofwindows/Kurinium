use crate::prelude::*;
use std::os::windows::process::CommandExt;
use std::process::Command;

#[poise::command(prefix_command, aliases("open"))]
pub async fn openurl(
    ctx: PoiseContext<'_>,
    #[description = "URL to open"]
    #[rest]
    url: String,
) -> Result<(), Error> {
    let url = url.trim();
    if url.is_empty() {
        ctx.say("ERROR: Usage: `.openurl <url>`").await?;
        return Ok(());
    }

    Command::new("cmd")
        .args(["/c", "start", "", url])
        .creation_flags(0x08000000)
        .spawn()?;

    ctx.say(format!("Opened: <{}>", url)).await?;
    Ok(())
}
