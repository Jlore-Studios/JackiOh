//! C+ #19.5 Bot Loser (SPEC §8.7 row 19.5): +{attackGain} Attack whenever it is R42's killer; while
//! Berserk it attacks its own hero at your start and end of turn; the Radiant face can't go Berserk (R412).

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-019-5";

/// "Whenever this destroys a Unit, it gets +{attackGain} Attack." (`when` is read for traps only, so the test is in `run`.)
fn on_kill() -> TriggerDef {
    TriggerDef::new("bot-loser-kill", &[GameEventType::Destroyed], |ctx, event| {
        let killed_by_me = match (event, ctx.self_.as_ref()) {
            (GameEvent::Destroyed { killer_id: Some(killer), .. }, Some(me)) => *killer == me.id,
            _ => false,
        };
        if killed_by_me {
            vec![buff(json_as(json!({ "target": { "of": "self" }, "attack": param(&*ctx, "attackGain") })))]
        } else {
            Vec::new()
        }
    })
}

/// "While Berserk: At the start and end of your turn, this attacks your hero."
fn berserk_attack(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    match ctx.live_self() {
        Some(me) if is_berserk(me) => {
            vec![forced_attack_own_hero(json_as(json!({ "attacker": { "of": "self" } })))]
        }
        _ => Vec::new(),
    }
}

pub fn script() -> CardScripts {
    let kill = on_kill();

    let base = Script {
        triggers: vec![kill.clone()],
        start_of_turn: Some(hook(berserk_attack)),
        end_of_turn: Some(hook(berserk_attack)),
        condition_met: Some(condition_hook(|c| c.zone == ConditionZone::Field && is_berserk(c.self_))),
        ..Script::default()
    };

    // "This can't go Berserk": no flag, and no Berserk attacks printed (R412).
    let radiant = Script {
        triggers: vec![kill],
        static_flags: Some(StaticFlags {
            never_berserk: Some(true),
            ..StaticFlags::default()
        }),
        // R195: this face prints no condition, so it never glows; R195's list pins the hook on both faces.
        condition_met: Some(condition_hook(|_c| false)),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C+ #19.5 Bot Loser — SPEC §8.7 row 19.5, BUILD M9 Classic+ row C+ 19.5: "Rush, First Strike; whenever
// it destroys a Unit (R42), a forced attack's kill included, it gets +5 Attack permanently; while
// Berserk (set by Jungle Loser, lost on leaving the field, R78) it makes a forced attack (R53) on its
// own controller's hero at the start and at the end of that player's turn, the hero never striking back
// and that hero's Armor and caps applying; not Berserk, nothing; `conditionMet` on the field answers
// whether it is Berserk (R195); the gain reads through `param()`; radiant Charge, First Strike, +10
// Attack, and it can't go Berserk (R412): Jungle Loser's base face never sets the flag".
//
// R195's proofs for this card are in its own file (the `describe("R195 conditionMet …")` block).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BOT: &str = "classicplus-019-5";
    const JUNGLE: &str = "classicplus-019-2";
    const VANILLA: &str = "core-008"; // 4/4
    const MOTHS: &str = "core-009"; // 1/14; start of turn: every enemy Unit attacks this
    const ANTI_ONESHOT: &str = "core-073"; // hits on the hero capped at 5
    const HIT_JOB: &str = "core-016"; // destroy a target Unit
    const FILLER: &str = "core-005";
    const DECK: [&str; 6] = [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER];

    fn bot(s: &Scenario, player: PlayerId) -> CardInstance {
        for lane in 1..=5 {
            if let Some(unit) = s.unit(player, lane) {
                if unit.def_id == BOT {
                    return unit;
                }
            }
        }
        panic!("no Bot Loser");
    }

    /// TS wrote through the live instance `s.card(ref)` (or `bot(s)`) handed back.
    fn card_mut<'a>(s: &'a mut Scenario, id: &str) -> &'a mut CardInstance {
        find_instance_mut(s.state_mut(), id).expect("the card is in no zone")
    }

    fn send_berserk(s: &mut Scenario, card: &CardInstance) {
        let (seed, cursor) = (s.state().seed.clone(), s.state().rng_cursor);
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&seed, cursor);
        let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            None,
            HookOptions { controller: Some(card.controller), ..HookOptions::default() },
        );
        apply_effects(
            &[go_berserk(json_as(json!({ "target": { "of": "instance", "instanceId": card.id } })))],
            &mut ctx,
        );
    }

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

    fn with_bot(p1: Value, radiant_face: bool, p2: Value) -> Scenario {
        crate::register_all();
        let mut side1 = spread(json!({ "hand": [FILLER], "library": DECK }), &p1);
        let mut field = vec![json!({ "def": BOT, "lane": 3, "radiant": radiant_face })];
        field.extend(p1.get("field").and_then(Value::as_array).cloned().unwrap_or_default());
        side1["field"] = Value::Array(field);
        let side2 = spread(json!({ "hand": [FILLER], "library": DECK }), &p2);
        scenario(json!({ "p1": side1, "p2": side2 }))
    }

    fn glows(s: &Scenario) -> bool {
        s.view(P1).you.units.get(2).and_then(|unit| unit.as_ref()).and_then(|unit| unit.condition_active)
            == Some(true)
    }

    /// TS `s.unit(p, lane) ?? ""`: the unit's id, or a reference that names nothing.
    fn unit_or_blank(s: &Scenario, player: PlayerId, lane: i32) -> String {
        s.unit(player, lane).map(|unit| unit.id).unwrap_or_default()
    }

    mod c_n19_5_bot_loser {
        use super::*;

        #[test]
        fn is_a_2_5_5_rush_first_strike_token_printed_legendary_the_radiant_face_is_charge_and_never_berserk() {
            let def = crate::card_def(ID);
            assert_eq!(def.base.keywords, vec![Keyword::Rush, Keyword::FirstStrike]);
            assert_eq!(def.radiant.keywords, vec![Keyword::Charge, Keyword::FirstStrike]);
            let scripts = script();
            assert_eq!(scripts.radiant.static_flags.as_ref().and_then(|flags| flags.never_berserk), Some(true));
            assert!(scripts.radiant.start_of_turn.is_none());
            assert!(scripts.base.start_of_turn.is_some());
        }

        mod base {
            use super::*;

            #[test]
            fn r42_whenever_it_destroys_a_unit_it_gets_5_attack_permanently() {
                let mut s = with_bot(json!({}), false, json!({ "field": [{ "def": VANILLA, "lane": 3 }] }));
                let me = bot(&s, P1);
                let target = unit_or_blank(&s, P2, 3);
                s.attack(&me, target);
                let me = bot(&s, P1);
                s.expect_stats(&me, json!({ "attack": 10, "health": 5 }));
                s.end_turn().end_turn();
                let me = bot(&s, P1);
                s.expect_stats(&me, json!({ "attack": 10 }));
            }

            #[test]
            fn r42_a_kill_in_a_forced_attack_counts_moths_to_the_flame_s_compulsion() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [FILLER], "field": [{ "def": BOT, "lane": 2 }], "library": DECK },
                    "p2": { "hand": [FILLER], "field": [{ "def": MOTHS, "lane": 1, "damage": 10 }], "library": DECK },
                }));
                // p2's start of turn: every enemy Unit attacks Moths.
                s.start_turn();
                assert!(s.unit(P2, 1).is_none());
                let me = bot(&s, P1);
                s.expect_stats(&me, json!({ "attack": 10 }));
            }

            #[test]
            fn a_kill_it_does_not_make_gives_it_nothing() {
                let mut s = with_bot(
                    json!({ "field": [{ "def": VANILLA, "lane": 1 }] }),
                    false,
                    json!({ "field": [{ "def": VANILLA, "lane": 1, "damage": 2 }] }),
                );
                let attacker = unit_or_blank(&s, P1, 1);
                let target = unit_or_blank(&s, P2, 1);
                s.attack(attacker, target);
                let me = bot(&s, P1);
                s.expect_stats(&me, json!({ "attack": 5 }));
            }

            #[test]
            fn r53_while_berserk_it_attacks_its_own_hero_at_the_start_and_at_the_end_of_your_turn_the_hero_never_strikes_back(
            ) {
                let mut s = with_bot(json!({}), false, json!({}));
                let me = bot(&s, P1);
                send_berserk(&mut s, &me);
                s.end_turn();
                // The end of p1's turn: 5 to p1's own hero.
                s.expect_health(P1, 25);
                let me = bot(&s, P1);
                s.expect_stats(&me, json!({ "health": 5 }));
                s.end_turn();
                // The start of p1's next turn: 5 more.
                s.expect_health(P1, 20);
                let own: Vec<bool> = s
                    .events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::AttackDeclared { target_id, forced, .. } if target_id == "hero-p1" => Some(*forced),
                        _ => None,
                    })
                    .collect();
                assert_eq!(own.len(), 2);
                for forced in own {
                    assert!(forced);
                }
            }

            #[test]
            fn s4_4_the_hit_on_its_own_hero_takes_that_hero_s_armor_and_caps() {
                let mut s = with_bot(
                    json!({ "armor": 2, "backrow": [{ "def": ANTI_ONESHOT, "lane": 1 }] }),
                    false,
                    json!({}),
                );
                let id = bot(&s, P1).id;
                step_param(card_mut(&mut s, &id), "attackGain", 0);
                card_mut(&mut s, &id).buffs.attack += 5;
                let me = bot(&s, P1);
                send_berserk(&mut s, &me);
                s.end_turn();
                // 10 attack, 2 Armor, then the cap of 5.
                s.expect_health(P1, 25);
            }

            #[test]
            fn not_berserk_it_attacks_nothing_at_the_start_or_end_of_a_turn() {
                let mut s = with_bot(json!({}), false, json!({}));
                s.end_turn().end_turn();
                s.expect_health(P1, 30);
            }

            #[test]
            fn r78_berserk_is_lost_when_it_leaves_the_field_back_by_reborn_it_is_not_berserk_and_attacks_nothing() {
                let mut s = with_bot(json!({ "hand": [HIT_JOB, FILLER] }), false, json!({}));
                let first = bot(&s, P1);
                card_mut(&mut s, &first.id).granted_keywords.push(Keyword::Reborn);
                send_berserk(&mut s, &first);
                assert_eq!(s.card(&first).berserk, Some(true));
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": first.id }] }));
                assert!(s.last_events().iter().any(|event| matches!(
                    event,
                    GameEvent::Destroyed { instance_id, .. } if *instance_id == first.id
                )));
                let back = bot(&s, P1);
                assert_eq!(back.berserk, None);
                s.end_turn();
                s.expect_health(P1, 30);
            }

            #[test]
            fn r412_jungle_loser_s_base_face_sends_it_berserk_when_it_kills_the_unit_across_from_it() {
                let mut s = with_bot(
                    json!({ "field": [{ "def": JUNGLE, "lane": 1 }] }),
                    false,
                    json!({ "field": [{ "def": VANILLA, "lane": 3 }] }),
                );
                let jungle = s.unit(P1, 1).unwrap_or_else(|| bot(&s, P1)).id;
                set_param(card_mut(&mut s, &jungle), "chance", 100);
                s.end_turn();
                assert_eq!(bot(&s, P1).berserk, Some(true));
                assert_eq!(
                    s.view(P2).opponent.units.get(2).and_then(|unit| unit.as_ref()).and_then(|unit| unit.berserk),
                    Some(true)
                );
            }

            #[test]
            fn r386_the_gain_reads_through_param_an_upgrade_makes_it_6() {
                let mut s = with_bot(json!({}), false, json!({ "field": [{ "def": VANILLA, "lane": 3 }] }));
                let id = bot(&s, P1).id;
                step_param(card_mut(&mut s, &id), "attackGain", 1);
                let target = unit_or_blank(&s, P2, 3);
                s.attack(&id, target);
                let me = bot(&s, P1);
                s.expect_stats(&me, json!({ "attack": 11 }));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn t_10_10_charge_first_strike_it_gets_10_attack_per_kill() {
                let mut s = with_bot(json!({}), true, json!({ "field": [{ "def": VANILLA, "lane": 3 }] }));
                let me = bot(&s, P1);
                let target = unit_or_blank(&s, P2, 3);
                s.attack(&me, target);
                let me = bot(&s, P1);
                s.expect_stats(&me, json!({ "attack": 20, "health": 10 }));
            }

            #[test]
            fn r412_it_can_t_go_berserk_jungle_loser_s_base_face_never_sets_the_flag() {
                let mut s = with_bot(
                    json!({ "field": [{ "def": JUNGLE, "lane": 1 }] }),
                    true,
                    json!({ "field": [{ "def": VANILLA, "lane": 3 }] }),
                );
                let jungle = s.unit(P1, 1).unwrap_or_else(|| bot(&s, P1)).id;
                set_param(card_mut(&mut s, &jungle), "chance", 100);
                s.end_turn();
                assert!(s.unit(P2, 3).is_none());
                assert_eq!(bot(&s, P1).berserk, None);
                s.end_turn();
                s.expect_health(P1, 30);
            }

            #[test]
            fn r412_a_berserk_bot_loser_made_radiant_stops_attacking_its_hero() {
                let mut s = with_bot(json!({}), false, json!({}));
                let me = bot(&s, P1);
                send_berserk(&mut s, &me);
                card_mut(&mut s, &me.id).radiant = true;
                s.end_turn().end_turn();
                s.expect_health(P1, 30);
            }
        }

        mod r195_conditionmet_on_the_field_whether_it_is_berserk {
            use super::*;

            #[test]
            fn lit_while_berserk_and_its_end_of_turn_attacks_the_hero() {
                let mut s = with_bot(json!({}), false, json!({}));
                assert!(!glows(&s));
                let me = bot(&s, P1);
                send_berserk(&mut s, &me);
                assert!(glows(&s));
                s.end_turn();
                s.expect_health(P1, 25);
            }

            #[test]
            fn unlit_when_not_berserk_and_its_end_of_turn_attacks_nothing() {
                let mut s = with_bot(json!({}), false, json!({}));
                assert!(!glows(&s));
                s.end_turn();
                s.expect_health(P1, 30);
            }

            #[test]
            fn never_on_the_opponent_s_view() {
                let mut s = with_bot(json!({}), false, json!({}));
                let me = bot(&s, P1);
                send_berserk(&mut s, &me);
                assert_eq!(
                    s.view(P2)
                        .opponent
                        .units
                        .get(2)
                        .and_then(|unit| unit.as_ref())
                        .and_then(|unit| unit.condition_active),
                    None
                );
            }

            #[test]
            fn the_radiant_face_prints_no_condition_it_never_glows_berserk_sent_or_not_and_attacks_nothing() {
                let mut s = with_bot(json!({}), true, json!({}));
                assert!(!glows(&s));
                let me = bot(&s, P1);
                send_berserk(&mut s, &me);
                assert!(!glows(&s));
                s.end_turn();
                s.expect_health(P1, 30);
            }
        }
    }
}
