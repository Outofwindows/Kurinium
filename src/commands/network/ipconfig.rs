use crate::commands::*;
use anyhow::Result;
use async_trait::async_trait;
use get_if_addrs::IfAddr; // import enum for matching
use serde_json::Value;
use std::collections::HashMap;
use twilight_http::Client as HttpClient;
use twilight_model::channel::message::embed::EmbedField;
use twilight_model::channel::message::Message;
use twilight_util::builder::embed::{EmbedBuilder, EmbedFooterBuilder};

pub struct IpconfigCommand;

#[async_trait]
impl BotCommand for IpconfigCommand {
    fn name(&self) -> &str { "ipconfig" }
    fn description(&self) -> &str { "Display network configuration and public IP" }
    fn category(&self) -> &str { "network" }
    fn usage(&self) -> &str { ".ipconfig" }
    fn examples(&self) -> &'static [&'static str] { &[".ipconfig"] }
    fn aliases(&self) -> &'static [&'static str] { &["ip", "netinfo"] }

    async fn execute(&self, http: &Arc<HttpClient>, msg: &Message, _args: Arguments) -> Result<()> {
        let thinking_msg = http
            .create_message(msg.channel_id)
            .content("`Fetching network information...`")
            .await?
            .model()
            .await?;

        let public_ip_info = get_public_ip_info().await;
        let interfaces = get_if_addrs::get_if_addrs().unwrap_or_default();

        let mut embed = EmbedBuilder::new()
            .title("Network Configuration")
            .color(0x00AEEF)
            .footer(EmbedFooterBuilder::new("Kurinium Network Utility"));

        // add public ip info first
        let public_ip_field = match public_ip_info {
            Ok(info) => info,
            Err(e) => format!("**Error:** {}", e),
        };
        embed = embed.field(EmbedField {
            name: "Public IP Information".to_string(),
            value: public_ip_field,
            inline: false,
        });

        // group addresses by interface name manually
        let mut grouped_ifaces: HashMap<String, (Vec<String>, Option<String>)> = HashMap::new();
        for iface in interfaces {
            if iface.is_loopback() {
                continue;
            }

            let entry = grouped_ifaces.entry(iface.name).or_default();

            match iface.addr {
                IfAddr::V4(addr) => {
                    entry.0.push(format!("**IPv4:** `{}`", addr.ip));
                }
                IfAddr::V6(addr) => {
                    if !addr.ip.is_loopback() && !addr.ip.to_string().starts_with("fe80") {
                        entry.0.push(format!("**IPv6:** `{}`", addr.ip));
                    }
                }
            }
        }

        // embed fields from the grouped data
        for (name, (ips, mac)) in grouped_ifaces {
            let mut values = Vec::new();
            if let Some(mac_addr) = mac {
                values.push(mac_addr);
            }
            values.extend(ips);

            if !values.is_empty() {
                embed = embed.field(EmbedField {
                    name: format!("Interface: {}", name),
                    value: values.join("\n"),
                    inline: true,
                });
            }
        }

        http.update_message(thinking_msg.channel_id, thinking_msg.id)
            .content(Some("Done."))
            .embeds(Some(&[embed.build()]))
            .await?;

        Ok(())
    }
}

// gets rich public ip info from ip-api.com
async fn get_public_ip_info() -> Result<String> {
    let resp = reqwest::get(obfstr::obfstr!("http://ip-api.com/json")).await?.text().await?;
    let v: Value = serde_json::from_str(&resp)?;

    if v["status"] != "success" {
        return Ok("Could not retrieve public IP.".to_string());
    }

    let ip = v["query"].as_str().unwrap_or("N/A");
    let country = v["country"].as_str().unwrap_or("N/A");
    let city = v["city"].as_str().unwrap_or("N/A");
    let isp = v["isp"].as_str().unwrap_or("N/A");

    Ok(format!(
        "**IP:** `{}`\n**Location:** {}, {}\n**ISP:** {}",
        ip, city, country, isp
    ))
}

