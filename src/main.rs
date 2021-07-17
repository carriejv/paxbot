use std::{
    collections::{HashMap, HashSet},
    env,
    fmt::Write,
    sync::Arc,
};

use serenity::prelude::*;
use serenity::{
    async_trait,
    client::bridge::gateway::{ShardId, ShardManager},
    framework::standard::{
        buckets::{LimitedFor, RevertBucket},
        help_commands,
        macros::{check, command, group, help, hook},
        Args,
        CommandGroup,
        CommandOptions,
        CommandResult,
        DispatchError,
        HelpOptions,
        Reason,
        StandardFramework,
    },
    http::Http,
    model::{
        channel::{Channel, Message, Reaction, ReactionType},
        gateway::Ready,
        id::UserId,
        permissions::Permissions,
    },
    utils::{content_safe, ContentSafeOptions},
};
use tokio::sync::Mutex;

mod consts;
use consts::{REACT_RESULTS_BACKWARD,REACT_RESULTS_FORWARD};

mod commands;
use commands::ask::{CMDASK_GROUP};
use commands::util::{CmdUtil,CMDUTIL_GROUP};

mod search;
use search::{SearchResponseKey,SearchResponseMap};
use search::backend::{SearchDataKey,build_search_backend};

struct ShardManagerContainer;
impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct CommandCounter;

impl TypeMapKey for CommandCounter {
    type Value = HashMap<String, u64>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        // Ignore own reactions
        if reaction.user_id == Some(ctx.cache.current_user_id().await) {
            return;
        }
        // Ignore reactions that aren't navigation.
        let react_back = ReactionType::Unicode(String::from(REACT_RESULTS_BACKWARD));
        let react_fwd = ReactionType::Unicode(String::from(REACT_RESULTS_FORWARD));
        if reaction.emoji != react_back && reaction.emoji != react_fwd {
            return;
        }
        // Get search cache
        let response_data = ctx.data.write().await;
        let mut response_map = response_data.get::<SearchResponseKey>().expect("Could not fetch search response cache.").lock().await;
        let response_key = (reaction.channel_id, reaction.message_id);
        // Ignore reactions to posts that don't have search cache
        if let Some(search_response) = response_map.get_mut(&response_key) {
            // Get a message handle
            let mut msg = match ctx.http.get_message(*reaction.channel_id.as_u64(), *reaction.message_id.as_u64()).await {
                Ok(msg) => msg,
                Err(err) => { 
                    eprintln!("Failed to get message handle for a reaction. {}", err);
                    return
                }
            };
            // Get new index. TODO: clean this mess up
            let new_index = if reaction.emoji == react_back {
                if search_response.index > 0 { search_response.index - 1 } else { search_response.results.len() - 1 }
            }
            else if reaction.emoji == react_fwd {
                if search_response.index < search_response.results.len() - 1 { search_response.index + 1 } else { 0 }
            }
            else {
                return;
            };
            // Render changes
            match search_response.render_result_to_message(new_index, &ctx, &mut msg).await {
                Ok(()) => (),
                Err(err) => eprintln!("Failed to edit a message. {}", err)
            };
            // Delete navigation reactions.
            match reaction.delete(ctx.http).await {
                Ok(()) => (),
                Err(err) => eprintln!("Failed to cull a reaction. {}", err)
            };
        }
    }
}

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("Set DISCORD_TOKEN to authenticate to discord.");
    let http = Http::new_with_token(&token);

    // Set up global owners
    let (owners, bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            if let Some(team) = info.team {
                owners.insert(team.owner_user_id);
            } else {
                owners.insert(info.owner.id);
            }
            match http.get_current_user().await {
                Ok(bot_id) => (owners, bot_id.id),
                Err(why) => panic!("Could not access the bot id: {:?}", why),
            }
        },
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    // Build command framework
    let framework = StandardFramework::new().configure(|c| c
        .with_whitespace(true)
        .on_mention(Some(bot_id))
        .prefix("?")
        .delimiters(vec![",", " "])
        .owners(owners))
        .group(&CMDASK_GROUP)
        .group(&CMDUTIL_GROUP);

    // Build search backend
    let search_data = search::backend::build_search_backend();

    // Start client
    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .framework(framework)
        .type_map_insert::<SearchDataKey>(search_data)
        .type_map_insert::<SearchResponseKey>(Arc::new(Mutex::new(SearchResponseMap::new())))
        .await
        .expect("Err creating client");
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
