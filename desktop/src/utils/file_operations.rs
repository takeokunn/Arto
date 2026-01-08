use std::path::Path;
use std::process::Command;

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
