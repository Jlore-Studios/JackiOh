//! TG #31 Level Up! (SPEC §8.11 row 31). (3) Field Spell, Epic. "After one of your Units attacks, if it
//! destroyed an enemy Unit, it becomes Radiant." (Radiant: "After one of your Units attacks, it becomes
//! Radiant.")
//! The engine's `allyAttack` hook (R2199000): run once the state check closing each combat of one of its
//! controller's Units has run, declared or forced (R53); an attack called off before it fought never
//! reaches it (R44). The hook is handed whether the attacker still stands (`survived`, R174) and whether
//! that combat destroyed a Unit of its opponent with it the killer (`destroyed_enemy`, R42, R3951000): the
//! base face makes a surviving attacker that destroyed one Radiant, the Radiant face every surviving
//! attacker (`set_radiant`, R22, R74). Issue #3951 gave it this text; through v0.6.4c it counted each
//! Unit's attacks since it arrived and made it Radiant at the third (Radiant: second), with the counts on
//! the Units (R2199007, R2321000), which `script_v0_6_4c` keeps for those replays (R767000).

use jackioh_engine::combat::ally_attack_of;
use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-031";

/// R3951000: one of its controller's Units that survived its attack becomes Radiant, on the base face only
/// when that combat destroyed an enemy Unit.
fn level_up(needs_kill: bool) -> Hook {
    hook(move |ctx| {
        let Some(facts) = ally_attack_of(ctx) else {
            return vec![];
        };
        if !facts.survived || (needs_kill && !facts.destroyed_enemy) {
            return vec![];
        }
        let Some(attacker) = find_instance(ctx.state, &facts.attacker_id) else {
            return vec![];
        };
        if attacker.controller != ctx.controller || attacker.radiant {
            return vec![];
        }
        vec![set_radiant(json_as(json!({ "instanceId": attacker.id })))]
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            ally_attack: Some(level_up(true)),
            ..Script::default()
        },
        radiant: Script {
            ally_attack: Some(level_up(false)),
            ..Script::default()
        },
    }
}

/// §10.1: where this card kept its counts through v0.6.4c.
const COUNTS: &str = "attacks";

fn level_up_v0_6_4c(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
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

/// R767000, R2199007: the scripts through v0.6.4c, which counted each Unit's attacks (issue #3951).
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_4c() -> CardScripts {
    let base = Script {
        ally_attack: Some(hook(level_up_v0_6_4c)),
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

    use super::script_v0_6_4c;

    const LEVEL_UP: &str = super::ID;
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FRAIL: &str = "stand-in (1) 4/1";
    const CLEAVER: &str = "stand-in (1) 4/4 Cleave";
    /// S Trap: "Reveals when an enemy Unit attacks one of your Units: Cancel the attack and Freeze the attacker."
    const CANCELLER: &str = "stinkyscape-t-ai-04";
    const FILLER: &str = "core-010";
    const PHOENIX: &str = "stand-in (1) 4/1 Reborn";

    fn wall() -> Value {
        json!({ "def": VANILLA, "statsOverride": { "attack": 0, "health": 30 } })
    }

    /// An enemy Unit a 4-Attack hit destroys, striking back for `attack`.
    fn prey(attack: i32) -> Value {
        json!({ "def": VANILLA, "statsOverride": { "attack": attack, "health": 3 } })
    }

    /// R767000: `run` under the script kept for v0.6.4c, with the `attacks` it declared then (3, Radiant 2).
    fn kept<T>(run: impl FnOnce() -> T) -> T {
        crate::under_kept_as(
            LEVEL_UP,
            script_v0_6_4c(),
            |def| {
                def.params = serde_json::from_value(json!([
                    { "key": "attacks", "base": 3, "radiant": 2, "better": "down", "step": 1, "min": 1 }
                ]))
                .expect("the v0.6.4c params");
            },
            run,
        )
    }

    fn radiant_set(s: &Scenario) -> bool {
        s.events().iter().any(|e| matches!(e, GameEvent::RadiantSet { .. }))
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
        fn r3951000_a_unit_that_destroys_the_unit_it_attacks_becomes_radiant() {
            let mut s = board(false, json!({ "field": [VANILLA] }), json!({ "field": [prey(0)] }));
            let attacker = s.unit(P1, 1).unwrap().id;
            swing(&mut s, 1);
            assert!(s.unit(P2, 1).is_none(), "the prey died");
            assert!(radiant_at(&s, 1));
            // After the combat: the hit was the base face's 4, the stats the Radiant face's now.
            s.expect_stats(attacker.as_str(), json!({ "attack": 12, "health": 12 }));
        }

        #[test]
        fn r3951000_an_attack_that_destroys_nothing_does_not() {
            let mut s = board(false, json!({ "field": [VANILLA] }), json!({ "field": [wall()] }));
            swing(&mut s, 1);
            assert!(!radiant_at(&s, 1));
            assert!(!radiant_set(&s));
        }

        #[test]
        fn r3951000_a_unit_its_cleave_destroys_counts() {
            let mut s = board(
                false,
                json!({ "field": [CLEAVER] }),
                json!({ "field": [wall(), prey(0)] }),
            );
            swing(&mut s, 1);
            assert!(s.unit(P2, 2).is_none(), "the Cleave destroyed the Unit beside the wall");
            assert!(radiant_at(&s, 1));
        }

        #[test]
        fn r174_r3951000_a_unit_that_dies_in_the_combat_it_wins_is_not_made_radiant() {
            let mut s = board(false, json!({ "field": [FRAIL] }), json!({ "field": [prey(1)] }));
            swing(&mut s, 1);
            assert!(s.unit(P2, 1).is_none(), "the prey died");
            s.expect_in_zone(FRAIL, "graveyard");
            assert!(!radiant_set(&s));
        }

        #[test]
        fn r3951000_an_enemy_units_kill_never_counts() {
            let mut s = board(false, json!({ "field": [prey(0)] }), json!({ "field": [VANILLA] }));
            s.end_turn();
            let attacker = s.unit(P2, 1).unwrap();
            let target = s.unit(P1, 1).unwrap();
            s.attack(&attacker, &target);
            assert!(s.unit(P1, 1).is_none(), "p1's Unit died");
            assert!(!s.unit(P2, 1).unwrap().radiant);
        }

        #[test]
        fn r3951000_a_defender_that_destroys_its_attacker_is_not_made_radiant() {
            let defender = json!({ "def": VANILLA, "statsOverride": { "attack": 1, "health": 30 } });
            let mut s = board(false, json!({ "field": [defender] }), json!({ "field": [FRAIL] }));
            s.end_turn();
            let attacker = s.unit(P2, 1).unwrap();
            let target = s.unit(P1, 1).unwrap();
            s.attack(&attacker, &target);
            assert!(s.unit(P2, 1).is_none(), "the attacker died on the Vanilla");
            assert!(!radiant_at(&s, 1));
        }

        #[test]
        fn r74_an_already_radiant_unit_is_unchanged() {
            let mut s = board(false, json!({ "field": [{ "def": VANILLA, "radiant": true }] }), json!({ "field": [prey(0)] }));
            swing(&mut s, 1);
            assert!(radiant_at(&s, 1));
            assert!(!radiant_set(&s));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3951000_any_attack_it_survives_makes_it_radiant() {
            let mut s = board(true, json!({ "field": [VANILLA] }), json!({ "field": [wall()] }));
            swing(&mut s, 1);
            assert!(radiant_at(&s, 1));
        }

        #[test]
        fn r174_a_reborn_body_is_not_the_unit_that_attacked() {
            // R2199000 hands the hook `survived: false` for a Unit that died in its combat and came back
            // through Reborn, a new arrival (R174).
            let killer = json!({ "def": VANILLA, "statsOverride": { "attack": 5, "health": 30 } });
            let mut s = board(true, json!({ "field": [PHOENIX] }), json!({ "field": [killer] }));
            let phoenix = s.unit(P1, 1).unwrap().id;
            swing(&mut s, 1);
            let reborn = s.last_events().iter().any(|e| {
                matches!(e, GameEvent::Summoned { instance_id, via: Some(KeywordKind::Reborn), .. }
                    if *instance_id == phoenix)
            });
            assert!(reborn, "Reborn brought the same card back");
            assert!(!radiant_at(&s, 1));
        }
    }

    /// R767000: the script kept for v0.6.4c, which counted each Unit's attacks (R2199007, R2321000).
    mod kept_v0_6_4c {
        use super::*;

        #[test]
        fn r2321000_each_units_count_rides_on_it_in_both_views() {
            kept(|| {
                let mut s = board(false, json!({ "field": [VANILLA, VANILLA] }), json!({ "field": [wall()] }));
                assert_eq!(tallies_seen(&s, P1, 1), None, "no attack yet, no count");
                swing(&mut s, 1);
                assert_eq!(tallies_seen(&s, P1, 1), counted(1, 3));
                assert_eq!(tallies_seen(&s, P2, 1), counted(1, 3), "a face-up Field Spell's count is public");
                assert_eq!(tallies_seen(&s, P1, 2), None, "each Unit counts apart");
            });
        }

        #[test]
        fn r2199000_r2199007_a_unit_that_attacks_3_times_over_several_turns_becomes_radiant_after_the_third() {
            kept(|| {
                let mut s = board(false, json!({ "field": [VANILLA] }), json!({ "field": [wall()] }));
                swing(&mut s, 1);
                next_round(&mut s);
                swing(&mut s, 1);
                assert!(!radiant_at(&s, 1));
                next_round(&mut s);
                swing(&mut s, 1);
                assert!(radiant_at(&s, 1));
            });
        }

        #[test]
        fn r2199007_attacks_made_before_this_arrived_do_not_count() {
            kept(|| {
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
            });
        }

        #[test]
        fn r44_a_canceled_attack_does_not_count() {
            kept(|| {
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
            });
        }

        #[test]
        fn r78_r174_bouncing_this_or_the_unit_restarts_the_count() {
            kept(|| {
                let mut s = board(false, json!({ "field": [VANILLA] }), json!({ "field": [wall()] }));
                swing(&mut s, 1);
                let level_up = s.backrow(P1, 1).unwrap();
                bounce_to_hand(&mut s, &level_up.id);
                assert_eq!(tallies_seen(&s, P1, 1), None);
            });
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
