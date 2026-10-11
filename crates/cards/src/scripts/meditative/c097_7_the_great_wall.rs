//! M #97.7 The Great Wall (SPEC §8.8 row 97.7, R1260). (2) Unit, Token (printed Epic), 0/50 → 0/100.
//!   Base:    "Tribute 3, Immutable, Can't attack\nCry: Lock every unit zone on your side."
//!   Radiant: "Tribute 3, Immutable, Armor 2, Can't attack\nCry: Lock every unit zone on your side."
//!   Engine:  "The Cry, when it is played (R1), Locks each of your five unit zones (§6.3 Lock, a `locked`
//!            each, one Locked already skipped), occupied ones included, since a Lock evicts nothing
//!            (§3.2); until one is Unlocked no Unit, a Stack play included, may be played there (R688), and
//!            'fill your board' effects (R64) fill nothing on that side, whoever's effect it is, while
//!            ordinary summons and moves still enter (R688). The Locks outlast the Wall (R1260)."
//!
//! One script serves both faces: Tribute 3 is its static flag, and Immutable (R23) and the Radiant face's
//! Armor 2 and 0/100 are catalog data.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-097-7";

/// §6.3 "Tribute 3": three of your Units, or one Sheep Token plus one more, worth 3 (§3.2).
const TRIBUTE_COST: i32 = 3;

fn the_great_wall() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE_COST),
            ..StaticFlags::default()
        }),
        // R1260: all five of the Wall's side, its own zone and the occupied ones included; `lock` skips
        // a zone Locked already and evicts no one.
        cry: Some(hook(|_ctx| {
            (1..=UNIT_ZONES)
                .map(|lane| {
                    lock(json_as(
                        json!({ "zone": { "of": "lane", "row": "units", "lane": lane } }),
                    ))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = the_great_wall();
    let radiant = the_great_wall();
    CardScripts { base, radiant }
}

// M 97.7 The Great Wall — SPEC §8.8 row 97.7, BUILD M10 row M 97.7: "Tribute 3, Immutable, Can't attack; Cry
// (played): all five of your unit zones are Locked, its own and the occupied ones included, nothing evicted
// (MD-F15, R1260); afterwards no Unit may be played into them and a fill fills nothing on your side, while a
// summon still lands; the Locks stay after the Wall leaves; an Unlock (M 27, C+ 77) frees them; radiant
// 0/100 and Armor 2".
//
// Every scenario keeps spare cards in each hand and library, so R82 does not end the turn after a play.
#[cfg(test)]
mod tests {
    use super::{ID, TRIBUTE_COST, script};
    use crate::js;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-010"; // (0) Spell.
    const SPARE: &str = "core-005"; // (1) Spell: library spare.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const TIMMY: &str = "core-011"; // (1) Unit 3/3.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const SOFTLOCK: &str = "classicplus-077"; // (2) Spell: Unlock every zone.
    const SCHOOL: &str = "meditative-097-3";
    const HEADQUARTERS: &str = "meditative-097-9";

    /// Mana to play everything a test casts in one turn.
    const MANA: i32 = 10;

    const LANES: [i32; 5] = [1, 2, 3, 4, 5];

    fn slot(player: PlayerId, lane: i32) -> ZoneSlot {
        ZoneSlot {
            player,
            row: Row::Units,
            lane,
        }
    }

    /// p1 holds the Wall and `hand`, with `field` on the board; spares keep R82 away.
    fn setup(radiant: bool, hand: &[&str], field: Value) -> Scenario {
        let mut cards = vec![json!({ "def": ID, "radiant": radiant })];
        cards.extend(hand.iter().map(|card| json!(card)));
        crate::scenario(json!({
            "p1": { "hand": cards, "field": field, "library": [SPARE, SPARE], "mana": MANA },
            "p2": { "hand": [FILLER], "library": [SPARE, SPARE] },
        }))
    }

    /// Three Vanillas in lanes 1 to 3 and a Timmy in lane 5.
    fn crowd() -> Value {
        json!([
            { "def": VANILLA, "lane": 1 },
            { "def": VANILLA, "lane": 2 },
            { "def": VANILLA, "lane": 3 },
            { "def": TIMMY, "lane": 5 },
        ])
    }

    /// Plays the Wall into lane 1 paying with the Units in lanes 1 to 3.
    fn play_wall(s: &mut Scenario) {
        let tributes: Vec<String> = (1..=3)
            .map(|lane| s.unit(P1, lane).expect("a Unit to tribute").id)
            .collect();
        s.play(ID, json!({ "zone": 1, "tributes": tributes }));
    }

    fn locked_events(s: &Scenario) -> usize {
        s.last_events()
            .iter()
            .filter(|event| event.event_type() == GameEventType::Locked)
            .count()
    }

    fn summoned(s: &Scenario) -> Vec<CardInstance> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { instance_id, .. } => Some(s.card(instance_id.as_str()).clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn is_a_2_cost_0_50_token_printed_epic_with_tribute_3_and_immutable_on_both_faces_armor_2_radiant() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.id, "meditative-097-7");
        assert_eq!(js(&def.cost), json!(2));
        assert!(def.token);
        assert_eq!(def.printed_rarity, Some(PrintedRarity::Epic));
        assert_eq!(
            [
                js(&def.base.attack),
                js(&def.base.health),
                js(&def.radiant.attack),
                js(&def.radiant.health)
            ],
            [json!(0), json!(50), json!(0), json!(100)]
        );
        assert_eq!(
            js(&def.base.keywords),
            json!([{ "kind": "Immutable" }, { "kind": "Can't attack" }])
        );
        assert_eq!(
            js(&def.radiant.keywords),
            json!([{ "kind": "Immutable" }, { "kind": "Armor", "n": 2 }, { "kind": "Can't attack" }])
        );
        assert_eq!(def.params, None);
        let scripts = script();
        for face in [&scripts.base, &scripts.radiant] {
            assert_eq!(
                face.static_flags.as_ref().and_then(|flags| flags.tribute),
                Some(TRIBUTE_COST)
            );
            assert!(face.cry.is_some());
        }
    }

    mod base {
        use super::*;

        #[test]
        fn s6_3_refuses_the_play_without_three_units_to_tribute() {
            let mut s = setup(false, &[], json!([VANILLA, VANILLA]));
            s.expect_refused_with(|s| s.play(ID, json!({})), "ribute");
            let pair: Vec<String> = (1..=2).map(|lane| s.unit(P1, lane).expect("a Unit").id).collect();
            s.expect_refused(|s| s.play(ID, json!({ "tributes": pair })));
        }

        #[test]
        fn r1260_its_cry_locks_all_five_unit_zones_its_own_and_occupied_ones_and_evicts_nothing() {
            let mut s = setup(false, &[], crowd());
            let timmy = s.unit(P1, 5).expect("the Timmy").id;
            play_wall(&mut s);
            for lane in LANES {
                assert!(is_locked(s.state(), slot(P1, lane)), "p1 lane {lane}");
                assert!(!is_locked(s.state(), slot(P2, lane)), "p2 lane {lane}");
            }
            assert_eq!(locked_events(&s), 5);
            assert_eq!(s.unit(P1, 5).map(|unit| unit.id), Some(timmy));
            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(ID.to_string()));
        }

        #[test]
        fn r1260_a_zone_locked_already_is_skipped() {
            let mut s = setup(false, &[], crowd());
            lock_zone(s.state_mut(), slot(P1, 4));
            play_wall(&mut s);
            assert_eq!(locked_events(&s), 4);
            for lane in LANES {
                assert!(is_locked(s.state(), slot(P1, lane)), "p1 lane {lane}");
            }
        }

        #[test]
        fn r1260_r688_no_unit_may_then_be_played_there() {
            let mut s = setup(false, &[VANILLA], crowd());
            play_wall(&mut s);
            s.expect_refused_with(|s| s.play(VANILLA, json!({ "zone": 2 })), "not open");
            s.expect_refused(|s| s.play(VANILLA, json!({})));
        }

        #[test]
        fn r1260_r688_a_fill_fills_nothing_while_a_summon_still_lands() {
            let mut s = setup(
                false,
                &[],
                json!([
                    { "def": VANILLA, "lane": 1 },
                    { "def": VANILLA, "lane": 2 },
                    { "def": VANILLA, "lane": 3 },
                    { "def": HEADQUARTERS, "lane": 4 },
                    { "def": SCHOOL, "lane": 5 },
                ]),
            );
            play_wall(&mut s);
            let cursor = s.state().rng_cursor;
            s.activate(HEADQUARTERS, json!({}));
            assert!(summoned(&s).is_empty());
            assert_eq!(s.state().rng_cursor, cursor);
            // The School's summon is no fill: every open zone is Locked, so it lands in the leftmost one (R688).
            s.activate(SCHOOL, json!({}));
            let units = summoned(&s);
            assert_eq!(units.len(), 1);
            assert_eq!(s.unit(P1, 2).map(|unit| unit.id), Some(units[0].id.clone()));
        }

        #[test]
        fn r1260_the_locks_outlast_the_wall() {
            let mut s = setup(false, &[HIT_JOB], crowd());
            play_wall(&mut s);
            s.end_turn().end_turn();
            let wall = s.card(ID).id.clone();
            s.play(
                HIT_JOB,
                json!({ "targets": [{ "pick": "instance", "instanceId": wall }] }),
            );
            assert!(s.unit(P1, 1).is_none_or(|unit| unit.def_id != ID));
            for lane in LANES {
                assert!(is_locked(s.state(), slot(P1, lane)), "p1 lane {lane}");
            }
        }

        #[test]
        fn r1260_an_unlock_frees_them() {
            let mut s = setup(false, &[SOFTLOCK], crowd());
            play_wall(&mut s);
            s.end_turn().end_turn();
            s.play(SOFTLOCK, json!({}));
            for lane in LANES {
                assert!(!is_locked(s.state(), slot(P1, lane)), "p1 lane {lane}");
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1260_the_radiant_wall_locks_all_five_and_has_armor_2_at_0_100() {
            let mut s = setup(true, &[], crowd());
            play_wall(&mut s);
            for lane in LANES {
                assert!(is_locked(s.state(), slot(P1, lane)), "p1 lane {lane}");
            }
            assert_eq!(locked_events(&s), 5);
            s.expect_stats(ID, json!({ "attack": 0, "maxHealth": 100 }));
            let wall = s.card(ID).clone();
            let keywords = js(&s.stats(&wall).keywords);
            assert!(
                keywords
                    .as_array()
                    .is_some_and(|list| list.contains(&json!({ "kind": "Armor", "n": 2 })))
            );
        }
    }
}
