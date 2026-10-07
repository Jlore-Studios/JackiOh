//! C+ #19.2 Jungle Loser (SPEC §8.7 row 19.2): end of turn, {chance}%: a forced attack on a random enemy
//! Unit; a kill across from your Bot Loser sends it Berserk (Radiant: credits it the kill, R412).

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-019-2";

const BOT_LOSER: &str = "classicplus-019-5";

/// The declared `chance` is a percentage.
const PERCENT: i32 = 100;

/// §3.1: lane N faces lane N. Each Bot Loser of yours with the enemy Unit across from it.
fn across_from_bot_losers(state: &GameState, controller: PlayerId) -> Vec<KillCredit> {
    let enemy = opponent_of(controller);
    slots_of(controller, Row::Units)
        .into_iter()
        .flat_map(|slot| {
            let bot = match card_at(state, &slot) {
                Some(bot) if bot.def_id == BOT_LOSER => bot,
                _ => return Vec::new(),
            };
            match card_at(state, &ZoneRef { player: enemy, row: Row::Units, lane: slot.lane }) {
                None => Vec::new(),
                Some(victim) => vec![KillCredit { victim_id: victim.id.clone(), to_id: bot.id.clone() }],
            }
        })
        .collect()
}

/// "{chance}% chance": rolled only when an enemy Unit it may attack stands (R129).
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
    let chance = param(&*ctx, "chance");
    if ctx.rng.chance(f64::from(chance) / f64::from(PERCENT)) {
        vec![me.id]
    } else {
        Vec::new()
    }
}

fn jungle(transfer: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |_ctx| {
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(attacks),
                each: Arc::new(move |instance_id: &str| {
                    let mut args = WithKillCreditArgs {
                        killer: json_as(json!({ "of": "instance", "instanceId": instance_id })),
                        pairs: Arc::new(|ctx: &EffectContext<'_>, _killer: &CardInstance| {
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
        base: jungle(false),
        radiant: jungle(true),
    }
}

// C+ #19.2 Jungle Loser — SPEC §8.7 row 19.2, BUILD M9 Classic+ row C+ 19.2: "At its controller's end of
// turn a seeded 25% roll; on success a forced attack (R53: no exertion, sickness ignored, the target
// strikes back) on a random enemy Unit it may attack under §4.2 step 2 (never a Can't be attacked
// Unit, a Top Loser only from its lane); an empty enemy board draws no roll (R129); if it destroys
// (R42) the enemy Unit in your Bot Loser's lane, your Bot Loser goes Berserk, and with no Bot Loser
// nothing more happens; the chance reads through `param()` (step 10%); radiant 50%, and that kill is
// credited to your Bot Loser instead: its "Whenever this destroys a Unit" trigger fires and the
// `destroyed` event names it as the killer".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const JUNGLE: &str = "classicplus-019-2";
    const BOT: &str = "classicplus-019-5";
    const TOP: &str = "classicplus-019-1";
    const VANILLA: &str = "core-008"; // 4/4
    const FIGHTER: &str = "classicplus-051"; // Jlockheed's J15 Fighter: Can't be attacked
    const MROW: &str = "core-086"; // 1/1, Can't attack; Death: take control of the Unit that destroyed this
    const FILLER: &str = "core-005";
    const DECK: [&str; 6] = [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER];

    /// TS `{ ...defaults, ...overrides }` on a side setup: every key of `overrides` replaces the default's.
    fn spread(defaults: Value, overrides: &Value) -> Value {
        let mut out = defaults;
        if let (Some(into), Some(from)) = (out.as_object_mut(), overrides.as_object()) {
            for (key, value) in from {
                into.insert(key.clone(), value.clone());
            }
        }
        out
    }

    /// p1's Jungle Loser in lane 2 (face as given), the rest of p1's field and p2's side.
    fn jungle(p1_field: Value, radiant_face: bool, p2: Value, seed: Option<&str>) -> Scenario {
        crate::register_all();
        let mut field = vec![json!({ "def": JUNGLE, "lane": 2, "radiant": radiant_face })];
        field.extend(p1_field.as_array().cloned().unwrap_or_default());
        let mut opts = json!({
            "p1": { "hand": [FILLER], "library": DECK, "field": field },
            "p2": spread(json!({ "hand": [FILLER], "library": DECK }), &p2),
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

    /// TS wrote through the live instance `loser(s)` handed back.
    fn loser_mut(s: &mut Scenario) -> &mut CardInstance {
        let id = loser(s).id;
        find_instance_mut(s.state_mut(), &id).expect("no Jungle Loser in lane 2")
    }

    /// Make the roll a certainty: the declared chance, tuned to its 100% bound.
    fn always(mut s: Scenario) -> Scenario {
        set_param(loser_mut(&mut s), "chance", 100);
        s
    }

    /// The `attackDeclared` events `id` made: each one's target and whether it was forced.
    fn attacks_by(s: &Scenario, id: &str) -> Vec<(String, bool)> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AttackDeclared { attacker_id, target_id, forced } if attacker_id == id => {
                    Some((target_id.clone(), *forced))
                }
                _ => None,
            })
            .collect()
    }

    /// TS `s.unit(p, lane) ?? ""`: the unit's id, or a reference that names nothing.
    fn unit_or_blank(s: &Scenario, player: PlayerId, lane: i32) -> String {
        s.unit(player, lane).map(|unit| unit.id).unwrap_or_default()
    }

    /// TS `toMatchObject({ killerId })` on a `destroyed` event: the killer it names.
    fn killer_of(event: Option<&GameEvent>) -> Option<String> {
        match event {
            Some(GameEvent::Destroyed { killer_id, .. }) => killer_id.clone(),
            _ => None,
        }
    }

    /// TS `toMatchObject`: every key of `pattern` is in `actual` with a matching value.
    fn matches_object(actual: &Value, pattern: &Value) -> bool {
        match (actual, pattern) {
            (Value::Object(have), Value::Object(want)) => want
                .iter()
                .all(|(key, value)| have.get(key).is_some_and(|got| matches_object(got, value))),
            (Value::Array(have), Value::Array(want)) => {
                have.len() == want.len() && have.iter().zip(want).all(|(got, value)| matches_object(got, value))
            }
            _ => actual == pattern,
        }
    }

    mod c_n19_2_jungle_loser {
        use super::*;

        #[test]
        fn is_a_2_5_5_unit_token_printed_legendary_that_acts_at_its_end_of_turn() {
            let def = crate::card_def(ID);
            assert!(def.token);
            let first = def.params.as_ref().and_then(|params| params.first()).expect("a declared number");
            assert!(matches_object(
                &serde_json::to_value(first).expect("JSON"),
                &json!({ "key": "chance", "base": 25, "radiant": 50, "step": 10, "max": 100 }),
            ));
            let scripts = script();
            assert!(scripts.base.end_of_turn.is_some());
            assert!(scripts.radiant.end_of_turn.is_some());
        }

        mod base {
            use super::*;

            #[test]
            fn r53_on_a_success_it_makes_a_forced_attack_on_an_enemy_unit_no_exertion_sickness_ignored_the_target_strikes_back(
            ) {
                let mut s = always(jungle(json!([]), false, json!({ "field": [{ "def": VANILLA, "lane": 4 }] }), None));
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
            fn the_25_roll_across_seeds_it_attacks_on_some_ends_of_turn_and_not_on_others_one_draw_each() {
                let mut attacked = 0;
                let tries = 24;
                for seed in 1..=tries {
                    let mut s = jungle(
                        json!([]),
                        false,
                        json!({ "field": [{ "def": VANILLA, "lane": 4 }] }),
                        Some(&format!("jungle-roll-{seed}")),
                    );
                    let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
                    let expected = rng.chance(0.25);
                    s.end_turn();
                    let made = !attacks_by(&s, &loser(&s).id).is_empty();
                    assert_eq!(made, expected);
                    if made {
                        attacked += 1;
                    }
                }
                assert!(attacked > 0);
                assert!(attacked < tries);
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
                    let mut s = always(jungle(
                        json!([]),
                        false,
                        json!({ "field": [{ "def": TOP, "lane": 1 }, { "def": VANILLA, "lane": 5 }] }),
                        Some(&format!("jungle-top-{seed}")),
                    ));
                    let top = s.unit(P2, 1).map(|unit| unit.id);
                    s.end_turn();
                    for (target, _) in attacks_by(&s, &loser(&s).id) {
                        assert_ne!(Some(target), top);
                    }
                }
            }

            #[test]
            fn s4_2_step_2_a_unit_that_can_t_be_attacked_is_never_drawn_c_n51_jlockheed_s_j15_fighter() {
                for seed in 1..=6 {
                    let mut s = always(jungle(
                        json!([]),
                        false,
                        json!({ "field": [{ "def": FIGHTER, "lane": 1 }, { "def": VANILLA, "lane": 5 }] }),
                        Some(&format!("jungle-fighter-{seed}")),
                    ));
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
                let mut s = always(jungle(
                    json!([{ "def": BOT, "lane": 4 }]),
                    false,
                    json!({ "field": [{ "def": VANILLA, "lane": 4 }] }),
                    None,
                ));
                let bot = unit_or_blank(&s, P1, 4);
                s.end_turn();
                assert_eq!(s.card(&bot).berserk, Some(true));
                // The kill is the Jungle Loser's own on the base face.
                let death = s.events().iter().find(|event| event.event_type() == GameEventType::Destroyed);
                assert_eq!(killer_of(death), Some(loser(&s).id));
                s.expect_stats(&bot, json!({ "attack": 5 }));
            }

            #[test]
            fn a_kill_of_a_unit_in_another_lane_sends_nobody_berserk() {
                let mut s = always(jungle(
                    json!([{ "def": BOT, "lane": 4 }]),
                    false,
                    json!({ "field": [{ "def": VANILLA, "lane": 1 }] }),
                    None,
                ));
                s.end_turn();
                assert!(s.unit(P2, 1).is_none());
                assert_eq!(s.card(unit_or_blank(&s, P1, 4)).berserk, None);
            }

            #[test]
            fn with_no_bot_loser_nothing_more_happens() {
                let mut s = always(jungle(
                    json!([{ "def": VANILLA, "lane": 4 }]),
                    false,
                    json!({ "field": [{ "def": VANILLA, "lane": 4 }] }),
                    None,
                ));
                s.end_turn();
                assert!(s.unit(P2, 4).is_none());
                assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Marked));
            }

            #[test]
            fn an_attack_that_leaves_the_unit_across_standing_sends_nobody_berserk() {
                let mut s = always(jungle(
                    json!([{ "def": BOT, "lane": 4 }]),
                    false,
                    json!({ "field": [{ "def": "core-019", "lane": 4 }] }),
                    None,
                ));
                s.end_turn();
                assert_eq!(s.card(unit_or_blank(&s, P1, 4)).berserk, None);
            }

            #[test]
            fn r386_the_chance_reads_through_param_a_degrade_to_15_and_an_upgrade_to_35() {
                for steps in [-1, 1] {
                    // Seeds where the tuned chance and the printed 25% disagree: the outcome must follow the tuned one.
                    let mut differs = 0;
                    for seed in 1..=40 {
                        let mut s = jungle(
                            json!([]),
                            false,
                            json!({ "field": [{ "def": VANILLA, "lane": 4 }] }),
                            Some(&format!("jungle-tuned-{seed}")),
                        );
                        step_param(loser_mut(&mut s), "chance", steps);
                        let (seed_text, cursor) = (s.state().seed.clone(), s.state().rng_cursor);
                        let expected = Rng::new(&seed_text, cursor).chance(f64::from(25 + 10 * steps) / 100.0);
                        if expected != Rng::new(&seed_text, cursor).chance(0.25) {
                            differs += 1;
                        }
                        s.end_turn();
                        assert_eq!(!attacks_by(&s, &loser(&s).id).is_empty(), expected);
                    }
                    assert!(differs > 0);
                }
            }

            #[test]
            fn s9_3_the_roll_and_the_random_target_replay_from_a_json_copy_to_the_same_hash() {
                let s = always(jungle(
                    json!([{ "def": BOT, "lane": 4 }]),
                    false,
                    json!({ "field": [{ "def": VANILLA, "lane": 4 }, { "def": VANILLA, "lane": 1 }] }),
                    Some("jungle-replay"),
                ));
                let action = Action::new(ActionBody::EndTurn, P1, "jungle-replay");
                let thawed: GameState =
                    serde_json::from_value(serde_json::to_value(s.state()).expect("JSON")).expect("a state");
                let live = reduce(s.state(), &action);
                assert!(live.error.is_none());
                assert!(live.events.iter().any(|event| event.event_type() == GameEventType::AttackDeclared));
                assert_eq!(hash_state(&reduce(&thawed, &action).state), hash_state(&live.state));
                let round: GameState =
                    serde_json::from_value(serde_json::to_value(&live.state).expect("JSON")).expect("a state");
                assert_eq!(round, live.state);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_50_roll() {
                for seed in 1..=16 {
                    let mut s = jungle(
                        json!([]),
                        true,
                        json!({ "field": [{ "def": VANILLA, "lane": 4 }] }),
                        Some(&format!("jungle-radiant-{seed}")),
                    );
                    let expected = Rng::new(&s.state().seed, s.state().rng_cursor).chance(0.5);
                    s.end_turn();
                    assert_eq!(!attacks_by(&s, &loser(&s).id).is_empty(), expected);
                }
            }

            #[test]
            fn r412_the_kill_across_from_your_bot_loser_is_credited_to_it_the_destroyed_event_names_it_and_its_trigger_fires(
            ) {
                let mut s = always(jungle(
                    json!([{ "def": BOT, "lane": 4 }]),
                    true,
                    json!({ "field": [{ "def": VANILLA, "lane": 4 }] }),
                    None,
                ));
                let bot = s.unit(P1, 4).expect("setup");
                let victim = s.unit(P2, 4).expect("setup");
                s.end_turn();
                let death = s.events().iter().find(|event| {
                    matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == victim.id)
                });
                assert_eq!(killer_of(death), Some(bot.id.clone()));
                s.expect_stats(&bot, json!({ "attack": 10 }));
                assert_eq!(s.card(&bot).berserk, None);
            }

            #[test]
            fn r412_a_kill_in_another_lane_stays_the_jungle_loser_s() {
                let mut s = always(jungle(
                    json!([{ "def": BOT, "lane": 4 }]),
                    true,
                    json!({ "field": [{ "def": VANILLA, "lane": 1 }] }),
                    None,
                ));
                s.end_turn();
                let death = s.events().iter().find(|event| event.event_type() == GameEventType::Destroyed);
                assert_eq!(killer_of(death), Some(loser(&s).id));
                let bot = unit_or_blank(&s, P1, 4);
                s.expect_stats(&bot, json!({ "attack": 5 }));
            }

            #[test]
            fn r42_r412_the_credited_kill_is_the_bot_loser_s_for_every_reader_miss_mrow_s_death_takes_control_of_it() {
                let mut s = always(jungle(
                    json!([{ "def": BOT, "lane": 4 }]),
                    true,
                    json!({ "field": [{ "def": MROW, "lane": 4 }] }),
                    None,
                ));
                let bot = s.unit(P1, 4).expect("setup");
                s.end_turn();
                assert_eq!(s.card(&bot).controller, P2);
            }
        }
    }
}
