use serenity::prelude::*;
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandGroup, CommandOptions, CommandResult,
    },
    model::channel::Message,
};

use crate::consts::*;

/// Container for non-admin-restricted utility commands.
#[group]
#[prefix = "!pax"]
#[commands(about, help)]
pub struct CmdUtil;

/// Helper function that can be used to print custom help text. Called by other commands.
pub async fn print_help(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, r#"
```text
*Search Commands*
?pax                    Prints this help message.
?pax [query]            Searches the paxbot tip database, returning any relevant results.

*Utility Commands*
?!pax about             Prints information about bot version, stats, and how to contribute.
?!pax diag              Prints system diagnostic information.
?!pax help              Prints this help message.

*Server Admin Commands*
?!pax chan [channel]    Sets paxbot to only listen in the mentioned channel.
```
    "#).await?;
    Ok(())
}

#[command]
async fn about(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.send_message(&ctx.http, |m| {
        m.embed(|e| {
            e.title(format!("Paxbot v{}", PAXBOT_VERSION));
            e.description(format!("`?pax` for help."));
            e.fields(vec![
                ("Users", "TODO", true),
                ("Servers", "TODO", true),
                ("Searches", "TODO", true),
            ]);
            e.fields(vec![
                ("Contribute Code", "https://github.com/carriejv/paxbot", true),
                ("Contribute Tips", "https://github.com/carriejv/paxbot/content", true),
            ]);
            e.fields(vec![
                ("Maintainers", "Kali Liada @ Exodus", false),
            ]);
            e
        });
        m
    }).await?;

    Ok(())
}

#[command]
async fn help(ctx: &Context, msg: &Message) -> CommandResult {
    print_help(&ctx, &msg).await
}
