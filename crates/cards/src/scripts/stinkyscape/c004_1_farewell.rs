//! S #4.1 Farewell (SPEC §8.10 row S 4.1, SPEC §7; §3.2, §6.3 Exile, R11, R68, R1839000, R1857002,
//! R1870001): (4) Spell, Terminus, Token (printed Mythic), one of Olivian Expanse's picks.
//!   Base:    "Exile all permanents and every card in both graveyards."
//!   Radiant: "… every card in both graveyards, both hands and both decks."
//!
//! Engine: `exile_all_piles` over both sides' unit zones and backrows, in R68's order: each Stack pile
//! goes whole, the cards it uncovers on the next pass, and a face-down card goes as a face-up one does;
//! a Dormant card stays, since no effect reaches it (R1839000). Then `exile_matching` over each
//! graveyard, its caster's first; the Radiant face then each hand and library the same way, so each
//! player's next draw is a fatigue draw (R1870001). An exile fires no Death (§6.2), and a unit token

use jackioh_engine::effects::{exile_all_piles, exile_matching};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-004-1";

/// Every card of both players' `zones`, its caster's first.
fn exile_both(zones: &[&str]) -> [Effect; 2] {
    ["self", "enemy"].map(|player| exile_matching(json_as(json!({ "zones": zones, "player": player }))))
}

fn farewell(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let mut effects = vec![exile_all_piles(json_as(
        json!({ "side": "any", "rows": ["units", "backrow"] }),
    ))];
    effects.extend(exile_both(&["graveyard"]));
    if ctx.radiant {
        effects.extend(exile_both(&["hand", "library"]));
    }
    effects
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(farewell)),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S #4.1 Farewell — SPEC §8.10 row S 4.1, BUILD M12 row S 4.1 (R1870001).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FAREWELL: &str = "stinkyscape-004-1";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const SHEEPLE: &str = "classic-082"; // Death: draw 1.
    const FILLER: &str = "core-005"; // (1) Spell Stockpile.
    const WELL: &str = "core-006"; // (3) Field Spell.
    const HONEYPOT: &str = "core-060"; // a Trap.

    fn farewell(radiant: bool) -> Value {
        json!({ "def": FAREWELL, "radiant": radiant })
    }

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "farewell",
            "p1": {
                "hand": [farewell(radiant), FILLER],
                "field": [VANILLA, SHEEPLE],
                "backrow": [{ "def": WELL, "faceUp": true }],
                "graveyard": [VANILLA, FILLER],
                "library": [FILLER, FILLER],
                "mana": 10,
            },
            "p2": {
                "hand": [FILLER],
                "field": [VANILLA],
                "backrow": [{ "def": HONEYPOT, "faceUp": false }, { "def": WELL, "faceUp": true }],
                "graveyard": [FILLER],
                "library": [FILLER, FILLER],
            },
        }))
    }

    fn field_ids(s: &Scenario) -> Vec<String> {
        let mut ids = Vec::new();
        for player in [P1, P2] {
            for lane in 1..=UNIT_ZONES {
                ids.extend(s.unit(player, lane).map(|card| card.id));
            }
            for lane in 1..=BACKROW_ZONES {
                ids.extend(s.backrow(player, lane).map(|card| card.id));
            }
        }
        ids
    }

    mod r1870001_farewell {
        use super::*;

        #[test]
        fn r1870001_exiles_units_backrow_and_face_down_traps_both_sides() {
            let mut s = board(false);
            let on_field = field_ids(&s);
            assert_eq!(on_field.len(), 6, "two Units and a Field Spell, a Unit, a Trap and a Field Spell");
            s.play(FAREWELL, json!({}));
            for id in &on_field {
                s.expect_in_zone(id.as_str(), "exile");
            }
            assert!(field_ids(&s).is_empty(), "both fields are empty");
        }

        #[test]
        fn r1870001_a_pile_goes_whole() {
            let mut s = scenario(json!({
                "seed": "farewell-pile",
                "p1": { "hand": [FAREWELL, FILLER] },
                "p2": { "hand": [FILLER], "field": [VANILLA, { "def": VANILLA, "stack": true }] },
            }));
            let pile: Vec<String> = s.state().players.p2.units[0]
                .as_ref()
                .expect("a pile in lane 1")
                .iter()
                .map(|card| card.id.clone())
                .collect();
            assert_eq!(pile.len(), 2);
            s.play(FAREWELL, json!({}));
            for id in &pile {
                s.expect_in_zone(id.as_str(), "exile");
            }
        }

        #[test]
        fn r1870001_fires_no_death() {
            let mut s = board(false);
            s.play(FAREWELL, json!({}));
            let kinds: Vec<GameEventType> = s.last_events().iter().map(GameEvent::event_type).collect();
            assert!(!kinds.contains(&GameEventType::Destroyed), "an exile destroys nothing: {kinds:?}");
            assert!(!kinds.contains(&GameEventType::Drawn), "Sheeople's Death draws nothing: {kinds:?}");
            assert_eq!(s.hand(P1).len(), 1, "the filler alone");
        }

        #[test]
        fn r1870001_empties_both_graveyards() {
            let mut s = board(false);
            let buried: Vec<String> = [P1, P2]
                .into_iter()
                .flat_map(|player| s.pile(player, "graveyard"))
                .map(|card| card.id)
                .collect();
            assert_eq!(buried.len(), 3);
            s.play(FAREWELL, json!({}));
            for id in &buried {
                s.expect_in_zone(id.as_str(), "exile");
            }
            assert_eq!(s.hand(P1).len(), 1);
            assert_eq!(s.hand(P2).len(), 1);
            assert_eq!(s.pile(P1, "library").len(), 2);
            assert_eq!(s.pile(P2, "library").len(), 2);
        }

        #[test]
        fn r1870001_dormant_card_stays() {
            let mut s = scenario(json!({
                "seed": "farewell-dormant",
                "p1": { "hand": [FAREWELL, FILLER] },
                "p2": {
                    "hand": [FILLER],
                    "field": [{ "def": VANILLA, "lane": 2, "slumber": { "turns": 2 } }, { "def": VANILLA, "lane": 3 }],
                },
            }));
            let dormant = s.unit_at(P2, 2);
            let awake = s.unit_at(P2, 3);
            s.play(FAREWELL, json!({}));
            s.expect_in_zone(&dormant, "field");
            s.expect_in_zone(&awake, "exile");
        }

        #[test]
        fn r1870001_farewell_lands_in_its_graveyard() {
            let mut s = board(false);
            let card = s
                .hand(P1)
                .into_iter()
                .find(|card| card.def_id == FAREWELL)
                .expect("Farewell in hand");
            s.play(FAREWELL, json!({}));
            s.expect_in_zone(&card, "graveyard");
            let graveyard: Vec<String> = s.pile(P1, "graveyard").into_iter().map(|card| card.id).collect();
            assert_eq!(graveyard, vec![card.id], "it alone is there after it");
        }

        #[test]
        fn r1870001_radiant_empties_hands_and_decks() {
            let mut s = board(true);
            let held: Vec<String> = [P1, P2]
                .into_iter()
                .flat_map(|player| {
                    let mut cards = s.hand(player);
                    cards.extend(s.pile(player, "library"));
                    cards
                })
                .filter(|card| card.def_id != FAREWELL)
                .map(|card| card.id)
                .collect();
            assert_eq!(held.len(), 6);
            s.play(FAREWELL, json!({}));
            for id in &held {
                s.expect_in_zone(id.as_str(), "exile");
            }
            for player in [P1, P2] {
                assert!(s.hand(player).is_empty());
                assert!(s.pile(player, "library").is_empty());
            }
            assert!(field_ids(&s).is_empty(), "the field goes too");
        }

        #[test]
        fn r1870001_radiant_next_draw_is_fatigue() {
            let mut s = board(true);
            // With both hands and boards gone neither turn holds anything meaningful (R345).
            for player in [P1, P2] {
                s.state_mut().players[player].auto_end_turn = Some(false);
            }
            s.play(FAREWELL, json!({}));
            s.end_turn();
            assert_eq!(s.state().active, P2);
            let fatigued: Vec<PlayerId> = s
                .last_events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Fatigue { player, .. } => Some(*player),
                    _ => None,
                })
                .collect();
            assert_eq!(fatigued, vec![P2], "p2's draw is a fatigue draw");
        }
    }
}
