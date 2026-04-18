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
}
