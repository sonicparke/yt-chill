//! fzf selector implementation

use crate::types::MenuItem;
use crate::utils::process::{resolve_executable_in_path, split_path_var};
use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Default)]
pub struct FzfSelector;

impl FzfSelector {
    pub fn new() -> Self {
        Self
    }

    pub fn select<T: Clone + Send + 'static>(
        &self,
        items: &[MenuItem<T>],
        prompt: &str,
    ) -> Option<T> {
        if items.is_empty() {
            return None;
        }

        // Build input: one item per line with index prefix
        let input: String = items
            .iter()
            .enumerate()
            .map(|(i, item)| format!("{}\t{}", i, item.label))
            .collect::<Vec<_>>()
            .join("\n");

        // Spawn fzf
        let mut child = Command::new("fzf")
            .args([
                "--prompt",
                &format!("{} > ", prompt),
                "--height",
                "40%",
                "--reverse",
                "--ansi",
                "--delimiter",
                "\t",
                "--with-nth",
                "2", // Show only the label
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .ok()?;

        // Write input to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input.as_bytes()).ok()?;
        }

        // Get output
        let output = child.wait_with_output().ok()?;

        if !output.status.success() {
            return None; // User cancelled
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let line = stdout.trim();

        if line.is_empty() {
            return None;
        }

        // Extract index from selection
        let index_str = line.split('\t').next()?;
        let index: usize = index_str.parse().ok()?;

        items.get(index).map(|item| item.value.clone())
    }

    pub fn is_available(&self) -> bool {
        let path = std::env::var("PATH").unwrap_or_default();
        let entries = split_path_var(&path);
        resolve_executable_in_path("fzf", &entries).is_some()
    }
}
