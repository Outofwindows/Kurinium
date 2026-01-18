use crate::prelude::*;
use crate::config::Config;

#[poise::command(prefix_command, aliases("h", "commands"))]
pub async fn help(
    ctx: PoiseContext<'_>,
    #[description = "Specific command to show help about"]
    #[rest]
    command: Option<String>,
) -> Result<(), Error> {
    let prefix = Config::BOT_PREFIX;
    
    if let Some(cmd_name) = command {
        let cmd_name = cmd_name.trim();
        
        let commands = &ctx.framework().options().commands;
        if let Some(cmd) = commands.iter().find(|c| c.name == cmd_name) {
            let aliases = if cmd.aliases.is_empty() {
                "None".to_string()
            } else {
                cmd.aliases.join(", ")
            };
            
            let description = cmd.description.as_deref().unwrap_or("No description");
            
            let embed = serenity::CreateEmbed::new()
                .title(format!("Command: {}", cmd.name.to_uppercase()))
                .color(0x00FF00)
                .description(format!(
                    "**Description**\n{}\n\n**Usage**\n`{}{}`\n\n**Aliases**\n`{}`",
                    description, prefix, cmd.name, aliases
                ))
                .footer(serenity::CreateEmbedFooter::new("Kurinium Bot"));
                
            ctx.send(poise::CreateReply::default().embed(embed)).await?;
        } else {
            ctx.say(format!("Command '{}' not found.", cmd_name)).await?;
        }
    } else {
        let commands = &ctx.framework().options().commands;

        let mut description = String::new();
        description.push_str(&format!("**Prefix:** `{}`\n\n", prefix));
        description.push_str("**Available Commands**\n");
        
        for cmd in commands.iter() {
            let line = format!("`{}` - {}\n", cmd.name, cmd.description.as_deref().unwrap_or("-"));
            description.push_str(&line);
        }
        
        description.push_str(&format!("\nType `{}help <command>` for details.", prefix));

        let embed = serenity::CreateEmbed::new()
            .title("Kurinium Help")
            .color(0x0099FF)
            .description(description)
            .footer(serenity::CreateEmbedFooter::new(format!("Total Commands: {}", commands.len())))
            .timestamp(serenity::Timestamp::now());

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
    }

    Ok(())
}
