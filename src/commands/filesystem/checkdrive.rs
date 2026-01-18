use crate::prelude::*;
use sysinfo::Disks;

#[poise::command(prefix_command, aliases("drives", "disks"))]
pub async fn checkdrive(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let disks = Disks::new_with_refreshed_list();

    let mut fields = Vec::new();

    for disk in disks.iter() {
        let total = disk.total_space();
        let available = disk.available_space();
        let used = total - available;
        let usage_pct = if total > 0 { (used as f64 / total as f64) * 100.0 } else { 0.0 };

        let filled = (usage_pct / 10.0).round() as usize;
        let empty = 10 - filled;
        let bar = format!("{}{}",
            "█".repeat(filled),
            "░".repeat(empty)
        );

        let info = format!(
            "{} **{:.1}%**\n`{:.1} GB` / `{:.1} GB` free: `{:.1} GB`",
            bar,
            usage_pct,
            used as f64 / (1024.0 * 1024.0 * 1024.0),
            total as f64 / (1024.0 * 1024.0 * 1024.0),
            available as f64 / (1024.0 * 1024.0 * 1024.0)
        );

        fields.push((
            format!("💾 {} ({})", 
                disk.mount_point().display(),
                disk.file_system().to_string_lossy()
            ),
            info,
            false
        ));
    }

    if fields.is_empty() {
        ctx.say("*No drives detected*").await?;
        return Ok(());
    }

    let embed = serenity::CreateEmbed::new()
        .title("Disk Usage")
        .fields(fields)
        .color(0x9b59b6);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
