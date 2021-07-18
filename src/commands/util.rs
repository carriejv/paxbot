use serenity::prelude::*;
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandGroup, CommandOptions, CommandResult,
    },
    model::channel::Message,
};

/// Container for non-admin-restricted utility commands.
#[group]
#[prefix = "!pax"]
#[commands(about)]
pub struct CmdUtil;

#[command]
async fn about(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id
        .say(&ctx.http, "Yup I'm paxbot and somehow I compiled.")
        .await?;

    Ok(())
}
