use crate::prelude::*;
use crate::commands::Arguments;
use std::fs;
use std::path::Path;

#[poise::command(prefix_command, aliases("mv", "ren"))]
pub async fn rename(
    ctx: PoiseContext<'_>,
    #[description = "Old and new paths (use quotes for spaces)"]
    #[rest]
    args: String,
) -> Result<(), Error> {
    let parsed_args = Arguments::parse_quoted_args(&args);

    if parsed_args.len() < 2 {
        ctx.say("ERROR: Please provide both old and new paths. Usage: `.rename <old_path> <new_path>`\nFor paths with spaces use quotes: `.rename \"old file.txt\" \"new file.txt\"`").await?;
        return Ok(());
    }

    let old_path = &parsed_args[0];
    let new_path = &parsed_args[1];

    let old_path_obj = Path::new(old_path);
    let new_path_obj = Path::new(new_path);

    if !old_path_obj.exists() {
        ctx.say(format!("ERROR: Source path not found: `{}`", old_path)).await?;
        return Ok(());
    }

    if new_path_obj.exists() {
        ctx.say(format!("ERROR: Destination path already exists: `{}`", new_path)).await?;
        return Ok(());
    }

    match fs::rename(old_path_obj, new_path_obj) {
        Ok(_) => {
            ctx.say(format!("SUCCESS: **Renamed successfully:**\n**From:** `{}`\n**To:** `{}`", old_path, new_path)).await?;
        }
        Err(e) => {
            ctx.say(format!("ERROR: Failed to rename `{}` to `{}`: {}", old_path, new_path, e)).await?;
        }
    }

    Ok(())
}
