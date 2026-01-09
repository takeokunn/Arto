use dioxus::document;
use dioxus::prelude::*;

use crate::state::{AppState, SearchMatch};

#[component]
pub fn SearchTab() -> Element {
    let state = use_context::<AppState>();
    let query = state.search_query.read().clone();
    let matches = state.search_matches.read().clone();
    let current = *state.search_current_index.read();

    rsx! {
        div {
            class: "search-tab",

            if let Some(q) = query {
                if !q.is_empty() {
                    // Header with query and count
                    div {
                        class: "search-tab-header",
                        span { class: "search-tab-query", "\"{q}\"" }
                        span { class: "search-tab-count", " - {matches.len()} matches" }
                    }

                    // Match list
                    if matches.is_empty() {
                        div {
                            class: "search-tab-empty",
                            "No matches found"
                        }
                    } else {
                        ul {
                            class: "search-tab-list",
                            for m in matches.iter() {
                                SearchMatchItem {
                                    match_info: m.clone(),
                                    is_current: m.index + 1 == current,
                                }
                            }
                        }
                    }
                } else {
                    // Empty query state
                    SearchTabPlaceholder {}
                }
            } else {
                // No search active
                SearchTabPlaceholder {}
            }
        }
    }
}

#[component]
fn SearchTabPlaceholder() -> Element {
    rsx! {
        div {
            class: "search-tab-placeholder",
            "Type in the search bar to find matches in this document"
        }
    }
}

#[component]
fn SearchMatchItem(match_info: SearchMatch, is_current: bool) -> Element {
    let index = match_info.index;
    let context = match_info.context.clone();
    let start = match_info.context_start;
    let end = match_info.context_end;

    // Split context into before, matched, and after parts
    // Use char boundaries to handle UTF-8 correctly
    let (before, matched, after) = split_context(&context, start, end);

    let class = if is_current {
        "search-match-item current"
    } else {
        "search-match-item"
    };

    rsx! {
        li {
            class: "{class}",
            onclick: move |_| scroll_to_match(index),

            span { class: "search-match-context", "{before}" }
            span { class: "search-match-highlight", "{matched}" }
            span { class: "search-match-context", "{after}" }
        }
    }
}

/// Split context string into (before, matched, after) parts.
/// The start/end are character indices from JavaScript (UTF-16 code units),
/// which we need to convert to byte indices for Rust string slicing.
fn split_context(context: &str, char_start: usize, char_end: usize) -> (String, String, String) {
    // Convert character indices to byte indices
    // JavaScript's string.length counts UTF-16 code units, but for BMP characters
    // (which includes most text), this equals the number of Unicode scalar values
    let byte_start = char_index_to_byte_index(context, char_start);
    let byte_end = char_index_to_byte_index(context, char_end);

    let before = &context[..byte_start];
    let matched = &context[byte_start..byte_end];
    let after = &context[byte_end..];

    (before.to_string(), matched.to_string(), after.to_string())
}

/// Convert a character index to a byte index in a UTF-8 string.
/// Returns the byte position of the nth character, or the string length if n exceeds char count.
fn char_index_to_byte_index(s: &str, char_index: usize) -> usize {
    s.char_indices()
        .nth(char_index)
        .map(|(byte_pos, _)| byte_pos)
        .unwrap_or(s.len())
}

/// Navigate to a specific match by index.
fn scroll_to_match(index: usize) {
    spawn(async move {
        let js = format!("window.Arto.search.navigateTo({});", index);
        let _ = document::eval(&js).await;
    });
}
