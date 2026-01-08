//! Reusable bookmark toggle button component for Quick Access feature.

use dioxus::prelude::*;
use std::path::PathBuf;

use crate::bookmarks::{toggle_bookmark, BOOKMARKS, BOOKMARKS_CHANGED};
use crate::components::icon::{Icon, IconName};

/// Reusable bookmark toggle button
#[component]
pub fn BookmarkButton(
    /// Path to bookmark/unbookmark
    path: PathBuf,
    /// Icon size in pixels (default: 14)
    #[props(default = 14)]
    size: u32,
) -> Element {
    let path_for_check = path.clone();
    let mut is_bookmarked = use_signal(move || BOOKMARKS.read().contains(&path_for_check));

    // Subscribe to bookmark changes from other windows/components
    let path_for_subscription = path.clone();
    use_future(move || {
        let path = path_for_subscription.clone();
        async move {
            let mut rx = BOOKMARKS_CHANGED.subscribe();
            while rx.recv().await.is_ok() {
                is_bookmarked.set(BOOKMARKS.read().contains(&path));
            }
        }
    });

    let handle_click = {
        let path = path.clone();
        move |evt: Event<MouseData>| {
            evt.stop_propagation();
            toggle_bookmark(&path);
        }
    };

    let icon_name = if *is_bookmarked.read() {
        IconName::StarFilled
    } else {
        IconName::Star
    };

    let title = if *is_bookmarked.read() {
        "Remove from Quick Access"
    } else {
        "Add to Quick Access"
    };

    let bookmarked_class = if *is_bookmarked.read() {
        "bookmarked"
    } else {
        ""
    };

    rsx! {
        button {
            class: "bookmark-button {bookmarked_class}",
            title: "{title}",
            draggable: false,
            onclick: handle_click,
            Icon { name: icon_name, size }
        }
    }
}
