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
use crate::search::{
    BestGuess, SearchResponse, SearchResponseKey, SearchResponseMap, SearchResult, search,
    backend::SearchDataKey
};

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
    let reply_msg = msg.channel_id.say(&ctx.http, "Searching...").await?;
    // Do a search
    let mut search_response = {
        let ctx_data = ctx.data.read().await;
        let search_data_ref = ctx_data.get::<SearchDataKey>().expect("Search data missing.");
        search(&search_query, search_data_ref).await
    };
    // Get a mutable message handle for rendering
    let mut editable_msg = ctx.http.get_message(*reply_msg.channel_id.as_u64(), *reply_msg.id.as_u64()).await?;
    match search_response.best_guess {
        BestGuess::Category => println!("TODO"),
        BestGuess::Name(guessed_name) => {
            editable_msg.suppress_embeds(&ctx.http).await?;
            match guessed_name {
                Some(best_guess) => editable_msg.edit(&ctx.http, |m| m.content(format!("No results found. Did you mean `{}`?", best_guess))).await?,
                None => editable_msg.edit(&ctx.http, |m| m.content("No results found.")).await?
            };
        },
        BestGuess::Result => {
            if search_response.results.len() > 1 {
                // Set up navigation reactions
                reply_msg.react(&ctx.http, ReactionType::Unicode(String::from(REACT_RESULTS_BACKWARD))).await?;
                reply_msg.react(&ctx.http, ReactionType::Unicode(String::from(REACT_RESULTS_FORWARD))).await?;
            }
            // Render result
            search_response.render_result_to_message(0, &ctx, &mut editable_msg).await?;
            // Set up feedback reactions
            reply_msg.react(&ctx.http, ReactionType::Unicode(String::from(REACT_FEEDBACK_GOOD))).await?;
            reply_msg.react(&ctx.http, ReactionType::Unicode(String::from(REACT_FEEDBACK_BAD))).await?;
            // Write context data
            let mut ctx_data = ctx.data.write().await;
            let resp_map = ctx_data.get_mut::<SearchResponseKey>().expect("Failed to get search response map.");
            resp_map.lock().await.insert((reply_msg.channel_id, reply_msg.id), search_response);
        }
    }
    Ok(())
}

