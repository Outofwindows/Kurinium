use crate::prelude::*;
use sysinfo::{System, Disks};
use std::env;

#[poise::command(prefix_command, aliases("about", "sysinfo"))]
pub async fn info(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let reply = ctx.say("Gathering system info...").await?;

    let result = tokio::task::spawn_blocking(|| {
        let mut sys = System::new_all();
        sys.refresh_all();

        let hostname = System::host_name().unwrap_or("Unknown".to_string());
        let os = System::name().unwrap_or("Windows".to_string());
        let os_version = System::os_version().unwrap_or_default();
        let kernel = System::kernel_version().unwrap_or_default();
        let username = env::var("USERNAME").unwrap_or("Unknown".to_string());

        let cpu_name = sys.cpus().first()
            .map(|c| c.brand().to_string())
            .unwrap_or("Unknown".to_string());
        let cpu_cores = sys.cpus().len();
        let cpu_usage: f32 = sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / cpu_cores as f32;

        let total_mem = sys.total_memory();
        let used_mem = sys.used_memory();
        let mem_percent = (used_mem as f64 / total_mem as f64 * 100.0) as u32;

        let disks = Disks::new_with_refreshed_list();
        let mut disk_info = String::new();
        for disk in disks.iter() {
            let total = disk.total_space();
            let available = disk.available_space();
            let used = total - available;
            let percent = (used as f64 / total as f64 * 100.0) as u32;
            let mount = disk.mount_point().to_string_lossy();
            disk_info.push_str(&format!(
                "{}: {:.1}GB / {:.1}GB ({}%)\n",
                mount,
                used as f64 / 1024.0 / 1024.0 / 1024.0,
                total as f64 / 1024.0 / 1024.0 / 1024.0,
                percent
            ));
        }
        if disk_info.is_empty() {
            disk_info = "No disks found".to_string();
        }

        let uptime_secs = System::uptime();
        let hours = uptime_secs / 3600;
        let mins = (uptime_secs % 3600) / 60;
        let uptime_str = format!("{}h {}m", hours, mins);

        let process_count = sys.processes().len();
        let cwd = env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or("Unknown".to_string());

        (
            hostname, username, os, os_version, kernel,
            cpu_name, cpu_cores, cpu_usage,
            total_mem, used_mem, mem_percent,
            disk_info, uptime_str, process_count, cwd
        )
    }).await;

    let (
        hostname, username, os, os_version, kernel,
        cpu_name, cpu_cores, cpu_usage,
        total_mem, used_mem, mem_percent,
        disk_info, uptime_str, process_count, cwd
    ) = match result {
        Ok(r) => r,
        Err(e) => {
            reply.edit(ctx, poise::CreateReply::default()
                .content(format!("Error: {}", e))).await?;
            return Ok(());
        }
    };

    let embed = serenity::CreateEmbed::new()
        .title(format!("{} @ {}", username, hostname))
        .field("OS", format!("{} {}", os, os_version), true)
        .field("Kernel", kernel, true)
        .field("Uptime", uptime_str, true)
        .field("CPU", format!("{}\n{} cores, {:.1}%", cpu_name, cpu_cores, cpu_usage), false)
        .field("Memory", format!(
            "{:.1} GB / {:.1} GB ({}%)",
            used_mem as f64 / 1024.0 / 1024.0 / 1024.0,
            total_mem as f64 / 1024.0 / 1024.0 / 1024.0,
            mem_percent
        ), true)
        .field("Processes", process_count.to_string(), true)
        .field("Disks", format!("```\n{}```", disk_info.trim()), false)
        .field("CWD", format!("`{}`", cwd), false)
        .color(0x5865F2)
        .footer(serenity::CreateEmbedFooter::new("Kurinium"));

    reply.edit(ctx, poise::CreateReply::default().content("").embed(embed)).await?;

    Ok(())
}
