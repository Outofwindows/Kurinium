use crate::prelude::*;
use crate::commands::Arguments;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

const UNRAR_URL: &str = "https://github.com/Mikasuru/Arc/raw/refs/heads/main/Assets/Scripts/UnRAR.exe";

fn get_unrar_dir() -> PathBuf {
    let local_low = env::var("LOCALAPPDATA")
        .map(|p| PathBuf::from(p).parent().unwrap_or(&PathBuf::from("C:\\Users")).join("LocalLow"))
        .unwrap_or_else(|_| PathBuf::from("C:\\Users\\Public\\LocalLow"));
    local_low.join("UnRAR")
}

fn get_unrar_path() -> PathBuf {
    get_unrar_dir().join("UnRAR.exe")
}

async fn ensure_unrar() -> Result<PathBuf, anyhow::Error> {
    let unrar_path = get_unrar_path();
    
    if unrar_path.exists() {
        return Ok(unrar_path);
    }

    let unrar_dir = get_unrar_dir();
    if !unrar_dir.exists() {
        fs::create_dir_all(&unrar_dir)?;
    }

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .timeout(Duration::from_secs(120))
        .build()?;

    let response = client.get(UNRAR_URL).send().await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Download failed with status: {}", response.status()));
    }

    let bytes = response.bytes().await?;
    let mut file = fs::File::create(&unrar_path)?;
    file.write_all(&bytes)?;

    Ok(unrar_path)
}

#[poise::command(prefix_command)]
pub async fn unrar(
    ctx: PoiseContext<'_>,
    #[description = "RAR file and optional destination"]
    #[rest]
    args: Option<String>,
) -> Result<(), Error> {
    let args = match args {
        Some(a) => a,
        None => {
            ctx.say("Usage: `.unrar <archive.rar> [destination]`\nWith password: `.unrar <password> <archive.rar> [destination]`").await?;
            return Ok(());
        }
    };

    let parsed = Arguments::parse_quoted_args(&args);
    if parsed.is_empty() {
        ctx.say("Usage: `.unrar <archive.rar> [destination]`").await?;
        return Ok(());
    }

    let (password, archive_path, dest) = if parsed.len() >= 2 && !Path::new(&parsed[0]).exists() && Path::new(&parsed[1]).exists() {
        let dest = if parsed.len() > 2 { parsed[2].clone() } else { ".".to_string() };
        (Some(parsed[0].clone()), parsed[1].clone(), dest)
    } else {
        let dest = if parsed.len() > 1 { parsed[1].clone() } else { ".".to_string() };
        (None, parsed[0].clone(), dest)
    };

    let path = Path::new(&archive_path);
    if !path.exists() {
        ctx.say(format!("ERROR: Archive not found: `{}`", archive_path)).await?;
        return Ok(());
    }

    let reply = ctx.say("Checking UnRAR...").await?;

    let unrar_path = match ensure_unrar().await {
        Ok(p) => p,
        Err(e) => {
            reply.edit(ctx, poise::CreateReply::default()
                .content(format!("Failed to get UnRAR: {}", e))).await?;
            return Ok(());
        }
    };

    reply.edit(ctx, poise::CreateReply::default()
        .content(format!("Extracting `{}`...", archive_path))).await?;

    let archive_path_clone = archive_path.clone();
    let dest_clone = dest.clone();
    let password_clone = password.clone();
    let unrar_path_clone = unrar_path.clone();
    
    let result = tokio::task::spawn_blocking(move || {
        extract_with_unrar(&unrar_path_clone, &archive_path_clone, &dest_clone, password_clone.as_deref())
    }).await;

    match result {
        Ok(Ok(_)) => {
            let embed = serenity::CreateEmbed::new()
                .title("Extraction Complete")
                .field("Archive", format!("`{}`", archive_path), true)
                .field("Destination", format!("`{}`", dest), false)
                .color(0x2ecc71);

            reply.edit(ctx, poise::CreateReply::default().content("").embed(embed)).await?;
        }
        Ok(Err(e)) => {
            reply.edit(ctx, poise::CreateReply::default()
                .content(format!("Extraction failed: {}", e))).await?;
        }
        Err(e) => {
            reply.edit(ctx, poise::CreateReply::default()
                .content(format!("Extraction error: {}", e))).await?;
        }
    }

    Ok(())
}

fn extract_with_unrar(unrar_path: &Path, archive_path: &str, destination: &str, password: Option<&str>) -> Result<(), anyhow::Error> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let dest_path = Path::new(destination);
    let dest_path = if dest_path.is_absolute() {
        dest_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(dest_path)
    };

    if !dest_path.exists() {
        fs::create_dir_all(&dest_path)?;
    }

    let dest_str = format!("{}\\", dest_path.display());

    let mut cmd = Command::new(unrar_path);
    cmd.arg("x")
       .arg("-y")
       .arg("-o+");

    if let Some(pwd) = password {
        cmd.arg(format!("-p{}", pwd));
    }

    cmd.arg("-idq")
       .arg(archive_path)
       .arg(&dest_str)
       .creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output()?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    if stderr.contains("password") || stdout.contains("password") {
        return Err(anyhow::anyhow!("Wrong password for RAR archive"));
    }

    Err(anyhow::anyhow!("UnRAR failed: {}", stderr))
}
