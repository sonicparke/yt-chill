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

/// Carry a successfully observed mpv volume to the next track in this process.
#[inline]
pub fn next_session_volume(current: u8, final_volume: Option<u8>) -> u8 {
    final_volume.unwrap_or(current)
}

/// Decide whether a successful interactive change should update persistent config.
///
/// An explicit CLI volume remains scoped to the current yt-chill process.
#[inline]
pub fn volume_to_persist(
    cli_volume: Option<u8>,
    configured_volume: u8,
    final_volume: Option<u8>,
) -> Option<u8> {
    if cli_volume.is_some() {
        return None;
    }

    final_volume.filter(|volume| *volume != configured_volume)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_interactive_change_updates_session_and_config() {
        assert_eq!(next_session_volume(50, Some(48)), 48);
        assert_eq!(volume_to_persist(None, 50, Some(48)), Some(48));
    }

    #[test]
    fn cli_override_changes_carry_within_run_but_do_not_rewrite_config() {
        assert_eq!(next_session_volume(50, Some(48)), 48);
        assert_eq!(volume_to_persist(Some(50), 75, Some(48)), None);
    }

    #[test]
    fn missing_observation_keeps_session_and_config_unchanged() {
        assert_eq!(next_session_volume(50, None), 50);
        assert_eq!(volume_to_persist(None, 50, None), None);
    }

    #[test]
    fn unchanged_volume_does_not_rewrite_config() {
        assert_eq!(volume_to_persist(None, 50, Some(50)), None);
    }

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
