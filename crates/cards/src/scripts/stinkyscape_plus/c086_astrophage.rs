//! SPEC §8.12 S+ #86 Astrophage (Unit, Swarm 0/1 → 0/2, cost 0, Common), a card of the held Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Evolve: When your opponent plays a Unit.
//!            Evolve: Place a Plague Counter on and steal a stat point from each Unit."
//!   Radiant: "Evolve: When your opponent plays a Unit.
//!            Evolve: Place a Plague Counter on and steal {unit} stat points from each Unit." (unit 3)
//!
//! "Evolve when your opponent plays a Unit" is the card's own `cardPlayed` trigger evolving itself
//! (R3536001), once an opponent's Unit has been played (R3536000: once ever). The Evolve text reads the
//! board as the hook runs: "each Unit" is every other Unit on both sides that a harmful walk reaches
//! (`cards_in_scope_aimed`, `excludeSelf`, so a Dormant card is passed by, R1839000), the Unit just played
//! among them. It places one Plague Counter on each (`place_plague_each`, R471), then steals the face's
//! points from each (R3541000): per Unit, Attack while it has any, then Health, never taking the
//! Health it needs to stay alive (the steal alone kills nothing), and Astrophage gains what was
//! taken, stat for stat, in one buff after the thefts. A Unit with nothing to give still gets its Counter.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-086";

/// "steal a stat point": the base face's one point from each Unit (the Radiant face's is its declared `unit`).
const BASE_POINTS: i32 = 1;

/// "Place a Plague Counter": one on each Unit.
const PLAGUE_COUNTERS: i32 = 1;

/// "each Unit": every other Unit, on both sides.
fn each_other_unit() -> BoardScope {
    json_as(json!({ "side": "any", "rows": ["units"], "excludeSelf": true }))
}

/// R3541000: what a Unit gives up of `points`: Attack while it has any, then Health down to 1.
fn stolen_from(view: &UnitView, points: i32) -> (i32, i32) {
    let attack = view.attack.clamp(0, points);
    let health = (points - attack).clamp(0, (view.health - 1).max(0));
    (attack, health)
}

/// "Evolve: Place a Plague Counter on and steal [N] stat point[s] from each Unit."
fn drain_each_unit() -> Hook {
    hook(|ctx| {
        let points = if ctx.radiant {
            param(ctx, "unit")
        } else {
            BASE_POINTS
        };
        let scope = each_other_unit();
        let mut effects = vec![place_plague_each(PlacePlagueEachArgs {
            scope: scope.clone(),
            amount: PLAGUE_COUNTERS,
        })];
        let (mut gained_attack, mut gained_health) = (0, 0);
        for victim in cards_in_scope_aimed(ctx, &scope, TargetAim::Harm) {
            let (attack, health) = stolen_from(&unit_view(ctx.state, &victim), points);
            if attack == 0 && health == 0 {
                continue;
            }
            effects.push(buff(json_as(json!({
                "target": { "of": "instance", "instanceId": victim.id },
                "attack": -attack,
                "health": -health,
            }))));
            gained_attack += attack;
            gained_health += health;
        }
        if gained_attack != 0 || gained_health != 0 {
            effects.push(buff(json_as(json!({
                "target": { "of": "self" },
                "attack": gained_attack,
                "health": gained_health,
            }))));
        }
        effects
    })
}

/// R3536001: an event condition is the card's own trigger evolving itself.
fn evolves_when_the_opponent_plays_a_unit() -> TriggerDef {
    TriggerDef::new(
        "evolveOnOpposingUnit",
        &[GameEventType::CardPlayed],
        |ctx, event| {
            let GameEvent::CardPlayed { player, def_id, .. } = event else {
                return vec![];
            };
            if *player == ctx.controller {
                return vec![];
            }
            let is_unit = find_def(Some(&*ctx.state), def_id).is_some_and(|def| def.type_ == CardType::Unit);
            if is_unit {
                vec![evolve(json_as(json!({ "target": { "of": "self" } })))]
            } else {
                vec![]
            }
        },
    )
}

pub fn script() -> CardScripts {
    let base = Script {
        evolve: Some(drain_each_unit()),
        triggers: vec![evolves_when_the_opponent_plays_a_unit()],
        ..Script::default()
    };
    // One script: the face's points are `BASE_POINTS` or its declared `unit` (`ctx.radiant`).
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #86 Astrophage — SPEC §8.12 row 86.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const TWO_TWO: &str = "stand-in (1) 2/2";
    const NO_ATTACK: &str = "stand-in (1) 0/3";
    const FRAGILE: &str = "stand-in (1) 0/1";
    const SPELL: &str = "stand-in (0) Spell";

    /// p1's Astrophage and `mine` on its side; p2's `theirs` on the field and `hand` in hand.
    fn board(radiant: bool, mine: Vec<Value>, theirs: Vec<Value>, hand: &[&str]) -> Scenario {
        let mut field = vec![json!({ "def": ID, "radiant": radiant })];
        field.extend(mine);
        scenario(json!({
            "p1": { "hand": [VANILLA], "field": field, "library": crate::filler(4) },
            "p2": { "mana": 10, "hand": hand, "field": theirs, "library": crate::filler(4) },
        }))
    }

    fn plague_on(s: &Scenario, card: &CardInstance) -> i32 {
        s.card(card).counters.plague.unwrap_or(0)
    }

    fn evolved_events(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| event.event_type() == GameEventType::Evolved)
            .count()
    }

    mod base {
        use super::*;

        #[test]
        fn r3541000_opponent_unit_evolves_it_drains_attack_then_plagues() {
            let mut s = board(false, vec![json!(VANILLA)], vec![json!(VANILLA)], &[TWO_TWO]);
            let astrophage = s.unit_at(P1, 1);
            let mine = s.unit_at(P1, 2);
            let theirs = s.unit_at(P2, 1);
            s.end_turn();
            s.play(TWO_TWO, json!({}));
            let played = s.unit_at(P2, 2);
            assert_eq!(s.card(&astrophage).evolved, Some(true));
            assert_eq!(evolved_events(&s), 1);
            // One Attack point from each of the three Units, the one just played among them.
            s.expect_stats(&mine, json!({ "attack": 3, "health": 4 }));
            s.expect_stats(&theirs, json!({ "attack": 3, "health": 4 }));
            s.expect_stats(&played, json!({ "attack": 1, "health": 2 }));
            s.expect_stats(&astrophage, json!({ "attack": 3, "health": 1 }));
            for victim in [&mine, &theirs, &played] {
                assert_eq!(
                    plague_on(&s, victim),
                    1,
                    "one Plague Counter on each Unit it drained"
                );
            }
            assert_eq!(plague_on(&s, &astrophage), 0, "never on itself");
            assert!(s.state().pending.is_none(), "it opens no prompt");
        }

        #[test]
        fn a_unit_with_no_attack_gives_a_health_point() {
            let mut s = board(false, vec![json!(NO_ATTACK)], vec![], &[VANILLA]);
            let astrophage = s.unit_at(P1, 1);
            let mine = s.unit_at(P1, 2);
            s.end_turn();
            s.play(VANILLA, json!({}));
            s.expect_stats(&mine, json!({ "attack": 0, "health": 2 }));
            // It took the Attack of the Unit just played, and a Health point from the one with none.
            s.expect_stats(&astrophage, json!({ "attack": 1, "health": 2 }));
        }

        #[test]
        fn a_unit_that_cannot_give_without_dying_gives_nothing_but_is_still_plagued() {
            let mut s = board(false, vec![json!(FRAGILE)], vec![], &[FRAGILE]);
            let astrophage = s.unit_at(P1, 1);
            let mine = s.unit_at(P1, 2);
            s.end_turn();
            s.play(FRAGILE, json!({}));
            let played = s.unit_at(P2, 1);
            s.expect_stats(&mine, json!({ "attack": 0, "health": 1 }));
            s.expect_stats(&played, json!({ "attack": 0, "health": 1 }));
            s.expect_stats(&astrophage, json!({ "attack": 0, "health": 1 }));
            assert_eq!(plague_on(&s, &mine), 1);
            assert_eq!(plague_on(&s, &played), 1);
            s.expect_in_zone(&mine, "field");
        }

        #[test]
        fn a_spell_the_opponent_plays_evolves_nothing() {
            let mut s = board(false, vec![json!(VANILLA)], vec![], &[SPELL]);
            let astrophage = s.unit_at(P1, 1);
            s.end_turn();
            s.play(SPELL, json!({}));
            assert_ne!(s.card(&astrophage).evolved, Some(true));
            assert_eq!(evolved_events(&s), 0);
        }

        #[test]
        fn a_unit_its_own_controller_plays_evolves_nothing() {
            let mut s = board(false, vec![], vec![], &[VANILLA]);
            let astrophage = s.unit_at(P1, 1);
            s.play(VANILLA, json!({}));
            assert_ne!(s.card(&astrophage).evolved, Some(true));
            assert_eq!(evolved_events(&s), 0);
        }

        #[test]
        fn it_evolves_once_a_second_unit_steals_nothing_more() {
            let mut s = board(false, vec![json!(VANILLA)], vec![], &[VANILLA, VANILLA]);
            let astrophage = s.unit_at(P1, 1);
            let mine = s.unit_at(P1, 2);
            s.end_turn();
            s.play(VANILLA, json!({}));
            s.expect_stats(&mine, json!({ "attack": 3, "health": 4 }));
            s.play(VANILLA, json!({}));
            s.expect_stats(&mine, json!({ "attack": 3, "health": 4 }));
            assert_eq!(evolved_events(&s), 1);
            assert_eq!(plague_on(&s, &mine), 1);
            s.expect_stats(&astrophage, json!({ "attack": 2, "health": 1 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3541000_radiant_drains_3_per_unit() {
            let mut s = board(true, vec![json!(TWO_TWO)], vec![json!(VANILLA)], &[NO_ATTACK]);
            let astrophage = s.unit_at(P1, 1);
            let mine = s.unit_at(P1, 2);
            let theirs = s.unit_at(P2, 1);
            s.expect_stats(&astrophage, json!({ "attack": 0, "health": 2 }));
            s.end_turn();
            s.play(NO_ATTACK, json!({}));
            let played = s.unit_at(P2, 2);
            assert_eq!(s.card(&astrophage).evolved, Some(true));
            // 2/2 gives its 2 Attack and 1 of the Health it can spare: 0/1.
            s.expect_stats(&mine, json!({ "attack": 0, "health": 1 }));
            // 4/4 gives 3 Attack: 1/4.
            s.expect_stats(&theirs, json!({ "attack": 1, "health": 4 }));
            // 0/3 gives 2 Health, keeping 1: 0/1.
            s.expect_stats(&played, json!({ "attack": 0, "health": 1 }));
            // 2 + 3 Attack and 1 + 2 Health, taken stat for stat.
            s.expect_stats(&astrophage, json!({ "attack": 5, "health": 5 }));
            for victim in [&mine, &theirs, &played] {
                assert_eq!(plague_on(&s, victim), 1);
            }
            assert_eq!(plague_on(&s, &astrophage), 0);
            assert!(s.state().pending.is_none());
        }
    }
}
