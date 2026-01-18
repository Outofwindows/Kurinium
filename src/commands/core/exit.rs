use crate::prelude::*;
use crate::core::exit_patcher::safe_exit;
use crate::log_debug;

#[poise::command(prefix_command, aliases("shutdown", "quit", "bye"))]
pub async fn exit(ctx: PoiseContext<'_>) -> Result<(), Error> {
    ctx.say("**Kurinium is Shutting Down...**").await?;

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    log_debug!("Exit command executed : shutting down bot");
    safe_exit(0);
}
