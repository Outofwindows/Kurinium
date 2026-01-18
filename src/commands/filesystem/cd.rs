use crate::prelude::*;
use std::env;
use std::path::Path;

#[poise::command(prefix_command, aliases("chdir"))]
pub async fn cd(
    ctx: PoiseContext<'_>,
    #[description = "Directory path"]
    #[rest]
    dir_path: Option<String>,
) -> Result<(), Error> {
    let dir_path = dir_path
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty());

    let Some(dir_path) = dir_path else {
        let current_dir = env::current_dir().unwrap_or_else(|_| Path::new("unknown").to_path_buf());
        ctx.say(format!("**Current directory:** `{}`", current_dir.display())).await?;
        return Ok(());
    };

    let path = Path::new(&dir_path);

    match env::set_current_dir(path) {
        Ok(_) => {
            let new_dir = env::current_dir().unwrap_or_else(|_| Path::new("unknown").to_path_buf());
            ctx.say(format!("SUCCESS: **Changed directory to:** `{}`", new_dir.display())).await?;
        }
        Err(e) => {
            ctx.say(format!("ERROR: Failed to change directory to `{}`: {}", dir_path, e)).await?;
        }
    }

    Ok(())
}