use dioxus::desktop::{tao::window::WindowId, window};
use dioxus::prelude::*;

use super::calculations::{calculate_grab_offset, is_tab_transferable};
use super::context_menu::TabContextMenu;
use super::tab_bar::PendingDrag;
use crate::components::icon::{Icon, IconName};
use crate::drag;
use crate::state::AppState;
use crate::utils::file_operations;

#[component]
pub fn TabItem(
    index: usize,
    tab: crate::state::Tab,
    is_active: bool,
    shift_class: Option<&'static str>,
    on_drag_start: EventHandler<PendingDrag>,
) -> Element {
    let mut state = use_context::<AppState>();
    let tab_name = tab.display_name();
    let transferable = is_tab_transferable(&tab.content);
    let file_path = tab.file().map(|p| p.to_path_buf());

    let mut show_context_menu = use_signal(|| false);
    let mut context_menu_position = use_signal(|| (0, 0));
    let mut other_windows = use_signal(Vec::new);

    // Store mounted element for accurate grab_offset calculation
    let mut tab_element: Signal<Option<std::rc::Rc<MountedData>>> = use_signal(|| None);

    // Handle pointer down for drag initiation
    // Uses PointerData for setPointerCapture compatibility (window-external drag)
    // Uses async to get accurate grab_offset via getBoundingClientRect
    let handle_pointerdown = move |evt: Event<PointerData>| async move {
        // Only start drag on left button
        if evt.data().trigger_button() != Some(dioxus::html::input_data::MouseButton::Primary) {
            return;
        }

        let pointer_id = evt.data().pointer_id();
        let client_coords = evt.client_coordinates();

        // Calculate grab_offset using getBoundingClientRect for accuracy
        // Clone signal data before await to avoid holding GenerationalRef across await point
        let mounted_data = tab_element.read().clone();
        let (grab_x, grab_y) = if let Some(ref mounted) = mounted_data {
            if let Ok(rect) = mounted.get_client_rect().await {
                calculate_grab_offset(
                    client_coords.x,
                    client_coords.y,
                    rect.origin.x,
                    rect.origin.y,
                )
            } else {
                // Fallback to element_coordinates
                let element_coords = evt.element_coordinates();
                (element_coords.x, element_coords.y)
            }
        } else {
            // Fallback to element_coordinates
            let element_coords = evt.element_coordinates();
            (element_coords.x, element_coords.y)
        };

        on_drag_start.call(PendingDrag {
            index,
            start_x: client_coords.x,
            start_y: client_coords.y,
            grab_offset: crate::window::Offset::new(grab_x, grab_y),
            pointer_id,
        });
    };

    // Handle right-click to show context menu
    let handle_context_menu = move |evt: Event<MouseData>| {
        evt.prevent_default();
        let mouse_data = evt.data();
        context_menu_position.set((
            mouse_data.client_coordinates().x as i32,
            mouse_data.client_coordinates().y as i32,
        ));

        // Refresh window list
        let windows = crate::window::main::list_visible_main_windows();
        let current_id = window().id();
        other_windows.set(
            windows
                .iter()
                .filter(|w| w.window.id() != current_id)
                .map(|w| (w.window.id(), w.window.title()))
                .collect(),
        );

        show_context_menu.set(true);
    };

    // Handler for "Open in New Window"
    // Create new window first, then close tab (in case it's the last tab)
    let handle_open_in_new_window = move |_| {
        if let Some(tab) = state.get_tab(index) {
            let directory = state.sidebar.read().root_directory.clone();

            spawn(async move {
                let params = crate::window::main::CreateMainWindowConfigParams {
                    directory,
                    ..Default::default()
                };
                crate::window::main::create_new_main_window(tab, params).await;

                // Close tab in source window after new window is created
                state.close_tab(index);
            });
        }
        show_context_menu.set(false);
    };

    // Handler for "Copy File Path"
    let handle_copy_path = {
        let file_path = file_path.clone();
        move |_| {
            if let Some(ref path) = file_path {
                crate::utils::clipboard::copy_text(path.to_string_lossy());
            }
            show_context_menu.set(false);
        }
    };

    // Handler for "Reload"
    let handle_reload = move |_| {
        state.reload_current_tab();
        show_context_menu.set(false);
    };

    // Handler for "Set Parent as Root"
    let handle_set_parent_as_root = {
        let file_path = file_path.clone();
        move |_| {
            if let Some(ref path) = file_path {
                if let Some(parent) = path.parent() {
                    state.set_root_directory(parent.to_path_buf());
                }
            }
            show_context_menu.set(false);
        }
    };

    // Handler for "Reveal in Finder"
    let handle_reveal_in_finder = {
        let file_path = file_path.clone();
        move |_| {
            if let Some(ref path) = file_path {
                file_operations::reveal_in_finder(path);
            }
            show_context_menu.set(false);
        }
    };

    // Handler for "Move to Window"
    // Uses TRANSFER_TAB_TO_WINDOW to preserve tab history when moving between windows
    let handle_move_to_window = move |target_id: WindowId| {
        if let Some(tab) = state.get_tab(index) {
            // Send tab transfer request to target window (preserves history)
            if crate::events::TRANSFER_TAB_TO_WINDOW
                .send((target_id, None, tab.clone()))
                .is_err()
            {
                tracing::warn!(
                    ?target_id,
                    "Failed to move tab: target window may be closed"
                );
                show_context_menu.set(false);
                return;
            }
            // Close tab in source window
            state.close_tab(index);
            // Focus the target window
            crate::window::main::focus_window(target_id);
            tracing::info!(?target_id, "Moved tab to window");
        }
        show_context_menu.set(false);
    };

    // Tab is always rendered normally - no placeholder needed since
    // dragged tab is removed at drag start (unified approach)
    let shift_class_str = shift_class.unwrap_or("");
    rsx! {
        div {
            class: "tab {shift_class_str}",
            class: if is_active { "active" },
            onpointerdown: handle_pointerdown,
            onclick: move |_| {
                // Only switch tab if not in a drag operation
                if !drag::is_tab_dragging() {
                    state.switch_to_tab(index);
                }
            },
            oncontextmenu: handle_context_menu,
            onmounted: move |evt| {
                // Store mounted data for accurate grab_offset calculation
                tab_element.set(Some(evt.data()));
            },

            span {
                class: "tab-name",
                "{tab_name}"
            }

            button {
                class: "tab-close",
                onclick: move |evt| {
                    evt.stop_propagation();
                    state.close_tab(index);
                },
                Icon { name: IconName::Close, size: 14 }
            }
        }

        if *show_context_menu.read() {
            TabContextMenu {
                position: *context_menu_position.read(),
                file_path: file_path.clone(),
                on_close: move |_| show_context_menu.set(false),
                on_copy_path: handle_copy_path,
                on_reload: handle_reload,
                on_set_parent_as_root: handle_set_parent_as_root,
                on_open_in_new_window: handle_open_in_new_window,
                on_move_to_window: handle_move_to_window,
                on_reveal_in_finder: handle_reveal_in_finder,
                other_windows: other_windows.read().clone(),
                disabled: !transferable,
            }
        }
    }
}
