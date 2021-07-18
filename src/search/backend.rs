//! This file implements the TOML file backend for search data.
//! This should probably be burned in favor of something less bad eventually.

use std::fs::read_to_string;

use serde::{Deserialize, Serialize};
use serenity::prelude::*;
use toml;

/// Full possible results fetched from the search backend.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchBackendData {
    /// Vec of all categories
    #[serde(rename = "category")]
    pub categories: Vec<SearchBackendCategory>,
    #[serde(rename = "search_result")]
    /// Vec of all search results
    pub search_results: Vec<SearchBackendItem>,
}

/// Category fetched from the search backend.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchBackendCategory {
    /// Category name
    pub name: String,
    /// Category description
    pub text: String,
}

/// Item fetched from the search backend.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchBackendItem {
    /// Category membership for item
    pub categories: Vec<String>,
    /// External links as strings. Supports markdown [pretty](url) links.
    pub ext_links: Vec<String>,
    /// Primary article name
    pub name: String,
    /// Short / abbreviated names
    pub shortname: Vec<String>,
    /// Result body text
    pub text: String,
}

pub struct SearchDataKey;

/// Reads data from the search backend, returning a [`SearchBackendData`]
/// containing the possible results.
pub fn build_search_backend() -> SearchBackendData {
    let file_data = match read_to_string("./content/content.toml") {
        Ok(file_data) => file_data,
        Err(err) => {
            panic!("Failed to load content.toml: {}", err)
        }
    };
    match toml::from_str::<SearchBackendData>(&file_data) {
        Ok(toml_data) => toml_data,
        Err(err) => panic!("Failed to parse content.toml: {}", err),
    }
}

impl TypeMapKey for SearchDataKey {
    type Value = SearchBackendData;
}
