use crate::commands::*;
use crate::core::exit_patcher::safe_exit;
use crate::log_debug;
use anyhow::Result;
use async_trait::async_trait;
use twilight_http::Client as HttpClient;
use twilight_model::channel::message::Message;

pub struct ExitCommand;

#[async_trait]
impl BotCommand for ExitCommand {
    fn name(&self) -> &str { "exit" }
    fn description(&self) -> &str { "Gracefully shutdown the bot" }
    fn category(&self) -> &str { "core" }
    fn usage(&self) -> &str { ".exit" }
    fn examples(&self) -> &'static [&'static str] { &[".exit"] }
    fn aliases(&self) -> &'static [&'static str] { &["shutdown", "quit", "bye"] }

    async fn execute(&self, http: &Arc<HttpClient>, msg: &Message, _args: Arguments) -> Result<()> {
        http.create_message(msg.channel_id)
            .content("**Kurinium is Shutting Down...**")
            .await?;

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        log_debug!("Exit command executed : shutting down bot");
        safe_exit(0);
    }
}
