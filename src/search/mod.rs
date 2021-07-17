use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize,Serialize};
use serenity::prelude::*;
use serenity::model::channel::Message;
use serenity::model::id::{ChannelId, MessageId};
use tokio::sync::Mutex;

use crate::consts::*;

/// Search result struct
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchResult {
    /// Category membership for item
    pub categories: Vec<String>,
    /// External links as tuples (texty, url)
    pub ext_links: Vec<(String, String)>,
    /// Primary article name
    pub name: String,
    /// Relevance score calculated from name, shortname, and text matches.
    pub score: f32,
    /// Short / abbreviated names
    pub shortname: Vec<String>,
    /// Result body text
    pub text: String,
}

/// Search response (containing all relevant results).
pub struct SearchResponse {
    /// Currently rendered result index
    pub index: usize,
    /// Vec of search results for a query
    pub results: Vec<SearchResult>,
}

pub struct SearchResponseKey;

pub type SearchResponseMap = HashMap<(ChannelId, MessageId), SearchResponse>;

impl TypeMapKey for SearchResponseKey {
    type Value = Arc<Mutex<SearchResponseMap>>;
}

impl SearchResponse {
    /// Edits a message, displaying a search result from a search response in it.
    pub async fn render_result_to_message(&mut self, index: usize, ctx: &Context, msg: &mut Message) -> Result<(), serenity::Error> {
        let result = &self.results[index];
        msg.edit(&ctx.http, |m| {
            m.content("sup dawg?");
            m.embed(|e| {
                e.title(format!("{} ({})", &result.name, &result.shortname.join(", ")));
                e.description("Description here?");
                e.fields(vec![
                    ("Categories", &result.categories.iter().map(|x| format!("{}\n", x)).collect::<String>(), true),
                    ("Result", &result.text, true)
                ]);
                e.fields(vec![
                    ("External Links", &result.ext_links.iter().map(|x| format!("[{}]({})\n", x.0, x.1)).collect::<String>(), false)
                ]);
                e.footer(|f| f.text(format!("Displaying result {} of {}. Use {} and {} to navigate.\nUse {} if you found this result helpful, or {} if not to let paxbot know.", index + 1, self.results.len(), REACT_RESULTS_BACKWARD, REACT_RESULTS_FORWARD, REACT_FEEDBACK_GOOD, REACT_FEEDBACK_BAD)));
                e
            });
            m
        }).await?;
        self.index = index;
        Ok(())
    }
}

pub async fn search(query: &str) -> SearchResponse {
    SearchResponse {
        index: 0,
        results: vec![
            SearchResult {
                categories: vec!["Awesome People".to_string()],
                ext_links: vec![("A website!".to_string(), "http://example.com".to_string())],
                name: "Kali Liada".to_string(),
                score: 0.85,
                shortname: vec!["Kali".to_string(), "Marz".to_string()],
                text: "Yep, I'm me.".to_string()
            },
            SearchResult {
                categories: vec!["Awesome People".to_string()],
                ext_links: vec![("Another website!".to_string(), "https://google.com".to_string())],
                name: "Nori Durnin".to_string(),
                score: 0.81,
                shortname: vec!["Nori".to_string()],
                text: "Yep, I'm cute.".to_string()
            }
        ]
    }
}