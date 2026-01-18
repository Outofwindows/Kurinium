use crate::prelude::*;
use std::fs;
use std::path::Path;

#[poise::command(prefix_command, aliases("md"))]
pub async fn mkdir(
    ctx: PoiseContext<'_>,
    #[description = "Directory path to create"]
    #[rest]
    dir_path: String,
) -> Result<(), Error> {
    let dir_path = dir_path.trim();

    if dir_path.is_empty() {
        ctx.say("ERROR: Please provide a directory path. Usage: `.mkdir <directory_path>`").await?;
        return Ok(());
    }

    let path = Path::new(dir_path);
    if path.exists() {
        ctx.say(format!("ERROR: Directory already exists: `{}`", dir_path)).await?;
        return Ok(());
    }

    match fs::create_dir_all(path) {
        Ok(_) => {
            ctx.say(format!("SUCCESS: **Directory created successfully:** `{}`", dir_path)).await?;
        }
        Err(e) => {
            ctx.say(format!("ERROR: Failed to create directory `{}`: {}", dir_path, e)).await?;
        }
    }

    Ok(())
}
