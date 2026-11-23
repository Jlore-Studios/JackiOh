//! SPEC §8.12 S+ #4.1 Profe Yates Prime (2) Unit, Human, Felinor, Acclaimed, Prime, Token (printed
//! Legendary), 6/6 → 12/12, a token of the Stinkyscape+ set, which #4 Profe Yates's Death shuffles into
//! its controller's deck since issue #4440 (R4440003, R4440004).
//!   Base:    "Cry: Exile a permanent with M, O, U, S and E in its name and text (CN cards are immune).
//!            Then summon a random Trap."
//!   Radiant: "Cry: Exile a permanent with M, O, U, S and E in its name and text (CN cards are immune).
//!            Then summon three random Traps."
//!
//! Engine: Profe Yates's text through v0.6.9n, as its card printed it (R4440004), so its faces are built
//! by #4's `exile_then` and `exile_traps`, which #4's kept script shares. The Cry exiles one declared
//! permanent of either side, a Unit or a backrow card (R81, TG #26 Path of Exile's pick, R3996000), that
//! is face-up, carries no CN tag, and whose name and the text of the face it wears hold each of M, O, U,
//! S and E in either case (R4154100): a face-down card is never offered, its name and text being hidden
//! (R33). With none such standing the play names none and the Cry summons its traps all the same (R90).
//! Then the traps: independent `summon_random` draws of non-token Traps/Field Traps of the shipped sets,
//! each face down in the leftmost open backrow zone (R3567001), one, three on the Radiant face, none of
//! them Radiant. No Activate. A Prime: any Prime pool (R1421).

use jackioh_engine::prelude::*;

use super::c004_profe_yates::{exile_then, exile_traps};

pub const ID: &str = "stinkyscapeplus-004-1";

pub fn script() -> CardScripts {
    CardScripts {
        base: exile_then(false, exile_traps),
        radiant: exile_then(true, exile_traps),
    }
}

// S+ #4.1 Profe Yates Prime — SPEC §8.12 row 4.1 (R4440004; R4154100, R3996000, R3567001).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, merged, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// S+ #4 Profe Yates, whose Catalyst it is the Prime of.
    const YATES: &str = "stinkyscapeplus-004"; // Profe Yates
    /// A Spell whose name holds M, O, U, S and E.
    const STOCKPILE: &str = "core-005"; // Stockpile
    /// A Field Spell whose name and text hold M, O, U, S and E: a backrow permanent for the Cry to exile.
    const MANA_WELL: &str = "core-006"; // Mana Well
    /// A Unit whose name holds M, O, U, S and E.
    const MUSHROOM: &str = "classicplus-020"; // Mushroom Power
    /// A Unit whose name holds no O, U, S or E, and which prints no text.
    const VANILLA: &str = "core-008"; // Mr. Vanilla
    /// A Unit whose M, O, U, S and E are split between its name and its text.
    const SPECIESIST: &str = "core-002"; // Speciesist
    /// A CN-tagged Unit whose name and text hold M, O, U, S and E.
    const LAR: &str = "classicplus-076"; // Brother Lar
    /// A Unit whose base face prints the U its Radiant face lacks.
    const TIMMY: &str = "core-011"; // Tempo Timmy
    /// A Trap whose name and text hold M, O, U, S and E, which no Unit's play reveals.
    const SHADOWSTEP: &str = "classic-014"; // Shadowstep

    /// The Traps the Cry summons, as the card prints them: one, three on the Radiant face.
    const BASE_TRAPS: usize = 1;
    const RADIANT_TRAPS: usize = 3;

    fn backrow_count(s: &Scenario, player: PlayerId) -> usize {
        s.state().players[player].backrow.iter().flatten().count()
    }

    fn exile_count(s: &Scenario, player: PlayerId) -> usize {
        s.pile(player, "exile").len()
    }

    /// p1's Prime (`radiant` its face) in hand beside p1's `mine` Units, facing p2's side `theirs`.
    fn facing(seed: &str, radiant: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "mana": 10,
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": mine,
                "library": filler(4),
            },
            "p2": merged(json!({ "library": filler(4) }), theirs),
        }))
    }

    /// The Prime played into zone 1, exiling p2's Mana Well.
    fn played(seed: &str, radiant: bool) -> Scenario {
        let mut s = facing(
            seed,
            radiant,
            json!([]),
            json!({ "backrow": [{ "def": MANA_WELL, "lane": 1 }] }),
        );
        let well = s.backrow(P2, 1).expect("p2's Mana Well").id;
        s.play(ID, json!({ "zone": 1, "targets": pick::id(&well) }));
        s
    }

    /// The cards the Prime's plays now offer to exile, each once.
    fn pickable(s: &Scenario) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for id in pick::instance_ids(&s.play_targets(ID)) {
            if !out.contains(&id) {
                out.push(id);
            }
        }
        out
    }

    /// p1's backrow after a played Prime: `count` face-down Traps, none of them Radiant.
    fn sets_traps(s: &Scenario, count: usize) {
        let traps: Vec<CardInstance> = s.state().players[P1].backrow.iter().flatten().cloned().collect();
        assert_eq!(traps.len(), count);
        for trap in &traps {
            assert!(trap.face_up != Some(true), "face down");
            assert!(!trap.radiant, "{}", trap.def_id);
            assert!(
                matches!(
                    crate::card_def(&trap.def_id).type_,
                    CardType::Trap | CardType::FieldTrap
                ),
                "a Trap"
            );
        }
    }

    #[test]
    fn r4440004_it_is_profe_yates_s_prime_a_token_printed_legendary_with_its_old_text() {
        crate::register_all();
        let prime = crate::card_def(ID);
        let yates = crate::card_def(YATES);
        assert_eq!(prime.name, format!("{} Prime", yates.name));
        assert!(prime.token);
        assert_eq!(prime.rarity, Rarity::Token);
        assert_eq!(prime.printed_rarity, Some(PrintedRarity::Legendary));
        assert_eq!(prime.cost, CardCost::Fixed(2));
        // Yates's tags, the Prime in place of the Catalyst.
        for tag in [Tag::Human, Tag::Felinor, Tag::Acclaimed, Tag::Prime, Tag::Token] {
            assert!(prime.tags.contains(&tag), "{tag:?}");
        }
        assert!(!prime.tags.contains(&Tag::Catalyst));
        assert!(yates.tags.contains(&Tag::Catalyst));
        let exile = "Cry: Exile a permanent with M, O, U, S and E in its name and text (CN cards are immune).";
        assert_eq!(prime.base.text, format!("{exile} Then summon a random Trap."));
        assert_eq!(prime.radiant.text, format!("{exile} Then summon three random Traps."));
        let scripts = script();
        assert!(scripts.base.activations.is_empty());
        assert!(scripts.radiant.activations.is_empty());
    }

    #[test]
    fn r4440004_r1421_it_is_in_the_prime_pool() {
        crate::register_all();
        let ids: Vec<String> = crate::query::query(&json_as(json!({ "tags": ["Prime"] })))
            .iter()
            .map(|def| def.id.clone())
            .collect();
        assert!(ids.contains(&ID.to_string()), "{ids:?}");
    }

    #[test]
    fn r4154100_r4440004_it_exiles_a_declared_permanent_whose_name_and_text_hold_m_o_u_s_and_e() {
        for radiant in [false, true] {
            // Speciesist's letters are split between its name and its text; Mushroom Power's name
            // holds all five.
            let mut s = facing(
                "s004-1-mouse",
                radiant,
                json!([{ "def": MUSHROOM, "lane": 2 }]),
                json!({ "field": [{ "def": SPECIESIST, "lane": 4 }], "hand": [STOCKPILE] }),
            );
            let theirs = s.unit_at(P2, 4).id;
            let mine = s.unit_at(P1, 2).id;
            assert_eq!(pickable(&s), vec![mine.clone(), theirs.clone()], "radiant {radiant}");
            s.play(ID, json!({ "zone": 1, "targets": pick::id(&theirs) }));
            s.expect_in_zone(theirs.as_str(), "exile");
            // One permanent, and nothing off the field: Stockpile stays in p2's hand.
            assert_eq!(exile_count(&s, P1) + exile_count(&s, P2), 1, "radiant {radiant}");
            assert!(s.hand(P2).iter().any(|card| card.def_id == STOCKPILE));
            assert!(s.unit(P1, 2).is_some_and(|unit| unit.def_id == MUSHROOM));
        }
    }

    #[test]
    fn r4154100_its_pick_may_be_a_permanent_of_its_own_side() {
        let mut s = facing(
            "s004-1-mouse-own",
            false,
            json!([{ "def": MUSHROOM, "lane": 2 }]),
            json!({ "field": [{ "def": VANILLA, "lane": 4 }] }),
        );
        let mine = s.unit_at(P1, 2).id;
        s.play(ID, json!({ "zone": 1, "targets": pick::id(&mine) }));
        s.expect_in_zone(mine.as_str(), "exile");
        assert!(s.unit(P2, 4).is_some(), "the enemy Unit stands");
    }

    #[test]
    fn r4154100_a_permanent_missing_a_letter_is_neither_offered_nor_accepted() {
        // Mr. Vanilla's name holds no O, U, S or E, and it prints no text.
        let mut s = facing(
            "s004-1-mouse-missing",
            false,
            json!([]),
            json!({ "field": [{ "def": VANILLA, "lane": 4 }, { "def": SPECIESIST, "lane": 5 }] }),
        );
        let vanilla = s.unit_at(P2, 4).id;
        let speciesist = s.unit_at(P2, 5).id;
        assert_eq!(pickable(&s), vec![speciesist]);
        s.expect_refused(|s| s.play(ID, json!({ "zone": 1, "targets": pick::id(&vanilla) })));
        s.expect_in_zone(vanilla.as_str(), "field");
    }

    #[test]
    fn r4154100_the_text_read_is_the_face_the_card_wears() {
        // Tempo Timmy's base face prints "Rush" (its U); its Radiant face prints "Charge" instead.
        let mut s = facing(
            "s004-1-mouse-face",
            false,
            json!([]),
            json!({
                "field": [
                    { "def": TIMMY, "lane": 4 },
                    { "def": TIMMY, "radiant": true, "lane": 5 },
                ],
            }),
        );
        let base = s.unit_at(P2, 4).id;
        let radiant = s.unit_at(P2, 5).id;
        assert_eq!(pickable(&s), vec![base]);
        s.expect_refused(|s| s.play(ID, json!({ "zone": 1, "targets": pick::id(&radiant) })));
    }

    #[test]
    fn r4154100_a_cn_card_is_immune_whatever_its_letters() {
        // Brother Lar's name and text hold all five, and it is CN.
        let mut s = facing(
            "s004-1-mouse-cn",
            false,
            json!([{ "def": LAR, "lane": 2 }]),
            json!({ "field": [{ "def": LAR, "lane": 4 }, { "def": SPECIESIST, "lane": 5 }] }),
        );
        let lar = s.unit_at(P2, 4).id;
        let speciesist = s.unit_at(P2, 5).id;
        assert_eq!(pickable(&s), vec![speciesist]);
        s.expect_refused(|s| s.play(ID, json!({ "zone": 1, "targets": pick::id(&lar) })));
        s.expect_in_zone(lar.as_str(), "field");
    }

    #[test]
    fn r4154100_a_face_down_card_is_never_offered_a_face_up_one_is() {
        // Shadowstep's name and text hold all five: face-down, on either side, it is hidden and never
        // offered (to its own controller neither); face-up it may be exiled.
        let mut s = scenario(json!({
            "seed": "s004-1-mouse-face-down",
            "p1": {
                "mana": 10,
                "hand": [ID],
                "backrow": [{ "def": SHADOWSTEP, "faceUp": false, "lane": 2 }],
                "library": filler(4),
            },
            "p2": {
                "backrow": [
                    { "def": SHADOWSTEP, "faceUp": false, "lane": 3 },
                    { "def": SHADOWSTEP, "faceUp": true, "lane": 4 },
                ],
                "library": filler(4),
            },
        }));
        let hidden = s.backrow(P2, 3).expect("the face-down Trap").id;
        let shown = s.backrow(P2, 4).expect("the face-up Trap").id;
        assert_eq!(pickable(&s), vec![shown.clone()]);
        s.expect_refused(|s| s.play(ID, json!({ "zone": 1, "targets": pick::id(&hidden) })));
        s.play(ID, json!({ "zone": 1, "targets": pick::id(&shown) }));
        s.expect_in_zone(shown.as_str(), "exile");
        s.expect_in_zone(hidden.as_str(), "field");
    }

    #[test]
    fn r4154100_with_none_eligible_the_play_names_none_and_the_traps_still_come() {
        for (radiant, traps) in [(false, BASE_TRAPS), (true, RADIANT_TRAPS)] {
            let mut s = scenario(json!({
                "seed": "s004-1-mouse-none",
                "p1": {
                    "mana": 10,
                    "hand": [{ "def": ID, "radiant": radiant }],
                    "field": [{ "def": LAR, "lane": 2 }],
                    "library": filler(4),
                },
                "p2": {
                    "field": [{ "def": VANILLA, "lane": 4 }],
                    "backrow": [{ "def": SHADOWSTEP, "faceUp": false, "lane": 3 }],
                    "library": filler(4),
                },
            }));
            assert!(s.can_play(ID), "radiant {radiant}");
            assert!(pickable(&s).is_empty(), "radiant {radiant}");
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(exile_count(&s, P1) + exile_count(&s, P2), 0);
            assert_eq!(backrow_count(&s, P1), traps, "radiant {radiant}");
        }
    }

    #[test]
    fn r4154100_while_one_is_eligible_every_play_names_one() {
        let s = facing(
            "s004-1-mouse-must",
            false,
            json!([]),
            json!({ "field": [{ "def": VANILLA, "lane": 4 }, { "def": SPECIESIST, "lane": 5 }] }),
        );
        let plays = nonempty(s.plays_of(ID), "the Prime's plays");
        for play in &plays {
            let ActionBody::Play { targets, .. } = play else {
                unreachable!()
            };
            assert_eq!(targets.as_ref().map(Vec::len), Some(1), "{play:?}");
        }
    }

    #[test]
    fn r3996000_the_exile_comes_first_so_a_trap_takes_the_zone_it_freed() {
        // p1's backrow is full; the Prime exiles one of p1's own Mana Wells and one Trap fits.
        let mut s = scenario(json!({
            "seed": "s004-1-then",
            "p1": {
                "mana": 10,
                "hand": [ID],
                "backrow": [MANA_WELL, MANA_WELL, MANA_WELL, MANA_WELL, MANA_WELL],
                "library": filler(4),
            },
            "p2": { "library": filler(4) },
        }));
        let well = s.backrow(P1, 3).expect("p1's Mana Well").id;
        s.play(ID, json!({ "zone": 1, "targets": pick::id(&well) }));
        s.expect_in_zone(well.as_str(), "exile");
        let trap = s.backrow(P1, 3).expect("a Trap in the freed zone");
        assert!(matches!(
            crate::card_def(&trap.def_id).type_,
            CardType::Trap | CardType::FieldTrap
        ));
        assert_eq!(backrow_count(&s, P1), 5);
    }

    #[test]
    fn r3567001_r4440004_base_sets_one_face_down_trap() {
        sets_traps(&played("s004-1-traps", false), BASE_TRAPS);
    }

    #[test]
    fn r3567001_r4440004_radiant_sets_three_traps_none_of_them_radiant() {
        sets_traps(&played("s004-1-traps-radiant", true), RADIANT_TRAPS);
    }

    #[test]
    fn it_is_a_6_6_and_radiant_12_12() {
        for (radiant, stats) in [(false, 6), (true, 12)] {
            let mut s = played("s004-1-stats", radiant);
            let prime = s.unit_at(P1, 1).id;
            s.expect_stats(&prime, json!({ "attack": stats, "health": stats }));
        }
    }
}
