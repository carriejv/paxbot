use std::collections::HashMap;
use std::sync::Arc;

use rust_fuzzy_search::fuzzy_compare;
use serenity::model::channel::Message;
use serenity::model::id::{ChannelId, MessageId};
use serenity::prelude::*;
use tokio::sync::Mutex;

use crate::consts::*;

pub mod backend;
use backend::SearchBackendData;

/// Best guess from a search.
#[derive(Clone, Debug, PartialEq)]
pub enum RenderType {
    /// Render category results
    Category,
    /// Render a best guess string.
    Guess(Option<String>),
    /// Render item results.
    Result,
}

/// Search result for a category
#[derive(Clone, Debug, PartialEq)]
pub struct CategoryResult {
    /// A Vec<String> of member result names.
    pub members: Vec<String>,
    /// Category name
    pub name: String,
    /// Relevance score
    pub score: f32,
    /// Text description of the category.
    pub text: String,
}

/// Search result struct
#[derive(Clone, Debug, PartialEq)]
pub struct SearchResult {
    /// Category membership for item
    pub categories: Vec<String>,
    /// External links as strings. Supports markdown [pretty](url) links.
    pub ext_links: Vec<String>,
    /// Primary article name
    pub name: String,
    /// Relevance score calculated from name and shortname matches.
    pub score: f32,
    /// Short / abbreviated names
    pub shortname: Vec<String>,
    /// Result body text
    pub text: String,
}

/// Search response (containing all relevant results).
#[derive(Clone, Debug)]
pub struct SearchResponse {
    /// Category results, sorted by score
    pub category_results: Vec<CategoryResult>,
    /// Currently rendered result index
    pub index: usize,
    /// The original query
    pub query: String,
    /// The render_type currently in use.
    pub render_type: RenderType,
    /// Vec of search results for a query, sorted by score
    pub results: Vec<SearchResult>,
}

pub struct SearchResponseKey;

pub type SearchResponseMap = HashMap<(ChannelId, MessageId), SearchResponse>;

impl TypeMapKey for SearchResponseKey {
    type Value = Arc<Mutex<SearchResponseMap>>;
}

impl SearchResponse {
    /// Edits a message, displaying a category result from a search response in it.
    pub async fn render_category_to_message(
        &mut self, index: usize, ctx: &Context, msg: &mut Message,
    ) -> Result<(), serenity::Error> {
        let result = &self.category_results[index];
        let mut item_list = &result.members.iter().cloned().take(10).collect::<Vec<String>>().join("\n");
        if result.members.len() > 10 {
            item_list.push_str(format!("\n...and {} more.", result.members.len() - 10));
        }
        msg.edit(&ctx.http, |m| {
            m.content(format!("Results for: `{}`", self.query));
            m.embed(|e| {
                e.title(format!("{} (Category)", &result.name));
                e.description(&result.text);
                e.fields(vec![
                    ("Category Members", &item_list, true),
                ]);
                let footer_text = if self.category_results.len() + self.results.len() > 1 {
                    format!("Displaying result {} of {}. Use {} and {} to navigate.\nUse {} if paxbot found what you needed or {} if not.", index + 1, self.results.len(), REACT_RESULTS_BACKWARD, REACT_RESULTS_FORWARD, REACT_FEEDBACK_GOOD, REACT_FEEDBACK_BAD)
                }
                else {
                    format!("Use {} if paxbot found what you needed or {} if not.", REACT_FEEDBACK_GOOD, REACT_FEEDBACK_BAD)
                };
                e.footer(|f| f.text(footer_text));
                e
            });
            m
        }).await?;
        self.index = index;
        Ok(())
    }

    /// Edits a message, displaying a search result from a search response in it.
    pub async fn render_result_to_message(
        &mut self, index: usize, ctx: &Context, msg: &mut Message,
    ) -> Result<(), serenity::Error> {
        let result = &self.results[index];
        msg.edit(&ctx.http, |m| {
            m.content(format!("Results for: `{}`", self.query));
            m.embed(|e| {
                e.title(format!("{} ({})", &result.name, &result.shortname.join(", ")));
                e.description(format!("Score: {}", result.score));
                e.fields(vec![
                    ("Categories", &result.categories.join("\n"), true),
                    ("Result", &result.text, true)
                ]);
                e.fields(vec![
                    ("External Links", &result.ext_links.join("\n"), false)
                ]);
                let footer_text = if self.category_results.len() + self.results.len() > 1 {
                    format!("Displaying result {} of {}. Use {} and {} to navigate.\nUse {} if paxbot found what you needed or {} if not.", index + 1, self.results.len(), REACT_RESULTS_BACKWARD, REACT_RESULTS_FORWARD, REACT_FEEDBACK_GOOD, REACT_FEEDBACK_BAD)
                }
                else {
                    format!("Use {} if paxbot found what you needed or {} if not.", REACT_FEEDBACK_GOOD, REACT_FEEDBACK_BAD)
                };
                e.footer(|f| f.text(footer_text));
                e
            });
            m
        }).await?;
        self.index = index;
        Ok(())
    }
}

/// Performs a search on a given [`SearchBackendData`].
pub async fn search(query: &str, from_data: &SearchBackendData) -> SearchResponse {
    let mut search_response = SearchResponse {
        category_results: Vec::<CategoryResult>::new(),
        index: 0,
        query: String::from(query),
        render_type: RenderType::Guess(None),
        results: Vec::<SearchResult>::new(),
    };
    let mut best_score = 0f32;
    // Search categories
    for category_item in &from_data.categories {
        let category_score = fuzzy_compare(&category_item.name.to_lowercase(), &query.to_lowercase());
        if category_score > SEARCH_SCORE_THRESHOLD {
            search_response.category_results.push(CategoryResult {
                members: from_data
                    .search_results
                    .iter()
                    .filter(|x| x.categories.contains(&category_item.name))
                    .map(|x| x.name.clone())
                    .collect::<Vec<String>>(),
                name: category_item.name.clone(),
                score: category_score,
                text: category_item.text.clone()
            });
            if category_score > best_score {
                search_response.render_type = RenderType::Category;
                best_score = category_score;
            }
        } else {
            if category_score > best_score {
                search_response.render_type = RenderType::Guess(Some(category_item.name.clone()));
                best_score = category_score;
            }
        }
    }
    // Search items
    for search_item in &from_data.search_results {
        // Get search score
        let mut item_score = 0f32;
        let mut names = search_item.shortname.iter().map(String::as_str).collect::<Vec<&str>>();
        names.push(search_item.name.as_str());
        for name in names {
            let name_score = fuzzy_compare(&name.to_lowercase(), &query.to_lowercase());
            if name_score > item_score {
                item_score = name_score;
            }
        }
        // Push good results
        if item_score > SEARCH_SCORE_THRESHOLD {
            search_response.results.push(SearchResult {
                categories: search_item.categories.clone(),
                ext_links: search_item.ext_links.clone(),
                name: search_item.name.clone(),
                score: item_score,
                shortname: search_item.shortname.clone(),
                text: search_item.text.clone(),
            });
            if item_score > best_score {
                search_response.render_type = RenderType::Result;
                best_score = item_score;
            }
        } else {
            if item_score > best_score {
                search_response.render_type = RenderType::Guess(Some(search_item.name.clone()));
                best_score = item_score;
            }
        }
    }
    search_response
        .category_results
        .sort_by(|a, b| match b.score.partial_cmp(&a.score) {
            Some(score_cmp) => score_cmp,
            None => b.name.cmp(&a.name),
        });
    search_response
        .results
        .sort_by(|a, b| match b.score.partial_cmp(&a.score) {
            Some(score_cmp) => score_cmp,
            None => b.name.cmp(&a.name),
        });
    search_response
}
