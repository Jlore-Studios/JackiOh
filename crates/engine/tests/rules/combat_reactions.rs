//! Stinkyscape+ E6 (issue #3550, R3550000–R3550009): the set's small combat and reaction verbs,
//! proved on fixture scripts (`fixtures::combat_reactions`, the `testkit` feature's `scenario()`),
//! with no card yet.

use jackioh_engine::effects;
use jackioh_engine::subsystems::activate::{ability_uses_this_turn, activation_views_for};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat_reactions::*;
use crate::rules::fixtures::harness::{apply_now, types_of};

/// The side's pile as the `scenario()` JSON names it: p1 active in main with no auto-end, plain
/// cards to draw.
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    register_e6_fixtures();
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!([PLAIN, PLAIN, PLAIN, PLAIN]);
        }
    }
    let mut s = scenario(json!({ "seed": seed, "active": "p1", "p1": p1, "p2": p2 }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

fn id_at(s: &Scenario, player: PlayerId, lane: i32) -> String {
    s.unit(player, lane)
        .unwrap_or_else(|| panic!("no unit at {player:?}'s lane {lane}"))
        .id
}

fn backrow_id(s: &Scenario, player: PlayerId, lane: i32) -> String {
    s.backrow(player, lane)
        .unwrap_or_else(|| panic!("no backrow card at {player:?}'s lane {lane}"))
        .id
}

fn hand_id(s: &Scenario, player: PlayerId, def: &str) -> String {
    s.hand(player)
        .iter()
        .find(|card| card.def_id == def)
        .unwrap_or_else(|| panic!("no {def} in hand"))
        .id
        .clone()
}

fn pick(id: &str) -> Value {
    json!({ "targets": [{ "pick": "instance", "instanceId": id }] })
}

fn face_up(def: &str) -> Value {
    json!([{ "def": def, "faceUp": true }])
}

/// The attackers of the forced attacks among `events`, in order.
fn forced_attackers(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::AttackDeclared {
                attacker_id,
                forced: true,
                ..
            } => Some(attacker_id.clone()),
            _ => None,
        })
        .collect()
}

/// The ids every attack action `player` may take names as its target, for one attacker.
fn attack_targets_offered(s: &Scenario, attacker: &str) -> Vec<String> {
    let player = s.card(attacker).controller;
    legal_actions(s.state(), player)
        .into_iter()
        .filter_map(|action| match action {
            ActionBody::Attack {
                attacker_id,
                target_id,
            } if attacker_id == attacker => Some(target_id),
            _ => None,
        })
        .collect()
}

fn lists_activate(state: &GameState, instance: &str) -> bool {
    legal_actions(state, P1)
        .iter()
        .any(|body| matches!(body, ActionBody::Activate { instance_id, .. } if instance_id == instance))
}

fn zap(id: &str) -> ActionBody {
    ActionBody::Activate {
        instance_id: id.to_string(),
        ability: Some("zap".to_string()),
        targets: None,
        modes: None,
        tributes: None,
    }
}

fn slow(id: &str) -> ActionBody {
    ActionBody::Activate {
        instance_id: id.to_string(),
        ability: Some("slow".to_string()),
        targets: None,
        modes: None,
        tributes: None,
    }
}

fn cooldown_left(s: &Scenario, id: &str) -> Option<i32> {
    activation_views_for(s.state(), P1, s.card(id))
        .expect("the card shows its ability")
        .into_iter()
        .next()
        .expect("one ability")
        .cooldown_turns_left
}

mod r3550000_attack_an_enemy_as_it_is_summoned {
    use super::*;

    #[test]
    fn r3550000_an_enemy_summon_is_attacked_at_once_even_by_a_unit_that_cant_attack() {
        let mut s = board(
            "r3550000-summon",
            json!({ "field": [AMBUSHER] }),
            json!({ "hand": [PLAIN] }),
        );
        let ambusher = id_at(&s, P1, 1);
        s.end_turn();
        let plain = hand_id(&s, P2, PLAIN);
        s.play(plain.as_str(), json!({ "zone": 1 }));
        let events = s.events();
        assert_eq!(forced_attackers(events), vec![ambusher.clone()]);
        let victim = id_at(&s, P2, 1);
        s.expect_stats(victim.as_str(), json!({ "health": 3 - AMBUSHER_ATTACK }));
        // A forced attack exerts nothing (R53), though the attacker Can't attack.
        let exertion = &s.card(ambusher.as_str()).exertion;
        assert!(!exertion.attacked && exertion.attacks.unwrap_or(0) == 0);
        // Foreshadow's rider queues "lose 1 mana next turn" each time.
        assert_eq!(s.state().players[P1].mana.next_turn_mod, -1);
    }

    #[test]
    fn r3550000_a_played_enemy_unit_is_attacked_once_it_resolves_and_own_summons_are_not() {
        let mut s = board(
            "r3550000-played",
            json!({ "field": [AMBUSHER], "hand": [PLAIN] }),
            json!({ "hand": [PLAIN] }),
        );
        let ambusher = id_at(&s, P1, 1);
        // An own summon is not an enemy summon: no forced attack.
        let mine = hand_id(&s, P1, PLAIN);
        s.play(mine.as_str(), json!({ "zone": 2 }));
        assert!(forced_attackers(s.events()).is_empty());
        // A played enemy Unit is attacked once it resolves: one combat, not two.
        s.end_turn();
        let theirs = hand_id(&s, P2, PLAIN);
        s.play(theirs.as_str(), json!({ "zone": 1 }));
        assert_eq!(forced_attackers(s.events()), vec![ambusher]);
    }
}

mod r3550001_n_attacks_a_turn_and_only_across {
    use super::*;

    #[test]
    fn r3550001_a_unit_attacks_as_many_times_as_its_text_says() {
        let mut s = board(
            "r3550001-three",
            json!({ "field": [SPRAYER] }),
            json!({ "field": [BIG] }),
        );
        let sprayer = id_at(&s, P1, 1);
        let big = id_at(&s, P2, 1);
        for _ in 0..3 {
            s.attack(sprayer.as_str(), big.as_str());
        }
        s.expect_stats(big.as_str(), json!({ "health": 30 - 3 }));
        s.expect_refused(|s| s.attack(sprayer.as_str(), big.as_str()));
    }

    #[test]
    fn r3550001_it_attacks_only_the_unit_across_never_the_hero() {
        let s = board(
            "r3550001-across",
            json!({ "field": [SPRAYER] }),
            json!({ "field": [BIG, PLAIN] }),
        );
        let big = id_at(&s, P2, 1);
        assert_eq!(attack_targets_offered(&s, &id_at(&s, P1, 1)), vec![big]);
    }
}

mod r3550002_when_frozen_and_freeze_damaged {
    use super::*;

    fn freeze(id: &str) -> Effect {
        effects::freeze(json_as(json!({
            "target": { "of": "instance", "instanceId": id },
        })))
    }

    fn hurt(id: &str, amount: i32) -> Effect {
        effects::damage(json_as(json!({
            "to": { "of": "instance", "instanceId": id },
            "amount": amount,
        })))
    }

    #[test]
    fn r3550002_when_this_is_frozen_runs_on_each_freeze() {
        let mut s = board("r3550002-thaw", json!({ "field": [THAWER] }), json!({}));
        let thawer = id_at(&s, P1, 1);
        apply_now(s.state_mut(), hurt(&thawer, 2));
        s.expect_stats(thawer.as_str(), json!({ "health": 6 - 2 }));
        apply_now(s.state_mut(), freeze(&thawer));
        s.expect_stats(thawer.as_str(), json!({ "health": 6 }));
        // A re-freeze answers again.
        apply_now(s.state_mut(), hurt(&thawer, 1));
        apply_now(s.state_mut(), freeze(&thawer));
        s.expect_stats(thawer.as_str(), json!({ "health": 6 }));
    }

    #[test]
    fn r3550002_a_unit_this_damages_is_frozen_and_a_hero_is_not() {
        let mut s = board(
            "r3550002-frost",
            json!({ "field": [FROSTER] }),
            json!({ "field": [PLAIN] }),
        );
        let froster = id_at(&s, P1, 1);
        let plain = id_at(&s, P2, 1);
        s.attack(froster.as_str(), plain.as_str());
        assert!(
            s.card(plain.as_str()).frozen.is_some(),
            "the damaged Unit is Frozen"
        );
        s.end_turn();
        s.end_turn();
        s.attack(froster.as_str(), "hero");
        s.expect_health(P2, HERO_HEALTH - 1);
        assert!(
            !types_of(s.last_events()).contains(&GameEventType::Marked),
            "no freeze rides a hit on a hero"
        );
    }
}

mod r3550003_activate_cooldowns {
    use super::*;

    #[test]
    fn r3550003_a_three_turn_cooldown_sits_out_two_of_its_users_turns() {
        let mut s = board(
            "r3550003-cooler",
            json!({ "backrow": face_up(COOLER) }),
            json!({}),
        );
        let cooler = backrow_id(&s, P1, 1);
        assert!(lists_activate(s.state(), &cooler));
        s.act(zap(&cooler), P1, "zap");
        assert_eq!(
            ability_uses_this_turn(s.state(), s.card(cooler.as_str()), "zap"),
            1
        );
        // Its user's next turn: still cooling down.
        s.end_turn();
        s.end_turn();
        assert!(!lists_activate(s.state(), &cooler));
        s.expect_refused_with(
            |s| s.act(zap(&cooler), P1, "zap"),
            "that ability is cooling down this turn",
        );
        // And the turn after: still sitting out.
        s.end_turn();
        s.end_turn();
        assert!(!lists_activate(s.state(), &cooler));
        // Then usable again.
        s.end_turn();
        s.end_turn();
        assert!(lists_activate(s.state(), &cooler));
    }
}

mod r3550004_a_per_turn_damage_pool {
    use super::*;

    fn pool_left(s: &Scenario, id: &str) -> Option<i32> {
        damage_pool_left(s.state(), s.card(id))
    }

    fn grant_armor(s: &mut Scenario, id: &str, n: i32) {
        apply_now(
            s.state_mut(),
            effects::grant_keyword(json_as(json!({
                "target": { "of": "instance", "instanceId": id },
                "keyword": { "kind": "Armor", "n": n },
            }))),
        );
    }

    #[test]
    fn r3550004_the_pool_nullifies_after_armor_and_refills_on_its_controllers_turn() {
        let mut s = board(
            "r3550004-pool",
            json!({ "field": [PLAIN], "backrow": face_up(POOL) }),
            json!({ "field": [RANGER, RANGER, RANGER] }),
        );
        let pool = backrow_id(&s, P1, 1);
        let plain = id_at(&s, P1, 1);
        grant_armor(&mut s, &plain, 2);
        assert_eq!(pool_left(&s, &pool), Some(5));
        s.end_turn();
        let rangers: Vec<String> = (1..=3).map(|lane| id_at(&s, P2, lane)).collect();
        // Armor first: 3 past 2 Armor leaves 1 for the pool, and the Unit takes nothing.
        s.attack(rangers[0].as_str(), plain.as_str());
        s.expect_stats(plain.as_str(), json!({ "health": 3 }));
        assert_eq!(pool_left(&s, &pool), Some(4));
        // The hero is covered too: 3 more leaves 1 in the pool, the hero nothing.
        s.attack(rangers[1].as_str(), "hero");
        s.expect_health(P1, HERO_HEALTH);
        assert_eq!(pool_left(&s, &pool), Some(1));
        // Exhausted, the rest lands: 1 soaked, 2 to the hero.
        s.attack(rangers[2].as_str(), "hero");
        s.expect_health(P1, HERO_HEALTH - 2);
        assert_eq!(pool_left(&s, &pool), Some(0));
        // The pool refills at the start of its controller's turn.
        s.end_turn();
        assert_eq!(pool_left(&s, &pool), Some(5));
    }

    #[test]
    fn r3550004_the_preview_shows_what_is_left_and_an_indestructible_unit_spends_none() {
        let mut s = board(
            "r3550004-indestructible",
            json!({ "field": [PLAIN], "backrow": face_up(POOL) }),
            json!({ "field": [RANGER] }),
        );
        let pool = backrow_id(&s, P1, 1);
        let plain = id_at(&s, P1, 1);
        apply_now(
            s.state_mut(),
            effects::grant_keyword(json_as(json!({
                "target": { "of": "instance", "instanceId": plain },
                "keyword": { "kind": "Indestructible" },
            }))),
        );
        s.end_turn();
        s.attack(id_at(&s, P2, 1).as_str(), plain.as_str());
        s.expect_stats(plain.as_str(), json!({ "health": 3 }));
        assert_eq!(pool_left(&s, &pool), Some(5));
    }
}

mod r3550005_counter_a_ranged_attack_or_an_activate {
    use super::*;

    fn trap_fired_then_cancelled(events: &[GameEvent]) -> bool {
        let mut kinds = events.iter().filter_map(|event| match event {
            GameEvent::TrapFired { .. } => Some(0),
            GameEvent::AttackCancelled { .. } => Some(1),
            _ => None,
        });
        matches!((kinds.next(), kinds.next()), (Some(0), Some(1)))
    }

    #[test]
    fn r3550005_the_first_enemy_ranged_attack_a_turn_is_countered() {
        // SEGMENT is p2's: it counters p1's Ranged attacks.
        let mut s = board(
            "r3550005-ranged",
            json!({ "field": [RANGER, RANGER] }),
            json!({ "backrow": [SEGMENT] }),
        );
        let first = id_at(&s, P1, 1);
        let second = id_at(&s, P1, 2);
        s.attack(first.as_str(), "hero");
        assert!(
            trap_fired_then_cancelled(s.last_events()),
            "trapFired, then attackCancelled"
        );
        s.expect_health(P2, HERO_HEALTH);
        // The budget spent, the second attack lands.
        s.attack(second.as_str(), "hero");
        s.expect_health(P2, HERO_HEALTH - 3);
    }

    #[test]
    fn r3550005_an_enemy_activate_is_countered_its_use_spent_and_the_radiant_face_counters_all() {
        let mut s = board(
            "r3550005-activate",
            json!({ "backrow": [SEGMENT_ALL] }),
            json!({ "backrow": face_up(ZAPPER) }),
        );
        let segment = backrow_id(&s, P1, 1);
        assert_ne!(s.card(segment.as_str()).face_up, Some(true));
        s.end_turn();
        let zapper = backrow_id(&s, P2, 1);
        s.act(
            ActionBody::Activate {
                instance_id: zapper.clone(),
                ability: Some("zap".to_string()),
                targets: None,
                modes: None,
                tributes: None,
            },
            P2,
            "zap",
        );
        // The Activate did nothing, but its use is spent.
        s.expect_health(P1, HERO_HEALTH);
        assert_eq!(
            ability_uses_this_turn(s.state(), s.card(zapper.as_str()), "zap"),
            1
        );
        assert!(types_of(s.last_events()).contains(&GameEventType::TrapFired));
        // A Field Trap stays to counter again: the Radiant face counters all of them.
        assert_eq!(backrow_id(&s, P1, 1), segment);
        s.end_turn();
        s.end_turn();
        s.act(
            ActionBody::Activate {
                instance_id: zapper,
                ability: Some("zap".to_string()),
                targets: None,
                modes: None,
                tributes: None,
            },
            P2,
            "zap",
        );
        s.expect_health(P1, HERO_HEALTH);
    }

    #[test]
    fn r3550005_a_forced_attack_is_not_countered() {
        let mut s = board(
            "r3550005-forced",
            json!({ "field": [AMBUSHER], "backrow": [SEGMENT] }),
            json!({ "hand": [PLAIN] }),
        );
        let ambusher = id_at(&s, P1, 1);
        s.end_turn();
        let plain = hand_id(&s, P2, PLAIN);
        s.play(plain.as_str(), json!({ "zone": 1 }));
        assert_eq!(forced_attackers(s.events()), vec![ambusher]);
        assert!(
            !types_of(s.events()).contains(&GameEventType::TrapFired),
            "no counter answers a forced attack (R121)"
        );
    }
}

mod r3550006_a_kill_deadline {
    use super::*;

    fn kill_deadlines(s: &Scenario) -> Vec<String> {
        s.state()
            .delayed
            .iter()
            .filter(|entry| entry.resume.hook == effects::delay::DELAYED_KILL_DEADLINE_HOOK)
            .filter_map(|entry| entry.watch.clone())
            .collect()
    }

    fn deadline_marks(s: &Scenario) -> Vec<String> {
        s.state()
            .marks
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter(|mark| mark.mark == effects::delay::DEADLINE_MARK)
            .map(|mark| mark.instance_id)
            .collect()
    }

    #[test]
    fn r3550006_a_unit_that_kills_in_time_keeps_its_life_and_loses_the_mark() {
        let mut s = board(
            "r3550006-kept",
            json!({ "field": [RANGER], "hand": [PIP] }),
            json!({ "field": [PLAIN] }),
        );
        let ranger = id_at(&s, P1, 1);
        let plain = id_at(&s, P2, 1);
        s.play(PIP, pick(&ranger));
        assert_eq!(kill_deadlines(&s), vec![ranger.clone()]);
        assert_eq!(deadline_marks(&s), vec![ranger.clone()]);
        // The kill credited to it meets the deadline at once.
        s.attack(ranger.as_str(), plain.as_str());
        s.expect_in_zone(plain.as_str(), "graveyard");
        assert!(kill_deadlines(&s).is_empty());
        assert!(deadline_marks(&s).is_empty());
        // Past its controller's next turn it still stands: Ranged, it took no strike back.
        s.expect_stats(ranger.as_str(), json!({ "health": 3 }));
        s.end_turn();
        s.end_turn();
        s.expect_stats(ranger.as_str(), json!({ "health": 3 }));
    }

    #[test]
    fn r3550006_a_unit_that_fails_is_destroyed_and_the_radiant_face_adds_the_coins() {
        let mut s = board(
            "r3550006-failed",
            json!({ "field": [RANGER], "hand": [{ "def": PIP, "radiant": true }] }),
            json!({ "field": [BIG] }),
        );
        let ranger = id_at(&s, P1, 1);
        s.play(PIP, pick(&ranger));
        // No kill by the end of its controller's next turn: destroyed, and 3 The Coins.
        s.end_turn();
        s.end_turn();
        s.end_turn();
        s.expect_in_zone(ranger.as_str(), "graveyard");
        let coins = s
            .hand(P1)
            .into_iter()
            .filter(|card| card.def_id == COIN_DEF_ID)
            .count();
        assert_eq!(coins, PIP_COINS as usize);
    }
}

mod r3550007_a_side_snapshot_and_its_restore {
    use super::*;

    fn destroy(id: &str) -> Effect {
        effects::destroy(json_as(
            json!({ "target": { "of": "instance", "instanceId": id } }),
        ))
    }

    #[test]
    fn r3550007_death_returns_the_side_to_the_snapshot_its_cry_took() {
        let mut s = board(
            "r3550007-restore",
            json!({ "field": [PLAIN], "hand": [BOGOS, PLAIN] }),
            json!({}),
        );
        let bogos = hand_id(&s, P1, BOGOS);
        s.play(bogos.as_str(), json!({ "zone": 2 }));
        let bogos = id_at(&s, P1, 2);
        // After the Cry, a second Plain joins: the side no longer matches the snapshot.
        let extra = hand_id(&s, P1, PLAIN);
        s.play(extra.as_str(), json!({ "zone": 3 }));
        apply_now(s.state_mut(), destroy(&bogos));
        // The snapshot's cards stand where they stood; the extra is back in hand.
        assert!(s.unit(P1, 2).is_none(), "Bogos is gone");
        assert!(s.unit(P1, 3).is_none(), "the extra left the field");
        assert!(
            s.hand(P1).iter().any(|card| card.def_id == PLAIN),
            "the extra went back to hand"
        );
    }

    #[test]
    fn r3550007_the_radiant_face_makes_the_rebuilt_side_radiant() {
        let mut s = board(
            "r3550007-radiant",
            json!({ "field": [PLAIN], "hand": [{ "def": BOGOS, "radiant": true }] }),
            json!({}),
        );
        let bogos = hand_id(&s, P1, BOGOS);
        s.play(bogos.as_str(), json!({ "zone": 2 }));
        let bogos = id_at(&s, P1, 2);
        apply_now(s.state_mut(), destroy(&bogos));
        let kept = id_at(&s, P1, 1);
        assert!(s.card(kept.as_str()).radiant, "the rebuilt side is Radiant");
    }
}

mod r3550003_cooldown_view {
    use super::*;

    #[test]
    fn r3550003_the_view_shows_the_turns_left_and_an_extra_turn_cooldown_is_unchanged() {
        let mut s = board(
            "r3550003-view",
            json!({ "backrow": [
                { "def": COOLER, "faceUp": true },
                { "def": SLOW, "faceUp": true },
            ] }),
            json!({}),
        );
        let cooler = backrow_id(&s, P1, 1);
        let slower = backrow_id(&s, P1, 2);
        assert_eq!(cooldown_left(&s, &cooler), None);
        s.act(zap(&cooler), P1, "zap");
        // A 3-turn cooldown sits out 2: the view shows the turns left.
        assert_eq!(cooldown_left(&s, &cooler), Some(2));
        // The shipped extra-turn cooldown still sits out exactly one turn.
        s.act(slow(&slower), P1, "slow");
        assert_eq!(cooldown_left(&s, &slower), Some(1));
        s.end_turn();
        s.end_turn();
        assert!(!lists_activate(s.state(), &slower));
        s.end_turn();
        s.end_turn();
        assert!(lists_activate(s.state(), &slower));
        assert_eq!(cooldown_left(&s, &slower), None);
    }
}

mod r3550008_a_secret_pick_from_the_opponents_hand {
    use super::*;
    use crate::rules::fixtures::harness::{RandomGame, hand_ids};

    /// Soul Read set with p2 holding PLAIN and RANGER: the pick prompt opens for p1 alone.
    fn reading(seed: &str) -> Scenario {
        let mut s = board(
            seed,
            json!({ "hand": [SOUL_READ] }),
            json!({ "hand": [PLAIN, RANGER] }),
        );
        let trap = hand_id(&s, P1, SOUL_READ);
        s.play(trap.as_str(), json!({ "zone": 1 }));
        s
    }

    fn option_ids(view: &PlayerView) -> Vec<String> {
        match view.pending.as_ref() {
            Some(PendingView::ForYou(prompt)) => prompt
                .options
                .iter()
                .filter_map(|option| option.instance_id.clone())
                .collect(),
            _ => Vec::new(),
        }
    }

    #[test]
    fn r3550008_only_its_owner_sees_the_options_and_legal_actions_answer_each() {
        let s = reading("r3550008-options");
        let seen = hand_ids(s.state(), P2);
        assert_eq!(seen.len(), 2);
        // Its owner sees the opponent's hand as the options...
        assert_eq!(option_ids(&s.view(P1)), seen);
        // ...and only then: the opponent's view names none of them.
        assert!(option_ids(&s.view(P2)).is_empty());
        // Every option is a legal answer for the owner, and the opponent may only concede.
        let answers = legal_actions(s.state(), P1)
            .into_iter()
            .filter(|action| matches!(action, ActionBody::Answer { .. }))
            .count();
        assert_eq!(answers, seen.len());
        assert_eq!(legal_actions(s.state(), P2), vec![ActionBody::Concede]);
        // The AI policy answers the pick too.
        let policy = subsystems::ai_policy::policy_actions(s.state(), P1, Default::default());
        assert_eq!(
            policy
                .iter()
                .filter(|action| matches!(action, ActionBody::Answer { .. }))
                .count(),
            seen.len()
        );
    }

    #[test]
    fn r3550008_no_view_tells_which_card_was_chosen() {
        // The same seed twice: the two states differ only in the pick.
        let mut first = reading("r3550008-hidden");
        let mut second = reading("r3550008-hidden");
        let plain = hand_ids(first.state(), P2)[0].clone();
        let ranger = hand_ids(second.state(), P2)[1].clone();
        first.answer(json!([plain]));
        second.answer(json!([ranger]));
        for viewer in [P1, P2] {
            assert_eq!(
                serde_json::to_value(first.view(viewer)).expect("the view serializes"),
                serde_json::to_value(second.view(viewer)).expect("the view serializes"),
                "no {viewer:?} view tells which card was chosen"
            );
        }
    }

    #[test]
    fn r3550008_the_chosen_card_played_first_is_stolen_into_the_owners_hand() {
        let mut s = reading("r3550008-steal");
        let plain = hand_ids(s.state(), P2)[0].clone();
        s.answer(json!([plain.clone()]));
        s.end_turn();
        s.play(plain.as_str(), json!({ "zone": 1 }));
        assert!(types_of(s.last_events()).contains(&GameEventType::Countered));
        let stolen = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == PLAIN)
            .expect("the chosen card is stolen into its owner's hand");
        assert_eq!(stolen.owner, P1);
    }

    #[test]
    fn r3550008_another_seen_card_played_first_tributes_the_base_trap() {
        let mut s = reading("r3550008-tribute");
        let trap = backrow_id(&s, P1, 1);
        let seen = hand_ids(s.state(), P2);
        s.answer(json!([seen[0].clone()]));
        s.end_turn();
        // Another seen card played first: the base trap is tributed, the play resolves.
        s.play(seen[1].as_str(), json!({ "zone": 1 }));
        s.expect_in_zone(trap.as_str(), "graveyard");
        assert_eq!(id_at(&s, P2, 1), seen[1]);
    }

    #[test]
    fn r3550008_a_timeout_answers_the_pick() {
        let s = reading("r3550008-timeout");
        assert!(s.state().pending.is_some());
        let timed_out = reduce(s.state(), &Action::new(ActionBody::Timeout, P1, "srv-t"));
        assert_eq!(timed_out.error, None);
        assert!(
            timed_out.state.pending.is_none(),
            "the clock answers the pick through the AI policy"
        );
    }

    /// `play_random_game` with this file's catalog: `new_game` resets the thread-local catalog to
    /// the base one, which holds no `e6-` def, so the game is built here with the fixtures kept.
    /// The loop is that helper's, step for step.
    fn e6_random_game(seed: &str, decks: (Vec<String>, Vec<String>)) -> RandomGame {
        register_e6_fixtures();
        let catalog = registered_catalog().clone();
        let mut state = begin_game(&create_game(&CreateGameOptions {
            seed: seed.to_string(),
            decks: decks.clone(),
            catalog: Some(catalog),
            ..CreateGameOptions::default()
        }))
        .state;
        let mut policy = Rng::new(&format!("policy-{seed}"), 0);
        let mut log: Vec<Action> = Vec::new();

        let mut step = 0;
        while state.result.is_none() {
            if step > 4000 {
                panic!("game {seed} did not finish");
            }
            let player = seat_to_act(&state).unwrap_or(state.active);
            let actions: Vec<ActionBody> = legal_actions(&state, player)
                .into_iter()
                .filter(|action| {
                    !matches!(
                        action.action_type(),
                        ActionType::Concede | ActionType::OfferDraw | ActionType::AnswerDraw
                    )
                })
                .collect();
            if actions.is_empty() {
                panic!("no legal action for {player} in game {seed}");
            }

            let others: Vec<&ActionBody> = actions
                .iter()
                .filter(|action| action.action_type() != ActionType::EndTurn)
                .collect();
            let end_turn = actions
                .iter()
                .find(|action| action.action_type() == ActionType::EndTurn);
            let take_end =
                others.is_empty() || (end_turn.is_some() && policy.chance(AI_END_TURN_PROBABILITY));
            let chosen: ActionBody = if take_end {
                match end_turn {
                    Some(end) => end.clone(),
                    None => others[policy.int(others.len() as i32) as usize].clone(),
                }
            } else {
                others[policy.int(others.len() as i32) as usize].clone()
            };

            let action = Action::new(chosen, player, format!("a{}", log.len()));
            let result = reduce(&state, &action);
            if let Some(error) = &result.error {
                panic!("{} rejected in {seed}: {error}", action.action_type());
            }
            log.push(action);
            state = result.state;
            step += 1;
        }

        RandomGame { state, log, decks }
    }

    #[test]
    fn r3550008_random_games_with_the_fixtures_finish() {
        // No duplicate def ids (§2.6): the 17 fixtures plus vanilla filler.
        let deck = || {
            [
                PLAIN,
                RANGER,
                BIG,
                SPRAYER,
                THAWER,
                FROSTER,
                COOLER,
                ZAPPER,
                POOL,
                SEGMENT,
                SEGMENT_ALL,
                PIP,
                BOGOS,
                SOUL_READ,
                BUNNY,
                SLOW,
                "fx-1",
                "fx-2",
                "fx-3",
                "fx-4",
                "fx-5",
                "fx-6",
                "fx-7",
                "fx-8",
                "fx-9",
            ]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<String>>()
        };
        for n in 1..=3 {
            let game = e6_random_game(&format!("e6-random-{n}"), (deck(), deck()));
            assert!(
                game.state.result.is_some(),
                "random game {n} with the fixtures finishes"
            );
        }
    }
}

mod r3550009_take_control {
    use super::*;

    #[test]
    fn r3550009_a_death_takes_a_random_enemy_permanent() {
        let mut s = board(
            "r3550009-steal",
            json!({ "field": [BUNNY] }),
            json!({ "field": [PLAIN] }),
        );
        let bunny = id_at(&s, P1, 1);
        let plain = id_at(&s, P2, 1);
        apply_now(
            s.state_mut(),
            effects::destroy(json_as(json!({
                "target": { "of": "instance", "instanceId": bunny },
            }))),
        );
        // The one enemy permanent comes under its controller's control (R15, R33).
        let stolen = s.card(plain.as_str());
        assert_eq!(stolen.controller, P1);
        assert_eq!(id_at(&s, P1, 1), plain);
    }
}
