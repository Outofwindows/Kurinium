use crate::prelude::*;
use crate::commands::Arguments;
use std::fs::File;
use std::path::Path;

#[poise::command(prefix_command)]
pub async fn unzip(
    ctx: PoiseContext<'_>,
    #[description = "Zip file and destination path"]
    #[rest]
    args: String,
) -> Result<(), Error> {
    let parsed = Arguments::parse_quoted_args(&args);
    
    if parsed.is_empty() {
        ctx.say("ERROR: Usage: `.unzip <archive.zip> [destination]`").await?;
        return Ok(());
    }

    let archive_path = &parsed[0];
    let dest = if parsed.len() > 1 { &parsed[1] } else { "." };

    let path = Path::new(archive_path);
    if !path.exists() {
        ctx.say(format!("ERROR: Archive not found: `{}`", archive_path)).await?;
        return Ok(());
    }

    let reply = ctx.say(format!("Extracting `{}`...", archive_path)).await?;

    let file = File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let file_count = archive.len();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = Path::new(dest).join(file.mangled_name());

        if file.is_dir() {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p)?;
                }
            }
            let mut outfile = File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }

    reply.edit(ctx, poise::CreateReply::default()
        .content(format!("Extracted {} files to `{}`", file_count, dest))).await?;

    Ok(())
}
