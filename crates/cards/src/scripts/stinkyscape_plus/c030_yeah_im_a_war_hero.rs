//! SPEC §8.12 S+ #30 Yeah I'm a War Hero, a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    (1) Field Spell: "The first card you draw each turn is replaced with a Warhero Token."
//!   Radiant: "… replaced with a Radiant Warhero Token."
//!
//! Engine: the replacement is a draw hook (R3553002): `static_flags.warhero_draws` is the whole of
//! it. `draw_one` reads it off the backrow before taking the turn's first draw and `add_to_hand`s
//! the §8 index "T-warhero-token" token (§8.12's Engine cell: a hand card, never a summon), the
//! library top staying for the next draw; a Radiant source mints a Radiant token. The engine owns
//! R4 (a full hand burns the token), R3 (an empty library still fatigues) and the setup guard
//! (§2.1). §3.2 makes the Field Spell public and permanent: destroying it ends the replacement.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-030";

/// R3553002: the first draw each turn is a Warhero Token card instead; the source card's face
/// selects the base or Radiant token in the engine. Declares no numbers.
fn war_hero() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            warhero_draws: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = war_hero();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #30 Yeah I'm a War Hero — SPEC §8.12 row S+ 30 (R3553002; R3, R4, §2.1, §2.4, §3.2).
// The first draw each turn is a Warhero Token card; the library top stays.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const WARHERO: &str = "stinkyscapeplus-t-warhero-token";
    const FILLER: &str = "core-005"; // Stockpile: a Spell
    const ANCHOR: &str = "core-010"; // Rapid Replenish: a Spell
    const HINDER: &str = "core-021"; // Hinder: cast on draw
    const NETHER: &str = "core-088"; // Twisting Nether: destroy all permanents

    fn standing(radiant: bool, seed: &str, hand: Value, library: Value) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "backrow": [{ "def": ID, "radiant": radiant }], "hand": hand, "library": library },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    fn warhero_tokens(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        s.hand(player)
            .into_iter()
            .filter(|card| card.def_id == WARHERO)
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r3553002_the_first_draw_is_a_warhero_token_and_the_library_top_stays() {
            let mut s = standing(false, "s30-first", json!([FILLER]), json!([ANCHOR, FILLER]));

            s.start_turn();

            // The token, not the Anchor: the library is untouched, top first.
            let tokens = warhero_tokens(&s, P1);
            assert_eq!(tokens.len(), 1);
            assert!(!tokens[0].radiant);
            s.expect_in_zone(&tokens[0], "hand");
            assert_eq!(s.pile(P1, "library").len(), 2);
            s.expect_events(json!(["turnStarted", "drawn", "addedToHand"]));

            // The skipped card is drawn next, in order.
            s.play(FILLER, json!({}));
            assert!(s.hand(P1).iter().any(|card| card.def_id == ANCHOR));
            assert!(s.pile(P1, "library").is_empty());
            assert!(warhero_tokens(&s, P1).iter().any(|card| card.def_id == WARHERO));
        }

        #[test]
        fn second_draw_is_normal() {
            let mut s = standing(false, "s30-second", json!([FILLER]), json!([ANCHOR, FILLER, FILLER]));
            s.start_turn();
            assert_eq!(warhero_tokens(&s, P1).len(), 1);

            s.play(FILLER, json!({}));

            // Stockpile draws 2, both library cards in order; no second token.
            assert_eq!(warhero_tokens(&s, P1).len(), 1);
            assert!(s.pile(P1, "library").len() == 1);
        }

        #[test]
        fn empty_library_first_draw_fatigues_without_a_token() {
            let mut s = standing(false, "s30-empty", json!([FILLER]), json!([]));
            s.start_turn();
            assert!(warhero_tokens(&s, P1).is_empty());
            assert_eq!(s.state().players.p1.fatigue_count, 1);
            s.expect_health(P1, 29);
        }

        #[test]
        fn full_hand_burns_the_token() {
            let hand = Value::Array((0..HAND_CAP).map(|_| json!(FILLER)).collect());
            let mut s = standing(false, "s30-full", hand, json!([ANCHOR]));
            s.start_turn();
            assert!(warhero_tokens(&s, P1).is_empty(), "no room: the token burns (R4)");
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            assert_eq!(s.pile(P1, "library").len(), 1, "the library top stays");
            assert!(
                s.events().iter().any(|event| matches!(
                    event,
                    GameEvent::Burned { def_id, .. } if def_id == WARHERO
                )),
                "the token burned"
            );
        }

        #[test]
        fn destroying_the_field_spell_ends_it() {
            let mut s = standing(false, "s30-nether", json!([NETHER, FILLER]), json!([ANCHOR, FILLER]));
            s.start_turn();
            assert_eq!(warhero_tokens(&s, P1).len(), 1);
            s.play(NETHER, json!({}));
            s.expect_in_zone(ID, "graveyard");
            s.end_turn();
            s.end_turn();
            // The next turn's first draw is the library top again.
            assert_eq!(warhero_tokens(&s, P1).len(), 1, "no new token");
            assert!(s.hand(P1).iter().any(|card| card.def_id == ANCHOR));
        }

        #[test]
        fn opponents_draws_are_untouched() {
            let mut s = scenario(json!({
                "seed": "s30-opp",
                "p1": { "backrow": [ID], "hand": [FILLER], "library": [ANCHOR] },
                "p2": { "hand": [FILLER], "library": [ANCHOR, FILLER] },
            }));
            s.end_turn();
            // p2's turn start draws their library top, no token.
            assert!(warhero_tokens(&s, P2).is_empty());
            assert!(s.hand(P2).iter().any(|card| card.def_id == ANCHOR));
            s.end_turn();
            // …while p1's own turn start replaces.
            assert_eq!(warhero_tokens(&s, P1).len(), 1);
        }

        #[test]
        fn skipped_cast_on_draw_card_does_not_cast() {
            let mut s = standing(false, "s30-hinder", json!([FILLER]), json!([HINDER, FILLER]));
            s.start_turn();
            assert_eq!(warhero_tokens(&s, P1).len(), 1);
            assert!(
                !s.events().iter().any(|event| matches!(event, GameEvent::CardPlayed { .. })),
                "Hinder stayed on top, uncast"
            );
            assert_eq!(s.pile(P1, "library").len(), 2);
            assert_eq!(s.pile(P1, "library")[0].def_id, HINDER);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn radiant_face_gives_a_radiant_token() {
            let mut s = standing(true, "s30-radiant", json!([FILLER]), json!([ANCHOR]));
            s.start_turn();
            let tokens = warhero_tokens(&s, P1);
            assert_eq!(tokens.len(), 1);
            assert!(tokens[0].radiant, "a Radiant Warhero Token");
        }
    }
}
