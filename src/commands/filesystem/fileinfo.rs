use crate::prelude::*;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

#[poise::command(prefix_command, aliases("stat"))]
pub async fn fileinfo(
    ctx: PoiseContext<'_>,
    #[description = "File path"]
    #[rest]
    file_path: String,
) -> Result<(), Error> {
    let file_path = file_path.trim();

    if file_path.is_empty() {
        ctx.say("Usage: `.fileinfo <path>`").await?;
        return Ok(());
    }

    let path = Path::new(file_path);

    if !path.exists() {
        ctx.say(format!("ERROR: Path not found: `{}`", file_path)).await?;
        return Ok(());
    }

    let metadata = fs::metadata(path)?;
    let file_type = if metadata.is_dir() { "📁 Directory" } else if metadata.is_file() { "📄 File" } else { "❓ Other" };
    let size = metadata.len();
    let readonly = metadata.permissions().readonly();

    let size_str = if size >= 1024 * 1024 * 1024 {
        format!("{:.2} GB ({} bytes)", size as f64 / (1024.0 * 1024.0 * 1024.0), size)
    } else if size >= 1024 * 1024 {
        format!("{:.2} MB ({} bytes)", size as f64 / (1024.0 * 1024.0), size)
    } else if size >= 1024 {
        format!("{:.2} KB ({} bytes)", size as f64 / 1024.0, size)
    } else {
        format!("{} bytes", size)
    };

    let mut fields = vec![
        ("Type", file_type.to_string(), true),
        ("Size", size_str, true),
        ("Read-only", if readonly { "Yes" } else { "No" }.to_string(), true),
    ];

    if let Ok(modified) = metadata.modified() {
        if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
            let secs = duration.as_secs();
            let datetime = chrono::DateTime::from_timestamp(secs as i64, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            fields.push(("Modified", datetime, true));
        }
    }

    if let Ok(created) = metadata.created() {
        if let Ok(duration) = created.duration_since(UNIX_EPOCH) {
            let secs = duration.as_secs();
            let datetime = chrono::DateTime::from_timestamp(secs as i64, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            fields.push(("Created", datetime, true));
        }
    }

    let filename = path.file_name().unwrap_or_default().to_string_lossy();

    let embed = serenity::CreateEmbed::new()
        .title(format!("{}", filename))
        .description(format!("`{}`", path.display()))
        .fields(fields.into_iter().map(|(n, v, i)| (n, v, i)))
        .color(0xe67e22);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
