//! #69 Call to Arms (SPEC §8.3, §6.3 Recruit, R1, R64, R65).
//!
//! Base: "Recruit 3 Units costing 1 or less"; Radiant: "2 or less". A cell that changes only a number
//! changes only that number (§8 Conventions), so the count stays 3 and only the cost ceiling moves.
//! A Spell's script hangs off `cry`, the on-resolve hook of a Spell and the Cry of a permanent (§10.9).
//!
//! §6.3: the engine's `recruit` is one top-down scan, so "Recruit 3" is three in order. A card a scan
//! cannot place is never removed from the library (`summon_existing` looks for the zone first), so a
//! full board stops the recruiting and the library's order is otherwise untouched.
//! R64: no named zone takes the leftmost empty, unlocked, unreserved zone of its row.
//! R1: Recruit never fires a Cry, which is why #12 Duplicating Felinors cannot fill a board for 2 mana.
//! R65: the filter reads each def's cost out of play (`matches_filter` via `query_cost`).
//! The `type: "Unit"` filter matters as well as the cost: `recruit` would take any permanent.

use jackioh_engine::effects::{RecruitFilter, recruit};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-069";

/// §8.3: "Recruit 3", the same on both faces.
const RECRUITS: usize = 3;

fn units_costing(max: i32) -> RecruitFilter {
    json_as(json!({ "type": "Unit", "costRange": { "max": max } }))
}

/// The cost limit is the whole of the radiant text: the declared number `costLimit` (R386), read off
/// the face that is running.
fn call_to_arms() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let filter = units_costing(param(&*ctx, "costLimit"));
            (0..RECRUITS)
                .map(|_| recruit(json_as(json!({ "filter": filter, "player": "self" }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = call_to_arms();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// #69 Call to Arms — SPEC §8.3, BUILD M4-T4: "Three top-down recruits of cost ≤1, library order
// otherwise kept, stops when the board fills; radiant ≤2".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const CALL: &str = "core-069"; // Spell, cost 2.

    // The library fixtures. Costs are the printed ones (R65: read out of play).
    const TIMMY: &str = "core-011"; // Unit, cost 1, 3/3 — no Cry.
    const VANILLA: &str = "core-008"; // Unit, cost 1, 3/3 Immutable — no Cry.
    const DEFENDER: &str = "core-003"; // Unit, cost 1, 1/1 Divine Shield + Reborn — no Cry.
    const TOKEN_MAKER: &str = "core-015"; // Unit, cost 1, 1/1 — "Cry: summon a Rush Token" (R1's probe).
    const POINTMASTER: &str = "core-020"; // Unit, cost 2, 7/2 — above the base ceiling, inside the radiant one.
    const MENACE: &str = "core-019"; // Unit, cost 3 — above both ceilings.
    const TRAP: &str = "core-041"; // Sheepish, Trap, cost 1 — a cheap permanent that is not a Unit.
    const SPELL: &str = "core-010"; // Rapid Replenish, Spell, cost 0 — never a Recruit candidate (§6.3).
    const RUSH_TOKEN: &str = "core-t-rush"; // Unit token, cost 1 — a card that leaves a library only by a draw (R11).

    /// R82: a turn left with only end, concede and offer-draw auto-ends, and `reduce` checks that after
    /// EVERY action, so a play that empties the hand and leaves no unit hands the turn over and the
    /// numbers under test move. Every scenario below therefore keeps one free 0-cost Spell in p1's
    /// hand: never played, it only keeps one legal action on the turn.
    const ANCHOR: &str = "core-010"; // Rapid Replenish, Spell, cost 0 — always an affordable play.

    /// `scenario(opts)` with ANCHOR appended to p1's hand, the shipped cards registered first.
    fn board(opts: Value) -> Scenario {
        crate::register_all();
        let mut opts = opts;
        let mut p1 = opts.get("p1").cloned().unwrap_or_else(|| json!({}));
        let mut hand = p1.get("hand").and_then(Value::as_array).cloned().unwrap_or_default();
        hand.push(json!(ANCHOR));
        p1["hand"] = Value::Array(hand);
        opts["p1"] = p1;
        scenario(opts)
    }

    fn unit_ids(s: &Scenario) -> Vec<Option<String>> {
        (1..=5).map(|lane| s.unit(P1, lane).map(|card| card.def_id)).collect()
    }

    fn library_ids(s: &Scenario) -> Vec<String> {
        s.pile(P1, "library").into_iter().map(|card| card.def_id).collect()
    }

    fn row(ids: [Option<&str>; 5]) -> Vec<Option<String>> {
        ids.iter().map(|id| id.map(str::to_string)).collect()
    }

    #[test]
    fn r386_an_upgrade_recruits_a_2_cost_unit_and_a_radiant_degrade_stops_at_1() {
        for (radiant, upgrade, recruited) in [(false, true, POINTMASTER), (true, false, TIMMY)] {
            let mut s = board(json!({ "p1": { "hand": [{ "def": CALL, "radiant": radiant }], "library": [POINTMASTER, TIMMY] } }));
            let moved = if upgrade {
                crate::upgrade_number(&mut s, CALL, "costLimit")
            } else {
                crate::degrade_number(&mut s, CALL, "costLimit")
            };
            assert_eq!(moved, if upgrade { 2 } else { 1 });
            s.play(CALL, json!({}));
            assert_eq!(unit_ids(&s)[0].as_deref(), Some(recruited));
        }
    }

    mod call_to_arms {
        use super::*;

        #[test]
        fn r81_the_spell_asks_for_nothing_no_declared_target_and_no_mode() {
            crate::register_all();
            let scripts = super::super::script();
            assert!(scripts.base.targets.is_empty());
            assert!(scripts.base.modes.is_empty());
            assert!(scripts.radiant.targets.is_empty());
            assert!(scripts.base.cry.is_some());
        }

        // Base: three top-down scans of cost ≤ 1 (§6.3, R64, R65)

        #[test]
        fn s8_3_recruits_three_cost_1_units_top_down_into_lanes_1_3_s6_3_r64() {
            let mut s = board(json!({
                "p1": { "hand": [CALL], "library": [SPELL, MENACE, TIMMY, TRAP, VANILLA, POINTMASTER, DEFENDER, TIMMY] },
            }));
            s.play(CALL, json!({}));

            // Top down: the Spell is not a permanent, Menace costs 3, Timmy is the first match, the Trap is
            // not a Unit, Mr. Vanilla is the second, Pointmaster costs 2, the Right-house defender is third.
            assert_eq!(unit_ids(&s), row([Some(TIMMY), Some(VANILLA), Some(DEFENDER), None, None]));
        }

        #[test]
        fn s6_3_the_library_keeps_its_order_minus_exactly_the_three_cards_taken() {
            let mut s = board(json!({
                "p1": { "hand": [CALL], "library": [SPELL, MENACE, TIMMY, TRAP, VANILLA, POINTMASTER, DEFENDER, TIMMY] },
            }));
            s.play(CALL, json!({}));

            assert_eq!(library_ids(&s), [SPELL, MENACE, TRAP, POINTMASTER, TIMMY]);
        }

        #[test]
        fn s6_3_a_spell_is_never_recruited_and_neither_is_a_trap_recruit_takes_units_here() {
            let mut s = board(json!({ "p1": { "hand": [CALL], "library": [SPELL, TRAP, SPELL, TRAP] } }));
            s.play(CALL, json!({}));

            assert_eq!(unit_ids(&s), row([None, None, None, None, None]));
            assert_eq!(library_ids(&s), [SPELL, TRAP, SPELL, TRAP]);
            // §8 Conventions: the spell still counts as played, and it goes to the graveyard (§10.5).
            s.expect_in_zone(CALL, "graveyard").expect_mana(P1, 2);
        }

        #[test]
        fn r65_costing_1_or_less_skips_a_2_cost_unit_on_the_base_face() {
            let mut s = board(json!({ "p1": { "hand": [CALL], "library": [POINTMASTER, POINTMASTER, TIMMY] } }));
            s.play(CALL, json!({}));

            assert_eq!(unit_ids(&s), row([Some(TIMMY), None, None, None, None]));
            assert_eq!(library_ids(&s), [POINTMASTER, POINTMASTER]);
        }

        #[test]
        fn build_row_69_stops_when_the_board_fills_leaving_the_rest_of_the_library_in_order() {
            let mut s = board(json!({
                "p1": {
                    "hand": [CALL],
                    "field": [TIMMY, TIMMY, TIMMY],
                    "library": [DEFENDER, VANILLA, TIMMY, POINTMASTER],
                },
            }));
            s.play(CALL, json!({}));

            // Two free zones, three scans: the first two recruit, the third finds Timmy but no zone, so it
            // does nothing and leaves him in the library (`summon_existing` looks for the zone first).
            assert_eq!(
                unit_ids(&s),
                row([Some(TIMMY), Some(TIMMY), Some(TIMMY), Some(DEFENDER), Some(VANILLA)])
            );
            assert_eq!(library_ids(&s), [TIMMY, POINTMASTER]);
        }

        #[test]
        fn r688_a_locked_unit_zone_takes_a_recruit_once_no_unlocked_zone_is_open() {
            let mut s = board(json!({ "p1": { "hand": [CALL], "library": [TIMMY, VANILLA, DEFENDER] } }));
            // §3.2 Lock is a zone flag; the harness exposes no setter (reported as a harness gap).
            s.state_mut().players.p1.locks.units[0] = true;
            s.state_mut().players.p1.locks.units[1] = true;
            s.state_mut().players.p1.locks.units[3] = true;
            s.state_mut().players.p1.locks.units[4] = true;
            s.play(CALL, json!({}));

            // The first scan takes the one open unlocked zone (lane 3); with none left, the next scans
            // take the leftmost empty Locked lanes (R688 prefers unlocked, then fills Locked).
            assert_eq!(unit_ids(&s), row([Some(VANILLA), Some(DEFENDER), Some(TIMMY), None, None]));
            assert!(library_ids(&s).is_empty());
        }

        #[test]
        fn r1_a_recruited_unit_fires_no_cry_me_and_mr_token_brings_no_rush_token() {
            let mut s = board(json!({ "p1": { "hand": [CALL], "library": [TOKEN_MAKER] } }));
            s.play(CALL, json!({}));

            assert_eq!(unit_ids(&s), row([Some(TOKEN_MAKER), None, None, None, None]));
            // A Rush Token would have taken lane 2; §6.3 Summon fires no Cry, which is R1's whole point.
            assert!(!unit_ids(&s).contains(&Some("core-t-rush".to_string())));
        }

        #[test]
        fn s8_conventions_an_empty_library_leaves_the_spell_as_a_played_no_op() {
            let mut s = board(json!({ "p1": { "hand": [CALL], "library": [] } }));
            s.play(CALL, json!({}));

            assert_eq!(unit_ids(&s), row([None, None, None, None, None]));
            s.expect_in_zone(CALL, "graveyard")
                .expect_events(json!(["cardPlayed", "enteredGraveyard"]));
        }

        // Radiant: "2 or less" (§8 Conventions — only the number moves)

        #[test]
        fn s8_3_the_radiant_face_recruits_cost_2_units_as_well_still_three_of_them_still_top_down() {
            let mut s = board(json!({
                "p1": {
                    "hand": [{ "def": CALL, "radiant": true }],
                    "library": [MENACE, POINTMASTER, SPELL, TIMMY, MENACE, POINTMASTER],
                },
            }));
            s.play(CALL, json!({}));

            assert_eq!(
                unit_ids(&s),
                row([Some(POINTMASTER), Some(TIMMY), Some(POINTMASTER), None, None])
            );
            assert_eq!(library_ids(&s), [MENACE, SPELL, MENACE]);
        }

        #[test]
        fn s8_3_the_radiant_face_still_refuses_a_3_cost_unit() {
            let mut s = board(json!({ "p1": { "hand": [{ "def": CALL, "radiant": true }], "library": [MENACE, MENACE] } }));
            s.play(CALL, json!({}));

            assert_eq!(unit_ids(&s), row([None, None, None, None, None]));
            assert_eq!(library_ids(&s), [MENACE, MENACE]);
        }

        #[test]
        fn s8_3_the_radiant_face_stops_when_the_board_fills_too() {
            let mut s = board(json!({
                "p1": {
                    "hand": [{ "def": CALL, "radiant": true }],
                    "field": [TIMMY, TIMMY, TIMMY, TIMMY],
                    "library": [POINTMASTER, TIMMY, VANILLA],
                },
            }));
            s.play(CALL, json!({}));

            assert_eq!(
                unit_ids(&s),
                row([Some(TIMMY), Some(TIMMY), Some(TIMMY), Some(TIMMY), Some(POINTMASTER)])
            );
            assert_eq!(library_ids(&s), [TIMMY, VANILLA]);
        }
    }

    mod r218_a_recruit_passes_over_a_unit_token_card {
        use super::*;

        #[test]
        fn r218_call_to_arms_does_not_recruit_a_unit_token_card_out_of_the_library_r11() {
            // A Rush Token card in the library, as #33 shuffles in copies of one #75 gave (R34): it leaves a
            // library only by being drawn (R11), so the Recruit passes over it to the Timmy behind it.
            let mut s = board(json!({ "p1": { "hand": [CALL], "library": [RUSH_TOKEN, TIMMY, MENACE] } }));
            let token = s.pile(P1, "library").first().cloned();
            s.play(CALL, json!({}));

            assert_eq!(unit_ids(&s), row([Some(TIMMY), None, None, None, None]));
            let ids: Vec<String> = s.pile(P1, "library").into_iter().map(|card| card.id).collect();
            assert!(token.is_some_and(|token| ids.contains(&token.id)));
            assert_eq!(library_ids(&s), [RUSH_TOKEN, MENACE]);
        }
    }
}
