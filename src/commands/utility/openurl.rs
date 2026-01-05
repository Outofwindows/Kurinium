use crate::commands::*;
use anyhow::Result;
use async_trait::async_trait;
use std::os::windows::process::CommandExt;
use std::sync::Arc;
use twilight_http::Client as HttpClient;
use twilight_model::channel::message::Message;

pub struct OpenUrlCommand;

#[async_trait]
impl BotCommand for OpenUrlCommand {
    fn name(&self) -> &str { "openurl" }
    fn description(&self) -> &str { "Open a URL in the default browser" }
    fn category(&self) -> &str { "utility" }
    fn usage(&self) -> &str { ".openurl <url> [loop_count]" }
    fn examples(&self) -> &'static [&'static str] {
        &[
            ".openurl https://google.com",
            ".openurl https://github.com/Mikasuru/Kurinium 67"
        ]
    }
    fn aliases(&self) -> &'static [&'static str] { &["url"] }

    async fn execute(
        &self,
        http: &Arc<HttpClient>,
        msg: &Message,
        mut args: Arguments,
    ) -> Result<()> {
        let url = match args.next() {
            Some(url) => url.to_string(),
            None => {
                http.create_message(msg.channel_id)
                    .content("**Usage**: `.openurl <url>` or `.openurl <url> <loop_count>`")
                    .await?;
                return Ok(());
            }
        };

        let loop_count = if let Some(count_str) = args.next() {
            count_str.parse::<u32>().unwrap_or(1)
        } else {
            1
        };

        // Validate URL format
        if !url.starts_with("http://") && !url.starts_with("https://") {
            http.create_message(msg.channel_id)
                .content("**Invalid URL**: URL must start with `http://` or `https://`")
                .await?;
            return Ok(());
        }

        if loop_count == 0 {
            http.create_message(msg.channel_id)
                .content("**Invalid loop count**: Must be at least 1")
                .await?;
            return Ok(());
        }

        if loop_count > 100 {
            http.create_message(msg.channel_id)
                .content("**Loop count too high**: Maximum is 100")
                .await?;
            return Ok(());
        }

        for i in 1..=loop_count {
            match std::process::Command::new("cmd")
                .args(&["/C", "start", &url])
                .creation_flags(0x08000000)
                .spawn()
            {
                Ok(_) => {},
                Err(e) => {
                    http.create_message(msg.channel_id)
                        .content(&format!("**Failed to open URL** (iteration {}): {}", i, e))
                        .await?;
                    return Ok(());
                }
            }

            if i < loop_count {
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }
        }

        let message = if loop_count == 1 {
            format!("**Opened URL**: {}", url)
        } else {
            format!("**Opened URL** {} times: {}", loop_count, url)
        };

        http.create_message(msg.channel_id)
            .content(&message)
            .await?;

        Ok(())
    }
}
