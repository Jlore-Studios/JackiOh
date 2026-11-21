//! Targeting and the keywords, since Stinkyscape+ Balance Changes V (issue #4353). Evasive's targeting
//! half (R4353002): while in play, Spells and Field Spells can't target it. Targeting is R450's choosing,
//! so no declaration and no `target` prompt of theirs offers an Evasive card in play, its controller's
//! own Spells included, while a Unit's choice, a Trap, a Field Trap and a source that is no card still
//! may. A face-down card's Evasive is dormant. Immutable bars no targeting any more (R4353001, which
//! supersedes R781001): it stops the change, never the aim. And every Indestructible card is Immutable
//! innately (R4353000).

use jackioh_engine::restrictions::{evasive_bars, evasive_in_play, immutable_in_play, targeting_type_of};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{ids, in_hand, new_game, next_nonce, put, slot};
use crate::rules::fixtures::play_pipeline_a::{PA, with_play_a};

fn game(seed: &str) -> GameState {
    let mut ready = begin_game(&with_play_a(new_game(seed, None))).state;
    for player in [PlayerId::P1, PlayerId::P2] {
        let keep = ids(&ready.players[player].hand);
        ready = reduce_ok(&ready, player, json!({ "type": "mulligan", "keep": keep })).0;
    }
    for player in [PlayerId::P1, PlayerId::P2] {
        ready.players[player].mana.current = 8;
        ready.players[player].mana.max = 8;
        ready.players[player].hand = vec![];
    }
    ready
}

fn attempt(state: &GameState, player_id: PlayerId, body: Value) -> ReduceResult {
    reduce(state, &Action::new(json_as(body), player_id, next_nonce()))
}

fn reduce_ok(state: &GameState, player_id: PlayerId, body: Value) -> (GameState, Vec<GameEvent>) {
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

/// Gives the card `keyword`, as an aura or a grant would, and returns it as it stands.
fn grant(state: &mut GameState, card: &CardInstance, keyword: Keyword) -> CardInstance {
    let live = find_instance_mut(state, &card.id).expect("the card is in the state");
    live.granted_keywords.push(keyword);
    live.clone()
}

/// A unit standing in `lane` of `player`'s unit row, given `keyword`.
fn unit_with(state: &mut GameState, player: PlayerId, lane: i32, keyword: Keyword) -> CardInstance {
    let unit = put(state, "fx-1", slot(player, Row::Units, lane), json!({}));
    grant(state, &unit, keyword)
}

fn top(pile: &Option<Pile>) -> Option<&CardInstance> {
    pile.as_ref().and_then(|pile| pile.first())
}

mod r4353002_spells_and_field_spells_cant_target_an_evasive_card_in_play {
    use super::*;

    #[test]
    fn r4353002_a_spells_declaration_never_offers_it_and_a_units_cry_does() {
        let mut state = game("r4353002-declared");
        let ghost = unit_with(&mut state, PlayerId::P2, 1, Keyword::Evasive);
        let other = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 2), json!({}));
        let bolt = one(&mut state, PlayerId::P1, &PA.bolt.id);
        let zapper = one(&mut state, PlayerId::P1, &PA.zapper.id);

        let spell_picks = offered(&state, PlayerId::P1, &bolt);
        assert!(!spell_picks.contains(&at(&ghost)));
        assert!(spell_picks.contains(&at(&other)));
        assert!(offered(&state, PlayerId::P1, &zapper).contains(&at(&ghost)));
        // The reducer refuses what `legal_actions` never offers: one rule (§10.2).
        assert!(
            attempt(
                &state,
                PlayerId::P1,
                json!({ "type": "play", "instanceId": bolt.id, "targets": [at(&ghost)] })
            )
            .error
            .is_some()
        );
        let listed = legal_actions(&state, PlayerId::P1);
        let names_ghost = |action: &ActionBody| {
            serde_json::to_string(action)
                .is_ok_and(|text| text.contains(&bolt.id) && text.contains(&ghost.id))
        };
        assert!(!listed.iter().any(names_ghost));
        let (after, _) = reduce_ok(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": zapper.id, "targets": [at(&ghost)] }),
        );
        assert_eq!(top(&after.players.p2.units[0]).map(|card| card.damage), Some(1));
    }

    #[test]
    fn r4353002_its_controllers_own_spell_cannot_target_it_either() {
        let mut state = game("r4353002-own");
        let ghost = unit_with(&mut state, PlayerId::P1, 1, Keyword::Evasive);
        let bolt = one(&mut state, PlayerId::P1, &PA.bolt.id);

        assert!(!offered(&state, PlayerId::P1, &bolt).contains(&at(&ghost)));
    }

    #[test]
    fn r4353002_a_spells_target_prompt_never_offers_it() {
        let mut state = game("r4353002-prompt");
        let ghost = unit_with(&mut state, PlayerId::P2, 1, Keyword::Evasive);
        let other = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 2), json!({}));
        let chooser = one(&mut state, PlayerId::P1, &PA.chooser.id);

        let (asked, _) = reduce_ok(
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
        assert!(!picks.contains(&at(&ghost)));
    }

    #[test]
    fn r4353002_a_spell_whose_only_would_be_target_is_evasive_offers_no_unit() {
        let mut state = game("r4353002-only");
        let ghost = unit_with(&mut state, PlayerId::P2, 1, Keyword::Evasive);
        let bolt = one(&mut state, PlayerId::P1, &PA.bolt.id);

        // Only the heroes are left to the bolt, which may name a unit or a hero.
        let picks = offered(&state, PlayerId::P1, &bolt);
        assert!(!picks.contains(&at(&ghost)));
        assert!(
            picks
                .iter()
                .all(|pick| !matches!(pick, Selection::Instance { .. }))
        );
    }

    #[test]
    fn r4353002_spells_and_field_spells_are_barred_and_units_traps_field_traps_and_no_card_are_not() {
        let mut state = game("r4353002-sources");
        let ghost = unit_with(&mut state, PlayerId::P2, 1, Keyword::Evasive);
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

        assert!(evasive_bars(&state, Some(&spell), &ghost));
        assert!(evasive_bars(&state, Some(&field_spell), &ghost));
        assert!(!evasive_bars(&state, Some(&field_trap), &ghost));
        assert!(!evasive_bars(&state, Some(&trap), &ghost));
        assert!(!evasive_bars(&state, Some(&unit), &ghost));
        assert!(!evasive_bars(&state, None, &ghost));

        // An Animated card standing in a unit zone is a Unit, whatever it prints.
        let mut animated = field_spell.clone();
        animated.zone = Zone::Field {
            player: PlayerId::P1,
            row: Row::Units,
            lane: 4,
        };
        assert_eq!(targeting_type_of(&state, &animated), CardType::Unit);
        assert!(!evasive_bars(&state, Some(&animated), &ghost));
        // A Spell set face-down still reads as the Spell it prints (ME-ALTPLAY).
        assert_eq!(targeting_type_of(&state, &spell), CardType::Spell);
    }

    #[test]
    fn r4353002_out_of_play_or_face_down_it_protects_nothing_from_targeting() {
        let mut state = game("r4353002-where");
        let held = one(&mut state, PlayerId::P2, "fx-1");
        let held = grant(&mut state, &held, Keyword::Evasive);
        let spell = one(&mut state, PlayerId::P1, &PA.bolt.id);
        assert!(!evasive_in_play(&state, &held));
        assert!(!evasive_bars(&state, Some(&spell), &held));

        // A face-down card's text is dormant: its Evasive would tell the opponent what it is.
        let set = put(
            &mut state,
            &PA.hidden_trap.id,
            slot(PlayerId::P2, Row::Backrow, 1),
            json!({}),
        );
        let set = grant(&mut state, &set, Keyword::Evasive);
        assert!(!evasive_in_play(&state, &set));
        find_instance_mut(&mut state, &set.id)
            .expect("the trap is set")
            .face_up = Some(true);
        let revealed = find_instance(&state, &set.id).cloned().expect("the trap is set");
        assert!(evasive_in_play(&state, &revealed));
        assert!(evasive_bars(&state, Some(&spell), &revealed));
    }

    #[test]
    fn r4353002_without_evasive_nothing_changes() {
        let mut state = game("r4353002-plain");
        let plain = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 1), json!({}));
        let bolt = one(&mut state, PlayerId::P1, &PA.bolt.id);
        assert!(offered(&state, PlayerId::P1, &bolt).contains(&at(&plain)));
        assert!(!evasive_bars(&state, Some(&bolt), &plain));
    }
}

mod r4353001_immutable_bars_no_targeting {
    use super::*;

    #[test]
    fn r4353001_r781001_a_spell_may_target_an_immutable_card_in_play_and_its_damage_lands() {
        let mut state = game("r4353001-spell");
        let wall = unit_with(&mut state, PlayerId::P2, 1, Keyword::Immutable);
        let bolt = one(&mut state, PlayerId::P1, &PA.bolt.id);

        assert!(immutable_in_play(&state, &wall));
        assert!(offered(&state, PlayerId::P1, &bolt).contains(&at(&wall)));
        assert!(!evasive_bars(&state, Some(&bolt), &wall));
        let (after, _) = reduce_ok(
            &state,
            PlayerId::P1,
            json!({ "type": "play", "instanceId": bolt.id, "targets": [at(&wall)] }),
        );
        // Immutable stops a change to the card's text, never damage: the bolt's 2 kill the 2/2 wall.
        assert!(top(&after.players.p2.units[0]).is_none());
    }

    #[test]
    fn r4353001_r781002_field_spells_and_field_traps_may_target_it_too() {
        let mut state = game("r4353001-sources");
        let wall = unit_with(&mut state, PlayerId::P2, 1, Keyword::Immutable);
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
        assert!(!evasive_bars(&state, Some(&field_spell), &wall));
        assert!(!evasive_bars(&state, Some(&field_trap), &wall));
    }
}

mod r4353000_an_indestructible_card_is_immutable_innately {
    use super::*;

    #[test]
    fn r4353000_an_indestructible_card_in_play_is_immutable_in_play_and_still_targetable() {
        let mut state = game("r4353000-in-play");
        let rock = unit_with(&mut state, PlayerId::P2, 1, Keyword::Indestructible);
        let plain = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 2), json!({}));
        let bolt = one(&mut state, PlayerId::P1, &PA.bolt.id);

        assert!(immutable_in_play(&state, &rock));
        assert!(!immutable_in_play(&state, &plain));
        // Being Immutable bars no aim (R4353001).
        assert!(offered(&state, PlayerId::P1, &bolt).contains(&at(&rock)));
    }

    #[test]
    fn r4353000_an_indestructible_field_spell_is_not_immutable() {
        let mut state = game("r4353000-field-spell");
        let field = put(
            &mut state,
            &PA.field.id,
            slot(PlayerId::P2, Row::Backrow, 1),
            json!({}),
        );
        let field = grant(&mut state, &field, Keyword::Indestructible);
        assert!(!immutable_in_play(&state, &field));
        assert!(!jackioh_engine::layers::is_immutable(&state, &field));
        // The same Field Spell given the keyword itself is Immutable.
        let field = grant(&mut state, &field, Keyword::Immutable);
        assert!(immutable_in_play(&state, &field));
    }
}
