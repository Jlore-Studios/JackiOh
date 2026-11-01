//! S #22 Gatinsir (SPEC §8.10 row 22; R1849009, R1875002): (4) Trap, Epic, of the Stinkyscape set.
//!   Base:    "{chance}% chance to reveal when an enemy Unit attacks: Summon {disruptors|Warp Disruptor|Warp
//!            Disruptors}, then {phalanxes|Phalanx|Phalanxes}. The attack goes on."
//!   Radiant: the same, every summon Radiant.
//! Engine: set face-down, it answers an attack an enemy Unit declares (never a forced one, R121;
//! R1875002) with a chance to reveal (`revealChance`, R1849009): the match rng rolls once against the
//! declared `chance` out of 100, its controller's Luck rolling again (R987), and a miss leaves it set
//! and silent (R33). A hit fires it in §4.2 step 4's trap window, before the combat: it summons
//! {disruptors} S #23 Warp Disruptors into its controller's open backrow zones, then {phalanxes} S #82
//! Phalanxes into open unit zones (R64), a summon that finds no zone doing nothing. A summon that names
//! its card fires that card's Cry (R411): each Phalanx that arrived then runs its Cry for Gatinsir's
//! controller, in the order they arrived (R2176000; a Warp Disruptor prints none). The attack then goes
//! on as declared (ST9's AMBUSH, R1875002). The Warp Disruptors' aura acts at once.
//! Radiant: all of them Radiant (Phalanx 6/6). Tunes: chance 30 ↑ (step 10, max 100), disruptors 2 ↑,
//! phalanxes 2 ↑.

use jackioh_engine::effects::summon;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-022";

/// S #23 Warp Disruptor and S #82 Phalanx, what it summons.
const WARP_DISRUPTOR: &str = "stinkyscape-023";
const PHALANX: &str = "stinkyscape-082";

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

/// `count` summons of `def_id`, Radiant when `radiant`.
fn summons(def_id: &str, count: i32, radiant: bool) -> impl Iterator<Item = Effect> + '_ {
    (0..count).map(move |_| summon(json_as(json!({ "defId": def_id, "radiant": radiant }))))
}

/// R411, R2176000: the Cry of each Phalanx this ambush summoned, in the order they arrived.
fn phalanx_crys() -> Effect {
    for_each_card(ForEachCardArgs {
        cards: Arc::new(|ctx: &mut EffectContext<'_>| {
            summoned_so_far(ctx)
                .into_iter()
                .filter(|id| find_instance(&*ctx.state, id).is_some_and(|card| card.def_id == PHALANX))
                .collect()
        }),
        each: Arc::new(|instance_id: &str| trigger_cry(json_as(json!({ "instanceId": instance_id })))),
    })
}

fn gatinsir(radiant: bool) -> Script {
    let ambush = TriggerDef::new(
        "gatinsir",
        &[GameEventType::AttackDeclared],
        move |ctx, _event| {
            let mut effects: Vec<Effect> = summons(WARP_DISRUPTOR, param(&*ctx, "disruptors"), radiant).collect();
            effects.extend(summons(PHALANX, param(&*ctx, "phalanxes"), radiant));
            effects.push(phalanx_crys());
            effects
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
        base: gatinsir(false),
        radiant: gatinsir(true),
    }
}

// S #22 Gatinsir — SPEC §8.10 row 22, BUILD M12 row S 22: "set face-down, an enemy attack reveals it on a
// roll under 30 and not on one above (a seed of each); on a miss it stays set and no event is seen;
// revealed, two Warp Disruptors and then two Phalanxes arrive for its controller before the combat,
// which goes on to its declared target; with one open unit zone one Phalanx arrives; chance, disruptors
// and phalanxes read through `param()`; radiant all four are Radiant (Phalanx 6/6)". Each Phalanx it
// summons runs its Cry (R411, R2176000), and a forced attack rolls nothing (R1875002).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::{ID, PHALANX, WARP_DISRUPTOR};

    const VANILLA: &str = "core-008"; // (1) 4/4.
    /// Core #9 Moths to the Flame: "Start of turn: Every enemy Unit attacks this", a forced attack.
    const MOTHS: &str = "core-009";
    /// S #82 Phalanx's Cry: "Discover 2 … Weapons".
    const PHALANX_PICKS: usize = 2;
    /// Gatinsir's printed chance, out of `PERCENT`.
    const CHANCE: i32 = 30;

    /// p1 has a Vanilla in unit zone 1 and attacks; p2 has Gatinsir (`radiant` its face) set in backrow
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

    fn defs_in(s: &Scenario, row: Row) -> Vec<(String, bool)> {
        (1..=5)
            .filter_map(|lane| match row {
                Row::Units => s.unit(P2, lane),
                Row::Backrow => s.backrow(P2, lane),
            })
            .filter(|card| card.def_id != ID)
            .map(|card| (card.def_id, card.radiant))
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

    /// R411, R2176000: answers each Discover the Phalanxes' Crys open for Gatinsir's controller with the
    /// first card offered, and returns how many there were.
    fn answer_phalanx_discovers(s: &mut Scenario) -> usize {
        let mut asked = 0;
        while let Some(pending) = s.state().pending.clone() {
            assert_eq!((pending.kind, pending.player_id), (PromptKind::Discover, P2));
            let first = pending
                .options
                .iter()
                .find_map(|option| match &option.selection {
                    Selection::Mode { option } => Some(option.clone()),
                    _ => None,
                })
                .expect("a Weapon is offered");
            s.answer(json!(first));
            asked += 1;
        }
        asked
    }

    mod base {
        use super::*;

        #[test]
        fn r1849009_a_roll_under_30_reveals_it_and_the_ambush_lands_before_the_combat() {
            let mut s = trapped("gatinsir-hit", false, json!([]));
            roll_next(&mut s, CHANCE, true);
            let attacker = s.unit_at(P1, 1);
            s.attack(&attacker, "hero");
            assert!(fired(&s));
            assert_eq!(
                summoned_defs(&s),
                [WARP_DISRUPTOR, WARP_DISRUPTOR, PHALANX, PHALANX].map(String::from)
            );
            assert_eq!(
                defs_in(&s, Row::Backrow),
                vec![(WARP_DISRUPTOR.to_string(), false), (WARP_DISRUPTOR.to_string(), false)]
            );
            assert_eq!(
                defs_in(&s, Row::Units),
                vec![(PHALANX.to_string(), false), (PHALANX.to_string(), false)]
            );
            assert_eq!(answer_phalanx_discovers(&mut s), 2 * PHALANX_PICKS);
            // Spent once its summons and their Crys have resolved.
            s.expect_in_zone(ID, "graveyard");
            // R1875002: the attack goes on to its declared target, with the two Disruptors' +2 each.
            s.expect_health(P2, HERO_HEALTH - (4 + 2 + 2));
        }

        #[test]
        fn r2176000_each_phalanx_it_summons_runs_its_cry_for_its_controller_before_the_combat() {
            let mut s = trapped("gatinsir-cry", false, json!([]));
            roll_next(&mut s, CHANCE, true);
            let attacker = s.unit_at(P1, 1);
            s.attack(&attacker, "hero");
            // The first Phalanx's Cry asks first, the combat waiting on it (R411).
            let pending = s.state().pending.clone().expect("the first Phalanx's Discover");
            assert_eq!((pending.kind, pending.player_id), (PromptKind::Discover, P2));
            s.expect_health(P2, HERO_HEALTH);
            let phalanxes: Vec<String> = (1..=5)
                .filter_map(|lane| s.unit(P2, lane))
                .filter(|card| card.def_id == PHALANX)
                .map(|card| card.id)
                .collect();
            let bare: Vec<i32> = phalanxes.iter().map(|id| s.stats(id.as_str()).health).collect();
            assert_eq!(answer_phalanx_discovers(&mut s), 2 * PHALANX_PICKS);
            // Each took two Weapons, which raise its Health (R1862002).
            for (id, before) in phalanxes.iter().zip(bare) {
                assert!(s.stats(id.as_str()).health > before, "{id} was armed");
            }
            s.expect_health(P2, HERO_HEALTH - (4 + 2 + 2));
        }

        #[test]
        fn r2176000_a_phalanx_that_has_left_the_field_by_then_runs_nothing() {
            let mut s = trapped("gatinsir-cry-left", false, json!([]));
            roll_next(&mut s, CHANCE, true);
            let attacker = s.unit_at(P1, 1);
            s.attack(&attacker, "hero");
            // The first Phalanx's Cry asks; the second Phalanx leaves the field before its own runs.
            assert!(s.state().pending.is_some(), "the first Phalanx's Discover");
            let lanes: Vec<i32> = (1..=5)
                .filter(|lane| s.unit(P2, *lane).is_some_and(|card| card.def_id == PHALANX))
                .collect();
            assert_eq!(lanes.len(), 2);
            let gone = s.unit_at(P2, lanes[1]).id;
            let pile = s.state_mut().players[P2].units[(lanes[1] - 1) as usize].take().expect("its pile");
            s.state_mut().players[P2].graveyard.extend(pile);
            // Only the first Phalanx's two picks are asked; the one that left runs nothing.
            assert_eq!(answer_phalanx_discovers(&mut s), PHALANX_PICKS);
            s.expect_in_zone(gone.as_str(), "graveyard");
        }

        #[test]
        fn r1875002_a_forced_attack_rolls_nothing() {
            // Moths to the Flame on Gatinsir's side makes p1's Vanilla attack it at p2's start of turn: an
            // enemy Unit's attack that no one declared (R121).
            let mut s = trapped("gatinsir-forced", false, json!([{ "def": MOTHS, "lane": 2 }]));
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
        fn r1849009_a_roll_over_30_leaves_it_set_and_nothing_is_seen() {
            let mut s = trapped("gatinsir-miss", false, json!([]));
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
        fn r1875002_its_own_sides_attack_does_not_roll() {
            let mut s = trapped("gatinsir-own", false, json!([{ "def": VANILLA, "lane": 2 }]));
            let cursor = s.state().rng_cursor;
            s.end_turn();
            let own = s.unit_at(P2, 2);
            s.attack(&own, "hero");
            assert!(!fired(&s));
            assert_eq!(s.state().rng_cursor, cursor);
        }

        #[test]
        fn with_one_open_unit_zone_one_phalanx_arrives() {
            let field: Vec<Value> = (1..=4).map(|lane| json!({ "def": VANILLA, "lane": lane })).collect();
            let mut s = trapped("gatinsir-full", false, json!(field));
            roll_next(&mut s, CHANCE, true);
            let attacker = s.unit_at(P1, 1);
            s.attack(&attacker, "hero");
            assert!(fired(&s));
            assert_eq!(s.unit_at(P2, 5).def_id, PHALANX);
            assert_eq!(
                summoned_defs(&s),
                [WARP_DISRUPTOR, WARP_DISRUPTOR, PHALANX].map(String::from)
            );
            assert_eq!(answer_phalanx_discovers(&mut s), PHALANX_PICKS);
        }

        #[test]
        fn chance_disruptors_and_phalanxes_read_through_param() {
            let mut s = trapped("gatinsir-param", false, json!([]));
            let trap = s.backrow(P2, 5).expect("set").id;
            assert_eq!(crate::upgrade_number(&mut s, &trap, "chance"), CHANCE + 10);
            assert_eq!(crate::upgrade_number(&mut s, &trap, "disruptors"), 3);
            assert_eq!(crate::degrade_number(&mut s, &trap, "phalanxes"), 1);
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
            assert_eq!(
                summoned_defs(&s),
                [WARP_DISRUPTOR, WARP_DISRUPTOR, WARP_DISRUPTOR, PHALANX].map(String::from)
            );
            assert_eq!(answer_phalanx_discovers(&mut s), PHALANX_PICKS);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1849009_all_four_arrive_radiant() {
            let mut s = trapped("gatinsir-radiant", true, json!([]));
            roll_next(&mut s, CHANCE, true);
            let attacker = s.unit_at(P1, 1);
            s.attack(&attacker, "hero");
            assert!(fired(&s));
            assert_eq!(
                defs_in(&s, Row::Backrow),
                vec![(WARP_DISRUPTOR.to_string(), true), (WARP_DISRUPTOR.to_string(), true)]
            );
            assert_eq!(
                defs_in(&s, Row::Units),
                vec![(PHALANX.to_string(), true), (PHALANX.to_string(), true)]
            );
            let phalanx = s.unit_at(P2, 1);
            // A Phalanx 6/6 (8/8 until #2657), with the two Disruptors' +2 each, before its Cry arms it.
            s.expect_stats(&phalanx, json!({ "attack": 6 + 2 + 2, "health": 6 }));
            assert_eq!(answer_phalanx_discovers(&mut s), 2 * PHALANX_PICKS);
        }

        #[test]
        fn r1849009_a_miss_leaves_it_set() {
            let mut s = trapped("gatinsir-radiant-miss", true, json!([]));
            roll_next(&mut s, CHANCE, false);
            let attacker = s.unit_at(P1, 1);
            s.attack(&attacker, "hero");
            assert!(!fired(&s));
            assert_eq!(s.backrow(P2, 5).map(|card| card.def_id), Some(ID.to_string()));
        }
    }
}
