use crate::prelude::*;
use windows_volume_control::AudioController;

#[poise::command(prefix_command, aliases("vol"))]
pub async fn volume(
    ctx: PoiseContext<'_>,
    #[description = "Volume level 0-100, mute, unmute, or get"] 
    #[rest]
    action: Option<String>,
) -> Result<(), Error> {
    let action = action.unwrap_or_else(|| "".to_string()).to_lowercase();
    
    let result = tokio::task::spawn_blocking(move || {
        unsafe {
            let mut controller = AudioController::init(None);
            controller.GetSessions();
            controller.GetDefaultAudioEnpointVolumeControl();
            
            if let Some(session) = controller.get_session_by_name("master".to_string()) {
                match action.as_str() {
                    "get" => {
                        let vol = session.getVolume() * 100.0;
                        let muted = session.getMute();
                        let status = if muted { "Muted" } else { "Unmuted" };
                        Ok(format!("Volume: {:.0}%\nStatus: {}", vol, status))
                    },
                    "mute" => {
                        session.setMute(true);
                        Ok("Volume muted".to_string())
                    },
                    "unmute" => {
                        session.setMute(false);
                        Ok("Volume unmuted".to_string())
                    },
                    val => {
                         if let Ok(level_cnt) = val.parse::<f32>() {
                             let level = level_cnt.clamp(0.0, 100.0);
                             session.setVolume(level / 100.0);
                             Ok(format!("Volume set to {:.0}%", level))
                         } else {
                             Ok("Usage: `.volume <number|mute|unmute|get>`".to_string())
                         }
                    }
                }
            } else {
                Err("Failed to get master audio session".to_string())
            }
        }
    }).await?;

    match result {
        Ok(msg) => ctx.say(msg).await?,
        Err(e) => ctx.say(format!("Error: {}", e)).await?,
    };

    Ok(())
}
