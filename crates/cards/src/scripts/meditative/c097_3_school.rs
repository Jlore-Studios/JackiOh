//! M #97.3 School (SPEC §8.8 row 97.3). (2) Unit, Token (printed Rare), 0/8 → 0/16.
//!   Base:    "Can't attack\nActivate: Summon a random ({cost}) Cost Unit."
//!   Radiant: "Can't attack\nActivate: Summon a random Radiant ({cost}) Cost Unit."
//!   Engine:  "Activate (R384): one uniform pick of the non-token Units of every set at that printed cost
//!            (R380, R60), placed per R64 (a Locked zone only when no open one is left, R688), with no Cry
//!            (R1); no zone to land in draws nothing (R129). Tunes: cost 1 ↑."

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-097-3";

/// §8.8: "Activate" is once a turn.
const USES: i32 = 1;

fn class(radiant: bool) -> ActivationDecl {
    ActivationDecl {
        id: "class".to_string(),
        label: "Summon a Unit".to_string(),
        uses: ActivationUses::Count(USES),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(move |ctx| {
            vec![summon_random(json_as(json!({
                "query": { "type": "Unit", "cost": param(&*ctx, "cost") },
                "radiant": radiant,
            })))]
        }),
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            activations: vec![class(false)],
            ..Script::default()
        },
        radiant: Script {
            activations: vec![class(true)],
            ..Script::default()
        },
    }
}

// M 97.3 School — SPEC §8.8 row 97.3, BUILD M10 row M 97.3: "Can't attack; Activate, once a turn: a random
// non-token (1) Cost Unit of any set summoned with no Cry; a full board draws nothing (R129); cost reads
// through `param()`; radiant 0/16 and the Unit is Radiant".
//
// "Any set" is the sets that ship (R380, R1420), so the pool the tests rebuild is the shipped sets' non-token
// Units at that printed cost. Every scenario keeps a 0-cost Spell in each hand and spares in the libraries,
// so R82 does not end the turn after the Activate.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use crate::js;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-010"; // (0) Spell.
    const SPARE: &str = "core-005"; // (1) Spell: library spare.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.

    /// Seeds per sweep.
    const SEEDS: usize = 30;

    fn setup(seed: &str, radiant: bool, p1_field: &[Value]) -> Scenario {
        let mut field = vec![json!({ "def": ID, "radiant": radiant })];
        field.extend(p1_field.iter().cloned());
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

    /// R380, R1420: the non-token Units of the sets that ship at that printed cost.
    fn pool(cost: i32) -> Vec<String> {
        crate::register_all();
        crate::CATALOG
            .values()
            .filter(|def| {
                def.type_ == CardType::Unit
                    && !def.token
                    && matches!(def.cost, CardCost::Fixed(printed) if printed == cost)
                    && jackioh_engine::set_ships(def.set)
            })
            .map(|def| def.id.clone())
            .collect()
    }

    #[test]
    fn is_a_2_cost_0_8_token_printed_rare_with_one_activate_on_each_face_and_a_declared_cost() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.id, "meditative-097-3");
        assert_eq!(js(&def.cost), json!(2));
        assert!(def.token);
        assert_eq!(def.printed_rarity, Some(PrintedRarity::Rare));
        assert_eq!(
            [
                js(&def.base.attack),
                js(&def.base.health),
                js(&def.radiant.attack),
                js(&def.radiant.health)
            ],
            [json!(0), json!(8), json!(0), json!(16)]
        );
        assert_eq!(js(&def.base.keywords), json!([{ "kind": "Can't attack" }]));
        assert_eq!(js(&def.radiant.keywords), json!([{ "kind": "Can't attack" }]));
        assert_eq!(
            js(&def.params),
            json!([{ "key": "cost", "base": 1, "radiant": 1, "better": "up", "step": 1, "min": 1 }])
        );
        let scripts = script();
        let uses = |script: &Script| {
            script
                .activations
                .iter()
                .map(|ability| js(&ability.uses))
                .collect::<Vec<_>>()
        };
        assert_eq!(uses(&scripts.base), vec![json!(1)]);
        assert_eq!(uses(&scripts.radiant), vec![json!(1)]);
    }

    mod base {
        use super::*;

        #[test]
        fn r384_r1_summons_one_random_non_token_1_cost_unit_with_no_cry_once_a_turn() {
            let mut s = setup("school-one", false, &[]);
            s.activate(ID, json!({}));
            assert!(
                !s.last_events()
                    .iter()
                    .any(|event| event.event_type() == GameEventType::CardPlayed)
            );
            let units = summoned(&s);
            assert_eq!(units.len(), 1);
            let def = crate::card_def(&units[0].def_id);
            assert_eq!(def.type_, CardType::Unit);
            assert!(!def.token);
            assert!(matches!(def.cost, CardCost::Fixed(1)));
            assert!(jackioh_engine::set_ships(def.set));
            assert!(!units[0].radiant);
            assert_eq!(units[0].controller, P1);
            s.expect_refused(|s| s.activate(ID, json!({})));
        }

        #[test]
        fn r60_r380_every_pick_is_in_the_pool_of_the_sets_that_ship() {
            let pool = pool(1);
            assert!(pool.len() > 1);
            let mut seen: Vec<String> = Vec::new();
            for n in 0..SEEDS {
                let mut s = setup(&format!("school-{n}"), false, &[]);
                s.activate(ID, json!({}));
                let units = summoned(&s);
                assert_eq!(units.len(), 1);
                assert!(pool.contains(&units[0].def_id), "{}", units[0].def_id);
                seen.push(units[0].def_id.clone());
            }
            assert!(seen.iter().any(|id| !id.starts_with("core-")));
        }

        #[test]
        fn r129_a_full_board_summons_nothing_and_draws_nothing() {
            let mut s = setup(
                "school-full",
                false,
                &[json!(VANILLA), json!(VANILLA), json!(VANILLA), json!(VANILLA)],
            );
            let cursor = s.state().rng_cursor;
            s.activate(ID, json!({}));
            assert!(summoned(&s).is_empty());
            assert_eq!(s.state().rng_cursor, cursor);
            // The use is spent all the same.
            s.expect_refused(|s| s.activate(ID, json!({})));
        }

        #[test]
        fn r688_with_every_open_zone_locked_the_summon_lands_in_the_leftmost_locked_one() {
            let mut s = setup("school-locked", false, &[]);
            for lane in 2..=5 {
                lock_zone(
                    s.state_mut(),
                    ZoneSlot {
                        player: P1,
                        row: Row::Units,
                        lane,
                    },
                );
            }
            s.activate(ID, json!({}));
            let units = summoned(&s);
            assert_eq!(units.len(), 1);
            assert_eq!(s.unit(P1, 2).map(|unit| unit.id), Some(units[0].id.clone()));
        }

        #[test]
        fn the_cost_reads_through_param() {
            let pool = pool(2);
            assert!(!pool.is_empty());
            let mut s = setup("school-param", false, &[]);
            set_param(s.card_mut(ID), "cost", 2);
            s.activate(ID, json!({}));
            let units = summoned(&s);
            assert_eq!(units.len(), 1);
            assert!(matches!(
                crate::card_def(&units[0].def_id).cost,
                CardCost::Fixed(2)
            ));
            assert!(pool.contains(&units[0].def_id));
        }

        #[test]
        fn cant_attack() {
            let mut s = setup("school-attack", false, &[]);
            s.expect_refused(|s| s.attack(ID, "hero"));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_unit_is_radiant_and_the_school_0_16() {
            let mut s = setup("school-radiant", true, &[]);
            s.expect_stats(ID, json!({ "attack": 0, "maxHealth": 16 }));
            s.activate(ID, json!({}));
            let units = summoned(&s);
            assert_eq!(units.len(), 1);
            assert!(units[0].radiant);
            assert!(pool(1).contains(&units[0].def_id));
            s.expect_refused(|s| s.activate(ID, json!({})));
        }
    }
}
