use std::{
    collections::{HashMap, HashSet},
    env,
    sync::Arc,
};

use serenity::prelude::*;
use serenity::{
    async_trait,
    client::bridge::gateway::{ShardId, ShardManager},
    framework::standard:: StandardFramework,
    http::Http,
    model::{
        channel::{Message, Reaction, ReactionType},
        gateway::Ready,
        id::{ChannelId, MessageId},
        permissions::Permissions,
    }
};
use tokio::sync::Mutex;

mod consts;
use consts::{REACT_RESULTS_BACKWARD, REACT_RESULTS_FORWARD};

mod commands;
use commands::ask::CMDASK_GROUP;
use commands::util::CMDUTIL_GROUP;

mod search;
use search::backend::{build_search_backend, SearchDataKey};

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
        let mut response_map = response_data
            .get::<RenderableResponseKey>()
            .expect("Could not fetch renderable response map.")
            .lock()
            .await;
        let response_key = (reaction.channel_id, reaction.message_id);
        // Ignore reactions to posts that don't have search cache
        if let Some(render_response) = response_map.get_mut(&response_key) {
            // Get a message handle
            let mut msg = match ctx
                .http
                .get_message(*reaction.channel_id.as_u64(), *reaction.message_id.as_u64())
                .await
            {
                Ok(msg) => msg,
                Err(err) => {
                    eprintln!("Failed to get message handle for a reaction. {}", err);
                    return;
                }
            };
            // Get new index. TODO: clean this mess up
            let new_index = if reaction.emoji == react_back {
                if render_response.index > 0 {
                    render_response.index - 1
                } else {
                    render_response.messages.len() - 1
                }
            } else if reaction.emoji == react_fwd {
                if render_response.index < render_response.messages.len() - 1 {
                    render_response.index + 1
                } else {
                    0
                }
            } else {
                return;
            };
            // Render changes
            match render_response.render(new_index, &ctx, &mut msg).await {
                Ok(()) => (),
                Err(err) => eprintln!("Failed to edit a message. {}", err),
            };
            // Delete navigation reactions.
            match reaction.delete(ctx.http).await {
                Ok(()) => (),
                Err(err) => eprintln!("Failed to cull a reaction. {}", err),
            };
        }
    }
}

/// Defines data that can be rendered to an embed message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderableEmbed {
    /// Embed description
    pub description: Option<String>,
    /// Fields as (title, content, inline) tuples
    pub fields: Option<Vec<(String, String, bool)>>,
    /// Embed footer
    pub footer: Option<String>,
    /// Embed title
    pub title: String,
}

/// Defines data that can be rendered to a message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderableMessage {
    /// Message content.
    pub content: String,
    /// Embed content.
    pub embed: Option<RenderableEmbed>,
}

/// Contains an entire renderable response that can be navigated through.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderableResponse {
    /// Currently rendered index.
    index: usize,
    /// Vec of [`RenderableMessage`]s.
    messages: Vec<RenderableMessage>,
}

impl RenderableResponse {
    /// Edits an existing message, displaying the [`RenderableMessage`] from [`self.messages`] at a specific index in it.
    pub async fn render(&mut self, index: usize, ctx: &Context, msg: &mut Message) -> Result<(), serenity::Error> {
        let message = &self.messages[index];
        msg.edit(&ctx.http, |m| {
            m.content(&message.content);
            if let Some(embed) = &message.embed {
                m.embed(|e| {
                    e.title(&embed.title);
                    if let Some(desc) = &embed.description {
                        e.description(desc);
                    }
                    if let Some(fields) = embed.fields.clone() {
                        e.fields(fields);
                    }
                    if let Some(footer_text) = &embed.footer {
                        e.footer(|f| f.text(footer_text));
                    }
                    e
                });
            }
            m
        })
        .await?;
        self.index = index;
        Ok(())
    }
}

pub struct RenderableResponseKey;

pub type RenderableResponseMap = HashMap<(ChannelId, MessageId), RenderableResponse>;

impl TypeMapKey for RenderableResponseKey {
    type Value = Arc<Mutex<RenderableResponseMap>>;
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
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    // Build command framework
    let framework = StandardFramework::new()
        .configure(|c| {
            c.with_whitespace(true)
                .on_mention(Some(bot_id))
                .prefix("?")
                .delimiters(vec![",", " "])
                .owners(owners)
        })
        .group(&CMDASK_GROUP)
        .group(&CMDUTIL_GROUP);

    // Build search backend
    let search_data = build_search_backend();

    // Start client
    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .framework(framework)
        .type_map_insert::<SearchDataKey>(search_data)
        .type_map_insert::<RenderableResponseKey>(Arc::new(Mutex::new(RenderableResponseMap::new())))
        .await
        .expect("Err creating client");
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
