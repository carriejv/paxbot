use rust_fuzzy_search::fuzzy_compare;

use crate::consts::*;
use crate::{RenderableEmbed, RenderableMessage, RenderableResponse};

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
    /// The default render type for this response, based on the best scoring result type.
    pub render_type: RenderType,
    /// Vec of search results for a query, sorted by score
    pub results: Vec<SearchResult>,
}

impl SearchResponse {
    /// Gets all renderable messages in sorted order, including appropriate navigation footers.
    pub fn get_renderable_response(&self) -> RenderableResponse {
        let mut messages = Vec::<RenderableMessage>::new();
        match &self.render_type {
            RenderType::Category => {
                // Categories first
                messages.append(&mut self.categories_to_renderable_messages());
                messages.append(&mut self.results_to_renderable_messages());
                messages = messages
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(index, item)| {
                        let mut new_item = item.clone();
                        if let Some(mut embed) = item.embed {
                            embed.footer = Some(self.get_footer_text(index));
                            new_item.embed = Some(embed);
                        }
                        new_item
                    })
                    .collect();
            }
            RenderType::Result => {
                // Results first
                messages.append(&mut self.results_to_renderable_messages());
                messages.append(&mut self.categories_to_renderable_messages());
                messages = messages
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(index, item)| {
                        let mut new_item = item.clone();
                        if let Some(mut embed) = item.embed {
                            embed.footer = Some(self.get_footer_text(index));
                            new_item.embed = Some(embed);
                        }
                        new_item
                    })
                    .collect();
            }
            RenderType::Guess(guess_str) => match guess_str {
                Some(best_guess) => {
                    messages = vec![RenderableMessage {
                        content: format!("No results found. Did you mean `{}`?", best_guess),
                        embed: None,
                    }]
                }
                None => {
                    messages = vec![RenderableMessage {
                        content: "No results found".to_string(),
                        embed: None,
                    }]
                }
            },
        }
        RenderableResponse { index: 0, messages }
    }

    /// Returns formatted footer text for the item at a given index.
    pub fn get_footer_text(&self, for_index: usize) -> String {
        let total_len = self.category_results.len() + self.results.len();
        if total_len > 1 {
            format!("Displaying result {} of {}. Use {} and {} to navigate.\nUse {} if paxbot found what you needed or {} if not.", for_index + 1, total_len, REACT_RESULTS_BACKWARD, REACT_RESULTS_FORWARD, REACT_FEEDBACK_GOOD, REACT_FEEDBACK_BAD)
        } else {
            format!(
                "Use {} if paxbot found what you needed or {} if not.",
                REACT_FEEDBACK_GOOD, REACT_FEEDBACK_BAD
            )
        }
    }

    /// Returns a Vec<RenderableMessage> representing the category results. This does not fill footer text.
    fn categories_to_renderable_messages(&self) -> Vec<RenderableMessage> {
        let mut renderable_categories = Vec::<RenderableMessage>::new();
        for result in &self.category_results {
            let mut item_list = result
                .members
                .iter()
                .cloned()
                .take(10)
                .collect::<Vec<String>>()
                .join("\n");
            if result.members.len() > 10 {
                item_list.push_str(&format!("\n...and {} more.", result.members.len() - 10));
            }
            renderable_categories.push(RenderableMessage {
                content: format!("Results for: `{}`", self.query),
                embed: Some(RenderableEmbed {
                    description: Some(result.text.clone()),
                    fields: Some(vec![("Category Members".to_string(), item_list, true)]),
                    footer: None,
                    title: format!("{} (Category)", &result.name),
                }),
            });
        }
        renderable_categories
    }

    /// Returns a Vec<RenderableMessage> representing the search item results. This does not fill footer text.
    fn results_to_renderable_messages(&self) -> Vec<RenderableMessage> {
        let mut renderable_results = Vec::<RenderableMessage>::new();
        for result in &self.results {
            renderable_results.push(RenderableMessage {
                content: format!("Results for: `{}`", self.query),
                embed: Some(RenderableEmbed {
                    description: Some(result.categories.join(", ")),
                    fields: Some(vec![
                        ("Information".to_string(), result.text.clone(), false),
                        ("External Links".to_string(), result.ext_links.join("\n"), false),
                    ]),
                    footer: None,
                    title: format!("{} ({})", &result.name, &result.shortname.join(", ")),
                }),
            });
        }
        renderable_results
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
                text: category_item.text.clone(),
            });
            if category_score > best_score {
                search_response.render_type = RenderType::Category;
                best_score = category_score;
            }
        } else if category_score > best_score {
            search_response.render_type = RenderType::Guess(Some(category_item.name.clone()));
            best_score = category_score;
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
        } else if item_score > best_score {
            search_response.render_type = RenderType::Guess(Some(search_item.name.clone()));
            best_score = item_score;
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
