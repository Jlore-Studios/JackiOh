//! SPEC §8.14 FvH #105 Maddended Felinor ((3) Unit, Felinor, Epic, 12/12 → 24/24), a card of the held Felinors
//! vs Humans set (R5235000). The name keeps the designer's spelling.
//!   Base:    "Cry: This attacks each other Unit, one at a time in a random order."
//!   Radiant: "Cry: This attacks each enemy Unit, one at a time in a random order."
//!
//! Cry (R1), R5257000 (FV2 #105, the owner's call of 2026-11-28): the other Units on the field, both sides
//! (Radiant: the enemy's), are read once as the Cry resolves and put in a random order by the match rng
//! (R60), drawn inside the effect, never as the hook builds it. Each is a forced attack (`forced_attack`, R53),
//! with `allies` on the base face so its own side's Units are attacked too (R5238012): each its own combat and
//! state check, the target striking back. A Unit gone meanwhile is passed by (R174), and once this dies or
//! leaves the field the rest are passed by too, so the run stops there. Declares no numbers. Built by S07
//! (#5257).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-105";

/// R5257000: the Units this attacks, in the random order it attacks them: every other Unit acting on the
/// field, or only the enemy's on the Radiant face.
fn victims_in_random_order(ctx: &mut EffectContext<'_>, enemies_only: bool) -> Vec<String> {
    let me = ctx.self_.as_ref().map(|card| card.id.clone());
    let enemy = opponent_of(ctx.controller);
    let sides = if enemies_only {
        vec![enemy]
    } else {
        vec![ctx.controller, enemy]
    };
    let mut ids: Vec<String> = Vec::new();
    for side in sides {
        for unit in active_units_of(&*ctx.state, side) {
            if me.as_deref() != Some(unit.id.as_str()) {
                ids.push(unit.id.clone());
            }
        }
    }
    ctx.sink.rng.shuffle(&ids)
}

fn maddended(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(move |ctx: &mut EffectContext<'_>| victims_in_random_order(ctx, radiant)),
                each: Arc::new(move |target: &str| {
                    forced_attack(ForcedAttackArgs {
                        attacker: TargetSpec::SelfCard,
                        target: TargetSpec::Instance {
                            instance_id: target.to_string(),
                        },
                        // R5238012: the base face's run reaches its own side's Units.
                        allies: if radiant { None } else { Some(true) },
                    })
                }),
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: maddended(false),
        radiant: maddended(true),
    }
}

// FvH #105 Maddended Felinor — SPEC §8.14 row 105.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SMALL: &str = "stand-in (1) 1/1";
    const DEADLY: &str = "stand-in (1) 1/1 Poisonous";
    const FILLER: &str = "core-005"; // Stockpile: a card to hold

    fn board(radiant: bool, seed: &str, mine: &[&str], theirs: &[&str]) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": mine, "library": crate::filler(4) },
            "p2": { "hand": [FILLER], "field": theirs, "library": crate::filler(4) },
        }))
    }

    /// The one Maddended Felinor, wherever it is now (the field, or the graveyard once it died).
    fn felinor(s: &Scenario) -> CardInstance {
        s.card(ID).clone()
    }

    /// The targets of the forced attacks the Felinor declared, in order.
    fn attacked(s: &Scenario, attacker: &str) -> Vec<String> {
        s.events_of(GameEventType::AttackDeclared)
            .into_iter()
            .filter_map(|event| match event {
                GameEvent::AttackDeclared { attacker_id, target_id, forced: true, .. } if attacker_id == attacker => {
                    Some(target_id)
                }
                _ => None,
            })
            .collect()
    }

    fn ids_on(s: &Scenario, player: PlayerId) -> Vec<String> {
        active_units_of(s.state(), player).into_iter().map(|unit| unit.id.clone()).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r5257000_it_attacks_every_other_unit_on_both_sides_once() {
            let mut s = board(false, "maddended-all", &[SMALL, SMALL], &[SMALL, SMALL]);
            let mut others = ids_on(&s, P1);
            others.extend(ids_on(&s, P2));
            s.play(ID, json!({ "zone": 3 }));
            let me = felinor(&s);
            let mut targets = attacked(&s, &me.id);
            assert_eq!(targets.len(), 4, "one forced attack on each: {targets:?}");
            targets.sort();
            others.sort();
            assert_eq!(targets, others, "every other Unit, its own side's included");
            for id in nonempty(others, "the other Units") {
                s.expect_in_zone(id.as_str(), "graveyard");
            }
            assert_eq!(s.card(&me).damage, 4, "each struck back once");
            s.expect_in_zone(&me, "field");
        }

        #[test]
        fn r5257000_the_order_is_random() {
            let mut firsts: Vec<String> = Vec::new();
            for seed in 0..12 {
                let mut s = board(false, &format!("maddended-order-{seed}"), &[SMALL, SMALL], &[SMALL, SMALL]);
                let lanes: Vec<(String, String)> = [P1, P2]
                    .into_iter()
                    .flat_map(|side| {
                        (1..=2).map(move |lane| (side, lane)).collect::<Vec<_>>()
                    })
                    .map(|(side, lane)| (s.unit_at(side, lane).id, format!("{side}-{lane}")))
                    .collect();
                s.play(ID, json!({ "zone": 3 }));
                let me = felinor(&s);
                let first = must(attacked(&s, &me.id).into_iter().next(), "a first attack");
                let place = must(lanes.iter().find(|(id, _)| *id == first), "the first target's place").1.clone();
                if !firsts.contains(&place) {
                    firsts.push(place);
                }
            }
            assert!(firsts.len() > 1, "the first target changes with the seed: {firsts:?}");
        }

        #[test]
        fn r5257000_the_run_stops_once_it_dies() {
            let mut s = board(false, "maddended-stop", &[DEADLY], &[DEADLY, DEADLY]);
            let mut others = ids_on(&s, P1);
            others.extend(ids_on(&s, P2));
            s.play(ID, json!({ "zone": 3 }));
            let me = felinor(&s);
            assert_eq!(attacked(&s, &me.id).len(), 1, "a Poisonous strike back ends the run");
            s.expect_in_zone(&me, "graveyard");
            let standing = others.iter().filter(|id| s.card(id.as_str()).zone.z() == ZoneName::Field).count();
            assert_eq!(standing, 2, "the two it never reached still stand");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5257000_it_attacks_every_enemy_unit_and_spares_its_own() {
            let mut s = board(true, "maddended-radiant", &[SMALL], &[SMALL, SMALL, SMALL]);
            let mine = ids_on(&s, P1);
            let mut theirs = ids_on(&s, P2);
            s.play(ID, json!({ "zone": 3 }));
            let me = felinor(&s);
            let mut targets = attacked(&s, &me.id);
            targets.sort();
            theirs.sort();
            assert_eq!(targets, theirs, "each enemy Unit once, no ally");
            for id in nonempty(mine, "its own Units") {
                s.expect_in_zone(id.as_str(), "field");
                assert_eq!(s.card(id.as_str()).damage, 0);
            }
            assert!(active_units_of(s.state(), P2).is_empty());
        }
    }
}
