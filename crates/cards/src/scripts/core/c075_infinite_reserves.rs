//! #75 Infinite Reserves (SPEC §8.3): an empty-library draw gives a Rush Token card instead of
//! fatigue; both faces add "Cry: draw {draw}" (1, Radiant 3, R386) on top of the replacement.
//!
//! The replacement is a draw hook: `static_flags.infinite_reserves` is the whole of it. `draw_one`
//! reads it off the BACKROW before taking fatigue and `add_to_hand`s the §5 index "T-rush" token
//! (§8.3's Engine cell: a hand card, never a summon), skipping R3's fatigue. The engine owns R4 (a
//! full hand burns the token) and R11 (a unit-token card ceases to exist if it leaves other than
//! drawn or played). §3.2 makes the Field Spell public and permanent.
//!
//! The Cry is an ordinary draw (R1), deliberately NOT special-cased against its own flag: played on
//! an empty library it draws Rush Token cards, since §10.5 puts the card on the field before the Cry.

use jackioh_engine::effects::draw;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-075";

/// Both faces: the replacement, and the Cry drawing the declared number `draw` (R386).
fn infinite_reserves() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            infinite_reserves: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(|ctx| vec![draw(json_as(json!({ "count": param(&*ctx, "draw") })))])),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = infinite_reserves();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

/// R767000: through v0.4.12c only the Radiant face had the Cry (base with no Cry, Radiant drawing 3);
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_4_12c() -> CardScripts {
    CardScripts {
        base: Script {
            static_flags: Some(StaticFlags {
                infinite_reserves: Some(true),
                ..StaticFlags::default()
            }),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|ctx| vec![draw(json_as(json!({ "count": param(&*ctx, "draw") })))])),
            ..Script {
                static_flags: Some(StaticFlags {
                    infinite_reserves: Some(true),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            }
        },
    }
}

// #75 Infinite Reserves — SPEC §8.3, BUILD M4-T4.
// The empty-library draw is `library: []` plus `startTurn()` (§2.2); the control without the card
// takes R3's fatigue.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    fn setup(opts: Value) -> Scenario {
        crate::register_all();
        scenario(opts)
    }

    fn rush_tokens_in_hand(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        s.pile(player, "hand").into_iter().filter(|card| card.def_id == "core-t-rush").collect()
    }

    mod infinite_reserves_base {
        use super::*;

        #[test]
        fn an_empty_library_draw_yields_a_rush_token_card_in_hand_and_no_fatigue_damage_r3() {
            let mut s = setup(json!({
                "seed": "core-075-empty-draw",
                "p1": { "backrow": ["core-075"], "library": [], "hand": ["core-005"] },
                "p2": { "hand": ["core-005"], "library": ["core-035"] },
            }));
            assert_eq!(s.backrow(P1, 1).map(|card| card.def_id), Some("core-075".to_string()));

            s.start_turn();

            let tokens = rush_tokens_in_hand(&s, P1);
            assert_eq!(tokens.len(), 1);
            s.expect_in_zone(&tokens[0], "hand");
            s.expect_health(P1, 30);
            assert_eq!(s.state().players.p1.fatigue_count, 0);
            s.expect_events(json!(["turnStarted", "drawn", "addedToHand"]));
            assert!(s.unit(P1, 1).is_none());
        }

        #[test]
        fn without_infinite_reserves_the_very_same_draw_takes_r3s_fatigue_damage_control() {
            let mut s = setup(json!({
                "seed": "core-075-control",
                "p1": { "library": [], "hand": ["core-005"] },
                "p2": { "hand": ["core-005"], "library": ["core-035"] },
            }));

            s.start_turn();

            assert!(rush_tokens_in_hand(&s, P1).is_empty());
            // The Nth empty draw deals N damage; this is the first.
            s.expect_health(P1, 29);
            assert_eq!(s.state().players.p1.fatigue_count, 1);
            assert_eq!(s.hand(P1).len(), 1);
        }

        #[test]
        fn the_token_is_the_catalogs_own_rush_token_3_3_with_rush_no_stat_override_and_is_playable_from_hand() {
            let mut s = setup(json!({
                "seed": "core-075-token-identity",
                "p1": { "backrow": ["core-075"], "library": [], "hand": ["core-005"] },
                "p2": { "hand": ["core-005"], "library": ["core-035"] },
            }));

            s.start_turn();
            let token = rush_tokens_in_hand(&s, P1).first().cloned().expect("a Rush Token card in hand");
            assert_eq!(token.stats_override, None);

            s.play(&token, json!({}));

            let summoned = s.unit(P1, 1);
            assert_eq!(summoned.as_ref().map(|card| card.id.clone()), Some(token.id.clone()));
            let summoned = summoned.expect("the Rush Token in lane 1");
            s.expect_stats(&summoned, json!({ "attack": 3, "health": 3, "maxHealth": 3 }))
                .expect_events(json!(["cardPlayed", "summoned"]));
        }

        #[test]
        fn the_replacement_keeps_working_turn_after_turn_and_never_touches_the_opponents_draws() {
            let mut s = setup(json!({
                "seed": "core-075-repeat",
                "p1": { "backrow": ["core-075"], "library": [], "hand": ["core-005"] },
                "p2": { "hand": ["core-005"], "library": [] },
            }));

            s.start_turn();
            s.start_turn();

            assert_eq!(rush_tokens_in_hand(&s, P1).len(), 2);
            s.expect_health(P1, 30);
            // p2 has its own empty library and no Infinite Reserves; nothing here reached across.
            assert!(rush_tokens_in_hand(&s, P2).is_empty());
            assert_eq!(s.state().players.p2.fatigue_count, 0);
        }

        #[test]
        fn base_cry_draws_exactly_1() {
            let mut s = setup(json!({
                "seed": "core-075-base-cry",
                "p1": { "hand": ["core-075", "core-005"], "library": ["core-035", "core-036"] },
                "p2": { "hand": ["core-005"] },
            }));

            s.play("core-075", json!({}));

            // Two in hand, one played, one drawn.
            assert_eq!(s.hand(P1).len(), 2);
            assert_eq!(s.pile(P1, "library").len(), 1);
            s.expect_events(json!(["cardPlayed", "summoned", "drawn", "addedToHand"]));
            // Cost 0, so the mana is untouched.
            s.expect_mana(P1, 4);
            assert_eq!(s.backrow(P1, 1).map(|card| card.def_id), Some("core-075".to_string()));
        }
    }

    mod infinite_reserves_radiant {
        use super::*;

        #[test]
        fn radiant_cry_draw_3() {
            let mut s = setup(json!({
                "seed": "core-075-radiant-cry",
                "p1": {
                    "hand": [{ "def": "core-075", "radiant": true }, "core-005"],
                    "library": ["core-035", "core-036", "core-013", "core-019"],
                },
                "p2": { "hand": ["core-005"] },
            }));
            assert_eq!(s.hand(P1).first().map(|card| card.radiant), Some(true));

            s.play("core-075", json!({}));

            // Two in hand, one played, three drawn.
            assert_eq!(s.hand(P1).len(), 4);
            assert_eq!(s.pile(P1, "library").len(), 1);
            s.expect_events(json!(["cardPlayed", "summoned", "drawn", "drawn", "drawn"]))
                .expect_mana(P1, 4);
            assert_eq!(s.backrow(P1, 1).map(|card| card.radiant), Some(true));
        }

        #[test]
        fn same_keeps_the_replacement_on_the_radiant_face_too() {
            let mut s = setup(json!({
                "seed": "core-075-radiant-same",
                "p1": { "backrow": [{ "def": "core-075", "radiant": true }], "library": [], "hand": ["core-005"] },
                "p2": { "hand": ["core-005"], "library": ["core-035"] },
            }));

            s.start_turn();

            assert_eq!(rush_tokens_in_hand(&s, P1).len(), 1);
            s.expect_health(P1, 30);
            assert_eq!(s.state().players.p1.fatigue_count, 0);
        }

        #[test]
        fn a_radiant_infinite_reserves_played_onto_an_empty_library_draws_three_rush_token_cards() {
            let mut s = setup(json!({
                "seed": "core-075-radiant-empty-library",
                "p1": { "hand": [{ "def": "core-075", "radiant": true }, "core-005"], "library": [] },
                "p2": { "hand": ["core-005"] },
            }));

            // §10.5 puts the card on the field at step 4 and fires the Cry at step 5, so the flag is
            // already live for the Cry's own three draws.
            s.play("core-075", json!({}));

            assert_eq!(rush_tokens_in_hand(&s, P1).len(), 3);
            assert_eq!(s.hand(P1).len(), 4);
            s.expect_health(P1, 30);
            assert_eq!(s.state().players.p1.fatigue_count, 0);
        }

        #[test]
        fn r386_base_draw_1_upgrades_to_2_and_radiant_3_upgrades_to_4_degrades_to_2() {
            // (radiant face, upgrade?, moved value, draws on play)
            for (radiant, upgrade, moved_value, draws) in [
                (false, true, 2, 2),
                (true, true, 4, 4),
                (true, false, 2, 2),
            ] {
                let mut s = setup(json!({
                    "seed": "core-075-tuned",
                    "p1": { "hand": [{ "def": "core-075", "radiant": radiant }, "core-005"], "library": [] },
                    "p2": { "hand": ["core-005"] },
                }));
                let moved = if upgrade {
                    crate::upgrade_number(&mut s, "core-075", "draw")
                } else {
                    crate::degrade_number(&mut s, "core-075", "draw")
                };
                assert_eq!(moved, moved_value);
                s.play("core-075", json!({}));
                assert_eq!(rush_tokens_in_hand(&s, P1).len(), draws as usize);
                assert_eq!(s.state().players.p1.fatigue_count, 0);
            }
        }

        #[test]
        fn r386_base_draw_1_is_the_floor_no_degrade_moves_it() {
            let mut s = setup(json!({
                "seed": "core-075-floor",
                "p1": { "hand": ["core-075", "core-005"], "library": [] },
                "p2": { "hand": ["core-005"] },
            }));
            // An amount never drops below 1: no degrade step offers `draw`.
            assert!(!crate::can_degrade_number(&s, "core-075", "draw"));
            s.play("core-075", json!({}));
            assert_eq!(rush_tokens_in_hand(&s, P1).len(), 1);
            assert_eq!(s.state().players.p1.fatigue_count, 0);
        }
    }
}
