//! Immutable's targeting half (R781001): while in play, Spells, Field Spells and Field Traps can't
//! target it. Targeting is R450's choosing, so no declaration and no `target` prompt of theirs offers
//! an Immutable card in play, its controller's own Spells included, while a Unit's choice, a Trap that
//! is not a Field Trap and a source that is no card still may. A face-down card's Immutable is dormant,
//! and a card out of play is protected by its text half alone (R23, R386).

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::restrictions::{immutable_bars, immutable_in_play, targeting_type_of};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{in_hand, new_game, put, slot};
use crate::rules::fixtures::play_pipeline_a::{PA, with_play_a};

static NONCE: AtomicU32 = AtomicU32::new(0);

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn game(seed: &str) -> GameState {
    let mut ready = begin_game(&with_play_a(new_game(seed, None))).state;
    for player in [PlayerId::P1, PlayerId::P2] {
        let keep = ids(&ready.players[player].hand);
        ready = must(&ready, player, json!({ "type": "mulligan", "keep": keep })).0;
    }
    for player in [PlayerId::P1, PlayerId::P2] {
        ready.players[player].mana.current = 8;
        ready.players[player].mana.max = 8;
        ready.players[player].hand = vec![];
    }
    ready
}

fn attempt(state: &GameState, player_id: PlayerId, body: Value) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    reduce(
        state,
        &Action::new(json_as(body), player_id, format!("im-{nonce}")),
    )
}

fn must(state: &GameState, player_id: PlayerId, body: Value) -> (GameState, Vec<GameEvent>) {
    let result = attempt(state, player_id, body);
    if let Some(error) = result.error {
        panic!("{error}");
    }
    (result.state, result.events)
}

fn one(state: &mut GameState, player: PlayerId, def_id: &str) -> CardInstance {
    in_hand(state, def_id, player, 1)
        .into_iter()
        .next()
        .expect("no card")
}

fn at(card: &CardInstance) -> Selection {
    Selection::Instance {
        instance_id: card.id.clone(),
    }
}

fn offered(state: &GameState, player: PlayerId, card: &CardInstance) -> Vec<Selection> {
    let decl = declared_targets(state, card)
        .into_iter()
        .next()
        .expect("no declaration");
    legal_selections_for(state, player, card, &decl)
}

/// Gives the card Immutable, as an aura or a grant would, and returns it as it stands.
fn make_immutable(state: &mut GameState, card: &CardInstance) -> CardInstance {
    let live = find_instance_mut(state, &card.id).expect("the card is in the state");
    live.granted_keywords.push(Keyword::Immutable);
    live.clone()
}

/// A unit standing in `lane` of `player`'s unit row, made Immutable.
fn immutable_unit(state: &mut GameState, player: PlayerId, lane: i32) -> CardInstance {
    let unit = put(state, "fx-1", slot(player, Row::Units, lane), json!({}));
    make_immutable(state, &unit)
}

fn top(pile: &Option<Pile>) -> Option<&CardInstance> {
    pile.as_ref().and_then(|pile| pile.first())
}

mod r781001_spells_field_spells_and_field_traps_cant_target_it_in_play {
    use super::*;

    #[test]
    fn r781001_a_spells_declaration_never_offers_it_and_a_units_cry_does() {
        let mut state = game("r781001-declared");
        let wall = immutable_unit(&mut state, PlayerId::P2, 1);
        let other = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 2), json!({}));
        let bolt = one(&mut state, PlayerId::P1, &PA.bolt.id);
        let zapper = one(&mut state, PlayerId::P1, &PA.zapper.id);

        let spell_picks = offered(&state, PlayerId::P1, &bolt);
        assert!(!spell_picks.contains(&at(&wall)));
        assert!(spell_picks.contains(&at(&other)));
        assert!(offered(&state, PlayerId::P1, &zapper).contains(&at(&wall)));
        // The reducer refuses what `legal_actions` never offers: one rule (§10.2).
        assert!(
            attempt(
                &state,
                PlayerId::P1,
                json!({ "type": "play", "instanceId": bolt.id, "targets": [at(&wall)] })
            )
            .error
            .is_some()
        );
        let listed = legal_actions(&state, PlayerId::P1);
        let names_wall = |action: &ActionBody| {
            serde_json::to_string(action).is_ok_and(|text| text.contains(&bolt.id) && text.contains(&wall.id))
        };
        assert!(!listed.iter().any(names_wall));
        let (after, _) = must(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": zapper.id, "targets": [at(&wall)] }),
        );
        assert_eq!(top(&after.players.p2.units[0]).map(|card| card.damage), Some(1));
    }

    #[test]
    fn r781001_its_controllers_own_spell_cannot_target_it_either() {
        let mut state = game("r781001-own");
        let wall = immutable_unit(&mut state, PlayerId::P1, 1);
        let bolt = one(&mut state, PlayerId::P1, &PA.bolt.id);

        assert!(!offered(&state, PlayerId::P1, &bolt).contains(&at(&wall)));
    }

    #[test]
    fn r781001_a_spells_target_prompt_never_offers_it() {
        let mut state = game("r781001-prompt");
        let wall = immutable_unit(&mut state, PlayerId::P2, 1);
        let other = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 2), json!({}));
        let chooser = one(&mut state, PlayerId::P1, &PA.chooser.id);

        let (asked, _) = must(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": chooser.id }),
        );
        let picks: Vec<Selection> = asked
            .pending
            .as_ref()
            .map(|pending| {
                pending
                    .options
                    .iter()
                    .map(|option| option.selection.clone())
                    .collect()
            })
            .unwrap_or_default();
        assert!(picks.contains(&at(&other)));
        assert!(!picks.contains(&at(&wall)));
    }

    #[test]
    fn r781001_a_spell_whose_only_would_be_target_is_immutable_offers_no_pick() {
        let mut state = game("r781001-only");
        let wall = immutable_unit(&mut state, PlayerId::P2, 1);
        let bolt = one(&mut state, PlayerId::P1, &PA.bolt.id);

        // Only the heroes are left to the bolt, which may name a unit or a hero.
        let picks = offered(&state, PlayerId::P1, &bolt);
        assert!(!picks.contains(&at(&wall)));
        assert!(
            picks
                .iter()
                .all(|pick| !matches!(pick, Selection::Instance { .. }))
        );
    }

    #[test]
    fn r781001_spells_field_spells_and_field_traps_are_barred_and_units_traps_and_no_card_are_not() {
        let mut state = game("r781001-sources");
        let wall = immutable_unit(&mut state, PlayerId::P2, 1);
        let spell = one(&mut state, PlayerId::P1, &PA.bolt.id);
        let unit = one(&mut state, PlayerId::P1, &PA.zapper.id);
        let field_spell = put(
            &mut state,
            &PA.field.id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        let field_trap = put(
            &mut state,
            &PA.hidden_field_trap.id,
            slot(PlayerId::P1, Row::Backrow, 2),
            json!({}),
        );
        let trap = put(
            &mut state,
            &PA.hidden_trap.id,
            slot(PlayerId::P1, Row::Backrow, 3),
            json!({}),
        );

        assert!(immutable_bars(&state, Some(&spell), &wall));
        assert!(immutable_bars(&state, Some(&field_spell), &wall));
        assert!(immutable_bars(&state, Some(&field_trap), &wall));
        assert!(!immutable_bars(&state, Some(&trap), &wall));
        assert!(!immutable_bars(&state, Some(&unit), &wall));
        assert!(!immutable_bars(&state, None, &wall));

        // An Animated card standing in a unit zone is a Unit, whatever it prints.
        let mut animated = field_trap.clone();
        animated.zone = Zone::Field {
            player: PlayerId::P1,
            row: Row::Units,
            lane: 4,
        };
        assert_eq!(targeting_type_of(&state, &animated), CardType::Unit);
        assert!(!immutable_bars(&state, Some(&animated), &wall));
        // A Spell set face-down still reads as the Spell it prints (ME-ALTPLAY).
        assert_eq!(targeting_type_of(&state, &spell), CardType::Spell);
    }

    #[test]
    fn r781001_out_of_play_or_face_down_it_protects_nothing_from_targeting() {
        let mut state = game("r781001-where");
        let held = one(&mut state, PlayerId::P2, "fx-1");
        let held = make_immutable(&mut state, &held);
        let spell = one(&mut state, PlayerId::P1, &PA.bolt.id);
        assert!(!immutable_in_play(&state, &held));
        assert!(!immutable_bars(&state, Some(&spell), &held));

        // A face-down Trap's text is dormant: its Immutable would tell the opponent what it is.
        let set = put(
            &mut state,
            &PA.hidden_trap.id,
            slot(PlayerId::P2, Row::Backrow, 1),
            json!({}),
        );
        let set = make_immutable(&mut state, &set);
        assert!(!immutable_in_play(&state, &set));
        find_instance_mut(&mut state, &set.id)
            .expect("the trap is set")
            .face_up = Some(true);
        let fired = find_instance(&state, &set.id).cloned().expect("the trap is set");
        assert!(immutable_in_play(&state, &fired));
        assert!(immutable_bars(&state, Some(&spell), &fired));
    }

    #[test]
    fn r781001_without_immutable_nothing_changes() {
        let mut state = game("r781001-plain");
        let plain = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 1), json!({}));
        let bolt = one(&mut state, PlayerId::P1, &PA.bolt.id);
        assert!(offered(&state, PlayerId::P1, &bolt).contains(&at(&plain)));
        assert!(!immutable_bars(&state, Some(&bolt), &plain));
    }
}
