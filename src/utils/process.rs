//! Shared process helpers

use std::path::{Path, PathBuf};

/// Split a `PATH`-style string into directory entries (empty segments skipped).
pub fn split_path_var(path_var: &str) -> Vec<String> {
    let sep = if cfg!(windows) { ';' } else { ':' };
    path_var
        .split(sep)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

/// Resolve `executable` by walking `path_entries` only (pure; no subprocess).
///
/// On Unix, requires a regular file with at least one executable bit set.
/// On Windows, looks for `executable` and `executable.exe` in each directory.
pub fn resolve_executable_in_path(executable: &str, path_entries: &[String]) -> Option<PathBuf> {
    if executable.is_empty() {
        return None;
    }

    for dir in path_entries {
        let base = Path::new(dir);
        #[cfg(windows)]
        {
            for name in [executable, &format!("{executable}.exe")] {
                let candidate = base.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        #[cfg(unix)]
        {
            use std::fs;
            use std::os::unix::fs::PermissionsExt;

            let candidate = base.join(executable);
            if candidate.is_file() {
                let mode = fs::metadata(&candidate).ok()?.permissions().mode();
                if mode & 0o111 != 0 {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

/// True if `cmd` exists as an executable somewhere on `PATH`.
pub fn is_command_available(cmd: &str) -> bool {
    let path = std::env::var("PATH").unwrap_or_default();
    let entries = split_path_var(&path);
    resolve_executable_in_path(cmd, &entries).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_finds_file_in_path() {
        let tmp = std::env::temp_dir();
        let entries = vec![tmp.to_string_lossy().into_owned()];
        #[cfg(unix)]
        {
            use std::fs::OpenOptions;
            use std::os::unix::fs::OpenOptionsExt;
            let name = format!(
                "yt_chill_path_test_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            let path = tmp.join(&name);
            OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .mode(0o755)
                .open(&path)
                .unwrap();
            assert_eq!(
                resolve_executable_in_path(&name, &entries).as_ref(),
                Some(&path)
            );
            let _ = std::fs::remove_file(&path);
        }
        #[cfg(windows)]
        {
            // Skip creating executable on Windows in unit tests (permissions differ).
            let _ = entries;
        }
    }

    #[test]
    fn resolve_returns_none_for_missing() {
        assert!(
            resolve_executable_in_path(
                "definitely_missing_binary_xyz",
                &["/nonexistent/dir/12345".into()]
            )
            .is_none()
        );
    }

    #[test]
    fn resolve_empty_executable() {
        assert!(resolve_executable_in_path("", &["/bin".into(), "/usr/bin".into()]).is_none());
    }

    #[test]
    fn split_path_var_skips_empties() {
        assert_eq!(
            split_path_var("/a::/b"),
            vec!["/a".to_string(), "/b".to_string()]
        );
    }
}
