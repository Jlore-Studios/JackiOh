//! S #174 Aerelret (SPEC §8.10 row 174; R1849009, R1875002): (4) Trap, Epic, of the Stinkyscape set.
//!   Base:    "{chance}% chance to reveal when an enemy Unit attacks: Summon a Warp Disruptor, then
//!            {paragons|Paragon Mk1|Paragon Mk1s}. The attack goes on."
//!   Radiant: the same, every summon Radiant.
//! Engine: S #22 Gatinsir's chance to reveal (`revealChance`, R1849009) on an attack an enemy Unit
//! declares, never a forced one (R121, R1875002): a hit fires it in §4.2 step 4's trap window, before
//! the combat. It summons an S #23 Warp Disruptor into its controller's open backrow zone, then
//! {paragons} S #72 Paragon Mk1s (6/1 Units with Charge, Flying and Ranged) into open unit zones
//! (R64), as many as fit, no Cry (R1); the attack then goes on as declared (ST9's AMBUSH, R1875002).
//! Radiant: all six Radiant (Paragon Mk1 12/2 with Pierce). Tunes: chance 30 ↑ (step 10, max 100),
//! paragons 5 ↑.

use jackioh_engine::effects::summon;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-174";

/// S #23 Warp Disruptor and S #72 Paragon Mk1, what it summons.
const WARP_DISRUPTOR: &str = "stinkyscape-023";
const PARAGON_MK1: &str = "stinkyscape-072";

/// The catalog's declared chance, read off the face that is up; the engine reads the card's own
/// `chance` over it as it rolls (R386).
fn declared_chance(radiant: bool) -> i32 {
    match crate::card_def(ID)
        .params
        .as_ref()
        .and_then(|params| params.iter().find(|entry| entry.key == "chance"))
    {
        Some(param) if radiant => param.radiant,
        Some(param) => param.base,
        None => panic!("{ID} declares chance (catalog params)"),
    }
}

/// R1875002: "when an enemy Unit attacks": an attack declared by a Unit its controller's opponent
/// controls; a forced attack is declared by no one (R121).
fn enemy_attacks(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    match event {
        GameEvent::AttackDeclared {
            attacker_id,
            forced: false,
            ..
        } => find_instance(&*ctx.state, attacker_id).is_some_and(|attacker| attacker.controller != ctx.controller),
        _ => false,
    }
}

fn aerelret(radiant: bool) -> Script {
    let ambush = TriggerDef::new(
        "aerelret",
        &[GameEventType::AttackDeclared],
        move |ctx, _event| {
            let paragons = param(&*ctx, "paragons");
            std::iter::once(WARP_DISRUPTOR)
                .chain((0..paragons).map(|_| PARAGON_MK1))
                .map(|def_id| summon(json_as(json!({ "defId": def_id, "radiant": radiant }))))
                .collect()
        },
    )
    .with_when(|ctx, event| enemy_attacks(ctx, event));
    Script {
        static_flags: Some(StaticFlags {
            reveal_chance: Some(declared_chance(radiant)),
            ..StaticFlags::default()
        }),
        triggers: vec![ambush],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: aerelret(false),
        radiant: aerelret(true),
    }
}

// S #174 Aerelret — SPEC §8.10 row 174, BUILD M12 row S 174: "set face-down, an enemy attack reveals it
// on a roll under 30 and not on one above; on a miss it stays set and no event is seen; revealed, a Warp
// Disruptor and then five Paragon Mk1 arrive for its controller before the combat, which goes on to its
// declared target; with three open unit zones three Paragons arrive; chance and paragons read through
// `param()`; radiant all six are Radiant (Paragon Mk1 12/2)".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::{ID, PARAGON_MK1, WARP_DISRUPTOR};

    const VANILLA: &str = "core-008"; // (1) 4/4.
    /// Moths to the Flame: enemy Units must attack it (R121).
    const MOTHS: &str = "core-009";
    /// Aerelret's printed chance, out of `PERCENT`, and its printed Paragons.
    const CHANCE: i32 = 30;
    const PARAGONS: usize = 5;
    /// S #23 Warp Disruptor's +2 Attack to every Unit.
    const DISRUPTOR_ATTACK: i32 = 2;

    /// p1 has a Vanilla in unit zone 1 and attacks; p2 has Aerelret (`radiant` its face) set in backrow
    /// zone 5 and `p2_field` in its unit zones. Turn 9, no turn ending by itself.
    fn trapped(seed: &str, radiant: bool, p2_field: Value) -> Scenario {
        let mut s = crate::scenario(json!({
            "seed": seed,
            "p1": { "field": [{ "def": VANILLA, "lane": 1 }], "library": crate::filler(4) },
            "p2": {
                "field": p2_field,
                "backrow": [{ "def": ID, "radiant": radiant, "lane": 5 }],
                "library": crate::filler(4),
            },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    /// Sets the rng so its next draw against `percent` hits or misses as `hit` says.
    fn roll_next(s: &mut Scenario, percent: i32, hit: bool) {
        let seed = s.state().seed.clone();
        let chance = f64::from(percent) / f64::from(PERCENT);
        let cursor = cursor_where(&seed, |cursor| (Rng::new(&seed, cursor).next() < chance) == hit);
        s.state_mut().rng_cursor = cursor;
    }

    fn fired(s: &Scenario) -> bool {
        s.last_events().iter().any(|event| event.event_type() == GameEventType::TrapFired)
    }

    /// p2's Paragon Mk1s on the field, each with whether it is Radiant.
    fn paragons(s: &Scenario) -> Vec<bool> {
        (1..=5)
            .filter_map(|lane| s.unit(P2, lane))
            .filter(|card| card.def_id == PARAGON_MK1)
            .map(|card| card.radiant)
            .collect()
    }

    fn disruptors(s: &Scenario) -> Vec<bool> {
        (1..=5)
            .filter_map(|lane| s.backrow(P2, lane))
            .filter(|card| card.def_id == WARP_DISRUPTOR)
            .map(|card| card.radiant)
            .collect()
    }

    /// The order the arrivals were summoned in.
    fn summoned_defs(s: &Scenario) -> Vec<String> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { instance_id, .. } => Some(s.card(instance_id.as_str()).def_id.clone()),
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r1849009_a_roll_under_30_reveals_it_and_the_ambush_lands_before_the_combat() {
            let mut s = trapped("aerelret-hit", false, json!([]));
            roll_next(&mut s, CHANCE, true);
            let attacker = s.unit_at(P1, 1);
            s.attack(&attacker, "hero");
            assert!(fired(&s));
            s.expect_in_zone(ID, "graveyard");
            let mut order = vec![WARP_DISRUPTOR.to_string()];
            order.extend(std::iter::repeat_n(PARAGON_MK1.to_string(), PARAGONS));
            assert_eq!(summoned_defs(&s), order);
            assert_eq!(disruptors(&s), vec![false]);
            assert_eq!(paragons(&s), vec![false; PARAGONS]);
            // R1875002: the attack goes on to its declared target, with the Disruptor's +2.
            s.expect_health(P2, HERO_HEALTH - (4 + DISRUPTOR_ATTACK));
        }

        #[test]
        fn r1849009_a_roll_over_30_leaves_it_set_and_nothing_is_seen() {
            let mut s = trapped("aerelret-miss", false, json!([]));
            roll_next(&mut s, CHANCE, false);
            let attacker = s.unit_at(P1, 1);
            s.attack(&attacker, "hero");
            assert!(!fired(&s));
            let set = s.backrow(P2, 5).expect("still set");
            assert_eq!(set.def_id, ID);
            assert_ne!(set.face_up, Some(true));
            assert!(summoned_defs(&s).is_empty());
            s.expect_health(P2, HERO_HEALTH - 4);
        }

        #[test]
        fn r1875002_a_forced_attack_rolls_nothing() {
            // Moths to the Flame on Aerelret's side makes p1's Vanilla attack it at p2's start of turn:
            // an enemy Unit's attack that no one declared (R121).
            let mut s = trapped("aerelret-forced", false, json!([{ "def": MOTHS, "lane": 2 }]));
            let cursor = s.state().rng_cursor;
            s.end_turn();
            assert!(s.events().iter().any(|event| matches!(
                event,
                GameEvent::AttackDeclared { forced: true, .. }
            )));
            assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::TrapFired));
            assert_eq!(s.backrow(P2, 5).map(|card| card.def_id), Some(ID.to_string()));
            assert_eq!(s.state().rng_cursor, cursor);
        }

        #[test]
        fn r1875002_its_own_sides_attack_does_not_roll() {
            let mut s = trapped("aerelret-own", false, json!([{ "def": VANILLA, "lane": 2 }]));
            let cursor = s.state().rng_cursor;
            s.end_turn();
            let own = s.unit_at(P2, 2);
            s.attack(&own, "hero");
            assert!(!fired(&s));
            assert_eq!(s.state().rng_cursor, cursor);
        }

        #[test]
        fn with_three_open_unit_zones_three_paragons_arrive() {
            let mut s = trapped(
                "aerelret-full",
                false,
                json!([{ "def": VANILLA, "lane": 2 }, { "def": VANILLA, "lane": 4 }]),
            );
            roll_next(&mut s, CHANCE, true);
            let attacker = s.unit_at(P1, 1);
            s.attack(&attacker, "hero");
            assert!(fired(&s));
            assert_eq!(paragons(&s), vec![false; 3]);
        }

        #[test]
        fn chance_and_paragons_read_through_param() {
            let mut s = trapped("aerelret-param", false, json!([]));
            let trap = s.backrow(P2, 5).expect("set").id;
            assert_eq!(crate::upgrade_number(&mut s, &trap, "chance"), CHANCE + 10);
            assert_eq!(crate::degrade_number(&mut s, &trap, "paragons"), 4);
            // A roll that misses 30 but hits 40.
            let seed = s.state().seed.clone();
            let cursor = cursor_where(&seed, |cursor| {
                let roll = Rng::new(&seed, cursor).next();
                roll >= f64::from(CHANCE) / f64::from(PERCENT) && roll < f64::from(CHANCE + 10) / f64::from(PERCENT)
            });
            s.state_mut().rng_cursor = cursor;
            let attacker = s.unit_at(P1, 1);
            s.attack(&attacker, "hero");
            assert!(fired(&s));
            assert_eq!(paragons(&s), vec![false; 4]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1849009_all_six_arrive_radiant() {
            let mut s = trapped("aerelret-radiant", true, json!([]));
            roll_next(&mut s, CHANCE, true);
            let attacker = s.unit_at(P1, 1);
            s.attack(&attacker, "hero");
            assert!(fired(&s));
            assert_eq!(disruptors(&s), vec![true]);
            assert_eq!(paragons(&s), vec![true; PARAGONS]);
            // A Paragon Mk1 12/2, with the Disruptor's +2.
            let paragon = s.unit_at(P2, 1);
            s.expect_stats(&paragon, json!({ "attack": 12 + DISRUPTOR_ATTACK, "health": 2 }));
        }

        #[test]
        fn r1849009_a_miss_leaves_it_set() {
            let mut s = trapped("aerelret-radiant-miss", true, json!([]));
            roll_next(&mut s, CHANCE, false);
            let attacker = s.unit_at(P1, 1);
            s.attack(&attacker, "hero");
            assert!(!fired(&s));
            assert_eq!(s.backrow(P2, 5).map(|card| card.def_id), Some(ID.to_string()));
        }
    }
}
