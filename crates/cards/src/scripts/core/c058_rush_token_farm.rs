//! #58 Rush Token Farm (SPEC §8.3): Field Spell, cost 2. "Start of turn: summon a Rush Token" /
//! radiant "Aura: your Rush Tokens +3/+3; same". Engine cell: "Aura keyed on def T-rush".
//!
//! §8 Conventions: "'same' says so explicitly", so the radiant cell ADDS the aura and KEEPS the
//! base start-of-turn summon — the radiant face summons a token every turn AND pumps them.
//!
//! R62's turn sequence puts "start-of-turn triggers" after the mana refresh and the start-of-turn
//! delayed effects and before the draw, and §6.2 defines "Start of turn" as the CONTROLLER's turn
//! start, so this fires on its controller's turns only and never on the opponent's (`run_hooks_in_
//! trigger_order(sink, "startOfTurn", player)` in engine/src/triggers.rs owns that).
//!
//! R64 places the token: with no lane named, the leftmost empty, unlocked, unreserved unit zone.
//! A full board summons nothing and the trigger still resolved (§3.2: "a summon into a full row
//! fails silently"), which is #15's "Board full -> fewer" rule applied one token at a time.
//!
//! The aura is §10.4 layer 5: a function of the board, recomputed on every read by `unit_view`, so it
//! vanishes the instant this Field Spell leaves the backrow and it lowers nothing permanently.
//! §7's token rules name this card as the reason `stats_override` is not the mechanism here: "Rush
//! Token Farm radiant gives all your Rush Tokens +3/+3 as an aura", not as a summon-time stat.
//! Keyed on the def id per the Engine cell, so it touches only Rush Tokens (`core-t-rush`) that this
//! card's controller controls: an opponent's Rush Token is outside it, and so is any other 3/3.

use jackioh_engine::effects::summon;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-058";

/// §7's shared Rush Token, 3/3 with Rush. The aura and the summon must name the same def.
const RUSH_TOKEN: &str = "core-t-rush";

fn start_of_turn() -> Hook {
    hook(|_ctx| vec![summon(json_as(json!({ "defId": RUSH_TOKEN })))])
}

/// §10.4 layer 5. `applies` reads instance data only — `controller` and `def_id`, never `unit_view` —
/// because the layer stack would recurse otherwise (see `aura_mods` in engine/src/layers.rs).
/// `ctx.self_.controller` rather than the owner: control is what "your" means on the field (R12).
fn rush_token_aura() -> AuraHook {
    aura_hook(|ctx| {
        let controller = ctx.self_.controller;
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| unit.controller == controller && unit.def_id == RUSH_TOKEN),
            mod_: StatMod {
                attack: Some(3),
                max_health: Some(3),
                ..StatMod::default()
            },
        }]
    })
}

pub fn script() -> CardScripts {
    let start_of_turn = start_of_turn();
    CardScripts {
        base: Script {
            start_of_turn: Some(start_of_turn.clone()),
            ..Script::default()
        },
        radiant: Script {
            start_of_turn: Some(start_of_turn),
            aura: Some(rush_token_aura()),
            ..Script::default()
        },
    }
}

// #58 Rush Token Farm (SPEC §8.3, BUILD M4-T4 row 58: "Token each start of turn; radiant +3/+3
// aura only on Rush Tokens"). Engine cell: "Aura keyed on def T-rush".
//
// Rulings proved here: R62 and §6.2 (start-of-turn triggers fire on the controller's turn only, and
// after the mana refresh, before the draw), R64 (leftmost empty unlocked zone; a full row summons
// nothing), §10.4 layer 5 (the aura is a computed layer, not a buff), §7 ("Rush Token Farm radiant
// gives all your Rush Tokens +3/+3 as an aura").
//
// §8 Conventions: the radiant cell ends in "same", so the radiant face summons AND pumps.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// Every unit a side has on the field, lane 1 to 5.
    fn units_of(g: &Scenario, player: PlayerId) -> Vec<String> {
        (1..=5)
            .filter_map(|lane| g.unit(player, lane).map(|unit| unit.def_id))
            .collect()
    }

    /// Both sides need something to do, or R82's auto-end cascades the turn onward.
    fn busy() -> Value {
        json!({ "hand": ["core-005"], "library": ["core-008", "core-011"] })
    }

    /// TS `{ ...base, ...extra }` over two JSON objects: the later keys win.
    fn spread(base: Value, extra: Value) -> Value {
        let mut all = base.as_object().cloned().unwrap_or_default();
        for (key, value) in extra.as_object().cloned().unwrap_or_default() {
            all.insert(key, value);
        }
        Value::Object(all)
    }

    /// A unit that must be there: TS's `expect(x).not.toBeNull()` and its early return.
    fn present(unit: Option<CardInstance>) -> CardInstance {
        match unit {
            Some(unit) => unit,
            None => panic!("expected a unit, found an empty zone"),
        }
    }

    mod base {
        use super::*;

        #[test]
        fn s6_2_summons_a_rush_token_at_its_controller_s_start_of_turn() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": spread(busy(), json!({ "backrow": [{ "def": "core-058", "lane": 1 }] })),
                "p2": busy(),
            }));

            g.start_turn();

            assert_eq!(units_of(&g, P1), ["core-t-rush"]);
            g.expect_stats("core-t-rush", json!({ "attack": 3, "maxHealth": 3, "health": 3 }))
                .expect_events(json!(["turnStarted", "summoned"]));
        }

        #[test]
        fn r62_and_s6_2_it_fires_on_the_controller_s_turns_only_never_on_the_opponent_s() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": spread(busy(), json!({ "backrow": [{ "def": "core-058", "lane": 1 }] })),
                "p2": busy(),
            }));

            g.start_turn();
            assert_eq!(units_of(&g, P1).len(), 1);

            // p2's turn: p1's Field Spell is in play but it is not p1's start of turn.
            g.end_turn();
            assert_eq!(units_of(&g, P1).len(), 1);
            assert_eq!(units_of(&g, P2).len(), 0);

            // Back to p1: a second token.
            g.end_turn();
            assert_eq!(units_of(&g, P1), ["core-t-rush", "core-t-rush"]);
        }

        #[test]
        fn r64_takes_the_leftmost_empty_unit_zone() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": spread(busy(), json!({
                    "backrow": [{ "def": "core-058", "lane": 1 }],
                    "field": [{ "def": "core-008", "lane": 1 }],
                })),
                "p2": busy(),
            }));

            g.start_turn();

            assert_eq!(g.unit(P1, 1).map(|unit| unit.def_id).as_deref(), Some("core-008"));
            assert_eq!(g.unit(P1, 2).map(|unit| unit.def_id).as_deref(), Some("core-t-rush"));
        }

        #[test]
        fn s3_2_a_full_row_summons_nothing_and_the_trigger_still_resolves() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": spread(busy(), json!({
                    "backrow": [{ "def": "core-058", "lane": 1 }],
                    "field": [
                        { "def": "core-008", "lane": 1 },
                        { "def": "core-008", "lane": 2 },
                        { "def": "core-008", "lane": 3 },
                        { "def": "core-008", "lane": 4 },
                        { "def": "core-008", "lane": 5 },
                    ],
                })),
                "p2": busy(),
            }));

            g.start_turn();

            assert_eq!(units_of(&g, P1), vec!["core-008"; 5]);
            g.expect_in_zone("core-058", "field");
        }

        #[test]
        fn the_base_face_has_no_aura_a_rush_token_beside_it_is_a_plain_3_3() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "backrow": [{ "def": "core-058", "lane": 1 }], "field": [{ "def": "core-t-rush", "lane": 1 }] },
            }));

            g.expect_stats("core-t-rush", json!({ "attack": 3, "maxHealth": 3, "health": 3 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn s10_4_layer_5_gives_your_rush_tokens_3_3() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": {
                    "backrow": [{ "def": "core-058", "radiant": true, "lane": 1 }],
                    "field": [{ "def": "core-t-rush", "lane": 1 }],
                },
            }));

            g.expect_stats("core-t-rush", json!({ "attack": 6, "maxHealth": 6, "health": 6 }));
        }

        #[test]
        fn the_engine_cell_s_def_key_only_rush_tokens_and_only_the_ones_you_control() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": {
                    "backrow": [{ "def": "core-058", "radiant": true, "lane": 1 }],
                    "field": [
                        { "def": "core-t-rush", "lane": 1 },
                        // Tempo Timmy is also a 3/3 with Rush — a stats and keyword match is not a def match.
                        { "def": "core-011", "lane": 2 },
                    ],
                },
                "p2": { "field": [{ "def": "core-t-rush", "lane": 1 }] },
            }));

            let mine = present(g.unit(P1, 1));
            let theirs = present(g.unit(P2, 1));

            g.expect_stats(&mine, json!({ "attack": 6, "maxHealth": 6 }));
            g.expect_stats("core-011", json!({ "attack": 3, "maxHealth": 3 }));
            // "your Rush Tokens": control, not ownership of the def (R12).
            g.expect_stats(&theirs, json!({ "attack": 3, "maxHealth": 3 }));
        }

        #[test]
        fn s10_4_layer_5_and_not_layer_4_the_aura_changes_no_buffs_on_the_instance() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": {
                    "backrow": [{ "def": "core-058", "radiant": true, "lane": 1 }],
                    "field": [{ "def": "core-t-rush", "lane": 1 }],
                },
            }));

            let token = present(g.unit(P1, 1));
            assert_eq!(
                serde_json::to_value(token.buffs).expect("buffs serialise"),
                json!({ "attack": 0, "health": 0 })
            );
            g.expect_stats("core-t-rush", json!({ "attack": 6, "maxHealth": 6 }));
        }

        #[test]
        fn s8_conventions_same_the_radiant_face_still_summons_a_token_and_pumps_that_one_too() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": spread(busy(), json!({ "backrow": [{ "def": "core-058", "radiant": true, "lane": 1 }] })),
                "p2": busy(),
            }));

            g.start_turn();

            assert_eq!(units_of(&g, P1), ["core-t-rush"]);
            g.expect_stats("core-t-rush", json!({ "attack": 6, "maxHealth": 6, "health": 6 }));
        }

        #[test]
        fn the_3_3_is_real_in_combat_a_pumped_token_trades_with_a_6_attack_unit() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": {
                    "backrow": [{ "def": "core-058", "radiant": true, "lane": 1 }],
                    "field": [{ "def": "core-t-rush", "lane": 1 }],
                },
                "p2": { "field": [{ "def": "core-019", "lane": 1 }] },
            }));

            // 6 into Midrange Menace's 9/9, and 9 back into a 6-health token: the token dies (R11: a unit
            // token that leaves the field ceases to exist rather than reaching a graveyard).
            let token = present(g.unit(P1, 1));

            g.attack(&token, "core-019");

            g.expect_stats("core-019", json!({ "health": 3, "maxHealth": 9 }))
                .expect_in_zone(&token, "gone");
        }
    }
}
