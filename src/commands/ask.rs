use std::cmp::Ordering;

use serenity::prelude::*;
use serenity::{
    framework::standard::{
        Args,
        CommandGroup,
        CommandOptions,
        CommandResult,
        macros::{command, group},
    },
    model::{
        channel::{Channel, Message, ReactionType},
        gateway::Ready,
        id::UserId,
        permissions::Permissions,
    }
};

use crate::consts::*;
use crate::search::{SearchResponse, SearchResponseKey, SearchResponseMap, SearchResult, search};

/// Container for the primary query command.
#[group]
#[commands(ask)]
pub struct CmdAsk;

#[command("pax")] // This results in ?pax being read as the command, with the rest being args
async fn ask(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let search_query = args.rest();
    if search_query == "" {
        msg.channel_id.say(&ctx.http, "Usage: `?pax your search here`").await?;
        return Ok(());
    }
    // Post result container --- this will get edited when response arrives.
    let reply_msg = msg.channel_id.send_message(&ctx.http, |m| {
        m.embed(|e| {
            e.title("Searching...");
            e.description("🔍");
            e
        });
        m
    }).await?;
    // Do a search
    let mut search_response = search(&search_query).await;
    match search_response.results.len().cmp(&1) {
        Ordering::Less => {
            msg.channel_id.say(&ctx.http, "TODO: Suggestions here.").await?;
            return Ok(())
        }
        Ordering::Equal => {
            ()
        },
        Ordering::Greater => {
            // Set up navigation reactions
            reply_msg.react(&ctx.http, ReactionType::Unicode(String::from(REACT_RESULTS_BACKWARD))).await?;
            reply_msg.react(&ctx.http, ReactionType::Unicode(String::from(REACT_RESULTS_FORWARD))).await?;
        }
    }
    // Get a mutable message handle and render to it
    let mut editable_msg = ctx.http.get_message(*reply_msg.channel_id.as_u64(), *reply_msg.id.as_u64()).await?;
    search_response.render_result_to_message(0, &ctx, &mut editable_msg).await?;
    // Set up feedback reactions
    reply_msg.react(&ctx.http, ReactionType::Unicode(String::from(REACT_FEEDBACK_GOOD))).await?;
    reply_msg.react(&ctx.http, ReactionType::Unicode(String::from(REACT_FEEDBACK_BAD))).await?;
    // Write context data
    let mut ctx_data = ctx.data.write().await;
    let resp_map = ctx_data.get_mut::<SearchResponseKey>().expect("Failed to get search response map.");
    resp_map.lock().await.insert((reply_msg.channel_id, reply_msg.id), search_response);
    Ok(())
}

