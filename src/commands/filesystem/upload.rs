use crate::prelude::*;
use crate::utils::nullpointer;
use crate::utils::formatting::format_bytes;
use std::path::Path;

const DISCORD_MAX_SIZE: u64 = 8 * 1024 * 1024;

#[poise::command(prefix_command, aliases("up"))]
pub async fn upload(
    ctx: PoiseContext<'_>,
    #[description = "File path or 'save <filename>'"]
    #[rest]
    args: Option<String>,
) -> Result<(), Error> {
    if let poise::Context::Prefix(p_ctx) = ctx {
        if !p_ctx.msg.attachments.is_empty() {
            return handle_attachment_save(ctx, p_ctx.msg, args).await;
        }
    }

    let args = match args {
        Some(a) if !a.trim().is_empty() => a,
        _ => {
            ctx.say("**Usage:**\n\
                - `.upload <file_path>` - Upload file to Discord/0x0.st\n\
                - `.upload` (with attachment) - Save attachment locally\n\
                - `.upload save <filename>` (with attachment) - Save with custom name").await?;
            return Ok(());
        }
    };

    let args_trimmed = args.trim();
    let mut parts = args_trimmed.split_whitespace();
    let first_arg = parts.next().unwrap_or("");

    if first_arg.to_lowercase() == "save" {
        ctx.say("ERROR: No attachment found.\nUsage: Attach a file and use `.upload save <filename>`").await?;
        return Ok(());
    }

    handle_file_upload(ctx, args_trimmed).await
}

async fn handle_attachment_save(
    ctx: PoiseContext<'_>,
    msg: &serenity::Message,
    args: Option<String>,
) -> Result<(), Error> {
    let attachment = &msg.attachments[0];
    let args_str = args.unwrap_or_default();
    let args_trimmed = args_str.trim();

    let save_filename = if args_trimmed.is_empty() {
        attachment.filename.clone()
    } else if args_trimmed.to_lowercase().starts_with("save ") {
        args_trimmed[5..].trim().to_string()
    } else if args_trimmed.to_lowercase() == "save" {
        attachment.filename.clone()
    } else {
        args_trimmed.to_string()
    };

    if save_filename.is_empty() {
        ctx.say("ERROR: Please provide a filename.\nUsage: `.upload save <filename>`").await?;
        return Ok(());
    }

    let reply = ctx.say(format!("Downloading `{}` as `{}`...", 
        attachment.filename, save_filename)).await?;

    let client = reqwest::Client::new();
    match client.get(&attachment.url).send().await {
        Ok(resp) => {
            if !resp.status().is_success() {
                reply.edit(ctx, poise::CreateReply::default()
                    .content(format!("ERROR: Failed to download. Status: {}", resp.status()))).await?;
                return Ok(());
            }
            let bytes = resp.bytes().await?;
            let file_size = bytes.len();

            match tokio::fs::write(&save_filename, &bytes).await {
                Ok(_) => {
                    let size_str = format_size(file_size);
                    let embed = serenity::CreateEmbed::new()
                        .title("Attachment Saved")
                        .field("Original", format!("`{}`", attachment.filename), true)
                        .field("Saved As", format!("`{}`", save_filename), true)
                        .field("Size", size_str, true)
                        .color(0x2ecc71);

                    reply.edit(ctx, poise::CreateReply::default()
                        .content("")
                        .embed(embed)).await?;
                }
                Err(e) => {
                    reply.edit(ctx, poise::CreateReply::default()
                        .content(format!("ERROR: Failed to save file: {}", e))).await?;
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

async fn handle_file_upload(ctx: PoiseContext<'_>, file_path: &str) -> Result<(), Error> {
    let path = Path::new(file_path);

    if !path.exists() {
        ctx.say(format!("ERROR: File not found: `{}`", file_path)).await?;
        return Ok(());
    }

    if !path.is_file() {
        ctx.say(format!("ERROR: Path is not a file: `{}`", file_path)).await?;
        return Ok(());
    }

    let metadata = tokio::fs::metadata(path).await?;
    let file_size = metadata.len();
    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");

    if file_size <= DISCORD_MAX_SIZE {
        let content = tokio::fs::read(path).await?;
        let attachment = serenity::CreateAttachment::bytes(content, filename);
        
        ctx.send(poise::CreateReply::default()
            .content(format!("`{}` ({})", filename, format_size(file_size as usize)))
            .attachment(attachment)).await?;
        
        return Ok(());
    }

    let reply = ctx.say(format!("Uploading `{}` ({}) to file host...", 
        filename, format_size(file_size as usize))).await?;

    match nullpointer::upload_with_zip(path).await {
        Ok(result) => {
            let embed = serenity::CreateEmbed::new()
                .title("File Uploaded")
                .field("File", format!("`{}`", filename), true)
                .field("Size", format_size(file_size as usize), true)
                .field("Host", &result.host, true)
                .field("Link", &result.url, false)
                .footer(serenity::CreateEmbedFooter::new("Expires in 30+ days"))
                .color(0x2ecc71);

            reply.edit(ctx, poise::CreateReply::default()
                .content("")
                .embed(embed)).await?;
        }
        Err(e) => {
            reply.edit(ctx, poise::CreateReply::default()
                .content(format!("ERROR: Upload failed: {}", e))).await?;
        }
    }

    Ok(())
}

fn format_size(bytes: usize) -> String {
    format_bytes(bytes as u64)
}
