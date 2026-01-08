use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Reveal a file in Finder (macOS) or file explorer
pub fn reveal_in_finder(path: impl AsRef<Path>) {
    let path = path.as_ref();

    #[cfg(target_os = "macos")]
    {
        // Use `open -R` to reveal the file in Finder
        if let Err(e) = Command::new("open").arg("-R").arg(path).spawn() {
            tracing::error!(%e, ?path, "Failed to reveal in Finder");
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        // On other platforms, just open the parent directory
        if let Some(parent) = path.parent() {
            if let Err(e) = open::that(parent) {
                tracing::error!(%e, ?parent, "Failed to open parent directory");
            }
        }
    }
}

/// Copy text to the system clipboard
pub fn copy_to_clipboard(text: &str) {
    #[cfg(target_os = "macos")]
    {
        // Use pbcopy on macOS for reliable clipboard access
        match Command::new("pbcopy").stdin(Stdio::piped()).spawn() {
            Ok(mut child) => {
                if let Some(stdin) = child.stdin.as_mut() {
                    if let Err(e) = stdin.write_all(text.as_bytes()) {
                        tracing::error!(%e, "Failed to write to pbcopy stdin");
                    }
                }
                let _ = child.wait();
            }
            Err(e) => {
                tracing::error!(%e, "Failed to spawn pbcopy");
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Fallback to JavaScript clipboard API for other platforms
        use dioxus::prelude::*;
        let text = text.to_string();
        spawn(async move {
            // Use JSON encoding to safely escape the string for JavaScript
            let json_encoded = serde_json::to_string(&text).unwrap_or_default();
            let js = format!("navigator.clipboard.writeText({})", json_encoded);
            if let Err(e) = document::eval(&js).await {
                tracing::error!(%e, "Failed to copy to clipboard");
            }
        });
    }
}
