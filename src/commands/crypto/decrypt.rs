use crate::prelude::*;
use crate::utils::formatting::format_bytes as format_size;
use super::common::{NONCE_SIZE, SALT_SIZE, EXTENSION, derive_key};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[poise::command(prefix_command, aliases("dec"))]
pub async fn decrypt(
    ctx: PoiseContext<'_>,
    #[description = "Password and path"]
    #[rest]
    args: Option<String>,
) -> Result<(), Error> {
    let args = match args {
        Some(a) => a,
        None => {
            ctx.say("Usage: `.decrypt <password> <path>`").await?;
            return Ok(());
        }
    };

    let parsed = crate::commands::Arguments::parse_quoted_args(&args);
    if parsed.len() < 2 {
        ctx.say("Usage: `.decrypt <password> <path>`\nFor paths with spaces use quotes.").await?;
        return Ok(());
    }

    let password = &parsed[0];
    let target_path = &parsed[1];

    let path = Path::new(target_path);
    if !path.exists() {
        ctx.say(format!("ERROR: Path not found: `{}`", target_path)).await?;
        return Ok(());
    }

    let reply = ctx.say("Decrypting...").await?;

    let files_to_decrypt: Vec<_> = if path.is_file() {
        if path.to_string_lossy().ends_with(EXTENSION) {
            vec![path.to_path_buf()]
        } else {
            reply.edit(ctx, poise::CreateReply::default()
                .content(format!("ERROR: File must have `{}` extension", EXTENSION))).await?;
            return Ok(());
        }
    } else {
        WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file() && e.path().to_string_lossy().ends_with(EXTENSION))
            .map(|e| e.path().to_path_buf())
            .collect()
    };

    if files_to_decrypt.is_empty() {
        reply.edit(ctx, poise::CreateReply::default()
            .content(format!("No `{}` files found to decrypt.", EXTENSION))).await?;
        return Ok(());
    }

    let _total = files_to_decrypt.len();
    let mut decrypted = 0;
    let mut failed = 0;
    let mut total_size: u64 = 0;

    for file_path in &files_to_decrypt {
        match decrypt_file(file_path, password) {
            Ok(size) => {
                decrypted += 1;
                total_size += size;
                let _ = fs::remove_file(file_path);
            }
            Err(_) => {
                failed += 1;
            }
        }
    }

    let size_str = format_size(total_size);
    
    let (color, status) = if failed == 0 { (0x2ecc71, "All files decrypted successfully") }
    else if decrypted == 0 {(0xe74c3c, "Decryption failed - wrong password?") }
    else { (0xf39c12, "Some files failed to decrypt") };

    let embed = serenity::CreateEmbed::new()
        .title("Decryption Complete")
        .description(status)
        .field("Files Decrypted", decrypted.to_string(), true)
        .field("Failed", failed.to_string(), true)
        .field("Total Size", size_str, true)
        .color(color);

    reply.edit(ctx, poise::CreateReply::default().content("").embed(embed)).await?;
    Ok(())
}

fn decrypt_file(path: &Path, password: &str) -> Result<u64, anyhow::Error> {
    let data = fs::read(path)?;
    if data.len() < SALT_SIZE + NONCE_SIZE + 16 {
        return Err(anyhow::anyhow!("File too small or corrupted"));
    }

    let salt = &data[..SALT_SIZE];
    let nonce_bytes = &data[SALT_SIZE..SALT_SIZE + NONCE_SIZE];
    let ciphertext = &data[SALT_SIZE + NONCE_SIZE..];

    let key = derive_key(password, salt);
    
    let cipher = Aes256Gcm::new_from_slice(&key)?;
    let nonce = Nonce::from_slice(nonce_bytes);
    
    let decrypted = cipher.decrypt(nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("Decryption failed: wrong password or corrupted"))?;

    let path_str = path.to_string_lossy();
    let output_path = path_str.trim_end_matches(EXTENSION);
    
    fs::write(output_path, &decrypted)?;

    Ok(decrypted.len() as u64)
}

