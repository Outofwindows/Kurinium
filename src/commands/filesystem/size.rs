use crate::prelude::*;
use crate::utils::formatting::format_bytes_detailed as format_bytes;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[poise::command(prefix_command, aliases("du", "diskusage"))]
pub async fn size(
    ctx: PoiseContext<'_>,
    #[description = "Path to calculate size"]
    #[rest]
    path_str: String,
) -> Result<(), Error> {
    let path_str = path_str.trim();

    if path_str.is_empty() {
        ctx.say("Usage: `.size <path>`").await?;
        return Ok(());
    }

    let path = Path::new(path_str);

    if !path.exists() {
        ctx.say(format!("ERROR: Path not found: `{}`", path_str)).await?;
        return Ok(());
    }

    let reply = ctx.say("Calculating size...").await?;

    let metadata = fs::metadata(path)?;

    if metadata.is_file() {
        let file_size = metadata.len();
        let embed = serenity::CreateEmbed::new()
            .title("File Size")
            .description(format!("`{}`", path.display()))
            .field("Size", format_bytes(file_size), false)
            .color(0x1abc9c);

        reply.edit(ctx, poise::CreateReply::default().content("").embed(embed)).await?;
    } else if metadata.is_dir() {
        match calculate_dir_size(path) {
            Ok(result) => {
                let embed = serenity::CreateEmbed::new()
                    .title("Directory Size")
                    .description(format!("`{}`", path.display()))
                    .field("Total Size", format_bytes(result.total_size), true)
                    .field("Files", result.file_count.to_string(), true)
                    .field("Directories", result.dir_count.to_string(), true)
                    .field("Errors", if result.error_count > 0 { 
                        format!("{} inaccessible", result.error_count) 
                    } else { 
                        "None".to_string() 
                    }, true)
                    .color(0x1abc9c);

                reply.edit(ctx, poise::CreateReply::default().content("").embed(embed)).await?;
            }
            Err(e) => {
                reply.edit(ctx, poise::CreateReply::default()
                    .content(format!("ERROR: Failed to calculate directory size: {}", e))).await?;
            }
        }
    }

    Ok(())
}

struct SizeResult {
    total_size: u64,
    file_count: usize,
    dir_count: usize,
    error_count: usize,
}

fn calculate_dir_size(path: &Path) -> Result<SizeResult, Error> {
    let mut total_size: u64 = 0;
    let mut file_count: usize = 0;
    let mut dir_count: usize = 0;
    let mut error_count: usize = 0;

    for entry in WalkDir::new(path).into_iter() {
        match entry {
            Ok(entry) => {
                let entry_path = entry.path();
                
                if entry_path.is_file() {
                    match fs::metadata(entry_path) {
                        Ok(metadata) => {
                            total_size += metadata.len();
                            file_count += 1;
                        }
                        Err(_) => {
                            error_count += 1;
                        }
                    }
                } else if entry_path.is_dir() && entry_path != path {
                    dir_count += 1;
                }
            }
            Err(_) => {
                error_count += 1;
            }
        }
    }

    Ok(SizeResult {
        total_size,
        file_count,
        dir_count,
        error_count,
    })
}
