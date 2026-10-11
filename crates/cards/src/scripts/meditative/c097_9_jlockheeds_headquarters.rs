//! M #97.9 Jlockheed's Headquarters (SPEC §8.8 row 97.9, R1261). (4) Unit, Jlockeed, Token (printed
//! Mythic), 0/20 → 0/50.
//!   Base:    "Tribute 5, Indestructible, Can't attack\nActivate: Fill your board with random Jlockheed Units."
//!   Radiant: "Tribute 5, Indestructible, Can't attack\nActivate 2: Fill your board with random Radiant Jlockheed Units."
//!   Engine:  "Activate (R384; Activate 2 on the Radiant face). C+ #2 Groom Shroom's fill: each empty,
//!            unlocked, unreserved unit zone, left to right (R64), gets a random non-token Jlockeed Unit of
//!            every set (R380; this token is out, §5.1, R387), repeats allowed (R60), summoned, so with no
//!            Cry (R1); the Radiant face's second activation in a turn fills what the first left empty
//!            (R1261). Indestructible (R46; never Taunt, R347)."

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-097-9";

/// §8.8: "Activate" is once a turn, the Radiant face's "Activate 2" twice.
const BASE_USES: i32 = 1;
const RADIANT_USES: i32 = 2;

/// §6.3 "Tribute 5": five of your Units, or Sheep Tokens worth as many (§3.2).
const TRIBUTE_COST: i32 = 5;

/// "Jlockheed Units": the Jlockeed-tagged Units of every set (R278, R380).
fn jlockeed_units() -> Value {
    json!({ "type": "Unit", "tags": ["Jlockeed"] })
}

fn fill(uses: i32, radiant: bool) -> ActivationDecl {
    ActivationDecl {
        id: "fill".to_string(),
        label: "Fill your board".to_string(),
        uses: ActivationUses::Count(uses),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(move |ctx| {
            fill_board_zones(ctx.state, ctx.controller)
                .iter()
                .map(|zone| {
                    summon_random(json_as(json!({
                        "query": jlockeed_units(),
                        "lane": zone.lane,
                        "radiant": radiant,
                    })))
                })
                .collect()
        }),
    }
}

fn headquarters(uses: i32, radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE_COST),
            ..StaticFlags::default()
        }),
        activations: vec![fill(uses, radiant)],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: headquarters(BASE_USES, false),
        radiant: headquarters(RADIANT_USES, true),
    }
}

// M 97.9 Jlockheed's Headquarters — SPEC §8.8 row 97.9, BUILD M10 row M 97.9: "Tribute 5, Indestructible,
// Can't attack; Activate, once a turn: each empty, unlocked unit zone of yours, left to right, gets a
// random non-token Jlockeed Unit (MD-F17, R1261), with no Cry; a full or Locked board summons nothing and
// draws nothing; radiant 0/50, Activate 2 (a second use fills what the first left empty), and the Units are
// Radiant".
//
// The Jlockeed Units of the sets that ship are Jlockeed Shredder-10 (Core #13), Jlockheed's Lobbyist (C+ #48)
// and Jlockheed's J15 Fighter (C+ #51); the Headquarters is a token and never its own pick (§5.1, R387).
// Every scenario keeps spare cards in each hand and library, so R82 does not end the turn after a use.
#[cfg(test)]
mod tests {
    use super::{BASE_USES, ID, RADIANT_USES, TRIBUTE_COST, script};
    use crate::js;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-010"; // (0) Spell.
    const SPARE: &str = "core-005"; // (1) Spell: library spare.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    /// Every non-token Jlockeed Unit of the sets that ship, in catalog order (R380, R1420).
    const JLOCKEED_UNITS: [&str; 3] = ["core-013", "classicplus-048", "classicplus-051"];

    fn slot(lane: i32) -> ZoneSlot {
        ZoneSlot {
            player: P1,
            row: Row::Units,
            lane,
        }
    }

    /// The Headquarters in lane 1 with `others` beside it, spares keeping R82 away.
    fn setup(seed: &str, radiant: bool, others: &[Value]) -> Scenario {
        let mut field = vec![json!({ "def": ID, "radiant": radiant, "lane": 1 })];
        field.extend(others.iter().cloned());
        crate::scenario(json!({
            "seed": seed,
            "p1": { "hand": [FILLER], "field": field, "library": [SPARE, SPARE] },
            "p2": { "hand": [FILLER], "library": [SPARE, SPARE] },
        }))
    }

    /// The Units the last events summoned, in order.
    fn summoned(s: &Scenario) -> Vec<CardInstance> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { instance_id, .. } => Some(s.card(instance_id.as_str()).clone()),
                _ => None,
            })
            .collect()
    }

    fn count(s: &Scenario, kind: GameEventType) -> usize {
        s.last_events()
            .iter()
            .filter(|event| event.event_type() == kind)
            .count()
    }

    #[test]
    fn is_a_4_cost_0_20_jlockeed_token_printed_mythic_with_tribute_5_indestructible_and_activate_2_radiant() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.id, "meditative-097-9");
        assert_eq!(js(&def.cost), json!(4));
        assert!(def.token);
        assert_eq!(def.tags, vec![Tag::Jlockeed, Tag::Token]);
        assert_eq!(def.printed_rarity, Some(PrintedRarity::Mythic));
        assert_eq!(
            [
                js(&def.base.attack),
                js(&def.base.health),
                js(&def.radiant.attack),
                js(&def.radiant.health)
            ],
            [json!(0), json!(20), json!(0), json!(50)]
        );
        let keywords = json!([{ "kind": "Indestructible" }, { "kind": "Can't attack" }]);
        assert_eq!(js(&def.base.keywords), keywords);
        assert_eq!(js(&def.radiant.keywords), keywords);
        assert_eq!(def.params, None);
        let scripts = script();
        for face in [&scripts.base, &scripts.radiant] {
            assert_eq!(
                face.static_flags.as_ref().and_then(|flags| flags.tribute),
                Some(TRIBUTE_COST)
            );
        }
        let uses = |script: &Script| {
            script
                .activations
                .iter()
                .map(|ability| js(&ability.uses))
                .collect::<Vec<_>>()
        };
        assert_eq!(uses(&scripts.base), vec![json!(BASE_USES)]);
        assert_eq!(uses(&scripts.radiant), vec![json!(RADIANT_USES)]);
    }

    #[test]
    fn r380_the_jlockeed_units_of_the_sets_that_ship_are_the_three_the_tests_name() {
        crate::register_all();
        let pool: Vec<String> = crate::CATALOG
            .values()
            .filter(|def| {
                def.type_ == CardType::Unit
                    && !def.token
                    && def.tags.contains(&Tag::Jlockeed)
                    && jackioh_engine::set_ships(def.set)
            })
            .map(|def| def.id.clone())
            .collect();
        assert_eq!(pool, JLOCKEED_UNITS.map(String::from).to_vec());
    }

    mod base {
        use super::*;

        #[test]
        fn s6_3_refuses_the_play_without_five_units_to_tribute() {
            crate::register_all();
            let mut s = crate::scenario(json!({
                "p1": {
                    "hand": [ID, FILLER],
                    "field": [VANILLA, VANILLA, VANILLA, VANILLA],
                    "library": [SPARE, SPARE],
                    "mana": 10,
                },
                "p2": { "hand": [FILLER], "library": [SPARE, SPARE] },
            }));
            s.expect_refused_with(|s| s.play(ID, json!({})), "ribute");
            let four: Vec<String> = (1..=4).map(|lane| s.unit(P1, lane).expect("a Unit").id).collect();
            s.expect_refused(|s| s.play(ID, json!({ "tributes": four })));
        }

        #[test]
        fn r1261_r64_fills_every_empty_unit_zone_left_to_right_with_a_jlockeed_unit_and_no_cry() {
            let mut s = setup("hq-fill", false, &[json!({ "def": VANILLA, "lane": 3 })]);
            s.activate(ID, json!({}));
            let units = summoned(&s);
            assert_eq!(units.len(), 3);
            assert_eq!(
                [2, 4, 5]
                    .map(|lane| s.unit(P1, lane).map(|unit| unit.id))
                    .to_vec(),
                units.iter().map(|unit| Some(unit.id.clone())).collect::<Vec<_>>()
            );
            for unit in &units {
                assert!(JLOCKEED_UNITS.contains(&unit.def_id.as_str()), "{}", unit.def_id);
                assert!(!unit.radiant);
                assert_eq!(unit.controller, P1);
            }
            assert_eq!(s.unit(P1, 3).map(|unit| unit.def_id), Some(VANILLA.to_string()));
            assert_eq!(count(&s, GameEventType::CardPlayed), 0);
            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(ID.to_string()));
        }

        #[test]
        fn r1261_r60_repeats_allowed_and_all_three_reached() {
            let mut seen: Vec<String> = Vec::new();
            let mut repeated = false;
            for n in 0..12 {
                let mut s = setup(&format!("hq-{n}"), false, &[]);
                s.activate(ID, json!({}));
                let defs: Vec<String> = summoned(&s).into_iter().map(|unit| unit.def_id).collect();
                assert_eq!(defs.len(), 4);
                assert!(defs.iter().all(|def| JLOCKEED_UNITS.contains(&def.as_str())));
                repeated |= defs.iter().enumerate().any(|(at, def)| defs[..at].contains(def));
                seen.extend(defs);
            }
            assert!(repeated);
            for unit in JLOCKEED_UNITS {
                assert!(seen.iter().any(|def| def == unit), "{unit} never came");
            }
        }

        #[test]
        fn r1261_r64_skips_a_locked_zone_and_a_reserved_one() {
            let mut s = setup("hq-skip", false, &[]);
            lock_zone(s.state_mut(), slot(2));
            reserve_zone(s.state_mut(), slot(4));
            s.activate(ID, json!({}));
            assert_eq!(summoned(&s).len(), 2);
            assert!(s.unit(P1, 2).is_none());
            assert!(s.unit(P1, 3).is_some());
            assert!(s.unit(P1, 4).is_none());
            assert!(s.unit(P1, 5).is_some());
        }

        #[test]
        fn r1261_r129_a_full_board_summons_and_draws_nothing_and_the_use_is_spent() {
            let mut s = setup(
                "hq-full",
                false,
                &[json!(VANILLA), json!(VANILLA), json!(VANILLA), json!(VANILLA)],
            );
            let cursor = s.state().rng_cursor;
            s.activate(ID, json!({}));
            assert!(summoned(&s).is_empty());
            assert_eq!(s.state().rng_cursor, cursor);
            s.expect_refused(|s| s.activate(ID, json!({})));
        }

        #[test]
        fn r1261_r129_a_board_of_locked_zones_summons_and_draws_nothing() {
            let mut s = setup("hq-locked", false, &[]);
            for lane in 2..=5 {
                lock_zone(s.state_mut(), slot(lane));
            }
            let cursor = s.state().rng_cursor;
            s.activate(ID, json!({}));
            assert!(summoned(&s).is_empty());
            assert_eq!(s.state().rng_cursor, cursor);
        }

        #[test]
        fn r384_once_a_turn() {
            let mut s = setup("hq-once", false, &[json!({ "def": VANILLA, "lane": 2 })]);
            s.activate(ID, json!({}));
            s.expect_refused(|s| s.activate(ID, json!({})));
            s.end_turn().end_turn();
            s.activate(ID, json!({}));
        }

        #[test]
        fn cant_attack() {
            let mut s = setup("hq-attack", false, &[]);
            s.expect_refused(|s| s.attack(ID, "hero"));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1261_activate_2_fills_what_the_first_left_empty_with_radiant_units() {
            let mut s = setup("hq-radiant", true, &[]);
            s.expect_stats(ID, json!({ "attack": 0, "maxHealth": 50 }));
            lock_zone(s.state_mut(), slot(4));
            s.activate(ID, json!({}));
            let first = summoned(&s);
            assert_eq!(first.len(), 3);
            for lane in [2, 3, 5] {
                let unit = s.unit(P1, lane).expect("a filled zone");
                assert!(unit.radiant);
                assert!(JLOCKEED_UNITS.contains(&unit.def_id.as_str()));
            }
            assert!(s.unit(P1, 4).is_none());
            unlock_zone(s.state_mut(), slot(4));
            s.activate(ID, json!({}));
            let second = summoned(&s);
            assert_eq!(second.len(), 1);
            assert!(second[0].radiant);
            assert_eq!(s.unit(P1, 4).map(|unit| unit.id), Some(second[0].id.clone()));
            s.expect_refused(|s| s.activate(ID, json!({})));
        }
    }
}
