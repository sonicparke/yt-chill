//! Pure state-machine helpers for the CLI (unit-testable).

use crate::types::AppState;

/// After playback (or download / syncplay), return to the main menu on success
/// or exit on failure.
#[must_use]
pub fn next_state_after_play(play_succeeded: bool) -> AppState {
    if play_succeeded {
        AppState::Init
    } else {
        AppState::Exit
    }
}

/// After the user backs out without finishing (cancel in the selector, empty
/// results, recoverable errors), return to the main menu for a plain `yt-chill`
/// session, or exit for one-shot invocations (`--history`, `--feed`, a search
/// query on the command line, etc.).
#[must_use]
pub fn next_state_after_skipped_flow(started_from_main_menu: bool) -> AppState {
    if started_from_main_menu {
        AppState::Init
    } else {
        AppState::Exit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_returns_to_main_menu() {
        assert_eq!(next_state_after_play(true), AppState::Init);
    }

    #[test]
    fn failure_exits() {
        assert_eq!(next_state_after_play(false), AppState::Exit);
    }

    #[test]
    fn skipped_flow_returns_to_menu_when_interactive() {
        assert_eq!(next_state_after_skipped_flow(true), AppState::Init);
    }

    #[test]
    fn skipped_flow_exits_when_one_shot() {
        assert_eq!(next_state_after_skipped_flow(false), AppState::Exit);
    }
}
