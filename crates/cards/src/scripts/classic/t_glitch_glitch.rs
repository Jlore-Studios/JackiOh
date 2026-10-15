//! T-glitch Glitch (SPEC §7, issue #170; R673–R679). Spell, Token, cost 0. Hidden: in no pool, not in
//! the Almanac or the Deck Builder (R674).
//!   Base:    "" — the card is blank; its client face draws corrupted text over glitch art.
//!   Radiant: the same blank face (§7: Glitch prints no Radiant form, and its text has no number).
//!
//! How it gets into a hand or a deck is not this file's (R673): once a "… in the System" card has been
//! played in the match, `catalog.pickGenerated` may hand one out in place of a generated card. It is
//! always playable on its owner's turn — it costs (0) whatever modifies costs and no ban refuses it
//! (R675), which is the engine's (`mana.priceOf`, `costRules.whyPlayBanned`), not a hook here.
//!
//! When it resolves it does one of four things, drawn by the match rng (`glitch`, R676–R679): reset
//! the match, swap the seats, lay two other games' boards on the field, or void the match.

use jackioh_engine::effects::glitch;
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-t-glitch";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| vec![glitch()])),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// T-glitch Glitch (SPEC §7, issue #170; R673–R679). BUILD M9's row: "hidden Spell token, cost 0, blank;
// only R673's roll makes one, after a … in the System play, at n/10000 per generated card; always
// playable on its owner's turn; resolves as one of four outcomes drawn by the match rng".
//
// The engine's own test (packages/engine/test/glitch.test.ts) proves the odds' arithmetic and each
// outcome on a fixture; this file proves them again on the real catalog, through `scenario()`.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// #8 Mr. Vanilla, Radiant.
    fn other_board() -> Value {
        json!([{ "defId": "core-008", "radiant": true }])
    }

    /// A game at p1's turn with Glitch in hand and no mana to pay for anything else.
    fn with_glitch(seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [GLITCH_DEF_ID], "field": ["core-001"], "mana": 0 },
            "p2": { "field": ["core-002"] },
            "glitchBoards": [other_board(), []],
        }))
    }

    fn outcome_of(events: &[GameEvent]) -> Option<GlitchOutcome> {
        events.iter().find_map(|event| match event {
            GameEvent::Glitched { outcome, .. } => Some(*outcome),
            _ => None,
        })
    }

    /// The first seed whose Glitch draws `outcome`, played.
    fn played(outcome: GlitchOutcome) -> Scenario {
        for n in 0..200 {
            let mut g = with_glitch(&format!("t-glitch-{outcome}-{n}"));
            g.play(GLITCH_DEF_ID, json!({}));
            if outcome_of(g.last_events()) == Some(outcome) {
                return g;
            }
        }
        panic!("no seed draws {outcome}");
    }

    mod t_glitch_glitch {
        use super::*;

        #[test]
        fn is_a_0_spell_token_blank_on_both_faces_whose_one_hook_is_the_same_on_both() {
            crate::register_all();
            let def = crate::card_def(ID);
            assert_eq!(
                json!([def.cost, def.type_, def.token, def.rarity, def.base.text, def.radiant.text]),
                json!([0, "Spell", true, "Token", "", ""])
            );
            let scripts = script();
            assert!(scripts.base.cry.is_some());
            // TS `expect(radiant).toBe(base)`: the Radiant face is the same script, so the same hook.
            assert!(std::sync::Arc::ptr_eq(
                scripts.base.cry.as_ref().unwrap(),
                scripts.radiant.cry.as_ref().unwrap()
            ));
        }

        #[test]
        fn r673_after_a_in_the_system_play_a_generated_card_may_be_glitch_at_10000_in_10000_every_one_is() {
            crate::register_all();
            let mut g = scenario(json!({
                "seed": "t-glitch-odds",
                "p1": { "hand": ["classic-025", "core-057"], "mana": 4 },
            }));
            g.play("classic-025", json!({}));
            assert_eq!(g.state().system_plays, Some(1));
            g.state_mut().system_plays = Some(GLITCH_ODDS_DENOMINATOR);
            g.play("core-057", json!({}));
            let hand: Vec<String> = g.hand(P1).iter().map(|card| card.def_id.clone()).collect();
            assert_eq!(hand, vec![GLITCH_DEF_ID, GLITCH_DEF_ID, GLITCH_DEF_ID]);
        }

        #[test]
        fn r673_with_no_system_play_the_same_generation_makes_no_glitch_and_draws_as_it_always_did() {
            crate::register_all();
            let mut g = scenario(json!({
                "seed": "t-glitch-none",
                "p1": { "hand": ["core-057"], "mana": 4 },
            }));
            g.play("core-057", json!({}));
            assert_eq!(g.state().system_plays, None);
            let hand: Vec<String> = g.hand(P1).iter().map(|card| card.def_id.clone()).collect();
            assert!(!hand.iter().any(|id| id == GLITCH_DEF_ID));
        }

        #[test]
        fn r674_is_in_no_pool_classic_n23_dropshippings_every_token_pool_included() {
            crate::register_all();
            let with_tokens: Vec<String> = jackioh_engine::catalog::query(&json_as(json!({ "withTokens": true })))
                .iter()
                .map(|card| card.id.clone())
                .collect();
            assert!(!with_tokens.iter().any(|id| id == GLITCH_DEF_ID));
            let classic_tokens: Vec<String> =
                jackioh_engine::catalog::query(&json_as(json!({ "set": "Classic", "token": true })))
                    .iter()
                    .map(|card| card.id.clone())
                    .collect();
            assert!(!classic_tokens.iter().any(|id| id == GLITCH_DEF_ID));
        }

        #[test]
        fn r675_is_played_for_nothing_on_its_owners_turn_a_cost_increase_and_no_mana_notwithstanding() {
            crate::register_all();
            let mut g = with_glitch("t-glitch-free");
            let id = g.card(GLITCH_DEF_ID).id.clone();
            find_instance_mut(g.state_mut(), &id).unwrap().cost_mod = 3;
            g.play(GLITCH_DEF_ID, json!({}));
            assert!(outcome_of(g.last_events()).is_some());
        }

        #[test]
        fn r676_each_of_the_four_outcomes_comes_from_the_match_rng() {
            crate::register_all();
            let seen: IndexSet<Option<GlitchOutcome>> = GLITCH_OUTCOMES
                .iter()
                .map(|outcome| outcome_of(played(*outcome).events()))
                .collect();
            let mut seen: Vec<Option<String>> =
                seen.into_iter().map(|outcome| outcome.map(|o| o.to_string())).collect();
            seen.sort();
            let mut all: Vec<Option<String>> = GLITCH_OUTCOMES.iter().map(|o| Some(o.to_string())).collect();
            all.sort();
            assert_eq!(seen, all);
        }

        #[test]
        fn r676_reset_the_match_starts_again_at_its_mulligans() {
            crate::register_all();
            let g = played(GlitchOutcome::Reset);
            assert_eq!(g.state().phase, Phase::Mulligan);
            assert!(g.state().mulligan.is_some());
            assert!(g.unit(P1, 1).is_none());
        }

        #[test]
        fn r677_swap_each_account_now_plays_the_other_seat() {
            crate::register_all();
            let g = played(GlitchOutcome::Swap);
            assert_eq!(seat_played_by(g.state(), P1), P2);
            assert_eq!(g.unit(P1, 1).map(|card| card.def_id), Some("core-001".to_string()));
        }

        #[test]
        fn r678_boards_both_fields_become_the_frozen_other_games_boards() {
            crate::register_all();
            let g = played(GlitchOutcome::Boards);
            let unit = g.unit(P1, 1);
            assert_eq!(
                (
                    unit.as_ref().map(|card| card.def_id.clone()),
                    unit.as_ref().map(|card| card.radiant)
                ),
                (Some("core-008".to_string()), Some(true))
            );
            assert!(g.unit(P2, 1).is_none());
        }

        #[test]
        fn r679_void_the_game_ends_with_no_winner_reason_voided() {
            crate::register_all();
            assert_eq!(
                played(GlitchOutcome::Void).state().result,
                Some(GameResult {
                    winner: Winner::Draw,
                    reason: GameOverReason::Voided
                })
            );
        }
    }
}
