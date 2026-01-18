use crate::prelude::*;
use std::path::Path;
use tokio::fs;

#[poise::command(prefix_command, aliases("read"))]
pub async fn cat(
    ctx: PoiseContext<'_>,
    #[description = "File path to read"]
    #[rest]
    file_path: String,
) -> Result<(), Error> {
    let file_path = file_path.trim();

    if file_path.is_empty() {
        ctx.say("Usage: `.cat <file_path>`").await?;
        return Ok(());
    }

    let path = Path::new(file_path);

    let metadata = match fs::metadata(path).await {
        Ok(m) => m,
        Err(_) => {
            ctx.say(format!("ERROR: File not found: `{}`", file_path)).await?;
            return Ok(());
        }
    };

    if !metadata.is_file() {
        ctx.say(format!("ERROR: Path is not a file: `{}`", file_path)).await?;
        return Ok(());
    }

    match fs::read_to_string(path).await {
        Ok(content) => {
            let filename = path.file_name().unwrap_or_default().to_string_lossy();
            let ext = path.extension().unwrap_or_default().to_string_lossy();
            let line_count = content.lines().count();
            
            let lang = match ext.as_ref() {
                "rs"           => "rust",
                "py"           => "python",
                "js" | "ts"    => "javascript",
                "json"         => "json",
                "toml"         => "toml",
                "yaml" | "yml" => "yaml",
                "html"         => "html",
                "css"          => "css",
                "sh" | "bash"  => "bash",
                "ps1"          => "powershell",
                "c" | "cpp" | "h" => "cpp",
                "go"           => "go",
                "java"         => "java",
                "xml"          => "xml",
                "sql"          => "sql",
                "md"           => "markdown",
                _ => "",
            };

            let (display_content, truncated) = if content.len() > 3800 {
                (format!("{}...", &content[..3800]), true)
            } else {
                (content, false)
            };

            let code_block = if lang.is_empty() {
                format!("```\n{}\n```", display_content)
            } else {
                format!("```{}\n{}\n```", lang, display_content)
            };

            let mut footer = format!("{} lines", line_count);
            if truncated {
                footer.push_str(" (truncated)");
            }

            let embed = serenity::CreateEmbed::new()
                .title(format!("📄 {}", filename))
                .description(code_block)
                .footer(serenity::CreateEmbedFooter::new(footer))
                .color(0x2ecc71);

            ctx.send(poise::CreateReply::default().embed(embed)).await?;
        }
        Err(e) => {
            ctx.say(format!("ERROR: Failed to read file `{}`: {}", file_path, e)).await?;
        }
    }

    Ok(())
}
