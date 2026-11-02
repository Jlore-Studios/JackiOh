//! S #138 Pocket Peptides (SPEC §8.10 row 138; R70, R460, R946, R1875006, R1881007): (1) Trap, Common,
//! of the Stinkyscape set.
//!   Base:    "Reveals when one of your Units would die in combat: Cast CN Peptides on it."
//!   Radiant: "Reveals when one of your Units would die in combat: Cast a Radiant CN Peptides on each of
//!            your Units."
//! Engine: set face-down, it is a `wouldDie` replacement (§4.5 step 1, before any card moves; §6.3
//! Replacement): it answers when one of the Units the check collected is its controller's and would die
//! in combat — a hit killed it and the last hit it took was combat damage (`DyingUnit.combat`,
//! R1875006), never a Spell's hit, a text's or a destroy. It fires and is spent, and its follow-up
//! resolves at once (`resolveFirst`): a free cast of M #36 CN Peptides (R70) aimed at the first such
//! Unit in R68's order (R946, an aim it cannot take casting nothing, R1180005), so 50% it is Buffed its
//! times, which may lift its Health past its damage, and otherwise it is destroyed; the check after the
//! cast's effect judges it on the board the cast left (R460): a Unit the Buffs lifted lives. Radiant: a
//! Radiant CN Peptides (Lucky 1, so its coin rolls twice, R1440) cast once at each of its controller's
//! Units, the dying one first and then the rest in lane order (R1875006). Fused into a permanent, it answers no
//! death while a CN Peptides its controller cast is still resolving (R1881007). Declares no numbers: CN
//! Peptides' 50% and its count are its own.

use jackioh_engine::effects::{ForEachCardArgs, cast_new, for_each_card};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-138";

/// M #36 CN Peptides, what it casts.
const CN_PEPTIDES: &str = "meditative-036";

/// The follow-up its firing resolves (`ReplacementDef.then`).
const CAST_STEP: &str = "peptides";

/// R1875006: the first of `controller`'s Units the event names that would die in combat.
fn first_in_combat(event: &ReplacedEvent, controller: PlayerId) -> Option<String> {
    let ReplacedEvent::WouldDie { units } = event else {
        return None;
    };
    units
        .iter()
        .find(|unit| unit.controller == controller && unit.combat == Some(true))
        .map(|unit| unit.instance_id.clone())
}

/// R1881007: this card is not a set Trap (a Pocket Peptides M #47's Fuse took into a permanent, which is
/// not spent as it fires) and a CN Peptides its controller cast is still resolving, so it answers no death
/// until that cast has: the check inside the cast finds the same Unit still dying and would ask it again,
/// forever. A set Trap is spent as it fires, so another set Pocket Peptides still answers.
fn answering(state: &GameState, card: &CardInstance, controller: PlayerId) -> bool {
    !jackioh_engine::preview::is_face_down(state, card) && is_resolving(state, controller, &[CN_PEPTIDES])
}

/// R1875006: the Units it casts at — the one dying in combat first, so its own Buffs decide it; on the
/// Radiant face then each other Unit of its controller's, in lane order. Each cast is aimed only, so a
/// Unit an earlier cast took off the field has nothing cast at it.
fn aims(ctx: &EffectContext<'_>, radiant: bool) -> Vec<String> {
    let Some(dying) = replacement_of(ctx).and_then(|record| first_in_combat(&record.event, ctx.controller)) else {
        return Vec::new();
    };
    let mut aims = vec![dying.clone()];
    if radiant {
        aims.extend(
            active_units_of(&*ctx.state, ctx.controller)
                .iter()
                .map(|unit| unit.id.clone())
                .filter(|id| *id != dying),
        );
    }
    aims
}

fn pocket_peptides(radiant: bool) -> Script {
    Script {
        replacements: vec![ReplacementDef {
            id: "pocket-peptides".into(),
            on: ReplacementMoment::WouldDie,
            where_: None,
            when: Some(replacement_when(|rc| {
                first_in_combat(rc.event, rc.controller).is_some() && !answering(rc.state, rc.self_, rc.controller)
            })),
            instead: ReplacementInstead {
                resolve_first: Some(true),
                ..ReplacementInstead::default()
            },
            then: Some(CAST_STEP.into()),
            by: None,
        }],
        resume: IndexMap::from([(
            CAST_STEP,
            hook(move |_ctx| {
                // Read once as the list reaches it (R113), so a pause inside a cast resumes over the same
                // Units: the casts' number hangs on the board.
                vec![for_each_card(ForEachCardArgs {
                    cards: Arc::new(move |ctx: &mut EffectContext<'_>| aims(ctx, radiant)),
                    each: Arc::new(move |aim: &str| {
                        cast_new(json_as(json!({
                            "def": CN_PEPTIDES,
                            "radiant": radiant,
                            "aimAt": aim,
                            "aimOnly": true,
                        })))
                    }),
                })]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: pocket_peptides(false),
        radiant: pocket_peptides(true),
    }
}

// S #138 Pocket Peptides — SPEC §8.10 row 138, BUILD M12 row S 138.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::{CN_PEPTIDES, ID};

    const FILLER: &str = "core-005"; // (1) Spell Stockpile: a card in hand, so no turn ends by itself.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const MOTHS: &str = "core-009"; // (2) 1/14.
    const GENN: &str = "classic-086"; // (4) 14/14.
    const HIT_JOB: &str = "core-016"; // (3) Spell: "Destroy a Unit."
    const SORCERER: &str = "core-068"; // (2) 5/5: "Cry: Deal 4 damage."

    fn trapped(seed: &str, radiant: bool, hand: Value, field: Value) -> Scenario {
        let mut cards = vec![json!(FILLER)];
        cards.extend(hand.as_array().cloned().unwrap_or_default());
        let mut s = crate::scenario(json!({
            "seed": seed,
            "p1": {
                "hand": cards,
                "field": [{ "def": VANILLA, "lane": 1 }],
                "library": crate::filler(6),
                "mana": 10,
            },
            "p2": {
                "hand": [FILLER],
                "field": field,
                "backrow": [{ "def": ID, "radiant": radiant, "lane": 1 }],
                "library": crate::filler(6),
            },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    fn fired(s: &Scenario) -> bool {
        s.last_events().iter().any(|event| event.event_type() == GameEventType::TrapFired)
    }

    fn peptides_cast(s: &Scenario) -> Vec<bool> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::CardAnnounced { def_id, instance_id, .. } if def_id == CN_PEPTIDES => {
                    Some(s.card(instance_id.as_str()).radiant)
                }
                _ => None,
            })
            .collect()
    }

    fn defender_lives(seed: &str) -> bool {
        let mut s = trapped(seed, false, json!([]), json!([{ "def": VANILLA, "lane": 2 }]));
        let defender = s.unit_at(P2, 2);
        s.attack(s.unit_at(P1, 1), &defender);
        s.card(defender.id.as_str()).zone.z() == ZoneName::Field
    }

    mod base {
        use super::*;

        #[test]
        fn r1875006_a_unit_that_would_die_in_combat_has_cn_peptides_cast_on_it() {
            let mut s = trapped("pocket-peptides", false, json!([]), json!([{ "def": VANILLA, "lane": 2 }]));
            let defender = s.unit_at(P2, 2);
            s.attack(s.unit_at(P1, 1), &defender);
            assert!(fired(&s));
            s.expect_in_zone(ID, "graveyard");
            assert_eq!(peptides_cast(&s), vec![false]);
            assert!(s.pile(P2, "graveyard").iter().any(|card| card.def_id == CN_PEPTIDES));
        }

        #[test]
        fn r1875006_the_cast_buffs_it_alive_or_destroys_it_both_over_the_seeds() {
            let lives = seed_where("pocket-peptides-lives", defender_lives);
            let dies = seed_where("pocket-peptides-dies", |seed| !defender_lives(seed));
            let mut s = trapped(&lives, false, json!([]), json!([{ "def": VANILLA, "lane": 2 }]));
            let defender = s.unit_at(P2, 2);
            s.attack(s.unit_at(P1, 1), &defender);
            assert!(s.stats(defender.id.as_str()).health > 0);
            assert!(s.last_events().iter().any(|event| event.event_type() == GameEventType::Upgraded));
            let mut s = trapped(&dies, false, json!([]), json!([{ "def": VANILLA, "lane": 2 }]));
            let defender = s.unit_at(P2, 2);
            s.attack(s.unit_at(P1, 1), &defender);
            s.expect_in_zone(defender.id.as_str(), "graveyard");
        }

        #[test]
        fn r1875006_a_spells_hit_or_a_destroy_does_not_fire_it() {
            let mut s = trapped(
                "pocket-peptides-spell",
                false,
                json!([SORCERER, HIT_JOB]),
                json!([{ "def": VANILLA, "lane": 2 }, { "def": VANILLA, "lane": 3 }]),
            );
            let ping = s.unit_at(P2, 3);
            s.play(SORCERER, json!({ "zone": 2, "targets": pick::instance(&ping) }));
            assert!(!fired(&s), "a Cry's hit is not combat");
            s.expect_in_zone(ping.id.as_str(), "graveyard");
            let vanilla = s.unit_at(P2, 2);
            s.play(HIT_JOB, json!({ "targets": pick::instance(&vanilla) }));
            assert!(!fired(&s), "a destroy is not combat");
            assert_eq!(s.backrow(P2, 1).map(|card| card.def_id), Some(ID.to_string()));
        }

        #[test]
        fn r1875006_the_enemys_unit_dying_in_combat_does_not_fire_it() {
            let mut s = trapped("pocket-peptides-enemy", false, json!([]), json!([{ "def": GENN, "lane": 1 }]));
            let attacker = s.unit_at(P1, 1);
            s.attack(&attacker, s.unit_at(P2, 1));
            s.expect_in_zone(attacker.id.as_str(), "graveyard");
            assert!(!fired(&s));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1875006_each_of_its_controllers_units_has_a_radiant_cn_peptides_cast_on_it() {
            let mut s = trapped(
                "pocket-peptides-radiant",
                true,
                json!([]),
                json!([{ "def": MOTHS, "lane": 1 }, { "def": VANILLA, "lane": 2 }, { "def": MOTHS, "lane": 4 }]),
            );
            let lanes: Vec<Vec<String>> = [2, 1, 4].iter().map(|lane| vec![s.unit_at(P2, *lane).id]).collect();
            let defender = s.unit_at(P2, 2);
            s.attack(s.unit_at(P1, 1), &defender);
            assert!(fired(&s));
            // Three Radiant casts (Lucky 1: each coin rolls twice, R1440), one at each Unit.
            assert_eq!(peptides_cast(&s), vec![true; 3]);
            let aimed: Vec<Vec<String>> = s
                .last_events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::CardAnnounced { def_id, targets, .. } if def_id == CN_PEPTIDES => Some(targets.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(aimed, lanes);
            assert!(s.state().pending.is_none());
        }
    }

    mod set_twice {
        use super::*;

        fn attacked(seed: &str) -> (Scenario, String) {
            let mut s = crate::scenario(json!({
                "seed": seed,
                "p1": { "hand": [FILLER], "field": [{ "def": GENN, "lane": 1 }], "library": crate::filler(6) },
                "p2": {
                    "hand": [FILLER],
                    "field": [{ "def": VANILLA, "lane": 1 }],
                    "backrow": [{ "def": ID, "lane": 1 }, { "def": ID, "lane": 2 }],
                    "library": crate::filler(6),
                },
            }));
            for player in [P1, P2] {
                s.state_mut().players[player].auto_end_turn = Some(false);
            }
            let defender = s.unit_at(P2, 1).id;
            s.attack(GENN, defender.as_str());
            (s, defender)
        }

        fn first_cast_buffed(s: &Scenario) -> bool {
            s.last_events()
                .iter()
                .find_map(|event| match event.event_type() {
                    GameEventType::Upgraded => Some(true),
                    GameEventType::Destroyed => Some(false),
                    _ => None,
                })
                .unwrap_or(false)
        }

        #[test]
        fn r1881007_a_second_set_trap_still_answers_the_check_inside_the_first_ones_cast() {
            let seed = seed_where("pocket-peptides-twice", |seed| {
                first_cast_buffed(&attacked(seed).0)
            });
            let (s, _defender) = attacked(&seed);
            let fired = s
                .last_events()
                .iter()
                .filter(|event| event.event_type() == GameEventType::TrapFired)
                .count();
            assert_eq!(fired, 2, "the second set Pocket Peptides answered too");
            assert_eq!(peptides_cast(&s).len(), 2, "two CN Peptides casts");
            assert!(
                s.backrow(P2, 1).is_none() && s.backrow(P2, 2).is_none(),
                "each is spent as it fires"
            );
            assert_eq!(
                s.pile(P2, "graveyard")
                    .iter()
                    .filter(|card| card.def_id == ID)
                    .count(),
                2
            );
            assert!(s.state().result.is_none());
        }
    }

    mod fused {
        use super::*;
        use jackioh_engine::subsystems::{self, FuseArgs};

        fn fused_into_hand(s: &mut Scenario, def_ids: &[&str]) -> CardInstance {
            let mut events: Vec<GameEvent> = vec![];
            let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
            let made = {
                let state = s.state_mut();
                let ingredients: Vec<CardInstance> = def_ids
                    .iter()
                    .map(|def_id| new_instance(&mut *state, def_id, P2, Zone::Gone { player: P2 }))
                    .collect();
                let mut sink = EngineSink::new(state, &mut events, &mut rng);
                subsystems::fuse(
                    &mut sink,
                    FuseArgs {
                        ingredients,
                        to_hand: Some(P2),
                        ..FuseArgs::default()
                    },
                )
            };
            s.state_mut().rng_cursor = rng.cursor();
            made.expect("the fused card")
        }

        #[test]
        fn r1881007_fused_into_a_unit_it_answers_one_combat_death_once() {
            for i in 0..6 {
                let mut s = crate::scenario(json!({
                    "seed": format!("fused-peptides-{i}"),
                    "active": "p2",
                    "p1": { "hand": [FILLER], "field": [{ "def": GENN, "lane": 1 }], "library": crate::filler(6) },
                    "p2": { "hand": [FILLER], "library": crate::filler(6), "mana": 10 },
                }));
                let fused = fused_into_hand(&mut s, &[VANILLA, ID]);
                s.play(fused.id.as_str(), json!({ "zone": 1 }));
                let on_field = s.unit_at(P2, 1).id;
                s.end_turn();
                s.attack(GENN, on_field.as_str());
                let casts = s
                    .last_events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::CardAnnounced { def_id, .. } if def_id == CN_PEPTIDES))
                    .count();
                assert_eq!(casts, 1, "seed {i}: one cast");
                assert!(s.state().result.is_none());
            }
        }
    }
}
