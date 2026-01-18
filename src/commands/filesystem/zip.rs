use crate::prelude::*;
use crate::commands::Arguments;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use walkdir::WalkDir;
use zip::ZipWriter;

#[poise::command(prefix_command)]
pub async fn zip(
    ctx: PoiseContext<'_>,
    #[description = "Source path and destination zip file"]
    #[rest]
    args: String,
) -> Result<(), Error> {
    let parsed = Arguments::parse_quoted_args(&args);
    if parsed.len() < 2 {
        ctx.say("ERROR: Usage: `.zip <source_path> <destination.zip>`").await?;
        return Ok(());
    }

    let source = &parsed[0];
    let mut dest = parsed[1].clone();
    
    if !dest.to_lowercase().ends_with(".zip") {
        dest.push_str(".zip");
    }

    let source_path = Path::new(source);
    if !source_path.exists() {
        ctx.say(format!("ERROR: Source path not found: `{}`", source)).await?;
        return Ok(());
    }

    let reply = ctx.say(format!("Creating zip archive `{}`...", dest)).await?;

    let file = File::create(&dest)?;
    let mut zip_writer = ZipWriter::new(file);
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut file_count = 0;

    if source_path.is_file() {
        let filename = source_path.file_name().unwrap_or_default().to_string_lossy();
        zip_writer.start_file(filename.to_string(), options)?;
        let mut f = File::open(source_path)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        zip_writer.write_all(&buffer)?;
        file_count = 1;
    } else {
        for entry in WalkDir::new(source_path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = path.strip_prefix(source_path).unwrap_or(path);

            if path.is_file() {
                zip_writer.start_file(name.to_string_lossy().replace('\\', "/"), options)?;
                let mut f = File::open(path)?;
                let mut buffer = Vec::new();
                f.read_to_end(&mut buffer)?;
                zip_writer.write_all(&buffer)?;
                file_count += 1;
            }
        }
    }

    zip_writer.finish()?;

    let zip_size = std::fs::metadata(&dest)?.len();
    reply.edit(ctx, poise::CreateReply::default()
        .content(format!("Created `{}` ({} files, {:.2} KB)", dest, file_count, zip_size as f64 / 1024.0))).await?;

    Ok(())
}
