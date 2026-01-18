use crate::prelude::*;
use std::fs;
use std::path::Path;

#[poise::command(prefix_command, aliases("rm", "del"))]
pub async fn remove(
    ctx: PoiseContext<'_>,
    #[description = "Path to remove"]
    #[rest]
    path_str: String,
) -> Result<(), Error> {
    let path_str = path_str.trim();

    if path_str.is_empty() {
        ctx.say("ERROR: Please provide a path. Usage: `.remove <path>`").await?;
        return Ok(());
    }

    let path = Path::new(path_str);

    if !path.exists() {
        ctx.say(format!("ERROR: Path not found: `{}`", path_str)).await?;
        return Ok(());
    }

    if path.is_file() {
        match fs::remove_file(path) {
            Ok(_) => {
                ctx.say(format!("SUCCESS: **File removed successfully:** `{}`", path_str)).await?;
            }
            Err(e) => {
                ctx.say(format!("ERROR: Failed to remove file `{}`: {}", path_str, e)).await?;
            }
        }
    } else if path.is_dir() {
        match fs::remove_dir_all(path) {
            Ok(_) => {
                ctx.say(format!("SUCCESS: **Directory removed successfully:** `{}`", path_str)).await?;
            }
            Err(e) => {
                ctx.say(format!("ERROR: Failed to remove directory `{}`: {}", path_str, e)).await?;
            }
        }
    } else {
        ctx.say(format!("ERROR: Path is neither a file nor a directory: `{}`", path_str)).await?;
    }

    Ok(())
}
