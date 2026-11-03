//! R2783001: the keyword behind a hit, a heal, a death or a summon (`via`, §10.3), set by the engine
//! where the keyword applies (§4.3, §4.4 steps 2, 5 and 7 to 10, §4.5, §6.1), and First Strike's
//! unanswered strike: the death of a unit that fell to it says First Strike.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{
    armored, big_body, cleaver, first_striker, lifestealer, plain, poisonous, trampler,
};
use crate::rules::fixtures::harness::{live, new_game, put, run_on_sink, slot};

/// p1's main phase on turn 4, so nothing placed with `put` is summoning sick (§4.1).
fn board(seed: &str) -> GameState {
    let mut state = new_game(&format!("keyword-via-{seed}"), None);
    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;
    state
}

/// `declare_attack` with the attacker read live, at a unit or (with `None`) p2's hero.
fn attack(state: &mut GameState, attacker_id: &str, target: Option<&str>) -> Vec<GameEvent> {
    let (refused, events) = run_on_sink(state, |sink| {
        let attacker = live(sink.state, attacker_id);
        let target = match target {
            Some(id) => AttackTarget::Unit {
                instance: live(sink.state, id),
            },
            None => AttackTarget::Hero { player: P2 },
        };
        declare_attack(sink, &attacker, &target).err()
    });
    assert_eq!(refused.map(|error| error.message), None);
    events
}

/// Each `damage` as `(sourceId, targetId, via)`.
fn hits(events: &[GameEvent]) -> Vec<(Option<String>, String, Option<KeywordKind>)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Damage {
                source_id,
                target_id,
                via,
                ..
            } => Some((source_id.clone(), target_id.clone(), *via)),
            _ => None,
        })
        .collect()
}

/// Each `destroyed` as `(instanceId, via)`.
fn deaths(events: &[GameEvent]) -> Vec<(String, Option<KeywordKind>)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Destroyed { instance_id, via, .. } => Some((instance_id.clone(), *via)),
            _ => None,
        })
        .collect()
}

/// Each `healed` as `(targetId, via)`.
fn heals(events: &[GameEvent]) -> Vec<(String, Option<KeywordKind>)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Healed { target_id, via, .. } => Some((target_id.clone(), *via)),
            _ => None,
        })
        .collect()
}

fn grant(state: &mut GameState, id: &str, keyword: Keyword) {
    find_instance_mut(state, id)
        .expect("on the field")
        .granted_keywords
        .push(keyword);
}

#[test]
fn r2783001_cleaves_neighbour_hits_carry_cleave_and_the_hit_on_the_defender_none() {
    let mut state = board("cleave");
    let attacker = put(&mut state, &cleaver.id, slot(P1, Row::Units, 2), json!({})); // 3/8 Cleave
    let left = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
    let defender = put(&mut state, &plain.id, slot(P2, Row::Units, 2), json!({}));
    let right = put(&mut state, &armored.id, slot(P2, Row::Units, 3), json!({})); // Armor 7

    let events = attack(&mut state, &attacker.id, Some(&defender.id));
    let from = Some(attacker.id.clone());
    assert_eq!(
        hits(&events),
        vec![
            (from.clone(), defender.id.clone(), None),
            (from.clone(), left.id.clone(), Some(KeywordKind::Cleave)),
            (Some(defender.id.clone()), attacker.id.clone(), None),
        ]
    );
    // The neighbour whose Armor took Cleave's hit whole is reported with the same route.
    assert!(events.iter().any(|event| matches!(
        event,
        GameEvent::DamageAbsorbed { target_id, via: Some(KeywordKind::Cleave), .. } if *target_id == right.id
    )));
}

#[test]
fn r2783001_a_trample_excess_on_the_hero_carries_trample() {
    let mut state = board("trample");
    let attacker = put(&mut state, &trampler.id, slot(P1, Row::Units, 1), json!({})); // 6/4 Trample
    let defender = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({})); // 3/3

    let events = attack(&mut state, &attacker.id, Some(&defender.id));
    let from = Some(attacker.id.clone());
    assert_eq!(
        hits(&events),
        vec![
            (from.clone(), defender.id.clone(), None),
            (from, "hero-p2".to_string(), Some(KeywordKind::Trample)),
            (Some(defender.id.clone()), attacker.id.clone(), None),
        ]
    );
}

#[test]
fn r2783001_a_pierce_hit_carries_pierce_only_where_armor_stood() {
    let mut state = board("pierce");
    let source = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
    grant(&mut state, &source.id, Keyword::Pierce);
    let walled = put(&mut state, &armored.id, slot(P2, Row::Units, 1), json!({})); // Armor 7
    let bare = put(&mut state, &plain.id, slot(P2, Row::Units, 2), json!({}));

    let hit = |state: &mut GameState, target: DamageTarget| {
        run_on_sink(state, |sink| {
            let source = live(sink.state, &source.id);
            deal_damage(
                sink,
                DamageArgs {
                    source: Some(source),
                    target,
                    amount: 2,
                    flags: None,
                },
            );
        })
        .1
    };
    let on = |state: &GameState, id: &str| DamageTarget::Unit {
        instance: live(state, id),
    };

    let target = on(&state, &walled.id);
    assert_eq!(hits(&hit(&mut state, target))[0].2, Some(KeywordKind::Pierce));
    let target = on(&state, &bare.id);
    assert_eq!(hits(&hit(&mut state, target))[0].2, None);
    // A hero: Pierce where it has Armor, none where it has none.
    let events = hit(&mut state, DamageTarget::Hero { player: P2 });
    assert_eq!(hits(&events)[0].2, None);
    state.players[P2].hero.armor = 3;
    let events = hit(&mut state, DamageTarget::Hero { player: P2 });
    assert_eq!(hits(&events)[0].2, Some(KeywordKind::Pierce));
}

#[test]
fn r2783001_lifesteals_heal_carries_lifesteal_and_a_plain_heal_none() {
    let mut state = board("lifesteal");
    state.players[P1].hero.health -= 5;
    let attacker = put(&mut state, &lifestealer.id, slot(P1, Row::Units, 1), json!({})); // 3/3 Lifesteal

    let events = attack(&mut state, &attacker.id, None);
    assert_eq!(
        heals(&events),
        vec![("hero-p1".to_string(), Some(KeywordKind::Lifesteal))]
    );

    let (_, events) = run_on_sink(&mut state, |sink| heal_hero(sink, P1, 1));
    assert_eq!(heals(&events), vec![("hero-p1".to_string(), None)]);
}

#[test]
fn r2783001_a_poisonous_kill_carries_poisonous_and_a_plain_death_none() {
    let mut state = board("poisonous");
    let attacker = put(&mut state, &poisonous.id, slot(P1, Row::Units, 1), json!({})); // 1/1 Poisonous
    let defender = put(&mut state, &big_body.id, slot(P2, Row::Units, 1), json!({})); // 5/10

    let events = attack(&mut state, &attacker.id, Some(&defender.id));
    assert_eq!(
        deaths(&events),
        vec![
            (attacker.id.clone(), None),
            (defender.id.clone(), Some(KeywordKind::Poisonous)),
        ]
    );
    // The hits themselves have no route.
    assert!(hits(&events).iter().all(|hit| hit.2.is_none()));
}

#[test]
fn r2783001_a_reborn_return_carries_reborn() {
    let mut state = board("reborn");
    let card = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
    grant(&mut state, &card.id, Keyword::Reborn);
    find_instance_mut(&mut state, &card.id)
        .expect("on the field")
        .damage = 3;

    let (_, events) = run_on_sink(&mut state, state_check);
    assert_eq!(deaths(&events), vec![(card.id.clone(), None)]);
    let summoned: Vec<(String, Option<KeywordKind>)> = events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Summoned { instance_id, via, .. } => Some((instance_id.clone(), *via)),
            _ => None,
        })
        .collect();
    assert_eq!(summoned, vec![(card.id.clone(), Some(KeywordKind::Reborn))]);
}

#[test]
fn r2783001_a_first_strike_kill_marks_the_hit_and_the_death_and_nothing_strikes_back() {
    let mut state = board("first-strike-kill");
    let attacker = put(&mut state, &first_striker.id, slot(P1, Row::Units, 1), json!({})); // 4/4
    let defender = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({})); // 3/3

    let events = attack(&mut state, &attacker.id, Some(&defender.id));
    assert_eq!(
        hits(&events),
        vec![(
            Some(attacker.id.clone()),
            defender.id.clone(),
            Some(KeywordKind::FirstStrike)
        )]
    );
    assert_eq!(
        deaths(&events),
        vec![(defender.id.clone(), Some(KeywordKind::FirstStrike))]
    );

    // The mirror: a First Strike defender kills the attacker before it strikes.
    let mut state = board("first-strike-defender");
    let attacker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
    let defender = put(&mut state, &first_striker.id, slot(P2, Row::Units, 1), json!({}));
    let events = attack(&mut state, &attacker.id, Some(&defender.id));
    assert_eq!(
        hits(&events),
        vec![(
            Some(defender.id.clone()),
            attacker.id.clone(),
            Some(KeywordKind::FirstStrike)
        )]
    );
    assert_eq!(
        deaths(&events),
        vec![(attacker.id.clone(), Some(KeywordKind::FirstStrike))]
    );
}

#[test]
fn r2783001_a_first_strike_survivor_strikes_back_untagged() {
    let mut state = board("first-strike-survivor");
    let attacker = put(&mut state, &first_striker.id, slot(P1, Row::Units, 1), json!({})); // 4/4
    let defender = put(&mut state, &big_body.id, slot(P2, Row::Units, 1), json!({})); // 5/10

    let events = attack(&mut state, &attacker.id, Some(&defender.id));
    assert_eq!(
        hits(&events),
        vec![
            (
                Some(attacker.id.clone()),
                defender.id.clone(),
                Some(KeywordKind::FirstStrike)
            ),
            (Some(defender.id.clone()), attacker.id.clone(), None),
        ]
    );
    // The First Striker dies to the strike back, which First Strike did not cause.
    assert_eq!(deaths(&events), vec![(attacker.id.clone(), None)]);
}

#[test]
fn r2783001_two_first_strikers_or_a_hero_target_carry_no_first_strike() {
    let mut state = board("first-strike-both");
    let attacker = put(&mut state, &first_striker.id, slot(P1, Row::Units, 1), json!({}));
    let defender = put(&mut state, &first_striker.id, slot(P2, Row::Units, 1), json!({}));
    let events = attack(&mut state, &attacker.id, Some(&defender.id));
    assert_eq!(hits(&events).len(), 2);
    assert!(hits(&events).iter().all(|hit| hit.2.is_none()));
    assert!(deaths(&events).iter().all(|death| death.1.is_none()));

    let mut state = board("first-strike-hero");
    let attacker = put(&mut state, &first_striker.id, slot(P1, Row::Units, 1), json!({}));
    let events = attack(&mut state, &attacker.id, None);
    assert_eq!(
        hits(&events),
        vec![(Some(attacker.id.clone()), "hero-p2".to_string(), None)]
    );
}

#[test]
fn r2783001_the_rules_never_read_via_and_it_leaves_no_state_behind() {
    let mut state = board("rules-read");
    let attacker = put(&mut state, &trampler.id, slot(P1, Row::Units, 1), json!({}));
    let defender = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
    let events = attack(&mut state, &attacker.id, Some(&defender.id));
    let routed = events
        .iter()
        .find(|event| matches!(event, GameEvent::Damage { via: Some(_), .. }))
        .expect("the excess");
    assert!(matches!(
        routed.as_rules_read(),
        GameEvent::Damage { via: None, .. }
    ));
    // The state the loop holds names no route.
    assert!(!json_of_state(&state).contains("\"via\""));
}

fn json_of_state(state: &GameState) -> String {
    let mut value = serde_json::to_value(state).expect("a state serializes");
    if let Some(object) = value.as_object_mut() {
        object.remove("applied");
    }
    value.to_string()
}
