//! `shuffleInto` at the bottom of the library (ME-DECK-BOTTOM): a fresh card put last with no
//! rng draw, refused by a full library (R80). Meditative #49 YileGPT Tamed (R80, R310) is its user.

use jackioh_engine::config::LIBRARY_CAP;
use jackioh_engine::effects::shuffle_into;
use jackioh_engine::testkit::PlayerId::P1;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::damage_combat::{grunt, playing};
use crate::rules::fixtures::harness::sink_for;

/// TS `makeContext(sink, null, { controller })`'s options.
fn by(player: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(player),
        ..Default::default()
    }
}

mod deck_bottom {
    use super::*;

    #[test]
    fn r80_position_bottom_puts_the_fresh_card_last_with_no_rng_draw() {
        let mut state = playing("dc-deck-bottom");
        let before: Vec<String> = state
            .players
            .p1
            .library
            .iter()
            .map(|card| card.id.clone())
            .collect();
        let mut sink = sink_for(&mut state);
        let cursor = sink.rng.cursor();
        apply_effects(
            &[shuffle_into(json_as(json!({
                "defId": grunt.id,
                "count": 1,
                "position": "bottom",
            })))],
            &mut make_context(&mut sink, None, by(P1)),
        );
        // The fresh card is last, the rest untouched — and the rng cursor never moved.
        let after: Vec<String> = sink
            .state
            .players
            .p1
            .library
            .iter()
            .map(|card| card.id.clone())
            .collect();
        assert_eq!(after.len(), before.len() + 1);
        assert_eq!(after[..before.len()], before);
        let last = sink.state.players.p1.library.last().expect("a last card");
        assert_eq!(last.def_id, grunt.id);
        assert_eq!(sink.rng.cursor(), cursor);
        // Its owner's own list shows it (R310).
        assert_eq!(
            last.known_as.as_ref().map(|known| known.def_id.as_str()),
            Some(grunt.id.as_str())
        );
    }

    #[test]
    fn r80_a_full_library_refuses_it() {
        let mut state = playing("dc-deck-bottom-full");
        let filler = state.players.p1.library.first().cloned().expect("a card");
        while state.players.p1.library.len() < LIBRARY_CAP as usize {
            state.players.p1.library.push(filler.clone());
        }
        let mut sink = sink_for(&mut state);
        apply_effects(
            &[shuffle_into(json_as(json!({
                "defId": grunt.id,
                "count": 1,
                "position": "bottom",
            })))],
            &mut make_context(&mut sink, None, by(P1)),
        );
        // Nothing created, nothing moved: still at the cap, and the refusal is reported.
        assert_eq!(sink.state.players.p1.library.len(), LIBRARY_CAP as usize);
        assert!(
            sink.events
                .iter()
                .any(|event| matches!(event, GameEvent::LibraryOverflow { .. }))
        );
    }
}
