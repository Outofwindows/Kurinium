use crate::prelude::*;
use nokhwa::{
    utils::{CameraIndex, RequestedFormat, RequestedFormatType},
    Camera,
};
use std::io::Cursor;
use image::ImageFormat;

#[poise::command(prefix_command, aliases("cam"))]
pub async fn webcam(
    ctx: PoiseContext<'_>,
    #[description = "Camera index (default: 0)"] index_arg: Option<String>,
) -> Result<(), Error> {
    let index = match index_arg {
        Some(s) => {
            if let Ok(num) = s.parse::<u32>() {
                num
            } else {
                ctx.say("Usage: `.webcam [index]` (e.g., .webcam 0)").await?;
                return Ok(());
            }
        },
        None => 0,
    };
    
    let reply = ctx.say("Accessing webcam...").await?;
    
    let result = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, anyhow::Error> {
        let camera_index = CameraIndex::Index(index);
        let requested = RequestedFormat::new::<nokhwa::pixel_format::RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
        
        let mut camera = Camera::new(camera_index, requested)
            .map_err(|e| anyhow::anyhow!("Failed to connect to camera: {}", e))?;
            
        camera.open_stream()
            .map_err(|e| anyhow::anyhow!("Failed to open stream: {}", e))?;
            
        let frame = camera.frame()
            .map_err(|e| anyhow::anyhow!("Failed to capture frame: {}", e))?;
            
        camera.stop_stream().ok();
        let buffer = frame.decode_image::<nokhwa::pixel_format::RgbFormat>()
            .map_err(|e| anyhow::anyhow!("Failed to decode frame: {}", e))?;
            
        let mut jpeg_data = Vec::new();
        let mut cursor = Cursor::new(&mut jpeg_data);
        buffer.write_to(&mut cursor, ImageFormat::Jpeg)
            .map_err(|e| anyhow::anyhow!("Failed to encode image: {}", e))?;
            
        Ok(jpeg_data)
    }).await?;
    
    match result {
        Ok(data) => {
            reply.delete(ctx).await?;
            
            let attachment = serenity::CreateAttachment::bytes(data, "webcam.jpg");
            ctx.send(poise::CreateReply::default()
                .content(format!("Webcam Capture (Index: {})", index))
                .attachment(attachment)
            ).await?;
        }
        Err(e) => {
            reply.edit(ctx, poise::CreateReply::default()
                .content(format!("Webcam Error: {}", e))).await?;
        }
    }

    Ok(())
}
