use crate::prelude::*;
use crate::core::screenshot::Screenshot;

#[poise::command(prefix_command, aliases("ss"))]
pub async fn screenshot(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let result = tokio::task::spawn_blocking(|| Screenshot::capture_as_bytes()).await;

    match result {
        Ok(Ok((buffer, _))) => {
            let attachment = crate::prelude::serenity::CreateAttachment::bytes(buffer, "screenshot.png");
            ctx.send(poise::CreateReply::default()
                .content("Screenshot")
                .attachment(attachment)).await?;
        }
        Ok(Err(e)) => {
            ctx.say(format!("ERROR: {}", e)).await?;
        }
        Err(e) => {
            ctx.say(format!("ERROR: Task failed: {}", e)).await?;
        }
    }

    Ok(())
}
