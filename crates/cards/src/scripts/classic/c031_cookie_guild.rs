//! C #31 Cookie Guild (SPEC §8.6 row 31). (2) Unit, Human, Common, 2/4 → 4/8.
//!   Base:    "Cry: Recruit {units|Unit|Units} of ({costLimit}) Cost or less." — 1 Unit, (2)
//!   Radiant: "Cry: Recruit {units|Unit|Units} of ({costLimit}) Cost or less." — 3 Units, (2)
//!   Engine:  "Recruit (§6.3) filtered to Units whose deck cost (R65) is (2) or less, an X Unit counting
//!            0 there (R396); three scans on the Radiant face, as #69 Call to Arms does, stopping when
//!            the board is full. Tunes: cost limit 2 ↑; units 1 ↑."
//!
//! §6.3 Recruit is one top-down scan for the first permanent that matches, summoned per R64 into the
//! leftmost open zone with no Cry (R1), the library otherwise keeping its order. "Recruit N" is N scans
//! in order, as #69 Call to Arms writes it, and each scan takes the card it found, so the next finds the
//! next match further down. A scan that finds a Unit but no open zone leaves it in the library, which is
//! what "stopping when the board is full" is.
//!
//! The filter is `type: "Unit"` (the scan would otherwise take any permanent) and `costRange.max`: the
//! scan reads each library card's cost out of play (R65: its own cost plus its `costMod`, no player
//! discount; an X card 0, R396), which is `recruit`'s own reading.
//!
//! Both numbers are declared (`units`, `costLimit`, R386) and read through `param`; the faces differ
//! only in `units` (1 or 3), so both run this one script.

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-031";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            let filter = json!({ "type": "Unit", "costRange": { "max": param(&*ctx, "costLimit") } });
            let units = param(&*ctx, "units");
            (0..units)
                .map(|_| recruit(json_as(json!({ "filter": filter.clone(), "player": "self" }))))
                .collect()
        })),
        ..Script::default()
    };

    // The same script: the Radiant face's three scans are its declared `units`, which `param` reads off the running face.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C #31 Cookie Guild — SPEC §8.6 row 31, BUILD M9 Classic row C 31: "Cry: Recruit the first (2) Cost
// or less Unit from the top of your deck (its cost in the deck per R65: an X Unit 0, skipped unless the
// only valid target per R690), summoned without
// a Cry into your leftmost open zone; none, or a full unit row, → nothing; the deck otherwise keeps its
// order and no event carries a position; radiant 4/8: three scans, stopping when the row fills (as
// #69); its tuned numbers (cost limit, units) read through `param()` (R386)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const GUILD: &str = "classic-031";
    const FELINORS: &str = "core-012"; // (2) Unit 3/4, Cry: summon a copy of this.
    const BIG_D: &str = "core-001"; // (2) Unit.
    const TEMPO: &str = "core-011"; // (1) Unit 3/3.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MENACE: &str = "core-019"; // (3) Unit 9/9.
    const STOCKPILE: &str = "core-005"; // (1) Spell.
    const ARMOR: &str = "core-073"; // (2) Field Spell: a permanent, not a Unit.
    const BUFF_BILLY: &str = "classicplus-069"; // (X) Unit.
    const ANCHOR: &str = "core-010";

    fn library_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.pile(player, "library").into_iter().map(|card| card.def_id).collect()
    }

    fn unit_defs(s: &Scenario, player: PlayerId) -> Vec<Option<String>> {
        (1..=5).map(|lane| s.unit(player, lane).map(|card| card.def_id)).collect()
    }

    fn summoned_defs(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { def_id, .. } => Some(def_id.clone()),
                _ => None,
            })
            .collect()
    }

    /// The five lanes as TS writes them: a def id, or `null`.
    fn lanes(defs: [Option<&str>; 5]) -> Vec<Option<String>> {
        defs.into_iter().map(|def| def.map(str::to_string)).collect()
    }

    /// TS `stepParam(s.card(ref), key, steps)`: a Degrade or Upgrade of the live card in the state.
    fn step_card_param(s: &mut Scenario, card: &str, key: &str, steps: i32) {
        let id = s.card(card).id.clone();
        let live = find_instance_mut(s.state_mut(), &id).expect("the card is in the state");
        step_param(live, key, steps);
    }

    mod c_n31_cookie_guild {
        use super::*;

        #[test]
        fn runs_one_script_on_both_faces() {
            crate::register_all();
            assert_eq!(crate::card_def(GUILD).id, GUILD);
            let scripts = script();
            // TS `toBe(base)`: the Radiant face is the base Script itself, the same Cry.
            match (&scripts.base.cry, &scripts.radiant.cry) {
                (Some(base), Some(radiant)) => assert!(Arc::ptr_eq(base, radiant)),
                _ => panic!("both faces have the Cry"),
            }
        }

        mod base {
            use super::*;

            #[test]
            fn is_a_2_4() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "field": [GUILD], "hand": [ANCHOR] } }));
                s.expect_stats(GUILD, json!({ "attack": 2, "health": 4 }));
            }

            #[test]
            fn recruits_the_first_2_cost_or_less_unit_from_the_top_passing_over_bigger_units_spells_and_other_permanents() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [GUILD, ANCHOR], "library": [MENACE, STOCKPILE, ARMOR, FELINORS, TEMPO] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(GUILD, json!({}));

                assert_eq!(unit_defs(&s, P1), lanes([Some(GUILD), Some(FELINORS), None, None, None]));
                assert_eq!(library_defs(&s, P1), vec![MENACE, STOCKPILE, ARMOR, TEMPO]);
            }

            #[test]
            fn r1_the_recruited_unit_is_summoned_without_a_cry_a_duplicating_felinors_makes_no_copy() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [GUILD, ANCHOR], "library": [FELINORS] }, "p2": { "hand": [ANCHOR] } }));

                s.play(GUILD, json!({}));

                assert_eq!(summoned_defs(&s).iter().filter(|id| *id == FELINORS).count(), 1);
                assert_eq!(
                    unit_defs(&s, P1).iter().filter(|id| id.as_deref() == Some(FELINORS)).count(),
                    1
                );
            }

            #[test]
            fn r64_it_lands_in_your_leftmost_open_zone() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [GUILD, ANCHOR], "library": [TEMPO], "field": [{ "def": MENACE, "lane": 2 }] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(GUILD, json!({ "zone": 4 }));

                assert_eq!(unit_defs(&s, P1), lanes([Some(TEMPO), Some(MENACE), None, Some(GUILD), None]));
            }

            #[test]
            fn r65_the_cost_is_the_card_s_own_in_the_deck_a_3_unit_made_to_cost_1_less_is_recruited_a_1_unit_made_to_cost_2_more_is_not() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [GUILD, ANCHOR],
                        "library": [{ "def": TEMPO, "costMod": 2 }, { "def": MENACE, "costMod": -1 }],
                    },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(GUILD, json!({}));

                assert_eq!(unit_defs(&s, P1), lanes([Some(GUILD), Some(MENACE), None, None, None]));
                assert_eq!(library_defs(&s, P1), vec![TEMPO]);
            }

            #[test]
            fn r690_an_x_unit_is_skipped_when_a_non_x_match_sits_further_down_it_recruits_that_one_instead() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [GUILD, ANCHOR], "library": [MENACE, BUFF_BILLY, TEMPO] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(GUILD, json!({}));

                assert!(summoned_defs(&s).contains(&TEMPO.to_string()));
                assert!(!summoned_defs(&s).contains(&BUFF_BILLY.to_string()));
                assert_eq!(library_defs(&s, P1), vec![MENACE, BUFF_BILLY]);
            }

            #[test]
            fn r396_r690_an_x_unit_costs_0_in_the_deck_so_it_is_recruited_when_it_is_the_only_valid_target() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [GUILD, ANCHOR], "library": [MENACE, BUFF_BILLY] }, "p2": { "hand": [ANCHOR] } }));

                s.play(GUILD, json!({}));

                assert!(summoned_defs(&s).contains(&BUFF_BILLY.to_string()));
                assert_eq!(library_defs(&s, P1), vec![MENACE]);
            }

            #[test]
            fn with_no_match_in_the_deck_nothing_happens_and_the_deck_keeps_its_order() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [GUILD, ANCHOR], "library": [MENACE, STOCKPILE, ARMOR] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(GUILD, json!({}));

                assert_eq!(unit_defs(&s, P1), lanes([Some(GUILD), None, None, None, None]));
                assert_eq!(library_defs(&s, P1), vec![MENACE, STOCKPILE, ARMOR]);
            }

            #[test]
            fn with_a_full_unit_row_nothing_is_summoned_and_the_unit_stays_in_the_deck() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [GUILD, ANCHOR], "library": [TEMPO], "field": [MENACE, MENACE, MENACE, MENACE] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(GUILD, json!({}));

                assert_eq!(
                    unit_defs(&s, P1),
                    lanes([Some(MENACE), Some(MENACE), Some(MENACE), Some(MENACE), Some(GUILD)])
                );
                assert_eq!(library_defs(&s, P1), vec![TEMPO]);
            }

            #[test]
            fn no_event_carries_a_library_position_and_the_opponent_reads_only_the_summon() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [GUILD, ANCHOR], "library": [MENACE, TEMPO, BIG_D] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(GUILD, json!({}));

                assert!(!s.last_events().iter().any(|event| {
                    serde_json::to_value(event).expect("events serialise").get("position").is_some()
                }));
                let theirs = serde_json::to_string(&s.view(P2)).expect("views serialise");
                assert!(theirs.contains(TEMPO));
                assert!(!theirs.contains(MENACE));
                assert!(!theirs.contains(BIG_D));
            }

            #[test]
            fn r386_an_upgrade_of_units_makes_two_scans_of_the_cost_limit_a_3_unit_qualifies() {
                crate::register_all();
                let mut units = scenario(json!({
                    "p1": { "hand": [GUILD, ANCHOR], "library": [TEMPO, VANILLA, BIG_D] },
                    "p2": { "hand": [ANCHOR] },
                }));
                step_card_param(&mut units, GUILD, "units", 1);
                units.play(GUILD, json!({}));
                assert_eq!(unit_defs(&units, P1), lanes([Some(GUILD), Some(TEMPO), Some(VANILLA), None, None]));

                let mut limit = scenario(json!({ "p1": { "hand": [GUILD, ANCHOR], "library": [MENACE, TEMPO] }, "p2": { "hand": [ANCHOR] } }));
                step_card_param(&mut limit, GUILD, "costLimit", 1);
                limit.play(GUILD, json!({}));
                assert_eq!(unit_defs(&limit, P1), lanes([Some(GUILD), Some(MENACE), None, None, None]));
            }

            #[test]
            fn r386_a_degrade_of_the_cost_limit_to_1_passes_a_2_unit_over() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [GUILD, ANCHOR], "library": [FELINORS, TEMPO] }, "p2": { "hand": [ANCHOR] } }));
                step_card_param(&mut s, GUILD, "costLimit", -1);

                s.play(GUILD, json!({}));

                assert_eq!(unit_defs(&s, P1), lanes([Some(GUILD), Some(TEMPO), None, None, None]));
                assert_eq!(library_defs(&s, P1), vec![FELINORS]);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn is_a_4_8() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "field": [{ "def": GUILD, "radiant": true }], "hand": [ANCHOR] } }));
                s.expect_stats(GUILD, json!({ "attack": 4, "health": 8 }));
            }

            #[test]
            fn makes_three_top_down_scans_each_taking_the_next_match() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [{ "def": GUILD, "radiant": true }, ANCHOR],
                        "library": [FELINORS, MENACE, TEMPO, STOCKPILE, BIG_D, VANILLA],
                    },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(GUILD, json!({}));

                assert_eq!(
                    unit_defs(&s, P1),
                    lanes([Some(GUILD), Some(FELINORS), Some(TEMPO), Some(BIG_D), None])
                );
                assert_eq!(library_defs(&s, P1), vec![MENACE, STOCKPILE, VANILLA]);
            }

            #[test]
            fn fewer_matches_than_three_it_recruits_what_there_is() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": GUILD, "radiant": true }, ANCHOR], "library": [MENACE, TEMPO] },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(GUILD, json!({}));

                assert_eq!(unit_defs(&s, P1), lanes([Some(GUILD), Some(TEMPO), None, None, None]));
            }

            #[test]
            fn stops_when_the_row_fills_the_third_match_stays_in_the_deck() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [{ "def": GUILD, "radiant": true }, ANCHOR],
                        "library": [TEMPO, VANILLA, BIG_D],
                        "field": [MENACE, MENACE],
                    },
                    "p2": { "hand": [ANCHOR] },
                }));

                s.play(GUILD, json!({}));

                assert_eq!(
                    unit_defs(&s, P1),
                    lanes([Some(MENACE), Some(MENACE), Some(GUILD), Some(TEMPO), Some(VANILLA)])
                );
                assert_eq!(library_defs(&s, P1), vec![BIG_D]);
            }

            #[test]
            fn r386_a_degrade_of_units_makes_two_scans() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": GUILD, "radiant": true }, ANCHOR], "library": [TEMPO, VANILLA, BIG_D] },
                    "p2": { "hand": [ANCHOR] },
                }));
                step_card_param(&mut s, GUILD, "units", -1);

                s.play(GUILD, json!({}));

                assert_eq!(unit_defs(&s, P1), lanes([Some(GUILD), Some(TEMPO), Some(VANILLA), None, None]));
                assert_eq!(library_defs(&s, P1), vec![BIG_D]);
            }
        }
    }
}
