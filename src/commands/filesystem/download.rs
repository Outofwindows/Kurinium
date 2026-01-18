use crate::prelude::*;
use std::path::Path;

#[poise::command(prefix_command, aliases("dl"))]
pub async fn download(
    ctx: PoiseContext<'_>,
    #[description = "URL to download (or attach file)"]
    #[rest]
    args: Option<String>,
) -> Result<(), Error> {
    let (url, target_path) = if let poise::Context::Prefix(p_ctx) = ctx {
        if let Some(att) = p_ctx.msg.attachments.first() {
            let args = args.unwrap_or_default();
            let save_path = if args.trim().is_empty() {
                att.filename.clone()
            } else { args.trim().to_string() };
            (att.url.clone(), save_path)
        } else {
            let args = args.unwrap_or_default();
            let mut parts = args.split_whitespace();
            let url = match parts.next() {
                Some(u) => u.to_string(),
                None => {
                    ctx.say("Usage: `.download <url> [path]` or attach a file").await?;
                    return Ok(());
                }
            };
            let path = parts.collect::<Vec<_>>().join(" ");
            let filename = url.split('/').last().unwrap_or("download");
            let target = if path.is_empty() { filename.to_string() } else { path };
            (url, target)
        }
    } else {
        ctx.say("Usage: `.download <url> [path]` or attach a file").await?;
        return Ok(());
    };

    let filename = Path::new(&target_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let reply = ctx.say(format!("Downloading `{}`...", filename)).await?;

    let client = reqwest::Client::new();
    match client.get(&url).send().await {
        Ok(response) => {
            if !response.status().is_success() {
                reply.edit(ctx, poise::CreateReply::default()
                    .content(format!("ERROR: Server returned {}", response.status()))).await?;
                return Ok(());
            }
            
            let _content_length = response.content_length();
            
            match response.bytes().await {
                Ok(bytes) => {
                    let size = bytes.len();
                    
                    if let Err(e) = tokio::fs::write(&target_path, &bytes).await {
                        reply.edit(ctx, poise::CreateReply::default()
                            .content(format!("ERROR: Failed to save file: {}", e))).await?;
                        return Ok(());
                    }

                    let size_str = if size >= 1024 * 1024 {
                        format!("{:.2} MB", size as f64 / (1024.0 * 1024.0))
                    } else if size >= 1024 { format!("{:.2} KB", size as f64 / 1024.0)
                    } else { format!("{} bytes", size) };

                    let embed = serenity::CreateEmbed::new()
                        .title("Download Complete")
                        .field("File", format!("`{}`", filename), true)
                        .field("Size", size_str, true)
                        .field("Saved to", format!("`{}`", target_path), false)
                        .color(0x2ecc71);

                    reply.edit(ctx, poise::CreateReply::default().content("").embed(embed)).await?;
                }
                Err(e) => {
                    reply.edit(ctx, poise::CreateReply::default()
                        .content(format!("ERROR: Failed to read response: {}", e))).await?;
                }
            }
        }
        Err(e) => {
            reply.edit(ctx, poise::CreateReply::default()
                .content(format!("ERROR: Download failed: {}", e))).await?;
        }
    }

    Ok(())
}
