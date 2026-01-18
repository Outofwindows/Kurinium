use crate::prelude::*;
use std::env;
use powershell_script::PsScriptBuilder;

#[poise::command(prefix_command, aliases("printfile"))]
pub async fn print(
    ctx: PoiseContext<'_>,
    #[description = "File to print"] file: Option<serenity::Attachment>,
) -> Result<(), Error> {
    let attachment = match file {
        Some(att) => att,
        None => {
            if let poise::Context::Prefix(p_ctx) = ctx {
                if let Some(att) = p_ctx.msg.attachments.first() {
                     att.clone()
                } else {
                     ctx.say("Please attach a file to print.").await?;
                     return Ok(());
                }
            } else {
                ctx.say("Usage: `.print [attachment]`").await?;
                return Ok(());
            }
        }
    };

    let filename = &attachment.filename;
    let allowed_exts = ["png", "jpg", "jpeg", "pdf", "txt", "docx"];
    let ext = filename.split('.').last().unwrap_or("").to_lowercase();

    if !allowed_exts.contains(&ext.as_str()) {
        ctx.say("Unsupported file type. Supported: PNG, JPG, PDF, TXT, DOCX").await?;
        return Ok(());
    }

    let reply = ctx.say(format!("Downloading `{}`...", filename)).await?;

    let temp_dir = env::temp_dir();
    let file_path = temp_dir.join(filename);
    let content = attachment.download().await?;
    tokio::fs::write(&file_path, &content).await?;

    reply.edit(ctx, poise::CreateReply::default().content(format!("Printing `{}`...", filename))).await?;

    let file_path_str = file_path.to_string_lossy();
    let script = format!(
        "Start-Process -FilePath '{}' -Verb Print -PassThru | Out-Null",
        file_path_str
    );

    let result = tokio::task::spawn_blocking(move || {
        PsScriptBuilder::new()
            .no_profile(true)
            .non_interactive(true)
            .hidden(true)
            .print_commands(false)
            .build()
            .run(&script)
    }).await?;

    match result {
        Ok(_) => {
            reply.edit(ctx, poise::CreateReply::default().content(format!("Sent `{}` to default printer.", filename))).await?;
        },
        Err(e) => {
            reply.edit(ctx, poise::CreateReply::default().content(format!("Print failed: {}", e))).await?;
        }
    }

    let cleanup_path = file_path.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        let _ = tokio::fs::remove_file(cleanup_path).await;
    });

    Ok(())
}