use crate::ipc::OpenEvent;
use crate::state::Tab;
use crate::window::settings;
use dioxus::desktop::use_muda_event_handler;
use dioxus::desktop::{window, WindowCloseBehaviour};
use dioxus::prelude::*;
use std::path::PathBuf;

// ============================================================================
// MainApp component
// ============================================================================

/// MainApp - Component dedicated to the first window
/// Configures system event handling and WindowHides behavior
///
/// NOTE: This component should only be used for the first window launched from main.rs.
/// Additional windows should use the App component directly.
///
/// System events (Reopen, file open, IPC) are handled by the Tao event loop's
/// custom_event_handler and IPC's GCD wake callback.
/// This component only handles the initial event (first CLI path) for its own tab.
#[component]
pub fn MainApp() -> Element {
    // Configure WindowCloseBehaviour::WindowHides for first window
    use_hook(|| {
        tracing::debug!("Configuring main window with WindowHides behavior");
        window().set_close_behavior(WindowCloseBehaviour::WindowHides);

        // Set chrome inset (window frame offset) - only first call takes effect
        let win = &window().window;
        if let (Ok(inner), Ok(outer)) = (win.inner_position(), win.outer_position()) {
            crate::window::set_chrome_inset((inner.x - outer.x) as f64, (inner.y - outer.y) as f64);
        }
    });

    // Set up global menu event handling
    use_muda_event_handler(move |event| {
        crate::menu::handle_menu_event_global(event);
    });

    // Pop the first event from IPC queue (CLI path pushed by main.rs before launch)
    let first_event = crate::ipc::try_pop_first_event();
    if first_event.is_some() {
        tracing::debug!(?first_event, "Received initial open event from IPC queue");
    } else {
        tracing::debug!("No initial event, will show welcome screen");
    }

    // Resolve initial tab and directory from event
    let is_first_window = true;
    let (tab, directory_override) = match &first_event {
        Some(OpenEvent::File(path)) => (Tab::new(path.clone()), None),
        Some(OpenEvent::Directory(path)) => (Tab::default(), Some(path.clone())),
        _ => {
            let welcome_content = crate::assets::get_default_markdown_content();
            (Tab::with_inline_content(welcome_content), None)
        }
    };

    // Get initial configuration values
    let directory_pref = settings::get_directory_preference(is_first_window);
    let theme_pref = settings::get_theme_preference(is_first_window);
    let sidebar_pref = settings::get_sidebar_preference(is_first_window);
    let toc_pref = settings::get_toc_preference(is_first_window);

    // Directory resolution: override (from event) → config → tab parent → home → root
    let directory = directory_override
        .or(directory_pref.directory)
        .or_else(|| tab.file().and_then(|p| p.parent().map(|p| p.to_path_buf())))
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("/"));

    // Render App component with initial state
    // Subsequent system events are handled by custom_event_handler (main.rs)
    // and GCD wake callback (ipc.rs).
    rsx! {
        crate::components::app::App {
            tab: tab,
            directory: directory,
            theme: theme_pref.theme,
            sidebar_open: sidebar_pref.open,
            sidebar_width: sidebar_pref.width,
            sidebar_show_all_files: sidebar_pref.show_all_files,
            toc_open: toc_pref.open,
            toc_width: toc_pref.width,
            toc_tab: toc_pref.tab,
        }
    }
}
