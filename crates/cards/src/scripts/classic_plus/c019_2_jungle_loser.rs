//! C+ #19.2 Jungle Loser (SPEC §8.7 row 19.2): Can't attack; end of turn, a forced attack on a random
//! enemy Unit; a kill across from your Bot Loser sends it Berserk (Radiant: credits it the kill, R412).
//!
//! Can't attack is a printed keyword, so its controller never declares an attack with it. The end-of-turn
//! attack is a forced one, which skips §4.2 steps 1 to 3, Can't attack's included, and keeps only step 2's
//! target restrictions (R53): it is the card's only attack. It is
//! unconditional: no roll is drawn, only the random target's pick. `script_v0_6_0d` keeps the older
//! one, which drew a 100% roll (R386) each end of turn (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-019-2";

const BOT_LOSER: &str = "classicplus-019-5";

/// R767000: the declared `chance` of the script kept for v0.6.0d is a percentage.
const PERCENT: i32 = 100;

/// §3.1: lane N faces lane N. Each Bot Loser of yours with each enemy Unit across from it. A Unit standing
/// in a backrow zone is in that zone's lane (R1081000), a Bot Loser or an enemy Unit alike.
fn across_from_bot_losers(state: &GameState, controller: PlayerId) -> Vec<KillCredit> {
    let lane_of = |unit: &CardInstance| {
        slot_of(state, unit).filter(|_| in_units_row_or_standing(state, unit)).map(|slot| slot.lane)
    };
    let enemies = active_units_of(state, opponent_of(controller));
    active_units_of(state, controller)
        .into_iter()
        .filter(|bot| bot.def_id == BOT_LOSER)
        .flat_map(|bot| {
            let lane = lane_of(bot);
            enemies
                .iter()
                .filter(move |victim| lane.is_some() && lane_of(victim) == lane)
                .map(move |victim| KillCredit { victim_id: victim.id.clone(), to_id: bot.id.clone() })
        })
        .collect()
}

/// The attack is made whenever an enemy Unit it may attack stands, with no roll (R129).
fn attacks(ctx: &mut EffectContext<'_>) -> Vec<String> {
    let me = match ctx.live_self() {
        Some(me) => me.clone(),
        None => return Vec::new(),
    };
    if me.zone.z() != ZoneName::Field {
        return Vec::new();
    }
    if random_attack_targets(&*ctx.state, &me, AttackAmong::EnemyUnits).is_empty() {
        return Vec::new();
    }
    vec![me.id]
}

/// R767000: through v0.6.0d one `rng.chance(chance / 100)` was drawn each end of turn, even at 100%,
/// and only when an enemy Unit it may attack stood (R129).
fn attacks_v0_6_0d(ctx: &mut EffectContext<'_>) -> Vec<String> {
    let me = match ctx.live_self() {
        Some(me) => me.clone(),
        None => return Vec::new(),
    };
    if me.zone.z() != ZoneName::Field {
        return Vec::new();
    }
    if random_attack_targets(&*ctx.state, &me, AttackAmong::EnemyUnits).is_empty() {
        return Vec::new();
    }
    let chance = param(&*ctx, "chance");
    if ctx.rng.chance(f64::from(chance) / f64::from(PERCENT)) {
        vec![me.id]
    } else {
        Vec::new()
    }
}

fn jungle(transfer: bool, rolled: bool) -> Script {
    let cards: ForEachCardCards =
        if rolled { Arc::new(attacks_v0_6_0d) } else { Arc::new(attacks) };
    Script {
        end_of_turn: Some(hook(move |_ctx| {
            vec![for_each_card(ForEachCardArgs {
                cards: cards.clone(),
                each: Arc::new(move |instance_id: &str| {
                    let mut args = WithKillCreditArgs {
                        killer: json_as(json!({ "of": "instance", "instanceId": instance_id })),
                        pairs: Arc::new(|ctx: &mut EffectContext<'_>, _killer: &CardInstance| {
                            across_from_bot_losers(&*ctx.state, ctx.controller)
                        }),
                        transfer,
                        during: forced_attack_random(json_as(json!({
                            "attacker": { "of": "instance", "instanceId": instance_id },
                            "among": "enemyUnits",
                        }))),
                        then: None,
                    };
                    // Base: the Bot Loser across from the kill goes Berserk. Radiant: the credit is the whole of it.
                    if !transfer {
                        args.then = Some(Arc::new(|pair: &KillCredit| {
                            vec![go_berserk(json_as(json!({
                                "target": { "of": "instance", "instanceId": pair.to_id },
                            })))]
                        }));
                    }
                    with_kill_credit(args)
                }),
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: jungle(false, false),
        radiant: jungle(true, false),
    }
}

/// R767000: through v0.6.0d its end-of-turn attack drew a 100% roll first; `KEPT_SCRIPTS` folds those
/// replays under it.
pub fn script_v0_6_0d() -> CardScripts {
    CardScripts {
        base: jungle(false, true),
        radiant: jungle(true, true),
    }
}

// C+ #19.2 Jungle Loser — SPEC §8.7 row 19.2, BUILD M9 Classic+ row C+ 19.2 (R53, §4.2, R129, R42).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::js;
    use crate::merged;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const JUNGLE: &str = "classicplus-019-2";
    const BOT: &str = "classicplus-019-5";
    const TOP: &str = "classicplus-019-1";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FELINORS: &str = "core-012"; // Duplicating Felinors: seeded on the field, its Cry never fires (R1)
    const FIGHTER: &str = "classicplus-051"; // Jlockeed's J15 Fighter: Can't be attacked
    const MROW: &str = "core-086"; // "Miss" Mrow: Can't attack; Death: take control of the Unit that destroyed this
    const FILLER: &str = "core-005";
    const DECK: [&str; 6] = [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER];

    /// p1's Jungle Loser in lane 2 (face as given), the rest of p1's field and p2's side.
    fn jungle(p1_field: Value, radiant_face: bool, p2: Value, seed: Option<&str>) -> Scenario {
        crate::register_all();
        let mut field = vec![json!({ "def": JUNGLE, "lane": 2, "radiant": radiant_face })];
        field.extend(p1_field.as_array().cloned().unwrap_or_default());
        let mut opts = json!({
            "p1": { "hand": [FILLER], "library": DECK, "field": field },
            "p2": merged(json!({ "hand": [FILLER], "library": DECK }), p2),
        });
        if let Some(seed) = seed {
            opts["seed"] = json!(seed);
        }
        scenario(opts)
    }

    fn loser(s: &Scenario) -> CardInstance {
        match s.unit(P1, 2) {
            Some(unit) if unit.def_id == JUNGLE => unit,
            _ => panic!("no Jungle Loser in lane 2"),
        }
    }

    fn loser_mut(s: &mut Scenario) -> &mut CardInstance {
        let id = loser(s).id;
        find_instance_mut(s.state_mut(), &id).expect("no Jungle Loser in lane 2")
    }

    /// The `attackDeclared` events `id` made: each one's target and whether it was forced.
    fn attacks_by(s: &Scenario, id: &str) -> Vec<(String, bool)> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AttackDeclared { attacker_id, target_id, forced, .. } if attacker_id == id => {
                    Some((target_id.clone(), *forced))
                }
                _ => None,
            })
            .collect()
    }

    use crate::unit_or_blank;

    /// R53: on `radiant_face`, across 12 seeds, the Loser makes one forced attack at the end of p1's turn
    /// and a second at the end of p1's next, none at the end of p2's: the 100% roll never misses.
    fn every_end_of_turn(radiant_face: bool) {
        for seed in 1..=12 {
            let mut s = jungle(
                json!([]),
                radiant_face,
                json!({ "field": [{ "def": FELINORS, "lane": 4 }, { "def": FELINORS, "lane": 5 }] }),
                Some(&format!("jungle-every-{seed}")),
            );
            let id = loser(&s).id;
            s.end_turn();
            let made = attacks_by(&s, &id);
            assert_eq!(made.len(), 1, "seed {seed}");
            assert!(made.iter().all(|(_, forced)| *forced));
            s.end_turn();
            assert_eq!(attacks_by(&s, &id).len(), 1, "seed {seed}: not at the end of p2's turn");
            s.end_turn();
            let made = attacks_by(&s, &id);
            assert_eq!(made.len(), 2, "seed {seed}");
            assert!(made.iter().all(|(_, forced)| *forced));
        }
    }

    /// §4.2 step 1, §6.1: on `radiant_face`, its controller can declare no attack with it, by the legal
    /// actions or by the reducer, while an enemy Unit stands to attack.
    fn cant_declare(radiant_face: bool) {
        let mut s = jungle(json!([]), radiant_face, json!({ "field": [{ "def": VANILLA, "lane": 4 }] }), None);
        let id = loser(&s).id;
        let declared = legal_actions(s.state(), P1)
            .into_iter()
            .filter(|action| matches!(action, ActionBody::Attack { attacker_id, .. } if *attacker_id == id))
            .count();
        assert_eq!(declared, 0);
        let vanilla = unit_or_blank(&s, P2, 4);
        s.expect_refused_with(|s| s.attack(id.as_str(), vanilla.as_str()), "that unit cannot attack");
    }

    /// On `radiant_face`, across 12 seeds, with one target the Loser attacks it and the end of turn draws
    /// one number fewer than the roll of `script_v0_6_0d` did.
    fn attacks_with_no_roll(radiant_face: bool) {
        for seed in 1..=12 {
            let opts = json!({ "field": [{ "def": FELINORS, "lane": 4 }] });
            let name = format!("jungle-noroll-{seed}");
            let mut s = jungle(json!([]), radiant_face, opts.clone(), Some(&name));
            let id = loser(&s).id;
            let before = s.state().rng_cursor;
            s.end_turn();
            assert_eq!(attacks_by(&s, &id).len(), 1, "seed {seed}");
            let drawn = s.state().rng_cursor - before;
            // The definition it shipped with declared the `chance` it read.
            let chance = |def: &mut CardDef| {
                let declared = json!([{ "key": "chance", "base": 100, "radiant": 100, "better": "up", "step": 10, "min": 1, "max": 100 }]);
                def.params = serde_json::from_value(declared).ok();
            };
            let (old_id, old_drawn) = crate::under_kept_as(ID, script_v0_6_0d(), chance, || {
                let mut old = jungle(json!([]), radiant_face, opts, Some(&name));
                let old_id = loser(&old).id;
                let old_before = old.state().rng_cursor;
                old.end_turn();
                assert_eq!(attacks_by(&old, &old_id).len(), 1, "seed {seed}");
                (old_id, old.state().rng_cursor - old_before)
            });
            assert!(!old_id.is_empty());
            assert_eq!(old_drawn, drawn + 1, "seed {seed}: the kept script rolls once more");
        }
    }

    fn killer_of(event: Option<&GameEvent>) -> Option<String> {
        match event {
            Some(GameEvent::Destroyed { killer_id, .. }) => killer_id.clone(),
            _ => None,
        }
    }

    mod c_n19_2_jungle_loser {
        use super::*;

        #[test]
        fn base_enters_at_4_4_radiant_unchanged() {
            for (radiant_face, size) in [(false, 4), (true, 8)] {
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": JUNGLE, "radiant": radiant_face }, FILLER], "library": DECK },
                    "p2": { "hand": [FILLER], "library": DECK },
                }));
                s.play(JUNGLE, json!({ "zone": 1 }));
                let unit = unit_or_blank(&s, P1, 1);
                s.expect_stats(&unit, json!({ "attack": size, "health": size, "maxHealth": size }));
            }
        }

        mod base {
            use super::*;

            #[test]
            fn r53_on_a_success_it_makes_a_forced_attack_on_an_enemy_unit_no_exertion_sickness_ignored_the_target_strikes_back(
            ) {
                // A 3/4: the 4/4 Loser's 4 kills it, and its strike back of 3 leaves the Loser at 1.
                let mut s = jungle(json!([]), false, json!({ "field": [{ "def": FELINORS, "lane": 4 }] }), None);
                let turn = s.state().turn;
                loser_mut(&mut s).summoned_turn = Some(turn);
                let victim = s.unit(P2, 4).expect("setup");
                s.end_turn();
                let me = loser(&s);
                let made = attacks_by(&s, &me.id);
                assert_eq!(made.len(), 1);
                assert_eq!(made[0], (victim.id.clone(), true));
                s.expect_in_zone(&victim, "graveyard");
                s.expect_stats(&me, json!({ "health": 1 }));
                assert!(!s.card(&me).exertion.attacked);
            }

            #[test]
            fn r53_it_attacks_at_every_end_of_its_controllers_turn() {
                every_end_of_turn(false);
            }

            #[test]
            fn s4_2_step_1_it_cant_declare_an_attack() {
                cant_declare(false);
            }

            #[test]
            fn r129_an_empty_enemy_board_draws_no_roll() {
                let mut s = jungle(json!([]), false, json!({}), None);
                let cursor = s.state().rng_cursor;
                // The end of p1's turn rolls nothing; the next draw the rng makes is p2's own.
                s.end_turn();
                assert_eq!(attacks_by(&s, &loser(&s).id), Vec::<(String, bool)>::new());
                assert_eq!(s.state().rng_cursor, cursor);
            }

            #[test]
            fn s4_2_step_2_a_top_loser_is_never_drawn_from_another_lane() {
                for seed in 1..=10 {
                    let mut s = jungle(
                        json!([]),
                        false,
                        json!({ "field": [{ "def": TOP, "lane": 1 }, { "def": FELINORS, "lane": 5 }] }),
                        Some(&format!("jungle-top-{seed}")),
                    );
                    let top = s.unit(P2, 1).map(|unit| unit.id);
                    s.end_turn();
                    for (target, _) in attacks_by(&s, &loser(&s).id) {
                        assert_ne!(Some(target), top);
                    }
                }
            }

            #[test]
            fn s4_2_step_2_a_unit_that_can_t_be_attacked_is_never_drawn_c_n51_jlockeed_s_j15_fighter() {
                for seed in 1..=6 {
                    let mut s = jungle(
                        json!([]),
                        false,
                        json!({ "field": [{ "def": FIGHTER, "lane": 1 }, { "def": FELINORS, "lane": 5 }] }),
                        Some(&format!("jungle-fighter-{seed}")),
                    );
                    let fighter = s.unit(P2, 1).map(|unit| unit.id);
                    s.end_turn();
                    let targets: Vec<String> =
                        attacks_by(&s, &loser(&s).id).into_iter().map(|(target, _)| target).collect();
                    assert_eq!(targets.len(), 1);
                    if let Some(fighter) = &fighter {
                        assert!(!targets.contains(fighter));
                    }
                }
            }

            #[test]
            fn s4_2_step_2_with_only_a_unit_it_may_not_attack_nothing_is_rolled_or_attacked() {
                let mut s = jungle(json!([]), false, json!({ "field": [{ "def": TOP, "lane": 1 }] }), None);
                let cursor = s.state().rng_cursor;
                s.end_turn();
                assert_eq!(attacks_by(&s, &loser(&s).id), Vec::<(String, bool)>::new());
                assert_eq!(s.state().rng_cursor, cursor);
            }

            #[test]
            fn r42_if_it_destroys_the_enemy_unit_across_from_your_bot_loser_your_bot_loser_goes_berserk() {
                let mut s = jungle(
                    json!([{ "def": BOT, "lane": 4 }]),
                    false,
                    json!({ "field": [{ "def": FELINORS, "lane": 4 }] }),
                    None,
                );
                let bot = unit_or_blank(&s, P1, 4);
                s.end_turn();
                assert_eq!(s.card(&bot).berserk, Some(true));
                // The kill is the Jungle Loser's own on the base face.
                let death = s.events().iter().find(|event| event.event_type() == GameEventType::Destroyed);
                assert_eq!(killer_of(death), Some(loser(&s).id));
                s.expect_stats(&bot, json!({ "attack": 4 }));
            }

            /// R1081000: an enemy Unit standing in the backrow zone of your Bot Loser's lane is across from it.
            #[test]
            fn r1081000_a_kill_standing_in_the_backrow_across_from_your_bot_loser_sends_it_berserk() {
                let mut s = jungle(
                    json!([{ "def": BOT, "lane": 4 }]),
                    false,
                    json!({ "field": [{ "def": VANILLA, "lane": 4 }] }),
                    None,
                );
                let vanilla = s.card(VANILLA).id.clone();
                crate::stand_in_backrow(&mut s, &vanilla, 4);
                let bot = unit_or_blank(&s, P1, 4);
                s.end_turn();
                s.expect_in_zone(vanilla.as_str(), "graveyard");
                assert_eq!(s.card(&bot).berserk, Some(true));
            }

            #[test]
            fn a_kill_of_a_unit_in_another_lane_sends_nobody_berserk() {
                let mut s = jungle(
                    json!([{ "def": BOT, "lane": 4 }]),
                    false,
                    json!({ "field": [{ "def": VANILLA, "lane": 1 }] }),
                    None,
                );
                s.end_turn();
                assert!(s.unit(P2, 1).is_none());
                assert_eq!(s.card(unit_or_blank(&s, P1, 4)).berserk, None);
            }

            #[test]
            fn with_no_bot_loser_nothing_more_happens() {
                let mut s = jungle(
                    json!([{ "def": VANILLA, "lane": 4 }]),
                    false,
                    json!({ "field": [{ "def": VANILLA, "lane": 4 }] }),
                    None,
                );
                s.end_turn();
                assert!(s.unit(P2, 4).is_none());
                assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Marked));
            }

            #[test]
            fn an_attack_that_leaves_the_unit_across_standing_sends_nobody_berserk() {
                let mut s = jungle(
                    json!([{ "def": BOT, "lane": 4 }]),
                    false,
                    json!({ "field": [{ "def": "core-019", "lane": 4 }] }),
                    None,
                );
                s.end_turn();
                assert_eq!(s.card(unit_or_blank(&s, P1, 4)).berserk, None);
            }

            #[test]
            fn a_unit_it_may_attack_gets_the_attack_with_no_roll_drawn() {
                attacks_with_no_roll(false);
            }

            #[test]
            fn the_card_declares_no_chance_param() {
                crate::register_all();
                assert!(crate::card_def(ID).params.is_none());
            }

            #[test]
            fn s9_3_the_roll_and_the_random_target_replay_from_a_json_copy_to_the_same_hash() {
                let s = jungle(
                    json!([{ "def": BOT, "lane": 4 }]),
                    false,
                    json!({ "field": [{ "def": VANILLA, "lane": 4 }, { "def": VANILLA, "lane": 1 }] }),
                    Some("jungle-replay"),
                );
                let action = Action::new(ActionBody::EndTurn, P1, "jungle-replay");
                let thawed: GameState = serde_json::from_value(js(s.state())).expect("a state");
                let live = reduce(s.state(), &action);
                assert!(live.error.is_none());
                assert!(live.events.iter().any(|event| event.event_type() == GameEventType::AttackDeclared));
                assert_eq!(hash_state(&reduce(&thawed, &action).state), hash_state(&live.state));
                let round: GameState = serde_json::from_value(js(&live.state)).expect("a state");
                assert_eq!(round, live.state);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r53_it_attacks_at_every_end_of_its_controllers_turn() {
                every_end_of_turn(true);
            }

            #[test]
            fn s4_2_step_1_it_cant_declare_an_attack() {
                cant_declare(true);
            }

            #[test]
            fn a_unit_it_may_attack_gets_the_attack_with_no_roll_drawn() {
                attacks_with_no_roll(true);
            }

            #[test]
            fn r412_the_kill_across_from_your_bot_loser_is_credited_to_it_the_destroyed_event_names_it_and_its_trigger_fires(
            ) {
                let mut s = jungle(
                    json!([{ "def": BOT, "lane": 4 }]),
                    true,
                    json!({ "field": [{ "def": VANILLA, "lane": 4 }] }),
                    None,
                );
                let bot = s.unit(P1, 4).expect("setup");
                let victim = s.unit(P2, 4).expect("setup");
                s.end_turn();
                let death = s.events().iter().find(|event| {
                    matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == victim.id)
                });
                assert_eq!(killer_of(death), Some(bot.id.clone()));
                s.expect_stats(&bot, json!({ "attack": 7 }));
                assert_eq!(s.card(&bot).berserk, None);
            }

            #[test]
            fn r412_a_kill_in_another_lane_stays_the_jungle_loser_s() {
                let mut s = jungle(
                    json!([{ "def": BOT, "lane": 4 }]),
                    true,
                    json!({ "field": [{ "def": VANILLA, "lane": 1 }] }),
                    None,
                );
                s.end_turn();
                let death = s.events().iter().find(|event| event.event_type() == GameEventType::Destroyed);
                assert_eq!(killer_of(death), Some(loser(&s).id));
                let bot = unit_or_blank(&s, P1, 4);
                s.expect_stats(&bot, json!({ "attack": 4 }));
            }

            #[test]
            fn r42_r412_the_credited_kill_is_the_bot_loser_s_for_every_reader_miss_mrow_s_death_takes_control_of_it() {
                let mut s = jungle(
                    json!([{ "def": BOT, "lane": 4 }]),
                    true,
                    json!({ "field": [{ "def": MROW, "lane": 4 }] }),
                    None,
                );
                let bot = s.unit(P1, 4).expect("setup");
                s.end_turn();
                assert_eq!(s.card(&bot).controller, P2);
            }
        }
    }
}
