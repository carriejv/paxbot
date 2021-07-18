use serenity::prelude::*;
use serenity::{
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::channel::{Message, ReactionType},
};

use crate::consts::*;
use crate::search::{backend::SearchDataKey, search, RenderType};
use crate::RenderableResponseKey;

/// Container for the primary query command.
#[group]
#[commands(ask)]
pub struct CmdAsk;

#[command("pax")] // This results in ?pax being read as the command, with the rest being args
async fn ask(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let search_query = args.rest();
    if search_query.is_empty() {
        msg.channel_id.say(&ctx.http, "Usage: `?pax your search here`").await?;
        return Ok(());
    }
    // Post result container --- this will get edited when response arrives.
    let reply_msg = msg.channel_id.say(&ctx.http, "Searching...").await?;
    // Do a search
    let search_response = {
        let ctx_data = ctx.data.read().await;
        let search_data_ref = ctx_data.get::<SearchDataKey>().expect("Search data missing.");
        search(&search_query, search_data_ref).await
    };
    // Get a mutable message handle for rendering
    let mut editable_msg = ctx
        .http
        .get_message(*reply_msg.channel_id.as_u64(), *reply_msg.id.as_u64())
        .await?;
    // Render result
    let mut render_response = search_response.get_renderable_response();
    render_response.render(0, &ctx, &mut editable_msg).await?;
    // Set up navigation reactions
    if render_response.messages.len() > 1 {
        reply_msg
            .react(&ctx.http, ReactionType::Unicode(String::from(REACT_RESULTS_BACKWARD)))
            .await?;
        reply_msg
            .react(&ctx.http, ReactionType::Unicode(String::from(REACT_RESULTS_FORWARD)))
            .await?;
    }
    // Set up feedback reactions
    match search_response.render_type {
        RenderType::Category | RenderType::Result => {
            reply_msg
                .react(&ctx.http, ReactionType::Unicode(String::from(REACT_FEEDBACK_GOOD)))
                .await?;
            reply_msg
                .react(&ctx.http, ReactionType::Unicode(String::from(REACT_FEEDBACK_BAD)))
                .await?;
        }
        _ => (),
    }
    // Write context data
    let mut ctx_data = ctx.data.write().await;
    let resp_map = ctx_data
        .get_mut::<RenderableResponseKey>()
        .expect("Failed to get render response map.");
    resp_map
        .lock()
        .await
        .insert((reply_msg.channel_id, reply_msg.id), render_response);
    Ok(())
}
