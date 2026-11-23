//! M #56 House Party (SPEC §8.8 row 56): (3) Spell, Common ((4) after v0.4.0f through v0.6.5b).
//!   Base:    "Fill your board with Right-house defenders. They have Brittle {brittle}, and so do their
//!            Reborn copies." — 1
//!   Radiant: "Fill your board with Radiant Right-house defenders. They have Brittle {brittle}, and so do
//!            their Reborn copies." — 1
//! Engine: §7's fill with core-003 Right-house defender (R64), a Locked zone included (R1136000), no
//!   Cry; radiant defenders are Radiant and carry Death summoning a base defender. The fill's `brittle`
//!   (R3996100) gives each defender Brittle {brittle}, the declared number read as it resolves, its turn
//!   cycle started as it lands (R385, R638), and the body its Reborn returns the same Brittle again, a
//!   crumble's spent count included (R441); the base defender a Radiant one's Death summons is a new
//!   card, no Reborn copy, and has none. Through v0.6.5b the defenders had no Brittle (issue #3996):
//!   `script_v0_6_5b` keeps that fill (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-056";

/// Core #3 Right-house defender, what the fill summons.
const DEFENDER: &str = "core-003";

/// `brittle`: whether the defenders have the declared Brittle and so do their Reborn copies (R3996100),
/// as since issue #3996.
fn party(radiant: bool, brittle: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let count = brittle.then(|| param(&*ctx, "brittle"));
            vec![fill_board(json_as(json!({
                "defId": DEFENDER,
                "radiant": radiant,
                "brittle": count,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: party(false, true),
        radiant: party(true, true),
    }
}

/// R767000: the scripts v0.6.5b and every version before it ran, the defenders with no Brittle (issue
/// #3996). `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_5b() -> CardScripts {
    CardScripts {
        base: party(false, false),
        radiant: party(true, false),
    }
}

// M #56 House Party — SPEC §8.8 row 56, BUILD M10 row M 56.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario, upgrade_number};

    const PARTY: &str = "meditative-056";
    const DEFENDER: &str = "core-003";
    const TIMMY: &str = "core-011";
    const HIT_JOB: &str = "core-016"; // Hit Job: destroy a Unit
    const FILLER: &str = "core-005";
    const SHOWDOWN: &str = "meditative-098"; // Showdown: Lock every lane but an empty one you choose.

    fn defs_of(s: &Scenario, player: PlayerId) -> Vec<Option<String>> {
        (1..=5)
            .map(|lane| s.unit(player, lane).map(|unit| unit.def_id.clone()))
            .collect()
    }

    /// The Brittle count `card` holds, or `None`.
    fn brittle(s: &Scenario, card: &CardInstance) -> Option<i32> {
        s.card(card).brittle.map(|counter| counter.count)
    }

    /// p1's House Party (base unless `radiant`) cast on an empty board, both sides holding cards so
    /// no turn ends by itself.
    fn partied(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": PARTY, "radiant": radiant }, FILLER], "mana": 4, "library": filler(6) },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }));
        for player in [PlayerId::P1, PlayerId::P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s.play(PARTY, json!({}));
        s
    }

    /// p1's five Units, lanes 1 to 5, failing on an empty zone.
    fn row(s: &Scenario) -> Vec<CardInstance> {
        (1..=5)
            .map(|lane| s.unit(PlayerId::P1, lane).expect("a Unit in every lane"))
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn costs_3_refused_with_2_mana() {
            let mut s = scenario(json!({ "p1": { "hand": [PARTY, FILLER] }, "p2": { "hand": [FILLER] } }));
            crate::expect_costs(&mut s, PARTY, 3, json!({}));
        }

        /// Issue #3996: the defenders have Brittle 1, and so do their Reborn copies. Each crumbles at the
        /// start of p1's next turn and its Reborn returns it with Brittle 1 again, though the crumble spent
        /// its count (R441); that copy, with no Reborn left, crumbles a turn on and stays gone.
        #[test]
        fn r3996100_the_defenders_have_brittle_1_and_so_do_their_reborn_copies() {
            let mut s = partied(false);
            let defenders = row(&s);
            for defender in &defenders {
                assert_eq!(defender.def_id, DEFENDER);
                assert_eq!(brittle(&s, defender), Some(1));
            }
            s.end_turn().end_turn();
            assert_eq!(s.state().active, PlayerId::P1);
            for (defender, copy) in defenders.iter().zip(row(&s)) {
                assert_eq!(copy.id, defender.id, "Reborn's body, in its own zone");
                assert_eq!(copy.reborn_spent, Some(true));
                assert_eq!(brittle(&s, &copy), Some(1));
            }
            s.end_turn().end_turn();
            assert_eq!(defs_of(&s, PlayerId::P1), vec![None; 5]);
            for defender in &defenders {
                s.expect_in_zone(defender, "graveyard");
            }
        }

        /// R386: the Brittle is the declared number, read as the Party resolves, the Reborn copy's too.
        #[test]
        fn r3996100_brittle_reads_through_param() {
            let mut s = scenario(json!({
                "p1": { "hand": [PARTY, HIT_JOB, FILLER], "mana": 10 },
                "p2": { "hand": [FILLER] },
            }));
            assert_eq!(upgrade_number(&mut s, PARTY, "brittle"), 2);
            s.play(PARTY, json!({}));
            let victim = s.unit_at(PlayerId::P1, 1);
            assert_eq!(brittle(&s, &victim), Some(2));
            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": victim.id }] }));
            assert_eq!(brittle(&s, &s.unit_at(PlayerId::P1, 1)), Some(2));
        }

        /// A defender killed before its count ticks comes back with Brittle 1 too.
        #[test]
        fn r3996100_a_defender_destroyed_in_its_first_turn_comes_back_with_brittle_1() {
            let mut s = scenario(json!({
                "p1": { "hand": [PARTY, HIT_JOB, FILLER], "mana": 10 },
                "p2": { "hand": [FILLER] },
            }));
            s.play(PARTY, json!({}));
            let victim = s.unit_at(PlayerId::P1, 3);
            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": victim.id }] }));
            let copy = s.unit_at(PlayerId::P1, 3);
            assert_eq!(copy.id, victim.id);
            assert_eq!(copy.reborn_spent, Some(true));
            assert_eq!(brittle(&s, &copy), Some(1));
        }

        /// R767000: through v0.6.5b the defenders had no Brittle (issue #3996).
        #[test]
        fn r767000_the_script_kept_for_v0_6_5b_fills_the_board_with_defenders_without_brittle() {
            crate::under_kept(super::super::ID, super::super::script_v0_6_5b(), || {
                for radiant in [false, true] {
                    let s = partied(radiant);
                    for defender in row(&s) {
                        assert_eq!(defender.def_id, DEFENDER);
                        assert_eq!(defender.radiant, radiant);
                        assert_eq!(brittle(&s, &defender), None);
                    }
                }
            });
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn costs_3_refused_with_2_mana() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": PARTY, "radiant": true }, FILLER] },
                "p2": { "hand": [FILLER] },
            }));
            crate::expect_costs(&mut s, PARTY, 3, json!({}));
        }

        /// Issue #3996: the Radiant defenders have Brittle 1, and so do their Reborn copies. At p1's next
        /// turn all five crumble together, their zones held for Reborn, so their Deaths find no room; a
        /// turn on the Reborn copies crumble with no Reborn, and each Death summons a base defender into
        /// the zone it left. That defender is a new card, no Reborn copy, and has no Brittle (R3996100).
        #[test]
        fn r3996100_the_radiant_defenders_and_their_reborn_copies_crumble_and_their_deaths_base_defenders_stay() {
            let mut s = partied(true);
            for defender in row(&s) {
                assert!(defender.radiant);
                assert_eq!(brittle(&s, &defender), Some(1));
            }
            s.end_turn().end_turn();
            for copy in row(&s) {
                assert!(copy.radiant);
                assert_eq!(copy.reborn_spent, Some(true));
                assert_eq!(brittle(&s, &copy), Some(1));
            }
            s.end_turn().end_turn();
            for summoned in row(&s) {
                assert_eq!(summoned.def_id, DEFENDER);
                assert!(!summoned.radiant);
                assert_eq!(summoned.reborn_spent, None);
                assert_eq!(brittle(&s, &summoned), None);
            }
        }
    }

    #[test]
    fn r64_fills_every_empty_unit_zone_left_to_right_with_base_defenders() {
        let mut s = scenario(json!({
            "p1": { "hand": [PARTY], "field": [{ "def": TIMMY, "lane": 2 }], "mana": 4 },
            "p2": { "hand": [FILLER] },
        }));
        s.play(PARTY, json!({}));
        let row = defs_of(&s, PlayerId::P1);
        assert_eq!(row[1], Some(TIMMY.to_string()));
        for lane in [0, 2, 3, 4] {
            assert_eq!(row[lane], Some(DEFENDER.to_string()));
        }
        let defender = s.unit(PlayerId::P1, 1).expect("a defender");
        assert!(!defender.radiant);
    }

    /// Issue #3136: Right-house defender gains Can't Defend on both faces (R2657000), so the board this
    /// fills is a board that never enters Defense Position; the card itself is not edited.
    #[test]
    fn r2657000_the_defenders_it_fills_the_board_with_have_can_t_defend_on_both_faces() {
        for radiant in [false, true] {
            let mut s = scenario(json!({
                // Mana over the Party's 3 keeps FILLER playable, so the turn is still p1's after it.
                "p1": { "hand": [{ "def": PARTY, "radiant": radiant }, FILLER], "mana": 5 },
                "p2": { "hand": [FILLER] },
            }));
            s.play(PARTY, json!({}));
            for lane in 1..=5 {
                let defender = s.unit(PlayerId::P1, lane).expect("a defender");
                assert_eq!(defender.def_id, DEFENDER);
                assert!(keywords_of(s.state(), &defender).contains(&Keyword::CantDefend));
                s.expect_refused_with(|s| s.switch_position(&defender), "Defense Position");
                assert_ne!(s.card(&defender).position, Some(Position::Def));
            }
        }
    }

    #[test]
    fn r64_a_full_board_takes_none() {
        crate::register_all();
        let full = [TIMMY, "core-t-rush", "core-t-sheep", "core-019", "core-020"];
        let mut s = scenario(json!({ "p1": { "hand": [PARTY], "field": full, "mana": 4 } }));
        s.play(PARTY, json!({}));
        assert_eq!(
            defs_of(&s, PlayerId::P1),
            full.iter().map(|id| Some(id.to_string())).collect::<Vec<_>>()
        );
        s.expect_in_zone(PARTY, "graveyard");
    }

    #[test]
    fn r1136000_after_showdown_it_fills_all_five_zones_the_locked_ones_included() {
        let mut s = scenario(json!({
            "p1": { "hand": [SHOWDOWN, PARTY, FILLER], "mana": 10, "library": [FILLER, FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
        }));
        s.play(SHOWDOWN, json!({ "modes": ["lane-3"] }));
        s.play(PARTY, json!({}));
        assert_eq!(defs_of(&s, PlayerId::P1), vec![Some(DEFENDER.to_string()); 5]);
        for lane in [1, 2, 4, 5] {
            let zone = ZoneSlot { player: PlayerId::P1, row: Row::Units, lane };
            assert!(is_locked(s.state(), zone), "lane {lane} stays Locked");
        }
    }

    #[test]
    fn radiant_the_defenders_are_radiant_and_one_dying_summons_a_base_defender() {
        crate::register_all();
        // Timmy holds lane 5, so the Party fills lanes 1–4; a Hit Job then clears lane 5, the one
        // zone a defender's Death can summon into.
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": PARTY, "radiant": true }, HIT_JOB, HIT_JOB, FILLER],
                "field": [{ "def": TIMMY, "lane": 5 }],
                "mana": 10,
            },
            "p2": { "hand": [FILLER] },
        }));
        s.play(PARTY, json!({}));
        for lane in 1..=4 {
            let defender = s.unit(PlayerId::P1, lane).expect("a radiant defender");
            assert_eq!(defender.def_id, DEFENDER);
            assert!(defender.radiant);
        }
        let timmy = s.unit(PlayerId::P1, 5).expect("Timmy").id.clone();
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": timmy }] }));
        assert!(s.unit(PlayerId::P1, 5).is_none());
        let victim = s.unit(PlayerId::P1, 1).expect("a radiant defender").id.clone();
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": victim }] }));
        // Reborn brings the Radiant one back into its own zone, held for it (R64, core #3); its
        // Death summons a base defender into the open lane 5.
        let reborn = s.unit(PlayerId::P1, 1).expect("the reborn defender");
        assert_eq!(reborn.id, victim);
        assert!(reborn.radiant);
        let summoned = s.unit(PlayerId::P1, 5).expect("the summoned defender");
        assert_eq!(summoned.def_id, DEFENDER);
        assert!(!summoned.radiant);
        // R3996100: the Reborn copy has Brittle 1; the defender its Death summoned has none.
        assert_eq!(brittle(&s, &reborn), Some(1));
        assert_eq!(brittle(&s, &summoned), None);
    }
}
