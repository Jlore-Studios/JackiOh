//! `redact` and the play records (R185, R399, R451). The last Spell either player played and each
//! player's last face-up card are public history the seat watched happen, and the engine reads a
//! definition by the id they keep: a copier's text is the last Spell's (C #57 Echo), and T-AI-5
//! Autocomplete adds a copy of the opponent's last face-up card. A fused card (R77, R179) that has gone
//! back where the seat cannot see it is still named by them, so the redacted state keeps its definition.
//!
//! Rust only. Found by part 40's sweep: in `sweep:easy:core-076:2` the greedy seat's Pile On, with Book
//! Worm fused in by Classic+ #73, went back to the bottom of its deck by Pile On's own clause. Step 6
//! dropped its definition, since no card the AI could see used it, and the AI's reply simulation dealt
//! that seat an Echo, whose `copied_text::copied_chooses_x` panicked with `unknown defId`; the decision
//! fell back. TypeScript's `redact` dropped the definition the same way, and its `copiedChoosesX` threw
//! the same error (05f5cfd). The true state never loses one: `transient_defs` only grows.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN};

const ECHO: &str = "classic-057";
const PILE_ON: &str = "classic-060";
const AUTOCOMPLETE: &str = "classicplus-t-ai-05";
const STOCKPILE: &str = "core-005";
const VANILLA: &str = "core-008";

/// Pile On with Book Worm (C+ #39) fused in, as R179 names it: a Spell, as its first ingredient is.
const FUSED: &str = "t-1:classic-060+classicplus-039";

/// The human plays its fused Pile On, which Recruits nothing (its deck holds Spells only) and goes back
/// to the bottom of that deck by Pile On's own clause; then the turn comes round to the AI, holding
/// `held` beside a Stockpile.
fn fused_pile_on_gone_home(held: &str) -> Scenario {
    jackioh_cards::register_all();
    let mut s = scenario(json!({
        "active": "p2",
        "p1": { "hand": [held, STOCKPILE], "library": [VANILLA, VANILLA, VANILLA] },
        "p2": { "hand": [PILE_ON], "field": [VANILLA], "library": [STOCKPILE, STOCKPILE] },
    }));
    subsystems::rebuild_fused_def(s.state_mut(), FUSED, HUMAN).expect("Pile On and Book Worm fuse");
    let pile_on = s.card(PILE_ON).id.clone();
    s.card_mut(&pile_on).def_id = FUSED.to_string();
    s.play(&pile_on, json!({}));
    s.end_turn();
    assert_eq!(s.state().active, AI);
    assert_eq!(s.card(&pile_on).zone, Zone::Library { player: HUMAN });
    assert_eq!(
        last_spell_played(s.state()).map(|record| record.def_id),
        Some(FUSED.to_string())
    );
    s
}

/// The legal plays of `instance_id` for `player`.
fn plays_of(state: &GameState, player: PlayerId, instance_id: &str) -> Vec<ActionBody> {
    legal_actions(state, player)
        .into_iter()
        .filter(|action| matches!(action, ActionBody::Play { instance_id: id, .. } if id == instance_id))
        .collect()
}

/// The state with neither play record: no last Spell, and no player's last face-up card.
fn without_records(mut state: GameState) -> GameState {
    state.last_spell = None;
    for player in PLAYER_IDS {
        if let Some(log) = state.players[player].game_log.as_mut() {
            log.last_face_up_play = None;
        }
    }
    state
}

mod redact_keeps_what_the_play_records_name_r185 {
    use super::*;

    /// R185 R399 a copier in the seat's hand keeps the fused Spell it copies, whose card went back into a hidden deck
    #[test]
    fn r185_r399_a_copier_in_the_seats_hand_keeps_the_fused_spell_it_copies_whose_card_went_back_into_a_hidden_deck()
     {
        let s = fused_pile_on_gone_home(ECHO);
        let public = redact(s.state(), AI);
        assert!(public.transient_defs.contains_key(FUSED));
        // What panicked: the seat's own legal actions read the X of the text its Echo copies (R545).
        let echo = s.card(ECHO).id.clone();
        assert_eq!(plays_of(&public, AI, &echo).len(), 1);
        let view = serde_json::to_value(view_for(&public, AI)).expect("a view serialises");
        let held = view["you"]["hand"]
            .as_array()
            .and_then(|hand| {
                hand.iter()
                    .find(|card| card["instanceId"] == echo.as_str())
                    .cloned()
            })
            .expect("Echo in the seat's hand view");
        assert_eq!(held["copies"]["defId"], FUSED);
        assert_eq!(view["defs"][FUSED]["id"], FUSED);
    }

    /// R185 a decision holding that copier is searched, not a fallback
    #[test]
    fn r185_a_decision_holding_that_copier_is_searched_not_a_fallback() {
        let s = fused_pile_on_gone_home(ECHO);
        let decision = decide(
            s.state(),
            AI,
            &mut AiOptions::new(create_rng("redact-play-records", 0)),
        )
        .expect("the AI owes an action on its turn");
        assert_ne!(decision.reason, DecisionReason::Fallback);
    }

    /// R185 R451 T-AI-5 copies the opponent's last face-up card on the redacted state, a fused one with its definition
    #[test]
    fn r185_r451_t_ai_5_copies_the_opponents_last_face_up_card_on_the_redacted_state_a_fused_one_with_its_definition()
     {
        let s = fused_pile_on_gone_home(AUTOCOMPLETE);
        let public = redact(s.state(), AI);
        let autocomplete = s.card(AUTOCOMPLETE).id.clone();
        let body: ActionBody = json_as(json!({ "type": "play", "instanceId": autocomplete }));
        let played = reduce(&public, &Action::new(body, AI, "redact-play-records-1"));
        assert_eq!(played.error, None);
        let copy = played.state.players[AI]
            .hand
            .iter()
            .find(|card| card.def_id == FUSED)
            .map(|card| card.id.clone())
            .expect("Autocomplete's copy of the fused Pile On");
        assert!(find_def(Some(&played.state), FUSED).is_some());
        assert_eq!(plays_of(&played.state, AI, &copy).len(), 1);
        let _ = view_for(&played.state, AI);
    }

    /// R185 each record alone keeps the definition, and with none the hidden card takes it away
    #[test]
    fn r185_each_record_alone_keeps_the_definition_and_with_none_the_hidden_card_takes_it_away() {
        let s = fused_pile_on_gone_home(ECHO);
        let truth = s.state().clone();
        let kept = |state: &GameState| redact(state, AI).transient_defs.contains_key(FUSED);

        // Only the card in the human's deck uses it: step 6 drops it, as it always did.
        let none = without_records(truth.clone());
        assert!(!kept(&none));

        let mut last_spell = none.clone();
        last_spell.last_spell = truth.last_spell.clone();
        assert!(kept(&last_spell));

        let mut face_up = none.clone();
        face_up.players[HUMAN].game_log = truth.players[HUMAN].game_log.clone();
        assert!(kept(&face_up));

        // R546: the copy fixed on a copier as its play begins, whatever the last Spell is by now.
        let mut fixed = none.clone();
        let echo = s.card(ECHO).id.clone();
        subsystems::fix_copied_text(
            &mut fixed,
            &echo,
            Some(Some(PlayRecord {
                def_id: FUSED.to_string(),
                radiant: false,
            })),
        );
        assert!(kept(&fixed));

        // A kept continuation runs by the definition it names; one of the hidden card's own is dropped
        // with that card (step 5), and its definition with it.
        let delayed = |instance_id: Option<&str>| -> GameState {
            let mut state = none.clone();
            let mut resume =
                json!({ "defId": FUSED, "hook": "delayed", "step": "fused", "radiant": false, "data": {} });
            if let Some(id) = instance_id {
                resume["instanceId"] = json!(id);
            }
            state.delayed.push(json_as(json!({
                "id": "redact-play-records-delayed",
                "seq": 1,
                "owner": "p2",
                "at": { "phase": "end", "player": "p2" },
                "resume": resume,
            })));
            state
        };
        assert!(kept(&delayed(None)));
        let in_deck = truth.players[HUMAN]
            .library
            .iter()
            .find(|card| card.def_id == FUSED)
            .map(|card| card.id.clone())
            .expect("the fused Pile On in the human's deck");
        assert!(!kept(&delayed(Some(&in_deck))));
    }
}
