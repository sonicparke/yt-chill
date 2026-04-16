//! Shared process helpers

use tokio::process::Command;

/// Check if a command is available in PATH via `which`.
pub async fn is_command_available(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}
