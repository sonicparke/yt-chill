//! Playback policy helpers (CLI vs config).

use crate::types::PlayerType;

/// Use the Syncplay playback path when the CLI requests it **or** config sets
/// [`PlayerType::Syncplay`].
#[inline]
pub fn use_syncplay(cli_flag: bool, cfg_player: PlayerType) -> bool {
    cli_flag || cfg_player == PlayerType::Syncplay
}

/// Resolve a one-run CLI volume override against the persistent config value.
#[inline]
pub fn resolve_volume(cli_volume: Option<u8>, config_volume: u8) -> u8 {
    cli_volume.unwrap_or(config_volume)
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

    #[test]
    fn cli_volume_overrides_config_volume() {
        assert_eq!(resolve_volume(Some(30), 50), 30);
        assert_eq!(resolve_volume(None, 50), 50);
    }
}
