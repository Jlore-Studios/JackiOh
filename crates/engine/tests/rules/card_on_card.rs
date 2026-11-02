//! The card-on-card verbs (#1850, R1850000–R1850005) through the fixtures in `fixtures/card_on_card.rs`:
//! Duo and Squad, exile auras, holding a permanent beneath a card until it leaves, a beneficial Spell
//! cast for yourself instead, and Radiant on reaching N keywords. Copying a Unit's text (R1850000) is in
//! `unit_text.rs` (#2273), Crater on a Field Spell (R1850006) in `crater.rs`.

use jackioh_engine::reduce::{legal_actions, reduce};
use jackioh_engine::testkit::*;
use jackioh_engine::view_for::view_for;
use jackioh_engine::wire::PlayerId::{P1, P2};

use super::fixtures::card_on_card::{
    BOOST, RADIANT_BOOST, bolt, boost, cheap, cheap_exiler, cheap_field, cheap_trap, crier, dear, duo, human,
    rod, rod_field, sack, sacker, tag_exiler, ticket, token, wormhole,
};
use super::fixtures::card_on_card::{apply, at, destroy, instance, on_field, pass, play, playing, top};
use super::fixtures::combat::stacker;
use super::fixtures::harness::{act_ok, events_of_type, in_hand, json_of, put, slot};

/// The board settled with nothing applied: a state check, as after any effect.
fn check(state: &mut GameState) -> Vec<GameEvent> {
    apply(
        state,
        None,
        P1,
        effects::damage(json_as(json!({ "to": { "of": "selfHero" }, "amount": 0 }))),
    )
}

/// R1839000: make the card on the field `id` Dormant until an effect awakens it, or awaken it.
fn set_dormant(state: &mut GameState, id: &str, dormant: bool) {
    let live = find_instance_mut(state, id).expect("on the field");
    live.slumber = dormant.then_some(Slumber { turns: None });
}

mod r1850001_duo_and_squad {
    use super::*;

    fn marks_in_view(state: &GameState, viewer: PlayerId, id: &str) -> Vec<(String, String)> {
        let view = serde_json::to_value(view_for(state, viewer)).expect("a view serializes");
        let mut found: Vec<(String, String)> = Vec::new();
        for side in ["you", "opponent"] {
            for row in ["units", "backrow"] {
                for card in view[side][row].as_array().into_iter().flatten() {
                    if card["instanceId"] != json!(id) && card["id"] != json!(id) {
                        continue;
                    }
                    for mark in card["marks"].as_array().into_iter().flatten() {
                        found.push((
                            mark["mark"].as_str().unwrap_or_default().to_string(),
                            mark["color"].as_str().unwrap_or_default().to_string(),
                        ));
                    }
                }
            }
        }
        found
    }

    fn duo_marks(events: &[GameEvent], added: bool) -> Vec<String> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Marked {
                    instance_id,
                    mark,
                    color,
                    added: was_added,
                    ..
                } if mark == "duo" && color == "gold" && *was_added == added => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn r1850001_duo_marks_gold_in_both_views_and_makes_it_radiant_at_the_markers_next_turn() {
        let mut state = playing("duo");
        let target = put(&mut state, &cheap().id, slot(P2, Row::Units, 1), json!({}));
        let played = play(&state, P1, &duo().id, false, at("units", 2, &target.id));
        assert_eq!(duo_marks(&played.events, true), vec![target.id.clone()]);
        let gold = vec![("duo".to_string(), "gold".to_string())];
        for viewer in [P1, P2] {
            assert_eq!(marks_in_view(&played.state, viewer, &target.id), gold);
        }
        let theirs = pass(&played.state);
        assert!(
            !find_instance(&theirs.state, &target.id)
                .expect("the target")
                .radiant
        );
        let mine = pass(&theirs.state);
        assert!(
            find_instance(&mine.state, &target.id)
                .expect("the target")
                .radiant
        );
        let radiant_at = mine
            .events
            .iter()
            .position(|event| matches!(event, GameEvent::RadiantSet { instance_id, .. } if *instance_id == target.id))
            .expect("radiantSet");
        let unmarked_at = mine
            .events
            .iter()
            .position(|event| matches!(event, GameEvent::Marked { instance_id, added: false, .. } if *instance_id == target.id))
            .expect("the mark taken off");
        assert!(radiant_at < unmarked_at);
        assert!(marks_in_view(&mine.state, P2, &target.id).is_empty());
    }

    #[test]
    fn r1850001_a_marked_card_that_leaves_is_not_made_radiant() {
        let mut state = playing("leaves");
        let target = put(&mut state, &cheap().id, slot(P1, Row::Units, 1), json!({}));
        let played = play(&state, P1, &duo().id, false, at("units", 2, &target.id));
        let mut state = played.state;
        let events = destroy(&mut state, &target.id);
        assert_eq!(duo_marks(&events, false), vec![target.id.clone()]);
        let theirs = pass(&state);
        let mine = pass(&theirs.state);
        assert!(events_of_type(&mine.events, GameEventType::RadiantSet).is_empty());
        let dead = mine
            .state
            .players
            .p1
            .graveyard
            .iter()
            .find(|card| card.id == target.id)
            .expect("in the graveyard");
        assert!(!dead.radiant);
    }

    #[test]
    fn r1850001_squad_marks_every_permanent_of_yours() {
        let mut state = playing("squad");
        let unit = put(&mut state, &cheap().id, slot(P1, Row::Units, 1), json!({}));
        let shining = put(
            &mut state,
            &dear().id,
            slot(P1, Row::Units, 3),
            json!({ "radiant": true }),
        );
        let field = put(
            &mut state,
            &cheap_field().id,
            slot(P1, Row::Backrow, 1),
            json!({}),
        );
        let trap = put(&mut state, &cheap_trap().id, slot(P1, Row::Backrow, 2), json!({}));
        let enemy = put(&mut state, &cheap().id, slot(P2, Row::Units, 1), json!({}));
        let played = play(
            &state,
            P1,
            &duo().id,
            true,
            json!({ "zone": { "row": "units", "lane": 2 } }),
        );
        // Lane order, then the backrow: a face-up Radiant card (`shining`) is passed by, a face-down one is
        // marked whatever it wears, and the enemy's are not the player's.
        assert_eq!(
            duo_marks(&played.events, true),
            vec![unit.id.clone(), field.id.clone(), trap.id.clone()]
        );
        let theirs = pass(&played.state);
        let mine = pass(&theirs.state);
        for id in [&unit.id, &field.id, &trap.id, &shining.id] {
            assert!(find_instance(&mine.state, id).expect("on the field").radiant);
        }
        assert!(
            !find_instance(&mine.state, &enemy.id)
                .expect("on the field")
                .radiant
        );
    }

    #[test]
    fn r1850001_squad_passes_a_dormant_card_by() {
        let mut state = playing("squad-dormant");
        let asleep = put(&mut state, &cheap().id, slot(P1, Row::Units, 1), json!({}));
        set_dormant(&mut state, &asleep.id, true);
        let awake = put(&mut state, &cheap().id, slot(P1, Row::Units, 3), json!({}));
        let played = play(
            &state,
            P1,
            &duo().id,
            true,
            json!({ "zone": { "row": "units", "lane": 2 } }),
        );
        assert_eq!(duo_marks(&played.events, true), vec![awake.id.clone()]);
    }
}

mod r1850002_exile_auras {
    use super::*;

    fn exiled(events: &[GameEvent]) -> Vec<String> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Exiled { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn r1850002_the_cap_or_less_is_exiled_not_the_source_or_face_down() {
        let mut state = playing("cap");
        let mine = put(&mut state, &cheap().id, slot(P1, Row::Units, 1), json!({}));
        let dear = put(&mut state, &dear().id, slot(P1, Row::Units, 3), json!({}));
        let field = put(
            &mut state,
            &cheap_field().id,
            slot(P1, Row::Backrow, 1),
            json!({}),
        );
        let theirs = put(&mut state, &cheap().id, slot(P2, Row::Units, 1), json!({}));
        let trap = put(&mut state, &cheap_trap().id, slot(P2, Row::Backrow, 1), json!({}));
        let source = put(&mut state, &cheap_exiler().id, slot(P1, Row::Units, 2), json!({}));
        let events = check(&mut state);
        // R68's board order: the active player's Units, their backrow, then the opponent's.
        assert_eq!(
            exiled(&events),
            vec![mine.id.clone(), field.id.clone(), theirs.id.clone()]
        );
        for id in [&mine.id, &field.id] {
            assert!(state.players.p1.exile.iter().any(|card| card.id == *id));
        }
        assert!(state.players.p2.exile.iter().any(|card| card.id == theirs.id));
        for id in [&dear.id, &trap.id, &source.id] {
            assert!(on_field(&state, id));
        }
    }

    #[test]
    fn r1850002_the_radiant_face_exiles_the_enemys_alone() {
        let mut state = playing("radiant");
        let mine = put(&mut state, &cheap().id, slot(P1, Row::Units, 1), json!({}));
        let theirs = put(&mut state, &cheap().id, slot(P2, Row::Units, 1), json!({}));
        let field = put(
            &mut state,
            &cheap_field().id,
            slot(P2, Row::Backrow, 1),
            json!({}),
        );
        put(
            &mut state,
            &cheap_exiler().id,
            slot(P1, Row::Units, 2),
            json!({ "radiant": true }),
        );
        let events = check(&mut state);
        assert_eq!(exiled(&events), vec![theirs.id.clone(), field.id.clone()]);
        assert!(on_field(&state, &mine.id));
    }

    #[test]
    fn r1850002_a_later_arrival_is_exiled() {
        let mut state = playing("later");
        put(&mut state, &cheap_exiler().id, slot(P1, Row::Units, 2), json!({}));
        check(&mut state);
        let before = state.players.p2.hero.health;
        let played = play(
            &state,
            P1,
            &crier().id,
            false,
            json!({ "zone": { "row": "units", "lane": 1 } }),
        );
        // Its Cry resolves, then the state check after it exiles it.
        assert_eq!(played.state.players.p2.hero.health, before - 1);
        assert!(top(&played.state, P1, 1).is_none());
        let damage_at = played
            .events
            .iter()
            .position(|event| matches!(event, GameEvent::Damage { .. }))
            .expect("the Cry's damage");
        let exiled_at = played
            .events
            .iter()
            .position(|event| matches!(event, GameEvent::Exiled { .. }))
            .expect("the exile");
        assert!(damage_at < exiled_at);
    }

    #[test]
    fn r1850002_a_tag_aura_spares_its_tags() {
        let mut state = playing("tags");
        let mine = put(&mut state, &human().id, slot(P1, Row::Units, 1), json!({}));
        let other = put(&mut state, &cheap().id, slot(P1, Row::Units, 3), json!({}));
        let field = put(
            &mut state,
            &cheap_field().id,
            slot(P1, Row::Backrow, 1),
            json!({}),
        );
        let theirs = put(&mut state, &human().id, slot(P2, Row::Units, 1), json!({}));
        let dear = put(&mut state, &dear().id, slot(P2, Row::Units, 2), json!({}));
        put(&mut state, &tag_exiler().id, slot(P1, Row::Units, 2), json!({}));
        let events = check(&mut state);
        assert_eq!(exiled(&events), vec![other.id.clone(), dear.id.clone()]);
        // Its scope is the Units: a Field Spell is no Unit.
        for id in [&mine.id, &theirs.id, &field.id] {
            assert!(on_field(&state, id));
        }
    }

    #[test]
    fn r1850002_a_keyword_the_aura_spares_keeps_its_card() {
        let mut state = playing("keywords");
        let piece = put(&mut state, &cheap().id, slot(P1, Row::Units, 1), json!({}));
        let ship = put(&mut state, &cheap().id, slot(P2, Row::Units, 1), json!({}));
        let other = put(&mut state, &cheap().id, slot(P2, Row::Units, 2), json!({}));
        for (id, keyword) in [(&piece.id, Keyword::StarshipPiece), (&ship.id, Keyword::Starship)] {
            find_instance_mut(&mut state, id)
                .expect("on the field")
                .granted_keywords
                .push(keyword);
        }
        put(&mut state, &tag_exiler().id, slot(P1, Row::Units, 2), json!({}));
        // Scornboi's Starship Piece and Starship, read through the layers: a granted one spares too.
        assert_eq!(exiled(&check(&mut state)), vec![other.id.clone()]);
        for id in [&piece.id, &ship.id] {
            assert!(on_field(&state, id));
        }
    }

    #[test]
    fn r1850002_the_aura_ends_with_its_card() {
        let mut state = playing("ends");
        let source = put(&mut state, &cheap_exiler().id, slot(P1, Row::Units, 2), json!({}));
        destroy(&mut state, &source.id);
        let after = put(&mut state, &cheap().id, slot(P1, Row::Units, 1), json!({}));
        assert!(exiled(&check(&mut state)).is_empty());
        assert!(on_field(&state, &after.id));

        // A Vanilla takes its text, the aura with it (§6.3).
        let quiet = put(&mut state, &cheap_exiler().id, slot(P1, Row::Units, 3), json!({}));
        assert_eq!(exiled(&check(&mut state)), vec![after.id.clone()]);
        apply(
            &mut state,
            None,
            P1,
            effects::vanilla(json_as(json!({ "target": instance(&quiet.id) }))),
        );
        let later = put(&mut state, &cheap().id, slot(P1, Row::Units, 1), json!({}));
        assert!(exiled(&check(&mut state)).is_empty());
        assert!(on_field(&state, &later.id));
    }

    #[test]
    fn r1850002_a_dormant_cards_aura_is_off_and_reaches_no_dormant_card() {
        let mut state = playing("dormant");
        let asleep = put(&mut state, &cheap().id, slot(P1, Row::Units, 1), json!({}));
        set_dormant(&mut state, &asleep.id, true);
        let theirs = put(&mut state, &cheap().id, slot(P2, Row::Units, 1), json!({}));
        let source = put(&mut state, &cheap_exiler().id, slot(P1, Row::Units, 2), json!({}));
        set_dormant(&mut state, &source.id, true);
        assert!(exiled(&check(&mut state)).is_empty());
        // Awake, its aura passes the Dormant card by, as every walk of the board does.
        set_dormant(&mut state, &source.id, false);
        assert_eq!(exiled(&check(&mut state)), vec![theirs.id.clone()]);
        assert!(on_field(&state, &asleep.id));
    }
}

mod r1850003_hold_beneath {
    use super::*;

    fn summoned(events: &[GameEvent]) -> Vec<(String, PlayerId)> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned {
                    instance_id, player, ..
                } => Some((instance_id.clone(), *player)),
                _ => None,
            })
            .collect()
    }

    fn ticketed(state: &mut GameState, held: &str) -> (String, Vec<GameEvent>) {
        let holder = put(state, &ticket().id, slot(P1, Row::Backrow, 1), json!({}));
        let events = apply(
            state,
            Some(&holder.id),
            P1,
            effects::hold_beneath(json_as(json!({ "target": instance(held) }))),
        );
        (holder.id, events)
    }

    #[test]
    fn r1850003_returns_to_its_controller_when_the_holder_dies() {
        let mut state = playing("dies");
        let held = put(&mut state, &crier().id, slot(P2, Row::Units, 2), json!({}));
        if let Some(live) = find_instance_mut(&mut state, &held.id) {
            live.buffs = AttackHealth { attack: 1, health: 1 };
        }
        let played = play(&state, P1, &ticket().id, false, at("backrow", 1, &held.id));
        let mut state = played.state;
        let holder = state.players.p1.backrow[0].clone().expect("the ticket");
        assert!(played.events.iter().any(|event| matches!(
            event,
            GameEvent::Held { player, instance_id, holder_id, .. }
                if *player == P2 && *instance_id == held.id && *holder_id == holder.id
        )));
        assert!(!on_field(&state, &held.id));
        assert_eq!(state.held.as_ref().map(Vec::len), Some(1));

        let before = state.players.p1.hero.health;
        let events = destroy(&mut state, &holder.id);
        assert_eq!(summoned(&events), vec![(held.id.clone(), P2)]);
        let back = top(&state, P2, 1).expect("back in R64's zone");
        assert_eq!(back.id, held.id);
        // A summon: summoning sick, no Cry (R1), and the stay it was taken from is over (R78).
        assert_eq!(back.summoned_turn, Some(state.turn));
        assert_eq!(state.players.p1.hero.health, before);
        assert_eq!(back.buffs, AttackHealth { attack: 0, health: 0 });
        assert_eq!(state.held, None);
    }

    #[test]
    fn r1850003_a_bounced_or_exiled_holder_releases() {
        let mut state = playing("bounced");
        let held = put(&mut state, &cheap().id, slot(P2, Row::Units, 1), json!({}));
        let (holder, _) = ticketed(&mut state, &held.id);
        assert!(!on_field(&state, &held.id));
        apply(
            &mut state,
            None,
            P1,
            effects::bounce(json_as(json!({ "target": instance(&holder) }))),
        );
        assert_eq!(top(&state, P2, 1).map(|card| card.id), Some(held.id.clone()));

        let mut state = playing("exiled");
        let held = put(&mut state, &cheap().id, slot(P2, Row::Units, 1), json!({}));
        let (holder, _) = ticketed(&mut state, &held.id);
        let events = apply(
            &mut state,
            None,
            P1,
            effects::exile(json_as(json!({ "target": instance(&holder) }))),
        );
        assert_eq!(summoned(&events), vec![(held.id.clone(), P2)]);
    }

    #[test]
    fn r1850003_a_new_holder_summons_it_for_its_controller() {
        // Base: the Sack is the enemy's, and so is what comes out of it.
        let mut state = playing("sack");
        let held = put(&mut state, &dear().id, slot(P2, Row::Units, 2), json!({}));
        let played = play(
            &state,
            P1,
            &sacker().id,
            false,
            json!({ "targets": [{ "pick": "instance", "instanceId": held.id }] }),
        );
        let mut state = played.state;
        let bag = top(&state, P2, 1).expect("the Sack in p2's first zone");
        assert_eq!(bag.def_id, sack().id);
        let summoned_at = played
            .events
            .iter()
            .position(
                |event| matches!(event, GameEvent::Summoned { instance_id, .. } if *instance_id == bag.id),
            )
            .expect("the Sack's summon");
        let held_at = played
            .events
            .iter()
            .position(|event| matches!(event, GameEvent::Held { holder_id, .. } if *holder_id == bag.id))
            .expect("held beneath the Sack");
        assert!(summoned_at < held_at);
        let events = destroy(&mut state, &bag.id);
        assert_eq!(summoned(&events), vec![(held.id.clone(), P2)]);

        // Radiant: the Sack is yours, and so is what comes out of it; its owner stays.
        let mut state = playing("sack-radiant");
        let held = put(&mut state, &dear().id, slot(P2, Row::Units, 2), json!({}));
        let played = play(
            &state,
            P1,
            &sacker().id,
            true,
            json!({ "targets": [{ "pick": "instance", "instanceId": held.id }] }),
        );
        let mut state = played.state;
        let bag = top(&state, P1, 1).expect("the Sack in p1's first zone");
        let events = destroy(&mut state, &bag.id);
        assert_eq!(summoned(&events), vec![(held.id.clone(), P1)]);
        let back = find_instance(&state, &held.id).expect("back");
        assert_eq!((back.controller, back.owner), (P1, P2));
    }

    #[test]
    fn r1850003_a_holder_beneath_a_stack_or_dormant_still_holds() {
        let mut state = playing("covered");
        let held = put(&mut state, &dear().id, slot(P2, Row::Units, 2), json!({}));
        let played = play(
            &state,
            P1,
            &sacker().id,
            false,
            json!({ "targets": [{ "pick": "instance", "instanceId": held.id }] }),
        );
        let mut state = played.state;
        let bag = top(&state, P2, 1).expect("the Sack in p2's first zone");
        // A Stack card tops the Sack: beneath it, the Sack is on the field still (§3.2), so it holds.
        let mut cover = new_instance(&mut state, &stacker.id, P2, Zone::Hand { player: P2 });
        assert!(place_on_field(
            &mut state,
            &mut cover,
            slot(P2, Row::Units, 1),
            json_as(json!({ "stack": true }))
        ));
        assert!(summoned(&check(&mut state)).is_empty());
        destroy(&mut state, &cover.id);
        assert_eq!(top(&state, P2, 1).map(|card| card.id), Some(bag.id.clone()));
        assert_eq!(state.held.as_ref().map(Vec::len), Some(1));
        set_dormant(&mut state, &bag.id, true);
        assert!(summoned(&check(&mut state)).is_empty());
        assert!(!on_field(&state, &held.id));
        set_dormant(&mut state, &bag.id, false);
        assert_eq!(
            summoned(&destroy(&mut state, &bag.id)),
            vec![(held.id.clone(), P2)]
        );
    }

    #[test]
    fn r1850003_with_no_room_it_is_exiled() {
        let mut state = playing("no-room");
        let held = put(&mut state, &cheap().id, slot(P2, Row::Units, 1), json!({}));
        let (holder, _) = ticketed(&mut state, &held.id);
        for lane in 1..=5 {
            put(&mut state, &dear().id, slot(P2, Row::Units, lane), json!({}));
        }
        let events = destroy(&mut state, &holder);
        assert!(summoned(&events).is_empty());
        assert!(
            events.iter().any(
                |event| matches!(event, GameEvent::Exiled { instance_id, .. } if *instance_id == held.id)
            )
        );
        assert!(state.players.p2.exile.iter().any(|card| card.id == held.id));
        assert_eq!(state.held, None);
    }

    /// The `held` events of the window `events`, as `viewer` reads them.
    fn held_seen(state: &GameState, events: Vec<GameEvent>, viewer: PlayerId) -> Vec<Value> {
        let mut state = state.clone();
        state.applied = vec![AppliedAction {
            nonce: "held".into(),
            events,
        }];
        view_for(&state, viewer)
            .events
            .iter()
            .filter(|event| event.event_type() == GameEventType::Held)
            .map(json_of)
            .collect()
    }

    #[test]
    fn r1850003_held_reads_as_where_the_card_and_its_holder_are_now() {
        let mut state = playing("held-view");
        let held = put(&mut state, &cheap().id, slot(P2, Row::Units, 1), json!({}));
        let (holder, mut events) = ticketed(&mut state, &held.id);
        for viewer in [P1, P2] {
            let seen = held_seen(&state, events.clone(), viewer);
            assert_eq!(seen.len(), 1);
            assert_eq!(seen[0]["instanceId"], json!(held.id));
            assert_eq!(seen[0]["defId"], json!(cheap().id));
            assert_eq!(seen[0]["holderId"], json!(holder));
        }

        // Holder and released card (id kept) bounced to p1's and p2's hands: each seat reads its own (R97).
        for id in [holder.clone(), held.id.clone()] {
            events.extend(apply(
                &mut state,
                None,
                P1,
                effects::bounce(json_as(json!({ "target": instance(&id) }))),
            ));
        }
        assert_eq!(state.players.p1.hand.last().map(|card| &card.id), Some(&holder));
        assert_eq!(state.players.p2.hand.last().map(|card| &card.id), Some(&held.id));
        let mine = &held_seen(&state, events.clone(), P1)[0];
        assert_eq!(mine["instanceId"], json!(HIDDEN_ID));
        assert_eq!(mine["defId"], json!(HIDDEN_ID));
        assert_eq!(mine["holderId"], json!(holder));
        let theirs = &held_seen(&state, events.clone(), P2)[0];
        assert_eq!(theirs["instanceId"], json!(held.id));
        assert_eq!(theirs["defId"], json!(cheap().id));
        assert_eq!(theirs["holderId"], json!(HIDDEN_ID));
        state.applied = vec![AppliedAction {
            nonce: "window".into(),
            events,
        }];
        assert!(
            !json_of(view_for(&state, P1).events)
                .to_string()
                .contains(&held.id)
        );
    }

    #[test]
    fn r1850003_a_held_card_reads_to_both_seats_held_and_released() {
        // I6: a held card is in no pile, yet it went face-up, so `held` naming it hides nothing from a seat.
        let mut state = playing("held-i6");
        let held = put(&mut state, &crier().id, slot(P2, Row::Units, 2), json!({}));
        let played = play(&state, P1, &ticket().id, false, at("backrow", 1, &held.id));
        let mut state = played.state;
        assert_eq!(state.held.as_ref().map(Vec::len), Some(1));
        let monitor = create_invariant_monitor(&state);
        assert_eq!(monitor.hidden(&state), Vec::<String>::new());

        let holder = state.players.p1.backrow[0].clone().expect("the ticket");
        let mut events = played.events.clone();
        events.extend(destroy(&mut state, &holder.id));
        assert!(on_field(&state, &held.id));
        state.applied = vec![AppliedAction {
            nonce: "released".into(),
            events,
        }];
        assert_eq!(monitor.hidden(&state), Vec::<String>::new());
    }

    #[test]
    fn r1850003_a_held_cards_hidden_nerfs_ride_it_and_no_seats_copy_holds_them() {
        // R1845000: a hidden Nerf rides its card beneath a holder too, and a seat's copy (the dry run's,
        // the AI's redaction) drops it there.
        const HIDDEN_NERFS: i32 = 2;
        let counts = |state: &GameState| -> Vec<Option<i32>> {
            state
                .held
                .iter()
                .flatten()
                .map(|entry| entry.card.hidden_nerfs)
                .collect()
        };
        let mut state = playing("hidden-nerfs");
        let held = put(&mut state, &cheap().id, slot(P2, Row::Units, 1), json!({}));
        if let Some(live) = find_instance_mut(&mut state, &held.id) {
            live.hidden_nerfs = Some(HIDDEN_NERFS);
        }
        ticketed(&mut state, &held.id);
        assert_eq!(counts(&state), vec![Some(HIDDEN_NERFS)]);
        for viewer in [P1, P2] {
            let copy = subsystems::scorer::concealed_copy(&state, viewer);
            assert_eq!(counts(&copy), vec![None], "{viewer:?}'s copy");
        }
    }

    #[test]
    fn r1850003_a_rollback_moves_a_held_card_back_and_it_is_held_no_longer() {
        // R419: the snapshot's card goes back to its zone from wherever it is now, beneath its holder too.
        let mut state = playing("rollback");
        let held = put(&mut state, &cheap().id, slot(P2, Row::Units, 1), json!({}));
        state.board_history = None;
        subsystems::board_history::record_board_snapshot(&mut state);
        let (holder, _) = ticketed(&mut state, &held.id);
        assert!(!on_field(&state, &held.id));
        let mut events: Vec<GameEvent> = Vec::new();
        {
            let mut rng = Rng::new(&state.seed.clone(), state.rng_cursor);
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            assert_eq!(
                subsystems::board_history::restore_board(&mut sink, P1, 0, &[P2]),
                Some(0)
            );
        }
        assert_eq!(top(&state, P2, 1).map(|card| card.id), Some(held.id.clone()));
        assert_eq!(state.held, None);

        // Its holder's leaving later summons nothing: the card stays where it went since.
        destroy(&mut state, &held.id);
        assert!(summoned(&destroy(&mut state, &holder)).is_empty());
        assert!(state.players.p2.graveyard.iter().any(|card| card.id == held.id));
        assert!(!on_field(&state, &held.id));
    }

    #[test]
    fn r1850003_a_held_token_ceases_to_exist() {
        let mut state = playing("token");
        let held = put(&mut state, &token().id, slot(P2, Row::Units, 1), json!({}));
        let (holder, events) = ticketed(&mut state, &held.id);
        assert_eq!(events_of_type(&events, GameEventType::Held).len(), 1);
        assert_eq!(state.held, None);
        assert!(find_instance(&state, &held.id).is_none());
        assert!(summoned(&destroy(&mut state, &holder)).is_empty());

        // A face-down card is never held: nothing hidden goes beneath a card.
        let mut state = playing("face-down");
        let trap = put(&mut state, &cheap_trap().id, slot(P2, Row::Backrow, 1), json!({}));
        let (_, events) = ticketed(&mut state, &trap.id);
        assert!(events_of_type(&events, GameEventType::Held).is_empty());
        assert!(on_field(&state, &trap.id));
    }
}

mod r1865000_set_aside_from_hand {
    use super::*;

    fn holder(state: &mut GameState, lane: i32) -> CardInstance {
        put(state, &ticket().id, slot(P1, Row::Backrow, lane), json!({}))
    }

    fn in_p1_hand(state: &mut GameState, def_id: &str) -> CardInstance {
        in_hand(state, def_id, P1, 1)
            .into_iter()
            .next()
            .expect("a hand card")
    }

    fn set_aside(state: &mut GameState, holder: &str, card: &str) -> Vec<GameEvent> {
        apply(
            state,
            Some(holder),
            P1,
            effects::set_aside_from_hand(json_as(json!({ "target": instance(card) }))),
        )
    }

    fn give_back(state: &mut GameState, holder: &str, args: Value) -> Vec<GameEvent> {
        apply(
            state,
            Some(holder),
            P1,
            effects::return_held_to_hand(json_as(args)),
        )
    }

    fn upgraded(events: &[GameEvent], id: &str) -> usize {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Upgraded { instance_id, .. } if instance_id == id))
            .count()
    }

    fn in_hand_of(state: &GameState, player: PlayerId, id: &str) -> bool {
        state.players[player].hand.iter().any(|card| card.id == id)
    }

    #[test]
    fn r1865000_a_hand_card_set_aside_leaves_the_hand_and_waits_beneath_its_holder() {
        let mut state = playing("set-aside");
        let keeper = holder(&mut state, 1);
        let card = in_p1_hand(&mut state, &cheap().id);
        let events = set_aside(&mut state, &keeper.id, &card.id);
        assert!(!in_hand_of(&state, P1, &card.id));
        let held = state.held.clone().expect("one card held");
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].holder_id, keeper.id);
        assert_eq!(held[0].card.id, card.id);
        assert_eq!(held[0].to, P1);
        assert_eq!(held[0].exile_on_release, Some(true));
        assert!(events.iter().any(|event| matches!(
            event,
            GameEvent::Held { player: P1, instance_id, holder_id, .. }
                if *instance_id == card.id && *holder_id == keeper.id
        )));
    }

    #[test]
    fn r1865000_it_comes_back_to_its_owners_hand_at_the_price_given_and_buffed() {
        let mut state = playing("give-back");
        let keeper = holder(&mut state, 1);
        let card = in_p1_hand(&mut state, &cheap().id);
        if let Some(live) = find_instance_mut(&mut state, &card.id) {
            live.cost_mod = 1;
        }
        set_aside(&mut state, &keeper.id, &card.id);
        let events = give_back(
            &mut state,
            &keeper.id,
            json!({ "costOverride": 0, "upgrades": 2 }),
        );
        assert!(in_hand_of(&state, P1, &card.id));
        let back = find_instance(&state, &card.id).expect("in hand");
        assert_eq!(back.cost_override, Some(0));
        assert_eq!(back.cost_mod, 0, "R766: its price reset first");
        assert_eq!(upgraded(&events, &card.id), 2);
        assert_eq!(state.held, None);
    }

    #[test]
    fn r1865000_a_full_hand_burns_it_unbuffed() {
        let mut state = playing("burned");
        let keeper = holder(&mut state, 1);
        let card = in_p1_hand(&mut state, &cheap().id);
        if let Some(live) = find_instance_mut(&mut state, &card.id) {
            live.buffs = AttackHealth { attack: 1, health: 1 };
        }
        set_aside(&mut state, &keeper.id, &card.id);
        let room = jackioh_engine::config::HAND_CAP - state.players.p1.hand.len() as i32;
        in_hand(&mut state, &cheap().id, P1, room);
        let events = give_back(
            &mut state,
            &keeper.id,
            json!({ "costOverride": 0, "upgrades": 2 }),
        );
        assert!(state.players.p1.graveyard.iter().any(|gone| gone.id == card.id));
        assert!(
            events.iter().any(
                |event| matches!(event, GameEvent::Burned { instance_id, .. } if *instance_id == card.id)
            )
        );
        assert_eq!(upgraded(&events, &card.id), 0);
        let burned = find_instance(&state, &card.id).expect("in the graveyard");
        assert_eq!(
            burned.buffs,
            AttackHealth { attack: 0, health: 0 },
            "R215: the printed card"
        );
        assert_eq!(burned.cost_override, None);
        assert_eq!(state.held, None);
    }

    #[test]
    fn r1865000_it_is_exiled_reset_when_its_holder_leaves_the_field_first() {
        let mut state = playing("exiled");
        let keeper = holder(&mut state, 1);
        let card = in_p1_hand(&mut state, &cheap().id);
        if let Some(live) = find_instance_mut(&mut state, &card.id) {
            live.cost_mod = 1;
            live.buffs = AttackHealth { attack: 1, health: 1 };
        }
        set_aside(&mut state, &keeper.id, &card.id);
        let events = destroy(&mut state, &keeper.id);
        let exiled = state
            .players
            .p1
            .exile
            .iter()
            .find(|gone| gone.id == card.id)
            .expect("in p1's exile");
        assert_eq!(exiled.cost_mod, 0);
        assert_eq!(exiled.buffs, AttackHealth { attack: 0, health: 0 });
        assert!(
            events.iter().any(
                |event| matches!(event, GameEvent::Exiled { instance_id, .. } if *instance_id == card.id)
            )
        );
        assert!(!events.iter().any(
            |event| matches!(event, GameEvent::Summoned { instance_id, .. } if *instance_id == card.id)
        ));
        assert_eq!(state.held, None);
    }

    #[test]
    fn r1865000_a_unit_token_ceases_to_exist_and_nothing_is_held() {
        let mut state = playing("token");
        let keeper = holder(&mut state, 1);
        let card = in_p1_hand(&mut state, &token().id);
        let events = set_aside(&mut state, &keeper.id, &card.id);
        assert_eq!(find_instance(&state, &card.id), None, "R11: it ceased to exist");
        assert_eq!(state.held, None);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, GameEvent::Held { instance_id, .. } if *instance_id == card.id))
        );
    }

    #[test]
    fn r1865000_nothing_is_set_aside_off_a_hand_or_by_a_holder_off_the_field() {
        let mut state = playing("off-hand");
        let keeper = holder(&mut state, 1);
        let unit = put(&mut state, &cheap().id, slot(P1, Row::Units, 1), json!({}));
        let events = set_aside(&mut state, &keeper.id, &unit.id);
        assert!(on_field(&state, &unit.id), "a card on the field is no hand card");
        assert_eq!(state.held, None);
        assert!(events_of_type(&events, "held").is_empty());

        let mut state = playing("off-field");
        let keeper = in_p1_hand(&mut state, &ticket().id);
        let card = in_p1_hand(&mut state, &cheap().id);
        set_aside(&mut state, &keeper.id, &card.id);
        assert!(
            in_hand_of(&state, P1, &card.id),
            "a holder in a hand holds nothing"
        );
        assert_eq!(state.held, None);
    }

    #[test]
    fn r1865000_it_returns_only_what_the_running_card_holds() {
        let mut state = playing("two-holders");
        let first = holder(&mut state, 1);
        let second = holder(&mut state, 2);
        let one = in_p1_hand(&mut state, &cheap().id);
        let two = in_p1_hand(&mut state, &dear().id);
        set_aside(&mut state, &first.id, &one.id);
        set_aside(&mut state, &second.id, &two.id);
        give_back(&mut state, &first.id, json!({}));
        assert!(in_hand_of(&state, P1, &one.id));
        assert!(!in_hand_of(&state, P1, &two.id));
        let held = state.held.clone().expect("the second still held");
        assert_eq!(
            held.iter()
                .map(|entry| (entry.holder_id.clone(), entry.card.id.clone()))
                .collect::<Vec<_>>(),
            vec![(second.id.clone(), two.id.clone())]
        );
        let back = find_instance(&state, &one.id).expect("in hand");
        assert_eq!(back.cost_override, None, "no price given, none set");
    }

    #[test]
    fn r1865000_a_set_aside_card_hides_nothing_from_either_seat() {
        // I6, the fuzz's hidden-information monitor: a set-aside card is in no pile and reads to both
        // seats there; back in its owner's hand, the events that named it read as that hand does (R97).
        let mut state = playing("set-aside-i6");
        let keeper = holder(&mut state, 1);
        let card = in_p1_hand(&mut state, &cheap().id);
        let mut events = set_aside(&mut state, &keeper.id, &card.id);
        state.applied = vec![AppliedAction {
            nonce: "set-aside".into(),
            events: events.clone(),
        }];
        let monitor = create_invariant_monitor(&state);
        assert_eq!(monitor.hidden(&state), Vec::<String>::new());

        events.extend(give_back(&mut state, &keeper.id, json!({ "upgrades": 1 })));
        assert!(in_hand_of(&state, P1, &card.id));
        state.applied = vec![AppliedAction {
            nonce: "given-back".into(),
            events,
        }];
        assert_eq!(monitor.hidden(&state), Vec::<String>::new());
        let shown = view_for(&state, P2);
        let named = serde_json::to_string(&shown).unwrap_or_default();
        assert!(
            !named.contains(&card.id),
            "p2 reads the card back in p1's hand by no id: {named}"
        );
    }
}

mod r1850004_cast_for_yourself {
    use super::*;

    fn guarded(seed: &str, radiant: bool) -> GameState {
        let mut state = playing(seed);
        put(
            &mut state,
            &wormhole().id,
            slot(P2, Row::Backrow, 1),
            json!({ "radiant": radiant }),
        );
        state
    }

    fn boost_on(state: &GameState, id: &str) -> ReduceResult {
        play(
            state,
            P1,
            &boost().id,
            false,
            json!({ "targets": [{ "pick": "instance", "instanceId": id }] }),
        )
    }

    fn the_boost(state: &GameState) -> Option<CardInstance> {
        [P1, P2]
            .into_iter()
            .flat_map(|player| state.players[player].graveyard.iter())
            .find(|card| card.def_id == boost().id)
            .cloned()
    }

    #[test]
    fn r1850004_countered_and_cast_by_the_traps_owner_on_their_side() {
        let mut state = guarded("cast", false);
        let mine = put(&mut state, &cheap().id, slot(P1, Row::Units, 1), json!({}));
        let first = put(&mut state, &dear().id, slot(P2, Row::Units, 1), json!({}));
        let second = put(&mut state, &cheap().id, slot(P2, Row::Units, 2), json!({}));
        let played = boost_on(&state, &mine.id);
        assert_eq!(events_of_type(&played.events, GameEventType::TrapFired).len(), 1);
        assert_eq!(events_of_type(&played.events, GameEventType::Countered).len(), 1);
        // The caster is the Trap's owner, offered their own side alone.
        let pending = played.state.pending.clone().expect("the cast's target prompt");
        assert_eq!(pending.player_id, P2);
        let offered: Vec<Selection> = pending
            .options
            .iter()
            .map(|option| option.selection.clone())
            .collect();
        let theirs = vec![
            Selection::Instance {
                instance_id: first.id.clone(),
            },
            Selection::Instance {
                instance_id: second.id.clone(),
            },
        ];
        assert_eq!(offered, theirs);
        // `legalActions` lists exactly those answers, and the reducer refuses another.
        let answers: Vec<Vec<Selection>> = legal_actions(&played.state, P2)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Answer { selection, .. } => Some(selection),
                _ => None,
            })
            .collect();
        assert_eq!(
            answers,
            theirs.iter().map(|pick| vec![pick.clone()]).collect::<Vec<_>>()
        );
        let refused = reduce(
            &played.state,
            &json_as(json!({
                "type": "answer",
                "choiceId": pending.id,
                "selection": [{ "pick": "instance", "instanceId": mine.id }],
                "playerId": "p2",
                "nonce": "coc-refused",
            })),
        );
        assert!(refused.error.is_some());
        let answered = act_ok(
            &played.state,
            json!({
                "type": "answer",
                "choiceId": pending.id,
                "selection": [{ "pick": "instance", "instanceId": first.id }],
                "playerId": "p2",
            }),
        );
        let state = answered.state;
        assert_eq!(
            find_instance(&state, &first.id).expect("p2's Unit").buffs,
            AttackHealth {
                attack: BOOST,
                health: BOOST
            }
        );
        assert_eq!(
            find_instance(&state, &mine.id).expect("p1's Unit").buffs.attack,
            0
        );
        let spell = the_boost(&state).expect("the Boost in a graveyard");
        assert_eq!(spell.owner, P2);
        assert!(state.players.p2.graveyard.iter().any(|card| card.id == spell.id));
    }

    #[test]
    fn r1850004_an_enemy_target_or_none_does_not_fire_it() {
        let mut state = guarded("enemy", false);
        let theirs = put(&mut state, &cheap().id, slot(P2, Row::Units, 1), json!({}));
        let played = boost_on(&state, &theirs.id);
        assert!(events_of_type(&played.events, GameEventType::TrapFired).is_empty());
        assert_eq!(
            find_instance(&played.state, &theirs.id)
                .expect("p2's Unit")
                .buffs
                .attack,
            BOOST
        );

        let played = play(&played.state, P1, &bolt().id, false, json!({}));
        assert!(events_of_type(&played.events, GameEventType::TrapFired).is_empty());
        assert_eq!(the_boost(&played.state).map(|card| card.owner), Some(P1));
    }

    #[test]
    fn r1850004_the_radiant_face_casts_it_radiant() {
        let mut state = guarded("radiant", true);
        let mine = put(&mut state, &cheap().id, slot(P1, Row::Units, 1), json!({}));
        let theirs = put(&mut state, &dear().id, slot(P2, Row::Units, 1), json!({}));
        let played = boost_on(&state, &mine.id);
        let pending = played.state.pending.clone().expect("the cast's target prompt");
        let answered = act_ok(
            &played.state,
            json!({
                "type": "answer",
                "choiceId": pending.id,
                "selection": [{ "pick": "instance", "instanceId": theirs.id }],
                "playerId": "p2",
            }),
        );
        assert_eq!(
            find_instance(&answered.state, &theirs.id)
                .expect("p2's Unit")
                .buffs
                .attack,
            RADIANT_BOOST
        );
        assert!(the_boost(&answered.state).expect("in a graveyard").radiant);
    }

    #[test]
    fn r1850004_no_own_side_target_passes_the_pick() {
        let mut state = guarded("none", false);
        let mine = put(&mut state, &cheap().id, slot(P1, Row::Units, 1), json!({}));
        let played = boost_on(&state, &mine.id);
        assert_eq!(events_of_type(&played.events, GameEventType::TrapFired).len(), 1);
        // p2 has no Unit: the declaration is passed, and the Spell resolves into nothing.
        assert_eq!(played.state.pending, None);
        assert_eq!(
            find_instance(&played.state, &mine.id)
                .expect("p1's Unit")
                .buffs
                .attack,
            0
        );
        assert!(
            played
                .state
                .players
                .p2
                .graveyard
                .iter()
                .any(|card| card.def_id == boost().id)
        );
    }
}

mod r1850005_radiant_at_n_keywords {
    use super::*;

    fn grant(state: &mut GameState, id: &str, keyword: Value) -> Vec<GameEvent> {
        apply(
            state,
            None,
            P1,
            effects::grant_keyword(json_as(json!({ "target": instance(id), "keyword": keyword }))),
        )
    }

    #[test]
    fn r1850005_n_keywords_make_it_radiant_once() {
        let mut state = playing("rod");
        let card = put(&mut state, &rod().id, slot(P1, Row::Units, 1), json!({}));
        assert!(events_of_type(&check(&mut state), GameEventType::RadiantSet).is_empty());
        // A kind it has already counts once.
        grant(&mut state, &card.id, json!({ "kind": "Taunt" }));
        assert!(!find_instance(&state, &card.id).expect("the rod").radiant);
        let events = grant(&mut state, &card.id, json!({ "kind": "Divine Shield" }));
        assert_eq!(events_of_type(&events, GameEventType::RadiantSet).len(), 1);
        assert!(find_instance(&state, &card.id).expect("the rod").radiant);
        let events = grant(&mut state, &card.id, json!({ "kind": "Lifesteal" }));
        assert!(events_of_type(&events, GameEventType::RadiantSet).is_empty());
    }

    #[test]
    fn r1850005_a_dormant_card_is_not_made_radiant_until_it_awakens() {
        let mut state = playing("rod-dormant");
        let card = put(&mut state, &rod().id, slot(P1, Row::Units, 1), json!({}));
        set_dormant(&mut state, &card.id, true);
        find_instance_mut(&mut state, &card.id)
            .expect("the rod")
            .granted_keywords
            .push(Keyword::DivineShield);
        // R1839000: its text is off while it is Dormant, so its keywords reach nothing.
        assert!(events_of_type(&check(&mut state), GameEventType::RadiantSet).is_empty());
        assert!(!find_instance(&state, &card.id).expect("the rod").radiant);
        set_dormant(&mut state, &card.id, false);
        assert_eq!(
            events_of_type(&check(&mut state), GameEventType::RadiantSet).len(),
            1
        );
    }

    #[test]
    fn r1850005_a_field_spell_counts_its_keywords() {
        let mut state = playing("rod-field");
        let card = put(&mut state, &rod_field().id, slot(P1, Row::Backrow, 1), json!({}));
        check(&mut state);
        assert!(!find_instance(&state, &card.id).expect("the rod").radiant);
        let events = grant(&mut state, &card.id, json!({ "kind": "Indestructible" }));
        assert_eq!(events_of_type(&events, GameEventType::RadiantSet).len(), 1);
        assert!(find_instance(&state, &card.id).expect("the rod").radiant);
    }
}
