use crate::prelude::*;
use crate::utils::formatting::format_bytes as format_size;
use super::common::{NONCE_SIZE, SALT_SIZE, EXTENSION, derive_key};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[poise::command(prefix_command, aliases("enc"))]
pub async fn encrypt(
    ctx: PoiseContext<'_>,
    #[description = "Password and path"]
    #[rest]
    args: Option<String>,
) -> Result<(), Error> {
    let args = match args {
        Some(a) => a,
        None => {
            ctx.say("Usage: `.encrypt <password> <path>`").await?;
            return Ok(());
        }
    };

    let parsed = crate::commands::Arguments::parse_quoted_args(&args);
    if parsed.len() < 2 {
        ctx.say("Usage: `.encrypt <password> <path>`\nFor paths with spaces use quotes.").await?;
        return Ok(());
    }

    let password = &parsed[0];
    let target_path = &parsed[1];

    if password.len() < 4 {
        ctx.say("ERROR: Password must be at least 4 characters").await?;
        return Ok(());
    }

    let path = Path::new(target_path);
    if !path.exists() {
        ctx.say(format!("ERROR: Path not found: `{}`", target_path)).await?;
        return Ok(());
    }

    let reply = ctx.say("Encrypting...").await?;

    let files_to_encrypt: Vec<_> = if path.is_file() {
        vec![path.to_path_buf()]
    } else {
        WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file() && !e.path().to_string_lossy().ends_with(EXTENSION))
            .map(|e| e.path().to_path_buf())
            .collect()
    };

    if files_to_encrypt.is_empty() {
        reply.edit(ctx, poise::CreateReply::default()
            .content("No files to encrypt found.")).await?;
        return Ok(());
    }

    let _total = files_to_encrypt.len();
    let mut encrypted = 0;
    let mut failed = 0;
    let mut total_size: u64 = 0;

    for file_path in &files_to_encrypt {
        match encrypt_file(file_path, password) {
            Ok(size) => {
                encrypted += 1;
                total_size += size;
                let _ = fs::remove_file(file_path);
            }
            Err(_) => {
                failed += 1;
            }
        }
    }

    let size_str = format_size(total_size);
    
    let embed = serenity::CreateEmbed::new()
        .title("Encryption Complete")
        .field("Files Encrypted", encrypted.to_string(), true)
        .field("Failed", failed.to_string(), true)
        .field("Total Size", size_str, true)
        .field("Extension", format!("`{}`", EXTENSION), true)
        .footer(serenity::CreateEmbedFooter::new("Keep your password safe"))
        .color(if failed == 0 { 0x2ecc71 } else { 0xe74c3c });

    reply.edit(ctx, poise::CreateReply::default().content("").embed(embed)).await?;
    Ok(())
}

fn encrypt_file(path: &Path, password: &str) -> Result<u64, anyhow::Error> {
    let data = fs::read(path)?;
    
    let mut salt = [0u8; SALT_SIZE];
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce_bytes);

    let key = derive_key(password, &salt);
    
    let cipher = Aes256Gcm::new_from_slice(&key)?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let encrypted = cipher.encrypt(nonce, data.as_ref())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

    let mut output = Vec::with_capacity(SALT_SIZE + NONCE_SIZE + encrypted.len());
    output.extend_from_slice(&salt);
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&encrypted);

    let output_path = format!("{}{}", path.display(), EXTENSION);
    fs::write(&output_path, &output)?;

    Ok(output.len() as u64)
}

