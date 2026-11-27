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

/// R5113000 (issue #5113): Immutable keeps another card's buffs, debuffs, keywords, auras and lends off a
/// card, an Indestructible Unit card's included, while the card's own text still changes it.
mod r5113000_immutable_takes_no_other_cards_buff_grant_aura_or_lend {
    use jackioh_engine::effects::{
        buff, buff_this_turn, grant_keyword, grant_keywords, grant_random_keywords, set_attack_this_turn,
    };

    use super::*;
    use crate::rules::fixtures::aura_deaths::aura_board;
    use crate::rules::fixtures::combat::big_body;
    use crate::rules::fixtures::harness::{live, run_with_targets};

    /// Each change R5113000 keeps off, aimed at the chosen card: a buff, a debuff, a keyword, random
    /// keywords, a buff this turn and a set Attack this turn.
    fn changes() -> Vec<Effect> {
        vec![
            buff(json_as(
                json!({ "target": { "of": "chosen" }, "attack": 2, "health": 2 }),
            )),
            buff(json_as(
                json!({ "target": { "of": "chosen" }, "attack": -1, "health": -1 }),
            )),
            grant_keyword(json_as(
                json!({ "target": { "of": "chosen" }, "keyword": { "kind": "Taunt" } }),
            )),
            grant_random_keywords(json_as(json!({ "target": { "of": "chosen" }, "count": 2 }))),
            buff_this_turn(json_as(
                json!({ "target": { "of": "chosen" }, "attack": 3, "keywords": [{ "kind": "Rush" }] }),
            )),
            set_attack_this_turn(json_as(json!({ "target": { "of": "chosen" }, "attack": 7 }))),
        ]
    }

    /// Runs every change at `card`, as `source`'s (none: no card of its own); the events.
    fn change(state: &mut GameState, card: &CardInstance, source: Option<&CardInstance>) -> Vec<GameEvent> {
        let mut events = Vec::new();
        for effect in changes() {
            events.extend(run_with_targets(
                state,
                effect,
                source,
                vec![at(card)],
                PlayerId::P1,
            ));
        }
        events
    }

    /// Whether any event reports a buff or a keyword given to `card`.
    fn reports_change(events: &[GameEvent], card: &CardInstance) -> bool {
        events.iter().any(|event| match event {
            GameEvent::Buffed { instance_id, .. } | GameEvent::KeywordGranted { instance_id, .. } => {
                *instance_id == card.id
            }
            _ => false,
        })
    }

    /// `card` exactly as it was: no buff, no keyword but `kept`, nothing this turn.
    fn untouched(state: &GameState, card: &CardInstance, kept: &[Keyword]) {
        let now = live(state, card);
        assert_eq!(now.buffs, AttackHealth { attack: 0, health: 0 }, "{}", card.id);
        assert_eq!(now.granted_keywords, kept.to_vec(), "{}", card.id);
        assert!(now.turn_buff.is_none(), "{}", card.id);
        assert!(now.attack_this_turn.is_none(), "{}", card.id);
    }

    #[test]
    fn r5113000_another_cards_buff_debuff_and_keywords_pass_an_immutable_unit_by() {
        let mut state = game("r5113000-immutable");
        let wall = unit_with(&mut state, PlayerId::P2, 1, Keyword::Immutable);
        let plain = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 2), json!({}));
        let before = unit_view(&state, &wall);

        let events = change(&mut state, &wall, None);
        assert!(!reports_change(&events, &wall));
        untouched(&state, &wall, &[Keyword::Immutable]);
        let after = unit_view(&state, &live(&state, &wall));
        assert_eq!(
            (after.attack, after.max_health),
            (before.attack, before.max_health)
        );
        assert_eq!(after.keywords, before.keywords);

        // The same changes land on a card that is not Immutable.
        let events = change(&mut state, &plain, None);
        assert!(reports_change(&events, &plain));
        let now = live(&state, &plain);
        assert_eq!(now.buffs, AttackHealth { attack: 1, health: 1 });
        assert!(has_keyword(&now.granted_keywords, KeywordKind::Taunt));
        assert!(now.turn_buff.is_some() && now.attack_this_turn.is_some());
    }

    #[test]
    fn r5113000_r4353000_an_indestructible_unit_takes_none_either() {
        let mut state = game("r5113000-indestructible");
        let rock = unit_with(&mut state, PlayerId::P2, 1, Keyword::Indestructible);
        let events = change(&mut state, &rock, None);
        assert!(!reports_change(&events, &rock));
        untouched(&state, &rock, &[Keyword::Indestructible]);
    }

    #[test]
    fn r5113000_r23_its_own_text_still_changes_it() {
        let mut state = game("r5113000-own");
        let wall = unit_with(&mut state, PlayerId::P2, 1, Keyword::Immutable);
        let events = change(&mut state, &wall, Some(&wall));
        assert!(reports_change(&events, &wall));
        let now = live(&state, &wall);
        assert_eq!(now.buffs, AttackHealth { attack: 1, health: 1 });
        assert!(has_keyword(&now.granted_keywords, KeywordKind::Taunt));
    }

    #[test]
    fn r5113000_no_other_cards_aura_and_no_lend_reaches_it() {
        // p2's Charger (lane 3) gives p2's other Units +2 Health.
        let (mut state, _, _, _) = aura_board("r5113000-aura", &big_body.id, 3);
        let wall = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 2), json!({}));
        let wall = grant(&mut state, &wall, Keyword::Immutable);
        let plain = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 4), json!({}));
        let (walled, open) = (unit_view(&state, &wall), unit_view(&state, &plain));
        assert_eq!(
            open.max_health,
            walled.max_health + 2,
            "the aura reaches the plain Unit alone"
        );

        // "Your Units have Trample" and "your Units have +3 Attack" on p2's side.
        for (id, kind) in [
            (
                "r5113000-keyword",
                ModifierKind::UnitsHaveKeyword {
                    keyword: Keyword::Trample,
                    from_spell: None,
                },
            ),
            (
                "r5113000-attack",
                ModifierKind::UnitsAttack {
                    amount: 3,
                    from_spell: None,
                },
            ),
        ] {
            state.players[PlayerId::P2].mods.push(PlayerModifier {
                id: id.to_string(),
                expiry: ModifierExpiry::Never,
                kind,
            });
        }
        let (walled_now, open_now) = (unit_view(&state, &wall), unit_view(&state, &plain));
        assert!(has_keyword(&open_now.keywords, KeywordKind::Trample));
        assert_eq!(open_now.attack, open.attack + 3);
        assert!(!has_keyword(&walled_now.keywords, KeywordKind::Trample));
        assert_eq!(walled_now.attack, walled.attack);
    }

    #[test]
    fn r5113001_r3547002_grant_keywords_gives_its_list_together_and_an_immutable_card_none() {
        let mut state = game("r5113001-list");
        let plain = put(&mut state, "fx-1", slot(PlayerId::P2, Row::Units, 1), json!({}));
        let wall = unit_with(&mut state, PlayerId::P2, 2, Keyword::Immutable);
        let list = || {
            grant_keywords(json_as(json!({
                "target": { "of": "chosen" },
                "keywords": [{ "kind": "Immutable" }, { "kind": "Divine Shield" }, { "kind": "Immune to Spells" }],
            })))
        };
        run_with_targets(&mut state, list(), None, vec![at(&plain)], PlayerId::P1);
        let given = live(&state, &plain).granted_keywords;
        for kind in [
            KeywordKind::Immutable,
            KeywordKind::DivineShield,
            KeywordKind::ImmuneToSpells,
        ] {
            assert!(has_keyword(&given, kind), "{kind:?}");
        }
        run_with_targets(&mut state, list(), None, vec![at(&wall)], PlayerId::P1);
        untouched(&state, &wall, &[Keyword::Immutable]);
    }
}
