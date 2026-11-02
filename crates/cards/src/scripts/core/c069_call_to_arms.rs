//! #69 Call to Arms (SPEC §8.3, §6.3 Recruit, R1, R64, R65).
//!
//! Base: "Recruit 3 Units costing 1 or less"; Radiant: "2 or less". A cell that changes only a number
//! changes only that number (§8 Conventions), so the count stays 3 and only the cost ceiling moves.
//! A Spell's script hangs off `cry`, its on-resolve hook (§10.9).
//!
//! §6.3: the engine's `recruit` is one top-down scan, so "Recruit 3" is three in order; a card a scan
//! cannot place stays in the library, so a full board stops the recruiting.
//! R64: no named zone takes the leftmost empty, unlocked, unreserved zone of its row.
//! R1: Recruit never fires a Cry, which is why #12 Duplicating Felinors cannot fill a board for 2 mana.
//! R65: the filter reads each def's cost out of play; its `type: "Unit"` keeps out other permanents.
//! Tunes: costLimit 1 ↑ (Radiant 2); units 3 ↑ (R1748003). Through v0.4.4b the count was printed, not
//! declared; `script_v0_4_4b` keeps that (R767000).

use jackioh_engine::effects::{RecruitFilter, recruit};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-069";

/// §8.3's "Recruit 3" as v0.4.4b printed it, before it was the declared `units` (R1748003).
const RECRUITS: i32 = 3;

fn units_costing(max: i32) -> RecruitFilter {
    json_as(json!({ "type": "Unit", "costRange": { "max": max } }))
}

/// The declared number `costLimit` (R386), read off the running face; the count is the declared
/// `units`, or the printed 3 when `declared` is off.
fn call_to_arms(declared: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let filter = units_costing(param(&*ctx, "costLimit"));
            let units = if declared { param(&*ctx, "units") } else { RECRUITS };
            (0..units)
                .map(|_| recruit(json_as(json!({ "filter": filter, "player": "self" }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = call_to_arms(true);
    let radiant = base.clone();
    CardScripts { base, radiant }
}

/// R767000: through v0.4.4b its count was printed; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_4_4b() -> CardScripts {
    let base = call_to_arms(false);
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// #69 Call to Arms — SPEC §8.3, BUILD M4-T4.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;
    use crate::def_ids;

    const P1: PlayerId = PlayerId::P1;

    const CALL: &str = "core-069"; // Spell, cost 2.

    // The library fixtures, at their printed costs (R65).
    const TIMMY: &str = "core-011"; // Unit, cost 1, 3/3 — no Cry.
    const VANILLA: &str = "core-008"; // Unit, cost 1, 3/3 Immutable — no Cry.
    const DEFENDER: &str = "core-003"; // Unit, cost 1, 1/1 Divine Shield + Reborn — no Cry.
    const TOKEN_MAKER: &str = "core-015"; // Unit, cost 1, 1/1 — "Cry: summon a Rush Token" (R1's probe).
    const POINTMASTER: &str = "core-020"; // Unit, cost 2, 7/2 — above the base ceiling, inside the radiant one.
    const MENACE: &str = "core-019"; // Unit, cost 3 — above both ceilings.
    const TRAP: &str = "core-041"; // Sheepish, Trap, cost 1 — a cheap permanent that is not a Unit.
    const SPELL: &str = "core-010"; // Rapid Replenish, Spell, cost 0 — never a Recruit candidate (§6.3).
    const RUSH_TOKEN: &str = "core-t-rush"; // Unit token, cost 1 — a card that leaves a library only by a draw (R11).

    /// R82: a turn left with only end, concede and offer-draw auto-ends after every action, so each
    /// scenario keeps this free Spell in p1's hand, never played, as one legal action on the turn.
    const ANCHOR: &str = "core-010"; // Rapid Replenish, Spell, cost 0 — always an affordable play.

    /// `scenario(opts)` with ANCHOR appended to p1's hand.
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

    #[test]
    fn r1748003_units_moves_with_a_buff_or_a_nerf() {
        for (upgrade, units) in [(true, 4), (false, 2)] {
            let mut s = board(json!({ "p1": { "hand": [CALL], "library": [TIMMY, VANILLA, DEFENDER, TIMMY, VANILLA] } }));
            let moved = if upgrade {
                crate::upgrade_number(&mut s, CALL, "units")
            } else {
                crate::degrade_number(&mut s, CALL, "units")
            };
            assert_eq!(moved, units);
            s.play(CALL, json!({}));
            assert_eq!(unit_ids(&s).iter().flatten().count(), units as usize);
        }
    }

    #[test]
    fn r1748003_r767000_the_v0_4_4b_kept_script_keeps_its_printed_units() {
        crate::under_kept(CALL, super::script_v0_4_4b(), || {
            let mut s = board(json!({ "p1": { "hand": [CALL], "library": [TIMMY, VANILLA, DEFENDER, TIMMY, VANILLA] } }));
            crate::upgrade_number(&mut s, CALL, "units");
            s.play(CALL, json!({}));
            assert_eq!(unit_ids(&s).iter().flatten().count(), 3, "the kept script recruits its printed 3");
        });
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

            // Top down, past the Spell, Menace (cost 3), the Trap and Pointmaster (cost 2).
            assert_eq!(unit_ids(&s), row([Some(TIMMY), Some(VANILLA), Some(DEFENDER), None, None]));
        }

        #[test]
        fn s6_3_the_library_keeps_its_order_minus_exactly_the_three_cards_taken() {
            let mut s = board(json!({
                "p1": { "hand": [CALL], "library": [SPELL, MENACE, TIMMY, TRAP, VANILLA, POINTMASTER, DEFENDER, TIMMY] },
            }));
            s.play(CALL, json!({}));

            assert_eq!(def_ids(&s.pile(P1, "library")), [SPELL, MENACE, TRAP, POINTMASTER, TIMMY]);
        }

        #[test]
        fn s6_3_a_spell_is_never_recruited_and_neither_is_a_trap_recruit_takes_units_here() {
            let mut s = board(json!({ "p1": { "hand": [CALL], "library": [SPELL, TRAP, SPELL, TRAP] } }));
            s.play(CALL, json!({}));

            assert_eq!(unit_ids(&s), row([None, None, None, None, None]));
            assert_eq!(def_ids(&s.pile(P1, "library")), [SPELL, TRAP, SPELL, TRAP]);
            // §8 Conventions: the spell still counts as played, and it goes to the graveyard (§10.5).
            s.expect_in_zone(CALL, "graveyard").expect_mana(P1, 2);
        }

        #[test]
        fn r65_costing_1_or_less_skips_a_2_cost_unit_on_the_base_face() {
            let mut s = board(json!({ "p1": { "hand": [CALL], "library": [POINTMASTER, POINTMASTER, TIMMY] } }));
            s.play(CALL, json!({}));

            assert_eq!(unit_ids(&s), row([Some(TIMMY), None, None, None, None]));
            assert_eq!(def_ids(&s.pile(P1, "library")), [POINTMASTER, POINTMASTER]);
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

            // Two free zones, three scans: the third finds Timmy but no zone (`summon_existing` looks
            // for the zone first), so he stays in the library.
            assert_eq!(
                unit_ids(&s),
                row([Some(TIMMY), Some(TIMMY), Some(TIMMY), Some(DEFENDER), Some(VANILLA)])
            );
            assert_eq!(def_ids(&s.pile(P1, "library")), [TIMMY, POINTMASTER]);
        }

        #[test]
        fn r688_a_locked_unit_zone_takes_a_recruit_once_no_unlocked_zone_is_open() {
            let mut s = board(json!({ "p1": { "hand": [CALL], "library": [TIMMY, VANILLA, DEFENDER] } }));
            // §3.2 Lock is a zone flag; the harness exposes no setter.
            s.state_mut().players.p1.locks.units[0] = true;
            s.state_mut().players.p1.locks.units[1] = true;
            s.state_mut().players.p1.locks.units[3] = true;
            s.state_mut().players.p1.locks.units[4] = true;
            s.play(CALL, json!({}));

            // R688 prefers the one unlocked zone (lane 3), then fills the leftmost Locked ones.
            assert_eq!(unit_ids(&s), row([Some(VANILLA), Some(DEFENDER), Some(TIMMY), None, None]));
            assert!(def_ids(&s.pile(P1, "library")).is_empty());
        }

        #[test]
        fn r1_a_recruited_unit_fires_no_cry_me_and_mr_token_brings_no_rush_token() {
            let mut s = board(json!({ "p1": { "hand": [CALL], "library": [TOKEN_MAKER] } }));
            s.play(CALL, json!({}));

            assert_eq!(unit_ids(&s), row([Some(TOKEN_MAKER), None, None, None, None]));
            // A Rush Token would have taken lane 2 (R1).
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
            assert_eq!(def_ids(&s.pile(P1, "library")), [MENACE, SPELL, MENACE]);
        }

        #[test]
        fn s8_3_the_radiant_face_still_refuses_a_3_cost_unit() {
            let mut s = board(json!({ "p1": { "hand": [{ "def": CALL, "radiant": true }], "library": [MENACE, MENACE] } }));
            s.play(CALL, json!({}));

            assert_eq!(unit_ids(&s), row([None, None, None, None, None]));
            assert_eq!(def_ids(&s.pile(P1, "library")), [MENACE, MENACE]);
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
            assert_eq!(def_ids(&s.pile(P1, "library")), [TIMMY, VANILLA]);
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
            assert_eq!(def_ids(&s.pile(P1, "library")), [RUSH_TOKEN, MENACE]);
        }
    }
}
