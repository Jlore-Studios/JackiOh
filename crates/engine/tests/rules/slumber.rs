// R1839000–R1839003 (issue #1839): Dormant, Stinkyscape's Hearthstone-style status (SPEC §6.1), with its
// §6.2 trigger "When this awakens" and its §6.3 verb Awaken. A Dormant card is on the field and fills its
// zone, and until it awakens nothing else reaches it (`zones::active_units_of`, `zones::acts_on_field`,
// `restrictions::unaffected_by`) and its lasting text is off (`preview::lasting_text_off`). Its countdown
// runs at its controller's turn start, after the Brittle tick (`slumber::awaken_at_turn_start`), and its own
// Activate may awaken it instead (`StaticFlags.slumber.wakesBy`). Proved on `fixtures/slumber.rs`.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{plain, stacker};
use crate::rules::fixtures::slumber::*;

/// p1's or p2's turn 9, its main phase, with the given hands and fields; both libraries hold plain
/// cards to draw, so no turn ends by itself (§2.5).
fn board(active: &str, p1_hand: Value, p1_field: Value, p2_hand: Value, p2_field: Value) -> Scenario {
    register_slumber_fixtures();
    let library = json!([plain.id, plain.id, plain.id, plain.id, plain.id, plain.id]);
    scenario(json!({
        "seed": "r1839000",
        "active": active,
        "p1": { "hand": p1_hand, "field": p1_field, "library": library },
        "p2": { "hand": p2_hand, "field": p2_field, "library": library },
    }))
}

fn id_at(s: &Scenario, player: PlayerId, lane: i32) -> String {
    s.unit(player, lane)
        .unwrap_or_else(|| panic!("no unit at {player:?}'s lane {lane}"))
        .id
}

fn slumber_of(s: &Scenario, id: &str) -> Option<Slumber> {
    s.card(id).slumber
}

fn dormant_for(turns: i32) -> Option<Slumber> {
    Some(Slumber { turns: Some(turns) })
}

fn pick(id: &str) -> Value {
    json!({ "targets": [{ "pick": "instance", "instanceId": id }] })
}

fn awakened_ids(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Awakened { instance_id, .. } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

fn health_of(s: &Scenario, player: PlayerId) -> i32 {
    s.state().players[player].hero.health
}

/// The reducer refuses `body` from `player` with the Dormant reason and hands the state back unchanged.
fn refused_dormant(s: &Scenario, body: ActionBody, player: PlayerId, nonce: &str) {
    let before = hash_state(s.state());
    let result = reduce(
        s.state(),
        &ActionInput {
            body,
            player_id: player,
        }
        .with_nonce(nonce),
    );
    assert_eq!(result.error.as_deref(), Some(SLUMBER_REFUSAL), "{nonce}");
    assert!(result.events.is_empty(), "{nonce}: a refusal emits nothing");
    assert_eq!(
        hash_state(&result.state),
        before,
        "{nonce}: a refusal changes nothing"
    );
}

/// Whether any legal action of `player` names `id` — as an attacker, an attack's target, a switch, an
/// activation or a play's pick.
fn acts_through(s: &Scenario, player: PlayerId, id: &str) -> bool {
    legal_actions(s.state(), player).iter().any(|action| {
        let json = serde_json::to_string(action).expect("serializes");
        json.contains(&format!("\"{id}\""))
    })
}

mod r1839000_a_dormant_card_fills_its_zone_and_nothing_reaches_it {
    use super::*;

    #[test]
    fn r1839000_it_enters_the_field_dormant_played_or_summoned_and_the_view_shows_it_on_both_seats() {
        let mut s = board(
            "p1",
            json!([imprisoned.id, call.id, plain.id]),
            json!([{ "def": imprisoned.id, "lane": 1, "slumber": { "turns": 2 } }]),
            json!([plain.id]),
            json!([]),
        );
        // Laid down Dormant by the setup.
        let placed = id_at(&s, P1, 1);
        assert_eq!(slumber_of(&s, &placed), dormant_for(2));
        // Played from hand, it enters Dormant by its text, its countdown printed.
        s.play(imprisoned.id.as_str(), json!({ "zone": 2 }));
        let played = id_at(&s, P1, 2);
        assert_eq!(slumber_of(&s, &played), dormant_for(2));
        // Summoned, the same: any way it arrives.
        s.play(call.id.as_str(), json!({}));
        assert_eq!(slumber_of(&s, &id_at(&s, P1, 3)), dormant_for(2));
        // A card without it enters awake.
        s.play(plain.id.as_str(), json!({ "zone": 4 }));
        assert_eq!(slumber_of(&s, &id_at(&s, P1, 4)), None);
        // Public on both seats, with its countdown; it can never act.
        for viewer in [P1, P2] {
            let view = s.view(viewer);
            let side = if viewer == P1 { &view.you } else { &view.opponent };
            let unit = side.units[0].as_ref().expect("lane 1 shows the Dormant card");
            assert_eq!(unit.slumber, dormant_for(2), "{viewer:?}");
            assert!(!unit.can_act, "{viewer:?}");
            assert_eq!(side.units[3].as_ref().and_then(|unit| unit.slumber), None);
        }
    }

    #[test]
    fn r1839000_it_neither_attacks_nor_switches_and_legal_actions_the_reducer_and_the_view_agree() {
        let s = board(
            "p1",
            json!([plain.id]),
            json!([{ "def": imprisoned.id, "lane": 1, "slumber": { "turns": 2 } }, { "def": plain.id, "lane": 2 }]),
            json!([plain.id]),
            json!([{ "def": plain.id, "lane": 1 }]),
        );
        let dormant = id_at(&s, P1, 1);
        let enemy = id_at(&s, P2, 1);
        assert!(
            !acts_through(&s, P1, &dormant),
            "no action of its controller's names it"
        );
        refused_dormant(
            &s,
            ActionBody::Attack {
                attacker_id: dormant.clone(),
                target_id: "hero-p2".into(),
            },
            P1,
            "attack the hero",
        );
        refused_dormant(
            &s,
            ActionBody::Attack {
                attacker_id: dormant.clone(),
                target_id: enemy,
            },
            P1,
            "attack a unit",
        );
        refused_dormant(
            &s,
            ActionBody::SwitchPosition {
                instance_id: dormant.clone(),
            },
            P1,
            "switch",
        );
        assert_eq!(
            why_cannot_declare(s.state(), s.card(&dormant))
                .err()
                .map(|e| e.message)
                .as_deref(),
            Some(SLUMBER_REFUSAL)
        );
    }

    #[test]
    fn r1839000_it_is_never_attacked_and_its_taunt_binds_nobody() {
        let s = board(
            "p2",
            json!([plain.id]),
            json!([{ "def": imprisoned.id, "lane": 1, "slumber": { "turns": 2 } }]),
            json!([plain.id]),
            json!([{ "def": plain.id, "lane": 1 }]),
        );
        let dormant = id_at(&s, P1, 1);
        let attacker = id_at(&s, P2, 1);
        assert!(!acts_through(&s, P2, &dormant), "no attack on it is offered");
        // Its Taunt binds nobody while it sleeps: the hero is open.
        assert!(legal_actions(s.state(), P2).iter().any(|action| matches!(
            action,
            ActionBody::Attack { attacker_id, target_id } if *attacker_id == attacker && target_id == "hero-p1"
        )));
        refused_dormant(
            &s,
            ActionBody::Attack {
                attacker_id: attacker,
                target_id: dormant,
            },
            P2,
            "attack it",
        );
    }

    #[test]
    fn r1839000_no_declared_target_offers_it_and_a_play_naming_it_is_refused() {
        let mut s = board(
            "p2",
            json!([plain.id]),
            json!([{ "def": imprisoned.id, "lane": 1, "slumber": { "turns": 2 } }, { "def": plain.id, "lane": 2 }]),
            json!([snipe.id, plain.id]),
            json!([]),
        );
        let dormant = id_at(&s, P1, 1);
        let awake = id_at(&s, P1, 2);
        assert!(
            !acts_through(&s, P2, &dormant),
            "the Spell's pick never offers it"
        );
        assert!(
            acts_through(&s, P2, &awake),
            "the awake Unit beside it is offered"
        );
        s.expect_refused(|s| s.play(snipe.id.as_str(), pick(&dormant)));
        assert_eq!(s.card(&dormant).damage, 0);
    }

    /// Every board-wide verb, each cast by p2 at a board of p1's holding the Dormant card in lane 1 and
    /// an awake plain in lane 2: the verb reaches the plain, and the Dormant card is as it was.
    #[test]
    fn r1839000_every_verb_passes_it_by_damage_destroy_exile_bounce_freeze_buff_steal_transform_allure() {
        for verb in [
            quake.id.as_str(),
            wipe.id.as_str(),
            void.id.as_str(),
            flood.id.as_str(),
            frost.id.as_str(),
            cheer.id.as_str(),
            heist.id.as_str(),
            morph.id.as_str(),
            lure.id.as_str(),
        ] {
            let mut s = board(
                "p2",
                json!([plain.id]),
                json!([{ "def": imprisoned.id, "lane": 1, "slumber": { "turns": 2 } }, { "def": plain.id, "lane": 2 }]),
                json!([verb, plain.id]),
                json!([]),
            );
            let dormant = s.card(id_at(&s, P1, 1).as_str()).clone();
            let awake = s.card(id_at(&s, P1, 2).as_str()).clone();
            s.play(verb, json!({}));
            assert_eq!(
                s.card(&dormant.id),
                &dormant,
                "{verb}: the Dormant card is untouched"
            );
            assert!(
                s.state()
                    .delayed
                    .iter()
                    .all(|effect| effect.watch.as_deref() != Some(dormant.id.as_str())),
                "{verb}: nothing waits on it"
            );
            let reached = match find_instance(s.state(), &awake.id) {
                Some(now) => {
                    now != &awake
                        || s.state()
                            .delayed
                            .iter()
                            .any(|e| e.watch.as_deref() == Some(awake.id.as_str()))
                }
                None => true,
            };
            assert!(reached, "{verb}: the awake plain beside it was reached");
        }
    }

    #[test]
    fn r1839000_its_aura_and_its_triggers_are_off_and_no_aura_reaches_it() {
        let mut s = board(
            "p1",
            json!([plain.id]),
            json!([{ "def": imprisoned.id, "lane": 1, "slumber": { "turns": 2 } }, { "def": plain.id, "lane": 2 }, { "def": banner.id, "lane": 3 }]),
            json!([plain.id]),
            json!([]),
        );
        let dormant = id_at(&s, P1, 1);
        let awake = id_at(&s, P1, 2);
        // The banner's +1/+1 reaches the plain and not the Dormant card; the Dormant card's own +1 Attack
        // reaches nobody.
        s.expect_stats(&dormant, json!({ "attack": 3, "health": 3 }));
        s.expect_stats(&awake, json!({ "attack": 4, "health": 4 }));
        // Its start-of-turn hook does not run while it sleeps.
        s.end_turn();
        s.end_turn();
        assert_eq!(s.state().active, P1);
        assert_eq!(slumber_of(&s, &dormant), dormant_for(1));
        assert_eq!(
            health_of(&s, P2),
            HERO_HEALTH,
            "no start-of-turn damage from a Dormant card"
        );
    }

    #[test]
    fn r1839000_it_fills_its_zone_for_board_space_but_is_no_unit_to_count_or_tribute() {
        let mut s = board(
            "p1",
            json!([swarm.id, census.id, plain.id]),
            json!([{ "def": imprisoned.id, "lane": 1, "slumber": { "turns": 2 } }, { "def": plain.id, "lane": 2 }]),
            json!([plain.id]),
            json!([]),
        );
        let dormant = id_at(&s, P1, 1);
        // "Count your Units": the awake plain alone.
        s.play(census.id.as_str(), json!({}));
        assert_eq!(health_of(&s, P2), HERO_HEALTH - 1);
        // A play's zone choice never offers its zone…
        let plain_in_hand = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == plain.id)
            .expect("a plain in hand");
        let zones: Vec<i32> = legal_actions(s.state(), P1)
            .iter()
            .filter_map(|action| match action {
                ActionBody::Play {
                    instance_id,
                    zone: Some(zone),
                    ..
                } if *instance_id == plain_in_hand.id => Some(zone.lane),
                _ => None,
            })
            .collect();
        assert!(!zones.is_empty() && !zones.contains(&1), "{zones:?}");
        // …and "fill your board" fills every zone but its own, which it still fills.
        s.play(swarm.id.as_str(), json!({}));
        for lane in 3..=UNIT_ZONES {
            assert_eq!(
                s.unit(P1, lane).map(|unit| unit.def_id),
                Some(plain.id.clone()),
                "lane {lane}"
            );
        }
        assert_eq!(id_at(&s, P1, 1), dormant);
        assert_eq!(
            s.state().players.p1.units[0].as_ref().map(Vec::len),
            Some(1),
            "nothing stacked on it"
        );
        assert_eq!(slumber_of(&s, &dormant), dormant_for(2));
    }

    /// Hearthstone's Magtheridon: a card played Dormant runs its Cry as it is played.
    #[test]
    fn r1839000_its_cry_runs_as_it_is_played_and_passes_it_by() {
        let mut s = board(
            "p1",
            json!([napper.id, plain.id]),
            json!([]),
            json!([plain.id]),
            json!([]),
        );
        s.play(napper.id.as_str(), json!({ "zone": 1 }));
        let napper_id = id_at(&s, P1, 1);
        assert_eq!(slumber_of(&s, &napper_id), dormant_for(1));
        assert_eq!(health_of(&s, P2), HERO_HEALTH - CRY_DAMAGE);
        assert_eq!(s.card(&napper_id).buffs, AttackHealth { attack: 0, health: 0 });
    }
}

mod r1839001_dormant_for_n_turns_counts_its_controllers_turns {
    use super::*;

    #[test]
    fn r1839001_r2259000_it_awakens_at_the_start_of_its_controllers_nth_turn_sick_its_trigger_first() {
        let mut s = board(
            "p1",
            json!([imprisoned.id, plain.id]),
            json!([]),
            json!([plain.id, plain.id]),
            json!([]),
        );
        s.play(imprisoned.id.as_str(), json!({ "zone": 1 }));
        let id = id_at(&s, P1, 1);
        assert_eq!(slumber_of(&s, &id), dormant_for(2));
        // The opponent's turn counts nothing.
        s.end_turn();
        assert_eq!(s.state().active, P2);
        assert_eq!(slumber_of(&s, &id), dormant_for(2));
        assert_eq!(
            s.view(P2).opponent.units[0]
                .as_ref()
                .and_then(|unit| unit.slumber),
            dormant_for(2)
        );
        // Its controller's first turn counts one down.
        s.end_turn();
        assert_eq!(s.state().active, P1);
        assert_eq!(slumber_of(&s, &id), dormant_for(1));
        assert!(awakened_ids(s.last_events()).is_empty());
        s.end_turn();
        assert_eq!(slumber_of(&s, &id), dormant_for(1));
        // Its controller's second turn: it awakens at the start, before the start-of-turn triggers, so its
        // "When this awakens" answers and then its own start-of-turn hook runs.
        s.end_turn();
        assert_eq!(s.state().active, P1);
        assert_eq!(slumber_of(&s, &id), None);
        assert_eq!(awakened_ids(s.last_events()), vec![id.clone()]);
        assert_eq!(
            health_of(&s, P2),
            HERO_HEALTH - AWAKENING_DAMAGE - START_OF_TURN_DAMAGE
        );
        let awakened = s
            .last_events()
            .iter()
            .position(|event| matches!(event, GameEvent::Awakened { .. }))
            .expect("awakened");
        let drawn = s
            .last_events()
            .iter()
            .position(|event| matches!(event, GameEvent::Drawn { .. }))
            .expect("the turn's draw");
        assert!(
            awakened < drawn,
            "it awakens at the start of the turn, before the draw"
        );
        // Awake, it is a Unit for every rule, and summoning sick for the turn it awakened in (R2259000).
        let attacks_hero = |s: &Scenario| {
            legal_actions(s.state(), P1).iter().any(
                |action| matches!(action, ActionBody::Attack { attacker_id, target_id } if *attacker_id == id && target_id == "hero-p2"),
            )
        };
        assert_eq!(s.card(&id).summoned_turn, Some(s.state().turn));
        assert!(!attacks_hero(&s));
        assert_eq!(
            s.view(P2).opponent.units[0]
                .as_ref()
                .and_then(|unit| unit.slumber),
            None
        );
        // Its controller's next turn, it may attack.
        s.end_turn();
        s.end_turn();
        assert_eq!(s.state().active, P1);
        assert!(attacks_hero(&s));
    }

    #[test]
    fn r1839001_dormant_for_1_turn_wakes_at_its_controllers_next_turn_and_a_game_without_one_emits_nothing() {
        let mut s = board(
            "p1",
            json!([napper.id, plain.id]),
            json!([]),
            json!([plain.id, plain.id]),
            json!([]),
        );
        s.play(napper.id.as_str(), json!({ "zone": 1 }));
        let id = id_at(&s, P1, 1);
        s.end_turn();
        assert!(
            awakened_ids(s.last_events()).is_empty(),
            "the opponent's turn start wakes nothing"
        );
        s.end_turn();
        assert_eq!(awakened_ids(s.last_events()), vec![id.clone()]);
        assert_eq!(slumber_of(&s, &id), None);
        s.expect_stats(&id, json!({ "attack": 2, "health": 2 }));
    }
}

mod r1839002_awakening_by_its_own_activate_and_the_awaken_verb {
    use super::*;

    fn activation(id: &str, ability: &str) -> ActionBody {
        ActionBody::Activate {
            instance_id: id.to_string(),
            ability: Some(ability.to_string()),
            targets: None,
            modes: None,
            tributes: None,
        }
    }

    fn offers(s: &Scenario, id: &str, ability: &str) -> bool {
        legal_actions(s.state(), P1).iter().any(|action| {
            matches!(action, ActionBody::Activate { instance_id, ability: Some(named), .. }
                if instance_id == id && named == ability)
        })
    }

    #[test]
    fn r1839002_r2259000_a_dormant_card_keeps_only_its_wake_ability_which_awakens_it_sick_this_turn() {
        let mut s = board(
            "p1",
            json!([starship.id, plain.id]),
            json!([]),
            json!([plain.id]),
            json!([]),
        );
        s.play(starship.id.as_str(), json!({ "zone": 1 }));
        let id = id_at(&s, P1, 1);
        assert_eq!(slumber_of(&s, &id), Some(Slumber { turns: None }));
        assert!(
            offers(&s, &id, LAUNCH),
            "its wake ability is offered while it sleeps"
        );
        assert!(!offers(&s, &id, ZAP), "no other ability is");
        refused_any(&s, activation(&id, ZAP));
        let mana = s.state().players.p1.mana.current;
        s.activate(&id, json!({ "ability": LAUNCH }));
        assert_eq!(slumber_of(&s, &id), None);
        assert_eq!(awakened_ids(s.last_events()), vec![id.clone()]);
        assert_eq!(s.state().players.p1.mana.current, mana - LAUNCH_MANA);
        // Awake, it is summoning sick for the rest of the turn (R2259000), and its other ability wakes too.
        let attacks = |s: &Scenario| {
            legal_actions(s.state(), P1)
                .iter()
                .any(|action| matches!(action, ActionBody::Attack { attacker_id, .. } if *attacker_id == id))
        };
        assert!(!attacks(&s));
        assert!(offers(&s, &id, ZAP));
        s.end_turn();
        s.end_turn();
        assert!(attacks(&s), "it attacks on its controller's next turn");
    }

    fn refused_any(s: &Scenario, body: ActionBody) {
        let result = reduce(
            s.state(),
            &ActionInput { body, player_id: P1 }.with_nonce("refused"),
        );
        assert!(result.error.is_some(), "refused");
    }

    #[test]
    fn r1839002_the_awaken_verb_names_a_dormant_card_by_id_and_does_nothing_to_an_awake_one() {
        let mut s = board(
            "p1",
            json!([rouse.id, plain.id]),
            json!([{ "def": imprisoned.id, "lane": 1, "slumber": { "turns": 2 } }, { "def": plain.id, "lane": 2 }]),
            json!([plain.id]),
            json!([]),
        );
        let dormant = id_at(&s, P1, 1);
        s.play(rouse.id.as_str(), json!({}));
        assert_eq!(slumber_of(&s, &dormant), None);
        assert_eq!(
            awakened_ids(s.last_events()),
            vec![dormant.clone()],
            "one awakening, the Dormant card's"
        );
        assert_eq!(
            health_of(&s, P2),
            HERO_HEALTH - AWAKENING_DAMAGE,
            "its \"When this awakens\" answered once"
        );
        s.expect_stats(id_at(&s, P1, 2).as_str(), json!({ "attack": 4, "health": 3 }));
    }
}

mod r1839003_a_stack_card_onto_a_dormant_one {
    use super::*;

    #[test]
    fn r1839003_the_stack_card_tops_the_pile_the_dormant_card_beneath_keeps_its_status_and_count_and_resumes_dormant()
     {
        let mut s = board(
            "p1",
            json!([stacker.id, plain.id]),
            json!([{ "def": imprisoned.id, "lane": 1, "slumber": { "turns": 2 } }]),
            json!([wipe.id, plain.id]),
            json!([]),
        );
        let dormant = id_at(&s, P1, 1);
        s.play(stacker.id.as_str(), json!({ "zone": 1 }));
        let top = id_at(&s, P1, 1);
        assert_eq!(s.card(&top).def_id, stacker.id);
        assert_eq!(
            slumber_of(&s, &dormant),
            dormant_for(2),
            "beneath, it is still Dormant"
        );
        // Beneath a Stack pile its countdown waits: its controller's turn starts count nothing.
        s.end_turn();
        s.end_turn();
        assert_eq!(s.state().active, P1);
        assert_eq!(slumber_of(&s, &dormant), dormant_for(2));
        // When the top leaves it resumes on top, still Dormant, and nothing destroyed it with the top.
        s.end_turn();
        s.play(wipe.id.as_str(), json!({}));
        assert_eq!(id_at(&s, P1, 1), dormant);
        assert_eq!(slumber_of(&s, &dormant), dormant_for(2));
        s.end_turn();
        assert_eq!(
            slumber_of(&s, &dormant),
            dormant_for(1),
            "on top again, it counts again"
        );
    }

    #[test]
    fn r1839003_a_stack_cards_transform_of_the_cards_beneath_passes_a_dormant_one_by() {
        let mut s = board(
            "p1",
            json!([copycat.id, plain.id]),
            json!([{ "def": imprisoned.id, "lane": 1, "slumber": { "turns": 2 } }]),
            json!([plain.id]),
            json!([]),
        );
        let dormant = s.card(id_at(&s, P1, 1).as_str()).clone();
        s.play(copycat.id.as_str(), json!({ "zone": 1 }));
        assert_eq!(s.card(&dormant.id).def_id, imprisoned.id, "not made a copy");
        assert_eq!(slumber_of(&s, &dormant.id), dormant_for(2));
    }
}
