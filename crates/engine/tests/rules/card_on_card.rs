//! The card-on-card verbs (#1850, R1850000–R1850005) through the fixtures in
//! `fixtures/card_on_card.rs`: copying a Unit's text, Duo and Squad, exile auras, holding a permanent
//! beneath a card until it leaves, a beneficial Spell cast for yourself instead, and Radiant on reaching
//! N keywords. Crater on a Field Spell (R1850006) is in `crater.rs`, with the rest of Crater.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::reduce::{begin_game, legal_actions, reduce};
use jackioh_engine::testkit::*;
use jackioh_engine::view_for::view_for;
use jackioh_engine::wire::PlayerId::{P1, P2};

use super::fixtures::card_on_card::{
    BOOST, LEGEND_PING, LEGEND_RADIANT_PING, RADIANT_BOOST, bolt, boost, cheap, cheap_exiler, cheap_field,
    cheap_trap, copier, crier, dear, duo, human, immutable_copier, legend, radiant_copier,
    register_card_on_card, rod, rod_field, sack, sacker, tag_exiler, ticket, token, wormhole,
};
use super::fixtures::harness::{events_of_type, in_hand, new_game, put, slot};

/// Every action this file sends gets a fresh nonce.
static NONCE: AtomicU32 = AtomicU32::new(0);

/// `reduce` with a fresh nonce, refused actions panicking with their reason.
fn act(state: &GameState, body: Value) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::Relaxed) + 1;
    let mut action = body;
    action["nonce"] = json!(format!("coc{nonce}"));
    let action: Action = json_as(action);
    let result = reduce(state, &action);
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result
}

/// p1's first main phase, both mulligans kept, ten mana a side and #1850's cards registered.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(&format!("coc-{seed}"), None)).state;
    register_card_on_card();
    for player in [P1, P2] {
        let keep: Vec<String> = state.players[player]
            .hand
            .iter()
            .map(|card| card.id.clone())
            .collect();
        state = act(
            &state,
            json!({ "type": "mulligan", "keep": keep, "playerId": player }),
        )
        .state;
    }
    for player in [P1, P2] {
        state.players[player].auto_end_turn = Some(false);
        state.players[player].mana.current = 10;
        state.players[player].mana.max = 10;
    }
    state
}

/// Put `def_id` in `player`'s hand (Radiant when `radiant`) and play it, `extra` merged into the action.
fn play(state: &GameState, player: PlayerId, def_id: &str, radiant: bool, extra: Value) -> ReduceResult {
    let mut state = state.clone();
    let mut card = in_hand(&mut state, def_id, player, 1)
        .into_iter()
        .next()
        .expect("a card to play");
    if radiant {
        card.radiant = true;
        if let Some(live) = find_instance_mut(&mut state, &card.id) {
            live.radiant = true;
        }
    }
    let mut body = json!({ "type": "play", "instanceId": card.id, "playerId": player });
    if let (Some(body), Some(extra)) = (body.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            body.insert(key.clone(), value.clone());
        }
    }
    act(&state, body)
}

/// A play's zone and its one target, as the action carries them.
fn at(row: &str, lane: i32, target: &str) -> Value {
    json!({
        "zone": { "row": row, "lane": lane },
        "targets": [{ "pick": "instance", "instanceId": target }],
    })
}

/// End the active player's turn.
fn pass(state: &GameState) -> ReduceResult {
    act(state, json!({ "type": "endTurn", "playerId": state.active }))
}

/// Apply one effect as `self_id`'s (none: no card) for `controller`, then settle, as a resolving
/// card's would. The events it made.
fn apply(
    state: &mut GameState,
    self_id: Option<&str>,
    controller: PlayerId,
    effect: Effect,
) -> Vec<GameEvent> {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed.clone(), state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        {
            let me = self_id.and_then(|id| find_instance(sink.state, id).cloned());
            let mut ctx = make_context(
                &mut sink,
                me.as_ref(),
                HookOptions {
                    controller: Some(controller),
                    ..Default::default()
                },
            );
            (effect.apply)(&mut ctx);
        }
        settle(&mut sink, SettleOptions::default());
    }
    state.rng_cursor = rng.cursor();
    events
}

/// The board settled with nothing applied: a state check, as after any effect.
fn check(state: &mut GameState) -> Vec<GameEvent> {
    apply(
        state,
        None,
        P1,
        effects::damage(json_as(json!({ "to": { "of": "selfHero" }, "amount": 0 }))),
    )
}

fn instance(id: &str) -> Value {
    json!({ "of": "instance", "instanceId": id })
}

fn destroy(state: &mut GameState, id: &str) -> Vec<GameEvent> {
    apply(
        state,
        None,
        P1,
        effects::destroy(json_as(json!({ "target": instance(id) }))),
    )
}

fn top(state: &GameState, player: PlayerId, lane: i32) -> Option<CardInstance> {
    state.players[player]
        .units
        .get((lane - 1) as usize)?
        .as_ref()?
        .first()
        .cloned()
}

fn on_field(state: &GameState, id: &str) -> bool {
    find_instance(state, id).is_some_and(|card| matches!(card.zone, Zone::Field { .. }))
}

fn has(state: &GameState, id: &str, kind: KeywordKind) -> bool {
    let card = find_instance(state, id).cloned().expect("the card");
    unit_has(state, &card, kind)
}

mod r1850000_copy_a_units_text {
    use super::*;

    #[test]
    fn r1850000_has_the_units_keywords_aura_and_triggers_on_its_own_stats() {
        let mut state = playing("copy");
        let legend = put(&mut state, &legend().id, slot(P1, Row::Units, 1), json!({}));
        let plain = put(&mut state, &cheap().id, slot(P1, Row::Units, 3), json!({}));
        let played = play(&state, P1, &copier().id, false, at("units", 2, &legend.id));
        let state = played.state;
        let me = top(&state, P1, 2).expect("the copier in lane 2");
        // Reported as a Vanilla is: an in-place `transformed`, its definition unchanged.
        assert!(played.events.iter().any(|event| matches!(
            event,
            GameEvent::Transformed { instance_id, new_instance_id, from_def_id, to_def_id, .. }
                if *instance_id == me.id && *new_instance_id == me.id && from_def_id == to_def_id
        )));
        assert_eq!(me.def_id, copier().id);
        // The Legendary's Taunt, on the copier's own 2/2 (+1 from the Legendary's aura).
        assert!(has(&state, &me.id, KeywordKind::Taunt));
        let view = unit_view(&state, &me);
        assert_eq!((view.attack, view.max_health), (3, 2));
        // Its copied aura lends +1 to its other Units, the Legendary among them.
        assert_eq!(unit_view(&state, &plain).attack, 1 + 2);
        assert_eq!(unit_view(&state, &legend).attack, 3 + 1);
        // Both "whenever you play a card" triggers answer the next play.
        let before = state.players.p2.hero.health;
        let state = play(
            &state,
            P1,
            &cheap().id,
            false,
            json!({ "zone": { "row": "units", "lane": 4 } }),
        )
        .state;
        assert_eq!(state.players.p2.hero.health, before - 2 * LEGEND_PING);
        // The copied Activate is the copier's to use.
        let before = state.players.p2.hero.health;
        let state = act(
            &state,
            json!({ "type": "activate", "instanceId": me.id, "ability": "ping", "playerId": "p1" }),
        )
        .state;
        assert_eq!(state.players.p2.hero.health, before - LEGEND_PING);
        // Both end-of-turn hooks deal their number.
        let before = state.players.p2.hero.health;
        let ended = pass(&state);
        assert_eq!(ended.state.players.p2.hero.health, before - 2 * LEGEND_PING);
    }

    #[test]
    fn r1850000_a_copied_death_runs() {
        let mut state = playing("death");
        let legend = put(&mut state, &legend().id, slot(P1, Row::Units, 1), json!({}));
        let me = put(&mut state, &copier().id, slot(P1, Row::Units, 2), json!({}));
        apply(
            &mut state,
            Some(&me.id),
            P1,
            effects::copy_unit_text(json_as(json!({ "target": instance(&legend.id) }))),
        );
        let before = state.players.p2.hero.health;
        destroy(&mut state, &me.id);
        assert!(!on_field(&state, &me.id));
        assert_eq!(state.players.p2.hero.health, before - LEGEND_PING);
    }

    #[test]
    fn r1850000_the_radiant_option_copies_the_radiant_face() {
        let mut state = playing("radiant");
        let legend = put(&mut state, &legend().id, slot(P1, Row::Units, 1), json!({}));
        let played = play(
            &state,
            P1,
            &radiant_copier().id,
            false,
            at("units", 2, &legend.id),
        );
        let mut state = played.state;
        let me = top(&state, P1, 2).expect("the copier in lane 2");
        // The Radiant text's Lifesteal, on the copier's base face: it is no Radiant card.
        assert!(has(&state, &me.id, KeywordKind::Lifesteal));
        assert!(has(&state, &me.id, KeywordKind::Taunt));
        assert!(!me.radiant);
        assert_eq!(unit_view(&state, &me).max_health, 2);
        // Its Death reads the Radiant face's number.
        let before = state.players.p2.hero.health;
        destroy(&mut state, &me.id);
        assert_eq!(state.players.p2.hero.health, before - LEGEND_RADIANT_PING);
    }

    #[test]
    fn r1850000_an_immutable_copier_copies_nothing() {
        let mut state = playing("immutable");
        let legend = put(&mut state, &legend().id, slot(P1, Row::Units, 1), json!({}));
        let me = put(
            &mut state,
            &immutable_copier().id,
            slot(P1, Row::Units, 2),
            json!({}),
        );
        let events = apply(
            &mut state,
            Some(&me.id),
            P1,
            effects::copy_unit_text(json_as(json!({ "target": instance(&legend.id) }))),
        );
        assert!(events_of_type(&events, GameEventType::Transformed).is_empty());
        assert!(!has(&state, &me.id, KeywordKind::Taunt));

        // Nor does a copier copy a Vanilla Unit, which has no text, or itself.
        let other = put(&mut state, &copier().id, slot(P1, Row::Units, 3), json!({}));
        apply(
            &mut state,
            None,
            P1,
            effects::vanilla(json_as(json!({ "target": instance(&legend.id) }))),
        );
        for target in [&legend.id, &other.id] {
            let events = apply(
                &mut state,
                Some(&other.id),
                P1,
                effects::copy_unit_text(json_as(json!({ "target": instance(target) }))),
            );
            assert!(events_of_type(&events, GameEventType::Transformed).is_empty());
        }
        assert!(!has(&state, &other.id, KeywordKind::Taunt));
    }

    #[test]
    fn r1850000_leaving_the_field_ends_it() {
        let mut state = playing("leaves");
        let legend = put(&mut state, &legend().id, slot(P1, Row::Units, 1), json!({}));
        let plain = put(&mut state, &cheap().id, slot(P1, Row::Units, 3), json!({}));
        let me = put(&mut state, &copier().id, slot(P1, Row::Units, 2), json!({}));
        apply(
            &mut state,
            Some(&me.id),
            P1,
            effects::copy_unit_text(json_as(json!({ "target": instance(&legend.id) }))),
        );
        assert_eq!(unit_view(&state, &plain).attack, 3);
        apply(
            &mut state,
            None,
            P1,
            effects::bounce(json_as(json!({ "target": instance(&me.id) }))),
        );
        let held = state
            .players
            .p1
            .hand
            .iter()
            .find(|card| card.id == me.id)
            .cloned()
            .expect("the copier back in hand");
        assert!(held.memory.is_empty());
        assert!(!unit_has(&state, &held, KeywordKind::Taunt));
        // Its copied aura went with it.
        assert_eq!(unit_view(&state, &plain).attack, 2);
    }
}

mod r1850001_duo_and_squad {
    use super::*;

    /// The marks `viewer` reads on the card `id`, wherever on the field it is.
    fn marks_in_view(state: &GameState, viewer: PlayerId, id: &str) -> Vec<(String, String)> {
        let view = serde_json::to_value(view_for(state, viewer)).expect("a view serialises");
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
        // Not at the start of the other player's turn: at its marker's next.
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
        // Lane order, then the backrow: a face-up Radiant card (the Squad itself, `shining`) is passed
        // by, a face-down one is marked whatever it wears, and the enemy's are not the player's.
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

    /// p1's ticket in backrow lane 1, holding p2's card `held`; the ticket's id.
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

mod r1850004_cast_for_yourself {
    use super::*;

    /// p1's main phase with p2's Wormhole set face-down in backrow lane 1 (Radiant when `radiant`).
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
        let answered = act(
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
        let answered = act(
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
