//! TG #26 Path of Exile (SPEC §8.11 row TG 26; §3.1, §6.3 Exile, R11, R55, R81, R2198003): (4) Spell,
//! Rare, a card of the held Training Grounds set (R1420, R2194000).
//!   Base:    "Exile a permanent. Draw {draw|card|cards}." — 1.
//!   Radiant: "Cleave / Exile a permanent. Draw {draw|card|cards}." — 2.
//!
//! Engine: Core #34 Collateral Damage's verbs: one declared target, any permanent on either side's
//! field (a Unit or a backrow card, R81), exiled past Indestructible, a unit token ceasing to exist
//! (R11, R55); then a `draw`. The Radiant face prints Cleave: on a Spell's board pick, R2198003 reads it
//! as the permanents beside the pick, lanes N−1 and N+1 in its row on its side, exiled FIRST while the
//! pick still stands in its lane (`exile_adjacent_to`, Core #34 Radiant's), a neighbour Immune to Spells
//! passed by.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-026";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "backrow"] }))]
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            targets: targets(),
            cry: Some(hook(|ctx| {
                vec![
                    exile(json_as(json!({ "target": { "of": "chosen" } }))),
                    draw(json_as(json!({ "count": param(&*ctx, "draw") }))),
                ]
            })),
            ..Script::default()
        },
        radiant: Script {
            targets: targets(),
            cry: Some(hook(|ctx| {
                vec![
                    // R2198003: Cleave first, while the pick is still in its lane for `adjacent` to read.
                    exile_adjacent_to(json_as(json!({ "target": { "of": "chosen" } }))),
                    exile(json_as(json!({ "target": { "of": "chosen" } }))),
                    draw(json_as(json!({ "count": param(&*ctx, "draw") }))),
                ]
            })),
            ..Script::default()
        },
    }
}

// TG #26 Path of Exile — SPEC §8.11 row TG 26, BUILD M13 row TG 26.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario, upgrade_number};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PATH: &str = "traininggrounds-026";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const ROCK: &str = "core-066"; // (4) Unit 10/10, Indestructible.
    const TRAP: &str = "core-041"; // Sheepish, a Trap.
    const RUSH_TOKEN: &str = "core-t-rush";
    const MEGABASE: &str = "stinkyscape-116"; // Unit, Immune to Spells.

    fn board(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        board_with(radiant, mine, theirs, json!([]))
    }

    /// `board` with `backrow` set in p2's backrow.
    fn board_with(radiant: bool, mine: Value, theirs: Value, backrow: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": PATH, "radiant": radiant }], "field": mine, "mana": 10, "library": filler(6) },
            "p2": { "hand": filler(1), "field": theirs, "backrow": backrow, "library": filler(6) },
        }))
    }

    fn on(card: &CardInstance) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": card.id }] })
    }

    mod base {
        use super::*;

        #[test]
        fn exiles_an_enemy_unit_an_indestructible_one_included() {
            let mut s = board(false, json!([]), json!([ROCK]));
            let rock = s.unit_at(P2, 1);
            s.play(PATH, on(&rock));
            s.expect_in_zone(&rock, "exile");
        }

        #[test]
        fn exiles_a_backrow_card_a_face_down_trap_included() {
            let mut s = board_with(false, json!([]), json!([]), json!([{ "def": TRAP, "lane": 2 }]));
            let trap = s.backrow(P2, 2).expect("the set Trap");
            assert_ne!(trap.face_up, Some(true));
            s.play(PATH, on(&trap));
            s.expect_in_zone(&trap, "exile");
        }

        #[test]
        fn can_pick_your_own_permanent() {
            let mut s = board(false, json!([VANILLA]), json!([]));
            let mine = s.unit_at(P1, 1);
            s.play(PATH, on(&mine));
            s.expect_in_zone(&mine, "exile");
        }

        #[test]
        fn a_unit_token_ceases_to_exist() {
            let mut s = board(false, json!([]), json!([RUSH_TOKEN]));
            let token = s.unit_at(P2, 1);
            s.play(PATH, on(&token));
            s.expect_in_zone(&token, "gone");
        }

        #[test]
        fn then_draws_one() {
            let mut s = board(false, json!([]), json!([VANILLA]));
            let theirs = s.unit_at(P2, 1);
            let held = s.hand(P1).len();
            s.play(PATH, on(&theirs));
            assert_eq!(s.hand(P1).len(), held, "the Path left and one card came in");
        }

        #[test]
        fn the_draw_reads_through_param() {
            let mut s = board(false, json!([]), json!([VANILLA]));
            assert_eq!(upgrade_number(&mut s, PATH, "draw"), 2);
            let theirs = s.unit_at(P2, 1);
            let held = s.hand(P1).len();
            s.play(PATH, on(&theirs));
            assert_eq!(s.hand(P1).len(), held + 1);
        }

        #[test]
        fn the_base_face_exiles_the_pick_alone() {
            let mut s = board(false, json!([]), json!([VANILLA, VANILLA, VANILLA]));
            let (left, pick, right) = (s.unit_at(P2, 1), s.unit_at(P2, 2), s.unit_at(P2, 3));
            s.play(PATH, on(&pick));
            s.expect_in_zone(&pick, "exile");
            s.expect_in_zone(&left, "field");
            s.expect_in_zone(&right, "field");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2198003_the_pick_and_its_neighbours_in_its_row_and_side_are_exiled_nothing_across() {
            let mut s = board_with(
                true,
                json!([{ "def": VANILLA, "lane": 2 }]),
                json!([
                    { "def": VANILLA, "lane": 1 },
                    { "def": VANILLA, "lane": 2 },
                    { "def": VANILLA, "lane": 3 },
                    { "def": VANILLA, "lane": 4 },
                ]),
                json!([{ "def": TRAP, "lane": 2 }]),
            );
            let (left, pick, right, far) = (s.unit_at(P2, 1), s.unit_at(P2, 2), s.unit_at(P2, 3), s.unit_at(P2, 4));
            let across = s.unit_at(P1, 2);
            let behind = s.backrow(P2, 2).expect("the set Trap");
            s.play(PATH, on(&pick));
            for gone in [&left, &pick, &right] {
                s.expect_in_zone(gone, "exile");
            }
            for kept in [&far, &across, &behind] {
                s.expect_in_zone(kept, "field");
            }
        }

        #[test]
        fn r2198003_a_backrow_pick_takes_its_backrow_neighbours() {
            let mut s = board_with(
                true,
                json!([]),
                json!([{ "def": VANILLA, "lane": 2 }]),
                json!([{ "def": TRAP, "lane": 2 }, { "def": TRAP, "lane": 3 }]),
            );
            let (beside, pick) = (s.backrow(P2, 2).unwrap(), s.backrow(P2, 3).unwrap());
            let front = s.unit_at(P2, 2);
            s.play(PATH, on(&pick));
            s.expect_in_zone(&pick, "exile");
            s.expect_in_zone(&beside, "exile");
            s.expect_in_zone(&front, "field");
        }

        #[test]
        fn r2198003_a_neighbour_immune_to_spells_is_passed_by() {
            let mut s = board(true, json!([]), json!([{ "def": MEGABASE, "lane": 1 }, { "def": VANILLA, "lane": 2 }]));
            let (immune, pick) = (s.unit_at(P2, 1), s.unit_at(P2, 2));
            s.play(PATH, on(&pick));
            s.expect_in_zone(&pick, "exile");
            s.expect_in_zone(&immune, "field");
        }

        #[test]
        fn draws_two() {
            let mut s = board(true, json!([]), json!([VANILLA]));
            let theirs = s.unit_at(P2, 1);
            let held = s.hand(P1).len();
            s.play(PATH, on(&theirs));
            assert_eq!(s.hand(P1).len(), held + 1);
        }

        #[test]
        fn the_catalog_lists_cleave_on_the_radiant_face() {
            crate::register_all();
            let def = catalog::def_of(None, PATH);
            assert!(def.radiant.keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Cleave));
            assert!(!def.base.keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Cleave));
        }
    }
}
