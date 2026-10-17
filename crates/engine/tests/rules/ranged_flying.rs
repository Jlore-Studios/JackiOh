//! Ranged and Flying (#783, R783000–R783003): a Ranged attacker takes no strike back unless the
//! defender is Ranged too (§4.3), and a Flying unit outside Defense Position may be attacked only by a
//! Flying or Ranged unit (§4.2 step 2), a restriction forced attacks and the Taunt wall obey as well.
//! The fixtures in `fixtures/keywords_783.rs` stand in for the cards #781 prints.

use jackioh_engine::subsystems::lethal::projected_damage;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use jackioh_engine::effects::{steal, swap_board, transform};
use jackioh_engine::subsystems::rotation::{RotationArgs, rotate_rings};

use super::fixtures::combat::cleaver;
use super::fixtures::field::fuser;
use super::fixtures::harness::{events_of_type, new_game, put, sink_for, slot};
use super::fixtures::keywords_783::{
    cloaked_bolt, cloaked_field, crater_trap, first_striker, flyer, flying_reborner, flying_stacker,
    flying_taunter, grunt, quake, ranged, ranged_cleaver, ranged_striker, ranged_trampler,
    register_keywords_783, sniper,
};
use super::fixtures::prompt_harness::{act, cast_now};

/// p1's main phase on turn 4 with #783's cards registered, so nothing `put` places is sick (§4.1).
fn board(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    register_keywords_783();
    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;
    state
}

fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id)
        .cloned()
        .unwrap_or_else(|| panic!("no card {id}"))
}

fn on_unit(state: &GameState, id: &str) -> AttackTarget {
    AttackTarget::Unit {
        instance: live(state, id),
    }
}

/// Run `f` on a sink over `state` and hand back what it emitted.
fn run<R>(state: &mut GameState, f: impl FnOnce(&mut EngineSink<'_>) -> R) -> (R, Vec<GameEvent>) {
    let mut sink = sink_for(state);
    let out = f(&mut sink);
    let events = sink.events.clone();
    (out, events)
}

/// Declare `attacker`'s attack on the unit `target`, answering the refusal's text or `None`.
fn declare(state: &mut GameState, attacker: &str, target: &str) -> (Option<String>, Vec<GameEvent>) {
    let attacker = live(state, attacker);
    let target = on_unit(state, target);
    run(state, |sink| {
        declare_attack(sink, &attacker, &target)
            .err()
            .map(|error| error.message)
    })
}

/// `force_attack` on the unit `target`, with both read live.
fn force(state: &mut GameState, attacker: &str, target: &str) -> Vec<GameEvent> {
    let attacker = live(state, attacker);
    let target = on_unit(state, target);
    run(state, |sink| force_attack(sink, &attacker, &target)).1
}

/// The damage `id` has taken.
fn damage_on(state: &GameState, id: &str) -> i32 {
    find_instance(state, id).map_or(0, |card| card.damage)
}

/// The attacker `id` took nothing back: it still stands in a unit zone with no damage, and no damage
/// event names it — so a strike back that killed it (and reset its damage as it left) cannot pass.
fn took_nothing_back(state: &GameState, events: &[GameEvent], id: &str) {
    let card = live(state, id);
    assert!(
        matches!(card.zone, Zone::Field { row: Row::Units, .. }),
        "the attacker still stands"
    );
    assert_eq!(card.damage, 0);
    assert!(
        events_of_type(events, GameEventType::Damage)
            .iter()
            .all(|event| !matches!(event, GameEvent::Damage { target_id, .. } if target_id == id)),
        "no damage event names the attacker"
    );
}

fn in_graveyard(state: &GameState, player: PlayerId, id: &str) -> bool {
    state.players[player].graveyard.iter().any(|card| card.id == id)
}

/// Every `attack` action `legal_actions` offers `attacker`, as target ids.
fn attack_targets(state: &GameState, attacker: &str) -> Vec<String> {
    legal_actions(state, P1)
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

/// The refusal `reduce` gives `attacker`'s attack on `target`, or `""`.
fn attack_refusal(state: &GameState, attacker: &str, target: &str) -> String {
    let action: Action = json_as(json!({
        "type": "attack",
        "attackerId": attacker,
        "targetId": target,
        "playerId": "p1",
        "nonce": format!("kw-{attacker}-{target}"),
    }));
    reduce(state, &action).error.unwrap_or_default()
}

fn set_position(state: &mut GameState, id: &str, position: Position) {
    if let Some(card) = find_instance_mut(state, id) {
        card.position = Some(position);
    }
}

mod r783000_ranged {
    use super::*;

    #[test]
    fn r783000_a_ranged_attacker_takes_nothing_back_declared_or_forced() {
        // Declared: the Ranged 2/3 hits the 3/3 for 2 and takes none of its 3 back.
        let mut state = board("r783000-declared");
        let archer = put(&mut state, &ranged().id, slot(P1, Row::Units, 1), json!({}));
        let target = put(&mut state, &grunt().id, slot(P2, Row::Units, 1), json!({}));
        let (refused, events) = declare(&mut state, &archer.id, &target.id);
        assert_eq!(refused, None);
        assert_eq!(damage_on(&state, &target.id), 2);
        took_nothing_back(&state, &events, &archer.id);

        // Forced: a forced attack is still an attack, and the same holds.
        let mut forced = board("r783000-forced");
        let archer = put(&mut forced, &ranged().id, slot(P1, Row::Units, 1), json!({}));
        let target = put(&mut forced, &grunt().id, slot(P2, Row::Units, 1), json!({}));
        let events = force(&mut forced, &archer.id, &target.id);
        assert_eq!(events_of_type(&events, GameEventType::AttackDeclared).len(), 1);
        assert_eq!(damage_on(&forced, &target.id), 2);
        took_nothing_back(&forced, &events, &archer.id);

        // A unit that is not Ranged still takes its strike back.
        let mut plain = board("r783000-plain");
        let attacker = put(&mut plain, &grunt().id, slot(P1, Row::Units, 1), json!({}));
        let target = put(&mut plain, &ranged().id, slot(P2, Row::Units, 1), json!({}));
        declare(&mut plain, &attacker.id, &target.id);
        assert_eq!(damage_on(&plain, &attacker.id), 2);
        assert!(in_graveyard(&plain, P2, &target.id));
    }

    #[test]
    fn r783000_against_a_ranged_defender_first_strike_and_poisonous_strike_back() {
        // Ranged on Ranged: the exchange is the plain one, both strike.
        let mut state = board("r783000-both");
        let archer = put(&mut state, &ranged().id, slot(P1, Row::Units, 1), json!({}));
        let other = put(&mut state, &ranged().id, slot(P2, Row::Units, 1), json!({}));
        declare(&mut state, &archer.id, &other.id);
        assert_eq!(damage_on(&state, &archer.id), 2);
        assert_eq!(damage_on(&state, &other.id), 2);

        // A Ranged defender with First Strike and Poisonous strikes first and destroys the attacker,
        // which deals nothing (§4.3 step 1).
        let mut striking = board("r783000-first-strike");
        let archer = put(&mut striking, &ranged().id, slot(P1, Row::Units, 1), json!({}));
        let striker = put(
            &mut striking,
            &ranged_striker().id,
            slot(P2, Row::Units, 1),
            json!({}),
        );
        declare(&mut striking, &archer.id, &striker.id);
        assert!(
            in_graveyard(&striking, P1, &archer.id),
            "Poisonous destroyed the attacker"
        );
        assert_eq!(damage_on(&striking, &striker.id), 0);
    }

    #[test]
    fn r783000_the_lethal_projection_counts_no_strike_back() {
        // A Ranged Trample 4/2 into a First Strike 3/1: without Ranged the defender would kill it first
        // and nothing would reach the hero (R176); with it, 3 tramples through.
        let mut state = board("r783000-lethal");
        let trampler = put(
            &mut state,
            &ranged_trampler().id,
            slot(P1, Row::Units, 1),
            json!({}),
        );
        let striker = put(
            &mut state,
            &first_striker().id,
            slot(P2, Row::Units, 1),
            json!({}),
        );
        let projected = projected_damage(&state, &live(&state, &trampler.id), &on_unit(&state, &striker.id));
        assert_eq!(projected, 3);

        let before = state.players.p2.hero.health;
        declare(&mut state, &trampler.id, &striker.id);
        assert_eq!(state.players.p2.hero.health, before - projected);
        assert_eq!(damage_on(&state, &trampler.id), 0);
    }
}

mod r783001_ranged_and_cleave {
    use super::*;

    #[test]
    fn r783001_a_ranged_cleavers_splash_lands_and_nothing_strikes_back() {
        let mut state = board("r783001-cleave");
        let cleaving = put(
            &mut state,
            &ranged_cleaver().id,
            slot(P1, Row::Units, 2),
            json!({}),
        );
        let left = put(&mut state, &grunt().id, slot(P2, Row::Units, 1), json!({}));
        let target = put(&mut state, &grunt().id, slot(P2, Row::Units, 2), json!({}));
        let right = put(&mut state, &grunt().id, slot(P2, Row::Units, 3), json!({}));
        let (refused, events) = declare(&mut state, &cleaving.id, &target.id);
        assert_eq!(refused, None);
        // The target and both neighbours take the hit; the splashed units never strike back, and the
        // target does not either.
        assert_eq!(damage_on(&state, &target.id), 2);
        assert_eq!(damage_on(&state, &left.id), 2);
        assert_eq!(damage_on(&state, &right.id), 2);
        took_nothing_back(&state, &events, &cleaving.id);
    }
}

mod r783002_flying {
    use super::*;

    #[test]
    fn r783002_only_flying_or_ranged_attack_a_flying_unit_legal_actions_and_refusal_agree() {
        let mut state = board("r783002-who");
        let walker = put(&mut state, &grunt().id, slot(P1, Row::Units, 1), json!({}));
        let archer = put(&mut state, &ranged().id, slot(P1, Row::Units, 2), json!({}));
        let wing = put(&mut state, &flyer().id, slot(P1, Row::Units, 3), json!({}));
        let bird = put(&mut state, &flyer().id, slot(P2, Row::Units, 1), json!({}));

        // `legal_actions` offers the Flying unit to the Ranged and the Flying attackers only.
        assert!(!attack_targets(&state, &walker.id).contains(&bird.id));
        assert!(attack_targets(&state, &archer.id).contains(&bird.id));
        assert!(attack_targets(&state, &wing.id).contains(&bird.id));
        // The other attacks are still offered.
        assert!(!attack_targets(&state, &walker.id).is_empty());

        // The reducer refuses exactly the one it does not offer.
        assert!(attack_refusal(&state, &walker.id, &bird.id).contains("Flying or Ranged"));
        assert_eq!(attack_refusal(&state, &archer.id, &bird.id), "");
        assert_eq!(attack_refusal(&state, &wing.id, &bird.id), "");
    }

    #[test]
    fn r783002_a_forced_attack_it_forbids_does_not_happen() {
        let mut state = board("r783002-forced");
        let walker = put(&mut state, &grunt().id, slot(P1, Row::Units, 1), json!({}));
        let archer = put(&mut state, &ranged().id, slot(P1, Row::Units, 2), json!({}));
        let bird = put(&mut state, &flyer().id, slot(P2, Row::Units, 1), json!({}));

        let events = force(&mut state, &walker.id, &bird.id);
        assert!(events_of_type(&events, GameEventType::AttackDeclared).is_empty());
        assert_eq!(damage_on(&state, &bird.id), 0);
        assert_eq!(damage_on(&state, &walker.id), 0);

        let events = force(&mut state, &archer.id, &bird.id);
        assert_eq!(events_of_type(&events, GameEventType::AttackDeclared).len(), 1);
        assert_eq!(damage_on(&state, &bird.id), 2);
    }

    #[test]
    fn r783002_spells_cries_all_effects_and_cleave_still_reach_it() {
        let mut state = board("r783002-reach");
        state.players.p1.mana = ManaState {
            current: 4,
            max: 4,
            next_turn_mod: 0,
            perm_mod: 0,
        };
        let bird = put(&mut state, &flyer().id, slot(P2, Row::Units, 2), json!({}));

        // An "all" effect.
        cast_now(&mut state, &quake().id, P1, false);
        assert_eq!(damage_on(&state, &bird.id), 1);

        // A Spell's target.
        let bolt = new_instance(&mut state, &cloaked_bolt().id, P1, Zone::Hand { player: P1 });
        state.players.p1.hand.push(bolt.clone());
        state = act(
            &state,
            json!({
                "type": "play",
                "instanceId": bolt.id,
                "targets": [{ "pick": "instance", "instanceId": bird.id }],
                "playerId": "p1",
            }),
            None,
        );
        assert!(in_graveyard(&state, P2, &bird.id), "the Spell's 2 finished it");

        // A Cry's target.
        let mut cried = board("r783002-cry");
        cried.players.p1.mana.current = 4;
        let bird = put(&mut cried, &flyer().id, slot(P2, Row::Units, 2), json!({}));
        let shooter = new_instance(&mut cried, &sniper().id, P1, Zone::Hand { player: P1 });
        cried.players.p1.hand.push(shooter.clone());
        cried = act(
            &cried,
            json!({
                "type": "play",
                "instanceId": shooter.id,
                "zone": { "row": "units", "lane": 1 },
                "targets": [{ "pick": "instance", "instanceId": bird.id }],
                "playerId": "p1",
            }),
            None,
        );
        assert_eq!(damage_on(&cried, &bird.id), 2);

        // Cleave's splash, which is no attack on it (R1026).
        let mut split = board("r783002-cleave");
        let axe = put(&mut split, &cleaver.id, slot(P1, Row::Units, 1), json!({}));
        let target = put(&mut split, &grunt().id, slot(P2, Row::Units, 1), json!({}));
        let bird = put(&mut split, &flyer().id, slot(P2, Row::Units, 2), json!({}));
        let (refused, _) = declare(&mut split, &axe.id, &target.id);
        assert_eq!(refused, None);
        assert!(damage_on(&split, &bird.id) > 0 || in_graveyard(&split, P2, &bird.id));
    }

    #[test]
    fn r783002_its_attack_position_taunt_binds_only_flying_and_ranged() {
        let mut state = board("r783002-taunt");
        let walker = put(&mut state, &grunt().id, slot(P1, Row::Units, 1), json!({}));
        let archer = put(&mut state, &ranged().id, slot(P1, Row::Units, 2), json!({}));
        let wall = put(
            &mut state,
            &flying_taunter().id,
            slot(P2, Row::Units, 1),
            json!({}),
        );
        let other = put(&mut state, &grunt().id, slot(P2, Row::Units, 2), json!({}));

        // The walker may not attack the Flying Taunt, so the Taunt does not bind it.
        let walking = attack_targets(&state, &walker.id);
        assert!(!walking.contains(&wall.id));
        assert!(walking.contains(&other.id));
        assert_eq!(attack_refusal(&state, &walker.id, &other.id), "");

        // The Ranged attacker may, so the Taunt binds it.
        assert_eq!(attack_targets(&state, &archer.id), vec![wall.id.clone()]);
        assert!(attack_refusal(&state, &archer.id, &other.id).contains("Taunt"));
    }
}

mod r783003_flying_lands {
    use super::*;

    #[test]
    fn r783003_in_defense_anything_attacks_it_and_its_taunt_binds_all() {
        let mut state = board("r783003-lands");
        let walker = put(&mut state, &grunt().id, slot(P1, Row::Units, 1), json!({}));
        let wall = put(
            &mut state,
            &flying_taunter().id,
            slot(P2, Row::Units, 1),
            json!({}),
        );
        let other = put(&mut state, &grunt().id, slot(P2, Row::Units, 2), json!({}));
        set_position(&mut state, &wall.id, Position::Def);

        // Landed, it may be attacked by anything, and its Taunt binds the walker too.
        assert_eq!(attack_targets(&state, &walker.id), vec![wall.id.clone()]);
        assert_eq!(attack_refusal(&state, &walker.id, &wall.id), "");
        assert!(attack_refusal(&state, &walker.id, &other.id).contains("Taunt"));

        // A plain Flying unit in Defense Position has that position's Taunt, and binds the same way.
        let mut plain = board("r783003-plain");
        let walker = put(&mut plain, &grunt().id, slot(P1, Row::Units, 1), json!({}));
        let bird = put(&mut plain, &flyer().id, slot(P2, Row::Units, 1), json!({}));
        set_position(&mut plain, &bird.id, Position::Def);
        assert_eq!(attack_targets(&plain, &walker.id), vec![bird.id.clone()]);
        let (refused, _) = declare(&mut plain, &walker.id, &bird.id);
        assert_eq!(refused, None);
        assert_eq!(damage_on(&plain, &walker.id), 2);
    }
}

mod r1081000_flying_backrow {
    use super::*;
    use jackioh_engine::subsystems::audit::{AuditArgs, audit_targets};
    use jackioh_engine::subsystems::board_history::{record_board_snapshot, restore_board};
    use jackioh_engine::subsystems::quests::{QuestDef, quest_progress_of};

    /// A `def_id` card in `player`'s hand, with mana to play it.
    fn in_hand_with_mana(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
        let card = new_instance(state, def_id, player, Zone::Hand { player });
        state.players[player].hand.push(card.clone());
        state.players[player].mana.current = 10;
        card
    }

    /// The refusal a play of `id` into `zone` meets, or `""`.
    fn play_refusal(state: &GameState, id: &str, zone: Value) -> String {
        let action: Action = json_as(json!({
            "type": "play",
            "instanceId": id,
            "zone": zone,
            "playerId": "p1",
            "nonce": format!("r1081000-{id}"),
        }));
        reduce(state, &action).error.unwrap_or_default()
    }

    fn unit_zone(lane: i32) -> ZoneChoice {
        ZoneChoice {
            row: Row::Units,
            lane,
        }
    }

    fn backrow_zone(lane: i32) -> ZoneChoice {
        ZoneChoice {
            row: Row::Backrow,
            lane,
        }
    }

    #[test]
    fn r1081000_flying_unit_lists_open_backrow_zones() {
        let mut state = board("r1081000-zones");
        let bird = in_hand_with_mana(&mut state, &flyer().id, P1);
        let zones = legal_zones_for(&state, P1, &bird, &[]);
        // An empty board: every unit lane and every backrow lane.
        for lane in 1..=5 {
            assert!(zones.contains(&unit_zone(lane)), "unit lane {lane} offered");
            assert!(zones.contains(&backrow_zone(lane)), "backrow lane {lane} offered");
        }
        // A Unit that cannot fly is offered its unit lanes only.
        let plain = in_hand_with_mana(&mut state, &grunt().id, P1);
        let plain_zones = legal_zones_for(&state, P1, &plain, &[]);
        assert!(!plain_zones.is_empty());
        assert!(plain_zones.iter().all(|zone| zone.row == Row::Units));
    }

    #[test]
    fn r1081000_flying_unit_plays_into_empty_backrow_and_acts() {
        let mut state = board("r1081000-stand");
        // A Unit that cannot fly is still refused the backrow.
        let plain = in_hand_with_mana(&mut state, &grunt().id, P1);
        assert!(
            play_refusal(&state, &plain.id, json!({ "row": "backrow", "lane": 1 }))
                .contains("played on top of")
        );
        let wing = put(&mut state, &flyer().id, slot(P1, Row::Backrow, 1), json!({}));
        let target = put(&mut state, &grunt().id, slot(P2, Row::Units, 1), json!({}));
        // It attacks from the backrow.
        let (refused, _) = declare(&mut state, &wing.id, &target.id);
        assert_eq!(refused, None);
        assert_eq!(damage_on(&state, &target.id), 2);

        // Played from hand with `zone: { row: backrow }`, it stands there.
        let mut played = board("r1081000-played");
        let bird = in_hand_with_mana(&mut played, &flyer().id, P1);
        let next = act(
            &played,
            json!({
                "type": "play",
                "instanceId": bird.id,
                "zone": { "row": "backrow", "lane": 2 },
                "playerId": "p1",
            }),
            None,
        );
        let stood = live(&next, &bird.id);
        assert!(
            matches!(
                stood.zone,
                Zone::Field {
                    row: Row::Backrow,
                    lane: 2,
                    ..
                }
            ),
            "the flyer stands in backrow lane 2"
        );
        // It counts among its controller's Units.
        assert!(active_units_of(&next, P1).iter().any(|unit| unit.id == bird.id));
        // Its backrow entry carries the Unit it is, with its live attack.
        let view = view_for(&next, P1);
        let BackrowView::Public(entry) = view.you.backrow[1].as_ref().expect("a public card") else {
            panic!("the backrow flyer is public");
        };
        let unit = entry.unit.as_ref().expect("the entry carries its unit");
        assert_eq!(unit.instance_id, bird.id);
        assert_eq!(unit.attack, 2);
        assert_eq!(unit.position, Position::Atk);
        assert!(
            unit.keywords
                .iter()
                .any(|keyword| keyword.kind() == KeywordKind::Flying)
        );

        // In Attack Position it is attacked only by Flying or Ranged.
        let mut fought = next;
        let walker = put(&mut fought, &grunt().id, slot(P2, Row::Units, 1), json!({}));
        let archer = put(&mut fought, &ranged().id, slot(P2, Row::Units, 2), json!({}));
        let (refused, _) = declare(&mut fought, &walker.id, &bird.id);
        assert!(refused.is_some_and(|refusal| refusal.contains("Flying or Ranged")));
        let (refused, _) = declare(&mut fought, &archer.id, &bird.id);
        assert_eq!(refused, None);
        assert_eq!(damage_on(&fought, &bird.id), 2);
    }

    #[test]
    fn r1081000_flying_stack_lists_every_zone() {
        let mut state = board("r1081000-stack");
        let _blocker = put(&mut state, &grunt().id, slot(P1, Row::Units, 1), json!({}));
        let flag = put(
            &mut state,
            &cloaked_field().id,
            slot(P1, Row::Backrow, 1),
            json!({}),
        );
        // Flying with Stack is offered the occupied zones of either row.
        let stacker = in_hand_with_mana(&mut state, &flying_stacker().id, P1);
        let zones = legal_zones_for(&state, P1, &stacker, &[]);
        assert!(zones.contains(&unit_zone(1)), "the occupied unit zone is offered");
        assert!(
            zones.contains(&backrow_zone(1)),
            "the occupied backrow zone is offered"
        );
        // Plain Flying is refused both: neither zone is open.
        let bird = in_hand_with_mana(&mut state, &flyer().id, P1);
        assert!(play_refusal(&state, &bird.id, json!({ "row": "units", "lane": 1 })).contains("not open"));
        assert!(play_refusal(&state, &bird.id, json!({ "row": "backrow", "lane": 1 })).contains("not open"));
        // Played onto the occupied backrow zone, the Stack flyer tops it as a pile.
        let next = act(
            &state,
            json!({
                "type": "play",
                "instanceId": stacker.id,
                "zone": { "row": "backrow", "lane": 1 },
                "playerId": "p1",
            }),
            None,
        );
        assert_eq!(
            card_at(&next, slot(P1, Row::Backrow, 1)).map(|card| card.id.clone()),
            Some(stacker.id.clone())
        );
        assert_eq!(
            beneath_at(&next, slot(P1, Row::Backrow, 1))
                .iter()
                .map(|card| card.id.clone())
                .collect::<Vec<_>>(),
            vec![flag.id.clone()]
        );
        assert!(
            active_units_of(&next, P1)
                .iter()
                .any(|unit| unit.id == stacker.id)
        );
    }

    #[test]
    fn r1081000_backrow_flyer_in_defense_lands() {
        let mut state = board("r1081000-defense");
        let walker = put(&mut state, &grunt().id, slot(P1, Row::Units, 1), json!({}));
        let wall = put(
            &mut state,
            &flying_taunter().id,
            slot(P2, Row::Backrow, 1),
            json!({}),
        );
        let other = put(&mut state, &grunt().id, slot(P2, Row::Units, 2), json!({}));
        // Landed: anything may attack it, and its Taunt binds every attacker.
        set_position(&mut state, &wall.id, Position::Def);
        assert_eq!(attack_targets(&state, &walker.id), vec![wall.id.clone()]);
        assert_eq!(attack_refusal(&state, &walker.id, &wall.id), "");
        assert!(attack_refusal(&state, &walker.id, &other.id).contains("Taunt"));
        let (refused, _) = declare(&mut state, &walker.id, &wall.id);
        assert_eq!(refused, None);
    }

    #[test]
    fn r1081000_locked_and_reserved_backrow_refuse_it() {
        let mut state = board("r1081000-locked");
        lock_zone(&mut state, slot(P1, Row::Backrow, 1));
        reserve_zone(&mut state, slot(P1, Row::Backrow, 2));
        let bird = in_hand_with_mana(&mut state, &flyer().id, P1);
        let zones = legal_zones_for(&state, P1, &bird, &[]);
        assert!(!zones.contains(&backrow_zone(1)), "a Locked lane is not offered");
        assert!(
            !zones.contains(&backrow_zone(2)),
            "a reserved lane is not offered"
        );
        assert!(zones.contains(&backrow_zone(3)), "other backrow lanes are");
        assert!(play_refusal(&state, &bird.id, json!({ "row": "backrow", "lane": 1 })).contains("not open"));
        assert!(play_refusal(&state, &bird.id, json!({ "row": "backrow", "lane": 2 })).contains("not open"));
        // Stack waives occupancy only: a Locked or reserved zone stays refused for it too.
        let stacker = in_hand_with_mana(&mut state, &flying_stacker().id, P1);
        let stacked = legal_zones_for(&state, P1, &stacker, &[]);
        assert!(!stacked.contains(&backrow_zone(1)));
        assert!(!stacked.contains(&backrow_zone(2)));
    }

    #[test]
    fn r1081000_no_double_count() {
        let mut state = board("r1081000-once");
        let bird = put(&mut state, &flyer().id, slot(P1, Row::Backrow, 2), json!({}));
        // One backrow flyer is one permanent.
        let quest: QuestDef = json_as(json!({
            "id": "q",
            "text": "q",
            "goal": { "kind": "permanentsControlled", "count": 9 },
            "rewards": [],
        }));
        assert_eq!(quest_progress_of(&state, &bird, &quest), 1);
        // One entry among the acting units, feeding every guard off that list.
        assert_eq!(
            active_units_of(&state, P1)
                .iter()
                .filter(|unit| unit.id == bird.id)
                .count(),
            1
        );
        // One card in the Audit's scan.
        let targets = audit_targets(
            &state,
            AuditArgs {
                controller: P1,
                active: P1,
                loc: 1,
                more: false,
                enemy_only: false,
            },
        );
        assert_eq!(targets.iter().filter(|card| card.id == bird.id).count(), 1);
        // One listing on the view: a public backrow entry carrying its unit, and no unit-row entry.
        let view = view_for(&state, P1);
        assert!(view.you.units.iter().all(|unit| unit.is_none()));
        let BackrowView::Public(entry) = view.you.backrow[1].as_ref().expect("a public card") else {
            panic!("the backrow flyer is public");
        };
        assert_eq!(entry.instance_id, bird.id);
        assert_eq!(
            entry.unit.as_ref().map(|unit| unit.instance_id.as_str()),
            Some(bird.id.as_str())
        );
    }

    #[test]
    fn r1081000_a_backrow_flyer_attacks_and_switches_by_action() {
        let mut state = board("r1081000-actions");
        let wing = put(&mut state, &flyer().id, slot(P1, Row::Backrow, 1), json!({}));
        let target = put(&mut state, &grunt().id, slot(P2, Row::Units, 1), json!({}));
        // `legal_actions` offers it its attack and its switch, and `reduce` takes every action offered.
        assert!(attack_targets(&state, &wing.id).contains(&target.id));
        let offered = legal_actions(&state, P1);
        assert!(offered.iter().any(
            |action| matches!(action, ActionBody::SwitchPosition { instance_id } if *instance_id == wing.id)
        ));
        for body in offered {
            let result = reduce(&state, &Action::new(body.clone(), P1, "r1081000-offered"));
            assert_eq!(result.error, None, "offered {body:?}, refused");
        }
        // Declared as an action, the attack lands on the hero, and the switch spends its exertion.
        let attacked = act(
            &state,
            json!({ "type": "attack", "attackerId": wing.id, "targetId": "hero-p2", "playerId": "p1" }),
            None,
        );
        assert!(live(&attacked, &wing.id).exertion.attacked);
        assert!(attacked.players.p2.hero.health < state.players.p2.hero.health);
        let switched = act(
            &state,
            json!({ "type": "switchPosition", "instanceId": wing.id, "playerId": "p1" }),
            None,
        );
        assert_eq!(live(&switched, &wing.id).position, Some(Position::Def));
    }

    #[test]
    fn r1081000_a_backrow_flyer_is_ready_again_at_its_controllers_turn() {
        let mut state = board("r1081000-ready");
        let wing = put(&mut state, &flyer().id, slot(P1, Row::Backrow, 1), json!({}));
        state = act(
            &state,
            json!({ "type": "switchPosition", "instanceId": wing.id, "playerId": "p1" }),
            None,
        );
        assert!(live(&state, &wing.id).exertion.switched);
        // Turns pass (R82 may pass p1's own) until p1's next one begins.
        let began = state.turn;
        while state.active != P1 || state.turn == began {
            let seat = state.active;
            state = act(
                &state,
                json!({ "type": "endTurn", "playerId": format!("{seat}") }),
                None,
            );
        }
        assert!(
            !live(&state, &wing.id).exertion.switched,
            "its exertion resets as a unit's does"
        );
    }

    #[test]
    fn r1081000_a_backrow_flyer_at_zero_health_dies_and_meets_every_unit_effect() {
        // An "all Units" effect reaches it, as the 1 damage of a quake shows on either side.
        let mut state = board("r1081000-all");
        let mine = put(&mut state, &flyer().id, slot(P1, Row::Backrow, 3), json!({}));
        let theirs = put(&mut state, &flyer().id, slot(P2, Row::Backrow, 2), json!({}));
        cast_now(&mut state, &quake().id, P1, false);
        assert_eq!(damage_on(&state, &mine.id), 1);
        assert_eq!(damage_on(&state, &theirs.id), 1);

        // Damage past its health is the §4.5 check's to collect, as for any unit.
        let mut doomed = board("r1081000-dies");
        let wing = put(&mut doomed, &flyer().id, slot(P1, Row::Backrow, 1), json!({}));
        if let Some(card) = find_instance_mut(&mut doomed, &wing.id) {
            card.damage = 3;
        }
        {
            let mut sink = sink_for(&mut doomed);
            state_check(&mut sink);
        }
        assert!(in_graveyard(&doomed, P1, &wing.id), "a flyer at 0 health dies");
        assert!(card_at(&doomed, slot(P1, Row::Backrow, 1)).is_none());
    }

    /// `effect` as `actor` casts it, straight through its `apply`.
    fn run_effect(state: &mut GameState, effect: Effect, actor: PlayerId) {
        let mut sink = sink_for(state);
        let mut ctx = make_context(
            &mut sink,
            None,
            HookOptions {
                controller: Some(actor),
                ..Default::default()
            },
        );
        (effect.apply)(&mut ctx);
    }

    /// Strip the text off `id`, Flying with it.
    fn vanilla(state: &mut GameState, id: &str) {
        if let Some(card) = find_instance_mut(state, id) {
            card.vanilla = true;
        }
    }

    fn stands(state: &GameState, player: PlayerId, id: &str) -> bool {
        standing_units_of(state, player).iter().any(|unit| unit.id == id)
    }

    #[test]
    fn r1081000_a_backrow_flyer_is_one_trigger_holder() {
        let mut state = board("r1081000-holder");
        let wing = put(&mut state, &flyer().id, slot(P1, Row::Backrow, 1), json!({}));
        let held = cards_in_trigger_order(&state)
            .into_iter()
            .filter(|holder| holder.card.id == wing.id)
            .count();
        assert_eq!(held, 1, "a Unit standing in the backrow holds its triggers once");
        assert_eq!(field_holders_where(&state, |card| card.id == wing.id).len(), 1);
    }

    #[test]
    fn r1081000_a_standing_unit_that_lost_flying_still_swaps_rotates_and_is_stolen_standing() {
        // A board swap sets it down standing on the other side, Flying or not.
        let mut state = board("r1081000-swap");
        let wing = put(&mut state, &flyer().id, slot(P1, Row::Backrow, 2), json!({}));
        vanilla(&mut state, &wing.id);
        run_effect(&mut state, swap_board(), P1);
        let swapped = live(&state, &wing.id);
        assert_eq!(swapped.controller, P2);
        assert!(stands(&state, P2, &wing.id), "it stands on the other side");
        assert!(active_units_of(&state, P2).iter().any(|unit| unit.id == wing.id));

        // A rotation across the centre line likewise.
        let mut ring = board("r1081000-rotate");
        let wing = put(&mut ring, &flyer().id, slot(P1, Row::Backrow, 5), json!({}));
        vanilla(&mut ring, &wing.id);
        {
            let mut sink = sink_for(&mut ring);
            rotate_rings(
                &mut sink,
                &RotationArgs {
                    direction: RotationDirection::Right,
                    perspective: P1,
                    radiant: None,
                },
            );
        }
        let moved = live(&ring, &wing.id);
        assert!(
            matches!(
                moved.zone,
                Zone::Field {
                    row: Row::Backrow,
                    ..
                }
            ) && moved.zone
                != Zone::Field {
                    player: P1,
                    row: Row::Backrow,
                    lane: 5
                },
            "it moved along the backrow ring"
        );
        let Zone::Field { player, .. } = moved.zone else {
            panic!("on the field");
        };
        assert!(stands(&ring, player, &wing.id), "and stands where it landed");

        // A steal takes it into the thief's open backrow zone, standing.
        let mut taken = board("r1081000-steal");
        let wing = put(&mut taken, &flyer().id, slot(P2, Row::Backrow, 3), json!({}));
        vanilla(&mut taken, &wing.id);
        run_effect(&mut taken, steal(json_as(json!({ "instanceId": wing.id }))), P1);
        assert_eq!(live(&taken, &wing.id).controller, P1);
        assert!(
            stands(&taken, P1, &wing.id),
            "stolen, it stands in the thief's backrow"
        );
        assert!(!stands(&taken, P2, &wing.id));
    }

    #[test]
    fn r1081000_cleave_beside_a_backrow_flyer_reaches_only_units_standing_there() {
        let mut state = board("r1081000-cleave");
        let wing = put(&mut state, &flyer().id, slot(P1, Row::Backrow, 2), json!({}));
        let neighbour = put(&mut state, &flyer().id, slot(P1, Row::Backrow, 3), json!({}));
        let trap = put(
            &mut state,
            &crater_trap().id,
            slot(P1, Row::Backrow, 1),
            json!({}),
        );
        let splasher = put(
            &mut state,
            &ranged_cleaver().id,
            slot(P2, Row::Units, 1),
            json!({}),
        );
        let (refused, events) = declare(&mut state, &splasher.id, &wing.id);
        assert_eq!(refused, None);
        assert_eq!(damage_on(&state, &wing.id), 2);
        assert_eq!(
            damage_on(&state, &neighbour.id),
            2,
            "the Unit beside it takes the splash"
        );
        assert_eq!(damage_on(&state, &trap.id), 0, "a Trap beside it is no Unit");
        assert!(
            events_of_type(&events, GameEventType::Damage)
                .iter()
                .all(|event| !matches!(event, GameEvent::Damage { target_id, .. } if *target_id == trap.id)),
            "no damage event names the Trap"
        );
    }

    #[test]
    fn r1081000_a_transform_replaces_a_standing_unit_with_a_unit_where_it_stands() {
        let mut state = board("r1081000-transform");
        let wing = put(&mut state, &flyer().id, slot(P1, Row::Backrow, 2), json!({}));
        run_effect(
            &mut state,
            transform(json_as(json!({ "instanceId": wing.id, "defId": grunt().id }))),
            P2,
        );
        let now = card_at(&state, slot(P1, Row::Backrow, 2))
            .cloned()
            .expect("a card stands there");
        assert_eq!(now.def_id, grunt().id);
        assert_ne!(now.id, wing.id);
        assert!(
            stands(&state, P1, &now.id),
            "the replacement stands there as a Unit"
        );
        assert!(
            find_instance(&state, &wing.id).is_none(),
            "the old card ceased to exist"
        );
    }

    #[test]
    fn r1081000_a_tribute_frees_a_backrow_zone_only_when_nothing_would_resume() {
        // A lone standing Flying Unit, tributed, leaves its zone empty: a Flying play may take it.
        let mut state = board("r1081000-tribute-lone");
        let lone = put(&mut state, &flyer().id, slot(P1, Row::Backrow, 1), json!({}));
        for lane in 2..=5 {
            put(&mut state, &flyer().id, slot(P1, Row::Backrow, lane), json!({}));
        }
        let bird = in_hand_with_mana(&mut state, &flyer().id, P1);
        let zones = legal_zones_for(&state, P1, &bird, std::slice::from_ref(&lone.id));
        assert!(
            zones.contains(&backrow_zone(1)),
            "the zone its Tribute empties is offered"
        );

        // One stacked on a Field Spell leaves the Field Spell to resume (R13), so it frees nothing.
        let mut stacked = board("r1081000-tribute-pile");
        let field = put(
            &mut stacked,
            &cloaked_field().id,
            slot(P1, Row::Backrow, 1),
            json!({}),
        );
        let mut rider = new_instance(&mut stacked, &flying_stacker().id, P1, Zone::Hand { player: P1 });
        assert!(place_standing_on_field(
            &mut stacked,
            &mut rider,
            slot(P1, Row::Backrow, 1),
            PlaceOnFieldOptions { stack: Some(true) },
        ));
        assert!(stands(&stacked, P1, &rider.id));
        assert_eq!(
            card_at(&stacked, slot(P1, Row::Backrow, 1)).map(|card| card.id.clone()),
            Some(rider.id.clone())
        );
        for lane in 2..=5 {
            put(&mut stacked, &flyer().id, slot(P1, Row::Backrow, lane), json!({}));
        }
        let bird = in_hand_with_mana(&mut stacked, &flyer().id, P1);
        let zones = legal_zones_for(&stacked, P1, &bird, std::slice::from_ref(&rider.id));
        assert!(
            !zones.contains(&backrow_zone(1)),
            "the card beneath {} would take its zone back",
            field.id
        );
    }

    #[test]
    fn r1081000_a_standing_reborn_unit_returns_standing() {
        let mut state = board("r1081000-reborn");
        let wing = put(
            &mut state,
            &flying_reborner().id,
            slot(P1, Row::Backrow, 1),
            json!({}),
        );
        if let Some(card) = find_instance_mut(&mut state, &wing.id) {
            card.damage = 2;
        }
        {
            let mut sink = sink_for(&mut state);
            state_check(&mut sink);
        }
        let back = card_at(&state, slot(P1, Row::Backrow, 1))
            .cloned()
            .expect("it came back");
        assert_eq!(back.id, wing.id);
        assert!(stands(&state, P1, &wing.id), "it returns standing where it stood");
        assert!(!in_graveyard(&state, P1, &wing.id));
    }

    #[test]
    fn r1081000_an_immutable_fuser_takes_no_flying_stack_unit_either() {
        // `field_game` registers the fuser (Classic+ #33's shape); #783's cards go on top of it.
        let mut state = super::super::fixtures::field::field_game("r1081000-fuser");
        register_keywords_783();
        state.turn = 4;
        state.active = P1;
        state.phase = Phase::Main;
        let holder = put(&mut state, &fuser.id, slot(P1, Row::Backrow, 2), json!({}));
        if let Some(card) = find_instance_mut(&mut state, &holder.id) {
            card.granted_keywords.push(Keyword::Immutable);
        }
        let stacker = in_hand_with_mana(&mut state, &flying_stacker().id, P1);
        assert!(!legal_zones_for(&state, P1, &stacker, &[]).contains(&backrow_zone(2)));
        assert!(
            play_refusal(&state, &stacker.id, json!({ "row": "backrow", "lane": 2 })).contains("Immutable")
        );
        if let Some(card) = find_instance_mut(&mut state, &holder.id) {
            card.granted_keywords.clear();
        }
        assert!(legal_zones_for(&state, P1, &stacker, &[]).contains(&backrow_zone(2)));
    }

    #[test]
    fn r1081000_r653_a_carrier_that_has_taken_its_one_unit_takes_no_flying_stack_unit() {
        let mut state = super::super::fixtures::field::field_game("r1081000-fuser-once");
        register_keywords_783();
        state.turn = 4;
        state.active = P1;
        state.phase = Phase::Main;
        let holder = put(&mut state, &fuser.id, slot(P1, Row::Backrow, 2), json!({}));
        // A Flying Unit names the carrier's zone as its Unit, and is the one it takes this stay.
        let bird = in_hand_with_mana(&mut state, &flyer().id, P1);
        let mut state = act(
            &state,
            json!({
                "type": "play",
                "instanceId": bird.id,
                "zone": { "row": "backrow", "lane": 2 },
                "playerId": "p1",
            }),
            None,
        );
        assert_eq!(
            carried_at(&state, slot(P1, Row::Backrow, 2)).map(|unit| unit.id.clone()),
            Some(bird.id.clone())
        );
        assert_eq!(stacked_onto(&live(&state, &holder.id)), Some(bird.id.as_str()));
        // That Unit goes; the carrier stays, and takes no other, Stack and Flying or not.
        if let Some(card) = find_instance_mut(&mut state, &bird.id) {
            card.damage = 10;
        }
        {
            let mut sink = sink_for(&mut state);
            state_check(&mut sink);
        }
        assert!(carried_at(&state, slot(P1, Row::Backrow, 2)).is_none());
        let stacker = in_hand_with_mana(&mut state, &flying_stacker().id, P1);
        assert!(!legal_zones_for(&state, P1, &stacker, &[]).contains(&backrow_zone(2)));
        assert!(
            play_refusal(&state, &stacker.id, json!({ "row": "backrow", "lane": 2 }))
                .contains("taken its one Unit")
        );
    }

    /// p1's flyer standing in backrow lane 2 with a Flying Stack Unit played on top of it: a pile with
    /// a Unit dormant beneath its top. Returns the pile's top and the Unit beneath it.
    fn flyer_beneath_a_stack_flyer(state: &mut GameState) -> (CardInstance, CardInstance) {
        let under = put(state, &flyer().id, slot(P1, Row::Backrow, 2), json!({}));
        let top = in_hand_with_mana(state, &flying_stacker().id, P1);
        *state = act(
            state,
            json!({
                "type": "play",
                "instanceId": top.id,
                "zone": { "row": "backrow", "lane": 2 },
                "playerId": "p1",
            }),
            None,
        );
        assert_pile(state, slot(P1, Row::Backrow, 2), &top.id, &under.id);
        (top, under)
    }

    /// `top` acts in `at` with `under` dormant beneath it, and `under` counts among no one's Units.
    fn assert_pile(state: &GameState, at: ZoneSlot, top: &str, under: &str) {
        assert_eq!(card_at(state, at).map(|card| card.id.as_str()), Some(top));
        assert_eq!(
            beneath_at(state, at)
                .iter()
                .map(|card| card.id.as_str())
                .collect::<Vec<_>>(),
            vec![under]
        );
        for player in [P1, P2] {
            assert!(active_units_of(state, player).iter().all(|unit| unit.id != under));
        }
    }

    #[test]
    fn r1081000_a_unit_dormant_beneath_a_backrow_top_swaps_rotates_and_rolls_back_with_its_pile() {
        // A board swap sets the pile down whole on the other side, the Unit beneath it again.
        let mut state = board("r1081000-pile-swap");
        let (top, under) = flyer_beneath_a_stack_flyer(&mut state);
        record_board_snapshot(&mut state);
        run_effect(&mut state, swap_board(), P1);
        assert_pile(&state, slot(P2, Row::Backrow, 2), &top.id, &under.id);
        assert_eq!(live(&state, &under.id).controller, P2);

        // A Rollback puts it back where the turn began, beneath its top, rather than in a hand.
        {
            let mut sink = sink_for(&mut state);
            restore_board(&mut sink, P1, 0, &[P1, P2]);
        }
        assert_pile(&state, slot(P1, Row::Backrow, 2), &top.id, &under.id);
        assert_eq!(live(&state, &under.id).zone.z(), ZoneName::Field);

        // A rotation moves it along the backrow ring whole, too.
        let mut ring = board("r1081000-pile-rotate");
        let (top, under) = flyer_beneath_a_stack_flyer(&mut ring);
        {
            let mut sink = sink_for(&mut ring);
            rotate_rings(
                &mut sink,
                &RotationArgs {
                    direction: RotationDirection::Right,
                    perspective: P1,
                    radiant: None,
                },
            );
        }
        let Zone::Field { player, row, lane } = live(&ring, &top.id).zone else {
            panic!("the top stays on the field");
        };
        assert_ne!((player, lane), (P1, 2), "it moved along the ring");
        assert_pile(&ring, ZoneSlot { player, row, lane }, &top.id, &under.id);
    }
}
