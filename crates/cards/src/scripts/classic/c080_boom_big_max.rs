//! C #80 BOOM! Big Max (SPEC §8.6 row 80). (4) Unit, Legendary, 13/8 → 26/16.
//!   Base:    "Tribute 3, Rush, Trample, Indestructible"
//!   Radiant: "Tribute 3, Charge, Trample, Indestructible"
//!   Engine:  "Keywords only. Tribute 3 (§6.3, R101) may pay for its own zone (R391, §3.2);
//!            Indestructible gives no Taunt (R347). Balance patch 1 set the base attack to 13, so the
//!            Radiant 26 doubles it exactly (R275) with no exception left, and the Radiant face trades
//!            Rush for Charge to meet the keyword half. Tunes: none."
//!
//! The one thing the script carries is the Tribute cost, §6.3's `staticFlags.tribute` (as #66 The Rock
//! carries its own): the play validator (`playChoices`) refuses the play when the board cannot pay 3
//! (a Sheep Token plus one more pays, worth 2 + 1, R101) and pairs each zone with the paying sets that leave it open,
//! so a full row's one-card pile it tributes is its zone (R391). The keywords are the catalog's:
//!   Rush / Charge   — §4.1: Rush attacks Units the turn it enters; the Radiant's Charge the hero too.
//!   Trample         — §4.4 step 9, R63: the excess over the defender's health hits that side's hero.
//!   Indestructible  — §4.4 step 4 (it takes no damage), §4.5 and R46 (a destroy knocks it into Attack
//!                     Position), R347 (never Taunt), R69 (it still dies if its max health reaches 0).
//! Its proof: `test/classic/080-boom-big-max.test.ts`.

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-080";

/// §6.3 "Tribute 3": three of your Units, or one Sheep Token plus one more, worth 3 (§3.2).
const TRIBUTE_COST: i32 = 3;

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE_COST),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };

    // The same script: the Radiant face's 16 health and Charge are catalog data, and it keeps Tribute 3.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C #80 BOOM! Big Max (SPEC §8.6 row 80; BUILD M9 row C 80). (4) Unit, Legendary, 13/8 → 26/16:
// Tribute 3, Rush, Trample, Indestructible; Radiant: Tribute 3, Charge, Trample, Indestructible. Balance
// patch 1 set the base attack to 13, so the Radiant 26 doubles it exactly (R275) with no exception left.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BOOM: &str = "classic-080";
    const VANILLA: &str = "core-008"; // 4/4
    const TIMMY: &str = "core-011"; // 3/3
    const SHEEP: &str = "core-t-sheep"; // worth 2 Tributes
    const HIT_JOB: &str = "core-016";
    const STOCKPILE: &str = "core-005";

    /// The harness, after the catalog and every card script are registered (TS's harness did it on import).
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    fn def() -> &'static CardDef {
        crate::register_all();
        crate::card_def(ID)
    }

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialisable")
    }

    fn must<T>(value: Option<T>, what: &str) -> T {
        match value {
            Some(found) => found,
            None => panic!("missing: {what}"),
        }
    }

    /// TS `const SPARE: SideSetup = { hand: [STOCKPILE], library: [VANILLA, VANILLA] }`.
    fn spare() -> Value {
        json!({ "hand": [STOCKPILE], "library": spare_library() })
    }

    fn spare_library() -> Value {
        json!([VANILLA, VANILLA])
    }

    /// TS `{ ...side, ...SPARE }`: the side's keys, then SPARE's over them.
    fn with_spare(side: Value) -> Value {
        let mut merged = side;
        if let (Some(into), Value::Object(spare)) = (merged.as_object_mut(), spare()) {
            for (key, value) in spare {
                into.insert(key, value);
            }
        }
        merged
    }

    /// The `play` actions `legalActions` offers p1 for BOOM!, as JSON.
    fn boom_plays(s: &Scenario) -> Vec<Value> {
        let boom = s.card(BOOM).id.clone();
        legal_actions(s.state(), P1)
            .iter()
            .map(|action| js(action))
            .filter(|action| action["type"] == json!("play") && action["instanceId"] == json!(boom))
            .collect()
    }

    /// is tagged Acclaimed (SPEC §8.6 row 80, patch v0.2.Y)
    #[test]
    fn is_tagged_acclaimed_spec_s8_6_row_80_patch_v0_2_y() {
        assert_eq!(js(&def().tags), json!(["Acclaimed"]));
    }

    /// base
    mod base {
        use super::*;

        /// R101 Tribute 3: it can't be played without Units worth 3; two Units are not enough
        #[test]
        fn r101_tribute_3_it_can_t_be_played_without_units_worth_3_two_units_are_not_enough() {
            let mut none = scenario(json!({ "p1": { "hand": [BOOM, STOCKPILE], "library": spare_library() }, "p2": spare() }));
            none.expect_refused_with(|s| s.play(BOOM, json!({})), "Tribute 3");
            let mut two = scenario(json!({
                "p1": { "hand": [BOOM, STOCKPILE], "field": [TIMMY, VANILLA], "library": spare_library() },
                "p2": spare()
            }));
            two.expect_refused(|s| s.play(BOOM, json!({ "tributes": [TIMMY, VANILLA] })));
        }

        /// R101 `legalActions` agrees: no play with two Units, and with three only the trio that pays
        #[test]
        fn r101_legalactions_agrees_no_play_with_two_units_and_with_three_only_the_trio_that_pays() {
            let two = scenario(json!({
                "p1": { "hand": [BOOM, STOCKPILE], "field": [TIMMY, VANILLA], "library": spare_library() },
                "p2": spare()
            }));
            assert!(boom_plays(&two).is_empty());
            let three = scenario(json!({
                "p1": { "hand": [BOOM, STOCKPILE], "field": [TIMMY, VANILLA, VANILLA], "library": spare_library() },
                "p2": spare()
            }));
            let plays = boom_plays(&three);
            assert!(!plays.is_empty());
            let mut trio = vec![
                must(three.unit(P1, 1), "lane 1").id,
                must(three.unit(P1, 2), "lane 2").id,
                must(three.unit(P1, 3), "lane 3").id,
            ];
            trio.sort();
            for play in &plays {
                let mut tributes: Vec<String> = play["tributes"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|id| id.as_str().map(String::from))
                    .collect();
                tributes.sort();
                assert_eq!(tributes, trio);
            }
        }

        /// R101 three Units pay, and so does a Sheep Token plus one more, worth 2 + 1
        #[test]
        fn r101_three_units_pay_and_so_does_a_sheep_token_plus_one_more_worth_2_1() {
            let mut three = scenario(json!({
                "p1": { "hand": [BOOM, STOCKPILE], "field": [TIMMY, VANILLA, VANILLA], "library": spare_library() },
                "p2": spare()
            }));
            let second = must(three.unit(P1, 2), "lane 2").id;
            let third = must(three.unit(P1, 3), "lane 3").id;
            three.play(BOOM, json!({ "tributes": [TIMMY, second, third] }));
            three.expect_in_zone(BOOM, "field").expect_in_zone(TIMMY, "graveyard");
            let mut sheep = scenario(json!({
                "p1": { "hand": [BOOM, STOCKPILE], "field": [SHEEP, TIMMY], "library": spare_library() },
                "p2": spare()
            }));
            sheep.play(BOOM, json!({ "tributes": [SHEEP, TIMMY] }));
            sheep.expect_in_zone(BOOM, "field");
            let mut sheep_alone = scenario(json!({
                "p1": { "hand": [BOOM, STOCKPILE], "field": [SHEEP], "library": spare_library() },
                "p2": spare()
            }));
            sheep_alone.expect_refused(|s| s.play(BOOM, json!({ "tributes": [SHEEP] })));
        }

        /// R391 on a full row it may take a tributed one-card pile's zone
        #[test]
        fn r391_on_a_full_row_it_may_take_a_tributed_one_card_pile_s_zone() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [BOOM, STOCKPILE],
                    "field": [TIMMY, VANILLA, VANILLA, VANILLA, VANILLA],
                    "library": spare_library()
                },
                "p2": spare()
            }));
            let second = must(s.unit(P1, 2), "lane 2");
            let third = must(s.unit(P1, 3), "lane 3");
            s.play(BOOM, json!({ "zone": 1, "tributes": [TIMMY, second.id, third.id] }));
            assert_eq!(s.unit(P1, 1).map(|card| card.def_id), Some(BOOM.to_string()));
        }

        /// §4.1, R63 Rush and Trample: it attacks a Unit the turn it enters, the excess over the defender's health hitting their hero
        #[test]
        fn s4_1_r63_rush_and_trample_it_attacks_a_unit_the_turn_it_enters_the_excess_over_the_defender_s_health_hitting_their_hero()
         {
            let mut s = scenario(json!({
                "p1": { "hand": [BOOM, STOCKPILE], "field": [TIMMY, SHEEP, VANILLA], "library": spare_library() },
                "p2": with_spare(json!({ "field": [VANILLA] }))
            }));
            s.play(BOOM, json!({ "tributes": [TIMMY, SHEEP] }));
            s.expect_refused(|s| s.attack(BOOM, "hero"));
            let defender = must(s.unit(P2, 1), "p2's Vanilla").id;
            s.attack(BOOM, defender.as_str());
            // 13 into a 4/4: 4 to it, 9 to its controller's hero.
            s.expect_health(P2, 21);
            s.expect_stats(BOOM, json!({ "attack": 13, "health": 8 }));
        }

        /// R46 Indestructible: it takes no damage, and a destroy knocks it into Attack Position
        #[test]
        fn r46_indestructible_it_takes_no_damage_and_a_destroy_knocks_it_into_attack_position() {
            let mut s = scenario(json!({
                "p1": with_spare(json!({ "field": [{ "def": BOOM, "position": "DEF" }] })),
                "p2": { "field": [VANILLA], "hand": [HIT_JOB, STOCKPILE], "library": spare_library() },
                "active": "p2"
            }));
            s.attack(VANILLA, BOOM);
            assert_eq!(s.card(BOOM).damage, 0);
            let boom = s.card(BOOM).id.clone();
            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": boom }] }));
            s.expect_in_zone(BOOM, "field");
            assert_eq!(js(&s.stats(BOOM).position), json!("ATK"));
        }

        /// R347 it never has Taunt, not even in Defense Position
        #[test]
        fn r347_it_never_has_taunt_not_even_in_defense_position() {
            let s = scenario(json!({
                "p1": with_spare(json!({ "field": [{ "def": BOOM, "position": "DEF" }] })),
                "p2": spare()
            }));
            assert!(has_keyword(&s.stats(BOOM).keywords, KeywordKind::Indestructible));
            assert!(!has_keyword(&s.stats(BOOM).keywords, KeywordKind::Taunt));
        }
    }

    /// radiant
    mod radiant {
        use super::*;

        /// §8.6 26/16 Charge: it may attack the hero the turn it enters
        #[test]
        fn s8_6_26_16_charge_it_may_attack_the_hero_the_turn_it_enters() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": BOOM, "radiant": true }, STOCKPILE], "field": [SHEEP, TIMMY], "library": spare_library() },
                "p2": spare()
            }));
            s.play(BOOM, json!({ "tributes": [SHEEP, TIMMY] }));
            s.expect_stats(BOOM, json!({ "attack": 26, "health": 16 }));
            s.attack(BOOM, "hero");
            s.expect_health(P2, 4);
        }

        /// R101 the Radiant face keeps Tribute 3, Trample and Indestructible
        #[test]
        fn r101_the_radiant_face_keeps_tribute_3_trample_and_indestructible() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": BOOM, "radiant": true }, STOCKPILE], "field": [TIMMY], "library": spare_library() },
                "p2": spare()
            }));
            s.expect_refused(|s| s.play(BOOM, json!({ "tributes": [TIMMY] })));
            let field = scenario(json!({ "p1": { "field": [{ "def": BOOM, "radiant": true }] } }));
            let keywords = field.stats(BOOM).keywords;
            assert!(
                [KeywordKind::Charge, KeywordKind::Trample, KeywordKind::Indestructible]
                    .into_iter()
                    .all(|kind| has_keyword(&keywords, kind))
            );
            assert!(!has_keyword(&keywords, KeywordKind::Rush));
        }
    }
}
