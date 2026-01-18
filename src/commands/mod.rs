pub mod core;
pub mod crypto;
pub mod filesystem;
pub mod system;
pub mod utility;
pub mod network;

use crate::prelude::*;
pub struct Arguments {
    args: Vec<String>,
    current: usize,
}

impl Arguments {
    pub fn new(input: &str) -> Self {
        let args = input.split_whitespace().map(|s| s.to_string()).collect();
        Self { args, current: 0 }
    }

    pub fn from_vec(args: Vec<String>) -> Self {
        Self { args, current: 0 }
    }

    pub fn next(&mut self) -> Option<&str> {
        if self.current < self.args.len() {
            let result = self.args[self.current].as_str();
            self.current += 1;
            Some(result)
        } else {
            None
        }
    }

    pub fn rest(&self) -> String {
        if self.current < self.args.len() {
            self.args[self.current..].join(" ")
        } else {
            String::new()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.args.is_empty() || self.current >= self.args.len()
    }

    pub fn all(&self) -> &[String] {
        &self.args
    }

    pub fn parse_quoted_args(input: &str) -> Vec<String> {
        let mut args = Vec::new();
        let mut current_arg = String::new();
        let mut in_quotes = false;
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '"' => {
                    in_quotes = !in_quotes;
                }
                ' ' if !in_quotes => {
                    if !current_arg.is_empty() {
                        args.push(current_arg.trim().to_string());
                        current_arg.clear();
                    }
                }
                _ => {
                    current_arg.push(ch);
                }
            }
        }

        if !current_arg.is_empty() {
            args.push(current_arg.trim().to_string());
        }

        args
    }
}

pub async fn send_message(ctx: PoiseContext<'_>, content: &str) -> Result<(), Error> {
    ctx.say(content).await?;
    Ok(())
}

pub async fn send_embed(
    ctx: PoiseContext<'_>,
    title: &str,
    description: &str,
    color: u32,
) -> Result<(), Error> {
    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title(title)
                .description(description)
                .color(color)
        )
    ).await?;
    Ok(())
}
