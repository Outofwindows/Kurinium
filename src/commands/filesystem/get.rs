use crate::prelude::*;
use crate::utils::nullpointer;
use std::path::Path;

const DISCORD_MAX_SIZE: u64 = 8 * 1024 * 1024;

#[poise::command(prefix_command)]
pub async fn get(
    ctx: PoiseContext<'_>,
    #[description = "File path to get"]
    #[rest]
    file_path: String,
) -> Result<(), Error> {
    let file_path = file_path.trim();

    if file_path.is_empty() {
        ctx.say("Usage: `.get <file_path>`\nSmall files (<8MB) -> Discord\nLarge files -> 0x0.st").await?;
        return Ok(());
    }

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
    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();

    if file_size <= DISCORD_MAX_SIZE {
        let content = tokio::fs::read(path).await?;
        let attachment = serenity::CreateAttachment::bytes(content, &filename);
        
        ctx.send(poise::CreateReply::default()
            .content(format!("`{}` ({:.2} KB)", filename, file_size as f64 / 1024.0))
            .attachment(attachment)).await?;
        
        return Ok(());
    }

    let reply = ctx.say(format!("Uploading `{}` ({:.2} MB) to 0x0.st...", 
        filename, file_size as f64 / (1024.0 * 1024.0))).await?;

    match nullpointer::upload_with_zip(path).await {
        Ok(result) => {
            let embed = serenity::CreateEmbed::new()
                .title("File Retrieved")
                .field("File", format!("`{}`", filename), true)
                .field("Size", format!("{:.2} MB", file_size as f64 / (1024.0 * 1024.0)), true)
                .field("Download", &result.url, false)
                .footer(serenity::CreateEmbedFooter::new("Hosted on 0x0.st"))
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