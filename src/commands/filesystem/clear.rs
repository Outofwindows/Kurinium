use crate::prelude::*;

#[poise::command(prefix_command, aliases("cls", "purge"))]
pub async fn clear(
    ctx: PoiseContext<'_>,
    #[description = "Number of messages to delete"] num: Option<u16>,
) -> Result<(), Error> {
    let num_to_delete = num.unwrap_or(10).min(100) as u8;

    let channel_id = ctx.channel_id();
    let messages = channel_id.messages(ctx.http(), 
        serenity::builder::GetMessages::new().limit(num_to_delete)
    ).await?;

    if messages.is_empty() {
        ctx.say("INFO: No messages to delete.").await?;
        return Ok(());
    }

    let message_ids: Vec<_> = messages.iter().map(|m| m.id).collect();
    channel_id.delete_messages(ctx.http(), message_ids.iter().copied()).await?;

    ctx.say(format!("SUCCESS: Deleted {} messages.\n-# TIPS: You can use .clear <number>\n-# Kurinium: https://github.com/Mikasuru/Kurinium", messages.len())).await?;

    Ok(())
}
