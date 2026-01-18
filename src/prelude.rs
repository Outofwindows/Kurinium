#![allow(unused_imports)]
// Serenity + Poise re-exports for the crate
pub use std::sync::Arc;

// Poise framework
pub use poise::serenity_prelude as serenity;
pub use poise::Context;

// Common Serenity types
pub use serenity::model::channel::Message;
pub use serenity::model::id::{ChannelId, GuildId, UserId};
pub use serenity::http::Http;
pub use serenity::prelude::*;

// Cancellation token for graceful shutdown
pub use tokio_util::sync::CancellationToken;

// Internal modules
pub use crate::config::*;

// Custom error type
pub use crate::error::KuriniumError;

// Error type for poise commands (Box<dyn Error> for poise compatibility)
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type PoiseContext<'a> = poise::Context<'a, Data, Error>;

// Convenience type alias for internal use with KuriniumError
pub type KResult<T> = Result<T, KuriniumError>;

// Shared data across all commands
#[derive(Debug)]
pub struct Data {
    pub device_channel_id: ChannelId,
    pub guild_id: GuildId,
    pub caps_active: Arc<std::sync::atomic::AtomicBool>,
    pub shutdown_token: CancellationToken,
}
