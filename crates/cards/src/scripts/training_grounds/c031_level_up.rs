//! TG #31 Level Up! (SPEC §8.11 row 31). (3) Field Spell, Epic. "After one of your Units has attacked
//! {attacks|time|times} since this arrived, it becomes Radiant." (Radiant: 2 times.)
//! The engine's `allyAttack` hook (R2199000): run once the state check closing each combat of one of its
//! controller's Units has run, declared or forced (R53); an attack called off before it fought never
//! reaches it (R44). Each Unit counts apart, across turns, from when this arrived (R2199007), in this
//! card's own `memory` (§10.1) as Unit id to `{ n, mark }`, so the counts go when it leaves (R78). `mark`
//! is the field's departures as the count began: a Unit that has left since is a new arrival (R174). It
//! becomes Radiant after its Nth combat if it still stands (`set_radiant`, R22, R74). R2321000: the
//! counts that still hold ride on their Units in both views (`tallies`).

use jackioh_engine::combat::ally_attack_of;
use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-031";

/// §10.1: where this card keeps its counts.
const COUNTS: &str = "attacks";

fn level_up(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(facts) = ally_attack_of(ctx) else {
        return vec![];
    };
    let mut counts = recalled(ctx, COUNTS)
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    let attacker = find_instance(ctx.state, &facts.attacker_id).cloned();
    let Some(attacker) = attacker.filter(|attacker| facts.survived && attacker.controller == ctx.controller) else {
        // A Unit that fell in the combat starts again if it ever returns (R174).
        if counts.remove(&facts.attacker_id).is_none() {
            return vec![];
        }
        return vec![remember(json_as(json!({ "key": COUNTS, "value": counts })))];
    };
    let earlier = counts.get(&attacker.id).filter(|entry| {
        entry["mark"]
            .as_u64()
            .is_some_and(|mark| !left_field_since_mark(ctx.state, mark as u32, &attacker.id))
    });
    let n = earlier.and_then(|entry| entry["n"].as_i64()).unwrap_or(0) + 1;
    let mark = earlier
        .and_then(|entry| entry["mark"].as_u64())
        .unwrap_or_else(|| u64::from(field_exit_mark(ctx.state)));
    counts.insert(attacker.id.clone(), json!({ "n": n, "mark": mark }));
    let mut effects = vec![remember(json_as(json!({ "key": COUNTS, "value": counts })))];
    if n >= i64::from(param(ctx, "attacks")) && !attacker.radiant {
        effects.push(set_radiant(json_as(json!({ "instanceId": attacker.id }))));
    }
    effects
}

/// R2321000: each count that still holds (its Unit on its controller's field, not left since, R174), on
/// that Unit toward its "attacks" (§10.8), so the screen shows what the next attack adds to.
fn tallies() -> TallyHook {
    condition_hook(|ctx| {
        let Some(counts) = ctx.self_.memory.get(COUNTS).and_then(Value::as_object) else {
            return vec![];
        };
        let goal = param(&ctx, "attacks");
        counts
            .iter()
            .filter(|(id, entry)| {
                find_instance(ctx.state, id).is_some_and(|unit| {
                    unit.controller == ctx.controller
                        && matches!(unit.zone, Zone::Field { .. })
                        && entry["mark"]
                            .as_u64()
                            .is_some_and(|mark| !left_field_since_mark(ctx.state, mark as u32, id))
                })
            })
            .filter_map(|(id, entry)| {
                Some(UnitTally {
                    instance_id: id.clone(),
                    value: i32::try_from(entry["n"].as_i64()?).ok()?,
                    goal,
                })
            })
            .collect()
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        ally_attack: Some(hook(level_up)),
        tallies: Some(tallies()),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// TG #31 Level Up! — SPEC §8.11 row 31, BUILD M13 row TG 31. p2's 0/30 Wall takes the attacks, so
// nothing dies unless a test says so.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::upgrade_number;

    const LEVEL_UP: &str = super::ID;
    const VANILLA: &str = "core-008"; // (1) 4/4
    const BIGOT: &str = "core-002"; // (2) 4/1
    /// S Trap: "Reveals when an enemy Unit attacks one of your Units: Cancel the attack and Freeze the attacker."
    const CANCELLER: &str = "stinkyscape-t-ai-04";
    const MOTHS: &str = "core-009"; // start of turn: every enemy Unit attacks this
    const FILLER: &str = "core-010";

    fn wall() -> Value {
        json!({ "def": VANILLA, "statsOverride": { "attack": 0, "health": 30 } })
    }

    fn board(radiant: bool, p1: Value, p2: Value) -> Scenario {
        crate::register_all();
        let mut p1 = p1;
        if p1.get("backrow").is_none() {
            p1["backrow"] = json!([{ "def": LEVEL_UP, "radiant": radiant, "faceUp": true, "lane": 1 }]);
        }
        p1["library"] = json!([FILLER, FILLER, FILLER, FILLER, FILLER, FILLER]);
        let mut p2 = p2;
        p2["library"] = json!([FILLER, FILLER, FILLER, FILLER, FILLER, FILLER]);
        scenario(json!({ "seed": "level-up", "p1": p1, "p2": p2 }))
    }

    /// p1's Unit in lane `lane` attacks p2's Unit in lane 1.
    fn swing(s: &mut Scenario, lane: i32) {
        let attacker = s.unit(P1, lane).unwrap();
        let target = s.unit(P2, 1).unwrap();
        s.attack(&attacker, &target);
    }

    /// p1's turn ends and its next begins (a side with nothing to do ends its turn by itself, R82).
    fn next_round(s: &mut Scenario) {
        s.end_turn();
        while s.state().active != P1 {
            s.end_turn();
        }
    }

    fn radiant_at(s: &Scenario, lane: i32) -> bool {
        s.unit(P1, lane).unwrap().radiant
    }

    /// What `viewer` reads on p1's Unit in `lane`: the counts the field keeps on it, as `(keeper, value, goal)`.
    fn tallies_seen(s: &Scenario, viewer: PlayerId, lane: i32) -> Option<Vec<(String, i32, i32)>> {
        let id = s.unit(P1, lane).unwrap().id;
        let view = s.view(viewer);
        view.you
            .units
            .iter()
            .chain(view.opponent.units.iter())
            .flatten()
            .find(|unit| unit.instance_id == id)
            .unwrap()
            .tallies
            .clone()
            .map(|tallies| tallies.into_iter().map(|tally| (tally.def_id, tally.value, tally.goal)).collect())
    }

    fn counted(value: i32, goal: i32) -> Option<Vec<(String, i32, i32)>> {
        Some(vec![(LEVEL_UP.to_string(), value, goal)])
    }

    mod base {
        use super::*;

        #[test]
        fn r2321000_each_units_count_rides_on_it_in_both_views() {
            let mut s = board(false, json!({ "field": [VANILLA, VANILLA] }), json!({ "field": [wall()] }));
            assert_eq!(tallies_seen(&s, P1, 1), None, "no attack yet, no count");
            swing(&mut s, 1);
            assert_eq!(tallies_seen(&s, P1, 1), counted(1, 3));
            assert_eq!(tallies_seen(&s, P2, 1), counted(1, 3), "a face-up Field Spell's count is public");
            assert_eq!(tallies_seen(&s, P1, 2), None, "each Unit counts apart");
            next_round(&mut s);
            swing(&mut s, 1);
            assert_eq!(tallies_seen(&s, P2, 1), counted(2, 3));
            next_round(&mut s);
            swing(&mut s, 1);
            assert!(radiant_at(&s, 1));
            assert_eq!(tallies_seen(&s, P1, 1), counted(3, 3));
        }

        #[test]
        fn r2321000_r174_a_bounced_and_replayed_unit_shows_no_stale_count() {
            let mut s = board(
                false,
                json!({ "field": [VANILLA], "hand": [FILLER], "mana": 10 }),
                json!({ "field": [wall()] }),
            );
            swing(&mut s, 1);
            let vanilla = s.unit(P1, 1).unwrap();
            bounce_to_hand(&mut s, &vanilla.id);
            s.play(VANILLA, json!({ "zone": 1 }));
            assert_eq!(tallies_seen(&s, P1, 1), None, "its count began again");
        }

        #[test]
        fn r2321000_r78_the_counts_go_with_this_card() {
            let mut s = board(false, json!({ "field": [VANILLA] }), json!({ "field": [wall()] }));
            swing(&mut s, 1);
            let level_up = s.backrow(P1, 1).unwrap();
            bounce_to_hand(&mut s, &level_up.id);
            assert_eq!(tallies_seen(&s, P1, 1), None);
        }

        #[test]
        fn r2199000_r2199007_a_unit_that_attacks_3_times_over_several_turns_becomes_radiant_after_the_third() {
            let mut s = board(false, json!({ "field": [VANILLA] }), json!({ "field": [wall()] }));
            swing(&mut s, 1);
            next_round(&mut s);
            swing(&mut s, 1);
            assert!(!radiant_at(&s, 1));
            next_round(&mut s);
            swing(&mut s, 1);

            assert!(radiant_at(&s, 1));
            // Not before its damage: the third combat's hit was the base face's 4.
            let hits: Vec<i32> = s
                .events()
                .iter()
                .filter_map(|e| match e {
                    GameEvent::Damage { amount, combat: true, target_id, .. } if *target_id == s.unit(P2, 1).unwrap().id => {
                        Some(*amount)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(hits, vec![4, 4, 4]);
            s.expect_stats(VANILLA, json!({ "attack": 12, "health": 12 }));
        }

        #[test]
        fn r2199007_two_units_attacking_once_and_twice_make_neither_radiant() {
            let mut s = board(false, json!({ "field": [VANILLA, VANILLA] }), json!({ "field": [wall()] }));
            swing(&mut s, 1);
            swing(&mut s, 2);
            next_round(&mut s);
            swing(&mut s, 2);

            assert!(!radiant_at(&s, 1));
            assert!(!radiant_at(&s, 2));
        }

        #[test]
        fn r2199007_attacks_made_before_this_arrived_do_not_count() {
            let mut s = board(
                false,
                json!({ "hand": [LEVEL_UP], "backrow": [], "field": [VANILLA], "mana": 10 }),
                json!({ "field": [wall()] }),
            );
            swing(&mut s, 1);
            s.play(LEVEL_UP, json!({}));
            next_round(&mut s);
            swing(&mut s, 1);
            next_round(&mut s);
            swing(&mut s, 1);

            assert!(!radiant_at(&s, 1), "only two attacks since it arrived");
            next_round(&mut s);
            swing(&mut s, 1);
            assert!(radiant_at(&s, 1));
        }

        #[test]
        fn r2199007_an_enemy_units_attacks_never_count() {
            let mut s = board(false, json!({ "field": [wall()] }), json!({ "field": [VANILLA] }));
            for _ in 0..3 {
                s.end_turn();
                let attacker = s.unit(P2, 1).unwrap();
                let target = s.unit(P1, 1).unwrap();
                s.attack(&attacker, &target);
                s.end_turn();
            }

            assert!(!s.unit(P2, 1).unwrap().radiant);
        }

        #[test]
        fn r53_a_forced_attack_counts() {
            let mut s = board(
                false,
                json!({ "field": [VANILLA] }),
                json!({ "field": [{ "def": MOTHS, "statsOverride": { "attack": 0, "health": 30 } }] }),
            );
            swing(&mut s, 1);
            // p2's turn begins: the Moths force the Vanilla's second attack.
            next_round(&mut s);
            swing(&mut s, 1);

            assert!(radiant_at(&s, 1), "a declared, a forced and a declared attack");
        }

        #[test]
        fn r44_a_canceled_attack_does_not_count() {
            let mut s = board(
                false,
                json!({ "field": [VANILLA] }),
                json!({ "field": [wall()], "backrow": [{ "def": CANCELLER, "faceUp": false, "lane": 2 }] }),
            );
            swing(&mut s, 1); // the trap calls this one off, and Freezes the attacker
            assert!(s.events().iter().any(|e| matches!(e, GameEvent::AttackCancelled { .. })));
            next_round(&mut s);
            next_round(&mut s);
            swing(&mut s, 1);
            next_round(&mut s);
            swing(&mut s, 1);
            assert!(!radiant_at(&s, 1), "two attacks fought");

            next_round(&mut s);
            swing(&mut s, 1);
            assert!(radiant_at(&s, 1));
        }

        #[test]
        fn r174_a_unit_that_dies_in_its_third_attack_is_not_made_radiant() {
            let mut s = board(false, json!({ "field": [BIGOT] }), json!({ "field": [{ "def": VANILLA, "statsOverride": { "attack": 0, "health": 30 } }] }));
            swing(&mut s, 1);
            next_round(&mut s);
            swing(&mut s, 1);
            next_round(&mut s);
            // The wall hits back for 1 now: the 4/1 dies in its third combat.
            let wall_id = s.unit(P2, 1).unwrap().id;
            s.card_mut(wall_id.as_str()).buffs = AttackHealth { attack: 1, health: 0 };
            swing(&mut s, 1);

            s.expect_in_zone(BIGOT, "graveyard");
            assert!(!s.card(BIGOT).radiant);
        }

        #[test]
        fn r174_bounce_and_replay_restarts_that_units_count() {
            let mut s = board(
                false,
                json!({ "field": [VANILLA], "hand": [FILLER], "mana": 10 }),
                json!({ "field": [wall()] }),
            );
            swing(&mut s, 1);
            next_round(&mut s);
            swing(&mut s, 1);
            // Bounce it to the hand and play it again: a new arrival.
            let vanilla = s.unit(P1, 1).unwrap();
            bounce_to_hand(&mut s, &vanilla.id);
            s.play(VANILLA, json!({ "zone": 1 }));
            next_round(&mut s);
            swing(&mut s, 1);

            assert!(!radiant_at(&s, 1), "its count began again at the replay");
        }

        #[test]
        fn r78_this_leaving_the_field_resets_every_count() {
            let mut s = board(
                false,
                json!({ "field": [VANILLA], "hand": [LEVEL_UP, FILLER], "mana": 10 }),
                json!({ "field": [wall()] }),
            );
            swing(&mut s, 1);
            next_round(&mut s);
            swing(&mut s, 1);
            // The first Level Up leaves; a second one starts with no counts.
            let first = s.backrow(P1, 1).unwrap();
            bounce_to_hand(&mut s, &first.id);
            s.play(LEVEL_UP, json!({}));
            next_round(&mut s);
            swing(&mut s, 1);

            assert!(!radiant_at(&s, 1));
        }

        #[test]
        fn r74_an_already_radiant_unit_is_unchanged() {
            let mut s = board(false, json!({ "field": [{ "def": VANILLA, "radiant": true }] }), json!({ "field": [wall()] }));
            for _ in 0..3 {
                swing(&mut s, 1);
                next_round(&mut s);
            }

            assert!(radiant_at(&s, 1));
            s.expect_stats(VANILLA, json!({ "attack": 12, "health": 12 }));
            assert!(!s.events().iter().any(|e| matches!(e, GameEvent::RadiantSet { .. })));
        }

        #[test]
        fn r386_attacks_reads_through_param() {
            let mut s = board(false, json!({ "field": [VANILLA] }), json!({ "field": [wall()] }));
            assert_eq!(upgrade_number(&mut s, LEVEL_UP, "attacks"), 2);
            swing(&mut s, 1);
            next_round(&mut s);
            swing(&mut s, 1);

            assert!(radiant_at(&s, 1));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2199007_two_attacks_suffice() {
            let mut s = board(true, json!({ "field": [VANILLA] }), json!({ "field": [wall()] }));
            swing(&mut s, 1);
            assert!(!radiant_at(&s, 1));
            assert_eq!(tallies_seen(&s, P1, 1), counted(1, 2), "R2321000: the Radiant face counts to 2");
            next_round(&mut s);
            swing(&mut s, 1);

            assert!(radiant_at(&s, 1));
        }
    }

    /// Returns a card on the field to its owner's hand, as a Bounce effect would (§6.3), outside any play.
    fn bounce_to_hand(s: &mut Scenario, id: &str) {
        let mut rng = create_rng(&s.state().seed, s.state().rng_cursor);
        let mut events: Vec<GameEvent> = Vec::new();
        {
            let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
            {
                let mut ctx = make_context(&mut sink, None, HookOptions { controller: Some(P1), ..HookOptions::default() });
                let target = json!({ "target": { "of": "instance", "instanceId": id } });
                apply_effects(&[jackioh_engine::effects::bounce(json_as(target))], &mut ctx);
            }
            settle(&mut sink, SettleOptions::default());
        }
        s.state_mut().rng_cursor = rng.cursor();
    }
}
