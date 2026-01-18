use crate::prelude::*;
use std::time::Instant;

#[poise::command(prefix_command, aliases("latency"))]
pub async fn ping(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let start_time = Instant::now();
    
    let reply = ctx.say("Pinging...").await?;
    
    let latency = start_time.elapsed().as_millis();
    
    reply.edit(ctx, poise::CreateReply::default()
        .content(format!("**Pong!**\n**Latency:** {}ms", latency))
    ).await?;

    Ok(())
}
