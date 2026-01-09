use crate::commands::*;
use anyhow::Result;
use async_trait::async_trait;
use std::env;
use std::path::Path;
use twilight_http::Client as HttpClient;
use twilight_model::channel::message::Message;

pub struct CdCommand;

#[async_trait]
impl BotCommand for CdCommand {
    fn name(&self) -> &str { "cd" }
    fn description(&self) -> &str { "Change the current working directory" }
    fn category(&self) -> &str { "filesystem" }
    fn usage(&self) -> &str { ".cd <directory_path>" }
    fn examples(&self) -> &'static [&'static str] { &[".cd /home", ".cd ..", ".cd \\\\KuriniumServer\\D"] }
    fn aliases(&self) -> &'static [&'static str] { &["chdir"] }

    async fn execute(&self, http: &Arc<HttpClient>, msg: &Message, args: Arguments) -> Result<()> {
        let dir_path_owned = args.rest();
        let dir_path = dir_path_owned.trim().trim_matches('"');

        if dir_path.is_empty() {
            // Show current directory
            let current_dir =
                env::current_dir().unwrap_or_else(|_| Path::new("unknown").to_path_buf());
            http.create_message(msg.channel_id)
                .content(&format!(
                    "**Current directory:** `{}`",
                    current_dir.display()
                ))
                .await?;
            return Ok(());
        }

        let path = Path::new(dir_path);
        
        match env::set_current_dir(path) {
            Ok(_) => {
                let new_dir =
                    env::current_dir().unwrap_or_else(|_| Path::new("unknown").to_path_buf());
                http.create_message(msg.channel_id)
                    .content(&format!(
                        "SUCCESS: **Changed directory to:** `{}`",
                        new_dir.display()
                    ))
                    .await?;
            }
            Err(e) => {
                http.create_message(msg.channel_id)
                    .content(&format!(
                        "ERROR: Failed to change directory to `{}`: {}",
                        dir_path, e
                    ))
                    .await?;
            }
        }

        Ok(())
    }
}