//! Playback policy helpers (CLI vs config).

use crate::types::PlayerType;

/// Use the Syncplay playback path when the CLI requests it **or** config sets
/// [`PlayerType::Syncplay`].
#[inline]
pub fn use_syncplay(cli_flag: bool, cfg_player: PlayerType) -> bool {
    cli_flag || cfg_player == PlayerType::Syncplay
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn use_syncplay_truth_table() {
        assert!(!use_syncplay(false, PlayerType::Mpv));
        assert!(use_syncplay(false, PlayerType::Syncplay));
        assert!(use_syncplay(true, PlayerType::Mpv));
        assert!(use_syncplay(true, PlayerType::Syncplay));
    }
}
