use crate::prelude::*;
use std::env;
use std::path::Path;
use tokio::fs;

#[poise::command(prefix_command, aliases("dir", "list"))]
pub async fn ls(
    ctx: PoiseContext<'_>,
    #[description = "Path to list"]
    #[rest]
    path_arg: Option<String>,
) -> Result<(), Error> {
    let path_arg = path_arg.as_deref().map(|s| s.trim()).unwrap_or(".");
    let path = if path_arg == "." || path_arg.is_empty() {
        env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf())
    } else {
        Path::new(path_arg).to_path_buf()
    };

    let metadata = match fs::metadata(&path).await {
        Ok(m) => m,
        Err(_) => {
            ctx.say(format!("ERROR: Path not found: `{}`", path.display())).await?;
            return Ok(());
        }
    };

    if !metadata.is_dir() {
        ctx.say(format!("ERROR: Path is not a directory: `{}`", path.display())).await?;
        return Ok(());
    }

    match fs::read_dir(&path).await {
        Ok(mut entries) => {
            let mut files = Vec::new();
            let mut directories = Vec::new();

            while let Ok(Some(entry)) = entries.next_entry().await {
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();

                if let Ok(metadata) = entry.metadata().await {
                    if metadata.is_dir() {
                        directories.push(format!("📁 {}", file_name_str));
                    } else {
                        let size = metadata.len();
                        let size_str = if size < 1024 {
                            format!("{} B", size)
                        } else if size < 1024 * 1024 {
                            format!("{:.1} KB", size as f64 / 1024.0)
                        } else {
                            format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
                        };
                        files.push(format!("📄 {} `{}`", file_name_str, size_str));
                    }
                }
            }

            directories.sort();
            files.sort();

            let dir_count = directories.len();
            let file_count = files.len();

            let mut description = String::new();
            if !directories.is_empty() { description.push_str(&directories.join("\n")); }
            if !directories.is_empty() && !files.is_empty() { description.push_str("\n\n"); }
            if !files.is_empty()       { description.push_str(&files.join("\n")); }
            if directories.is_empty()  && files.is_empty() { description.push_str("*Empty directory*"); }

            if description.len() > 4000 {
                description = format!("{}...\n\n*(truncated)*", &description[..3950]);
            }

            let embed = serenity::CreateEmbed::new()
                .title(format!("📂 {}", path.display()))
                .description(description)
                .footer(serenity::CreateEmbedFooter::new(
                    format!("{} directories, {} files", dir_count, file_count)
                ))
                .color(0x3498db);

            ctx.send(poise::CreateReply::default().embed(embed)).await?;
        }
        Err(e) => {
            ctx.say(format!("ERROR: Failed to read directory `{}`: {}", path.display(), e)).await?;
        }
    }

    Ok(())
}
