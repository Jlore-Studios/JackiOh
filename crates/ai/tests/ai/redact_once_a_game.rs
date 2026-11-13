//! R3560000, Stinkyscape+ #120 and #121 once-a-game marks: `once_a_game` dropped from the
//! opponent's side so the seat never reads which firsts they have used.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, clone, dealt_game};

mod r3560000_redact_drops_the_opponents_once_a_game_marks {
    use super::*;

    /// R3560000 redact drops the opponent's once-a-game marks; the seat's own stay, and a
    /// redaction of a state that differs only in the opponent's marks hashes the same.
    #[test]
    fn r3560000_redact_drops_the_opponents_once_a_game_marks() {
        let mut state = dealt_game("ai-once-a-game");
        state.players[AI].once_a_game = Some(vec!["stinkyscapeplus-120:board".to_string()]);
        state.players[HUMAN].once_a_game = Some(vec!["stinkyscapeplus-121:hand".to_string()]);
        assert!(state.players[AI].once_a_game.is_some());

        let seen = redact(&state, AI);
        assert_eq!(
            seen.players[AI].once_a_game,
            Some(vec!["stinkyscapeplus-120:board".to_string()])
        );
        assert_eq!(seen.players[HUMAN].once_a_game, None);

        let mut other = clone(&state);
        other.players[HUMAN].once_a_game = Some(vec!["stinkyscapeplus-121:other".to_string()]);
        assert_ne!(hash_state(&other), hash_state(&state));
        assert_eq!(hash_state(&redact(&other, AI)), hash_state(&seen));
    }
}
