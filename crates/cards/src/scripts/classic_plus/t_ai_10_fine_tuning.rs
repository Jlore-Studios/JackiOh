//! T-AI-10 Fine-Tuning (SPEC §8.7 row T-AI-10, R386; BUILD M9 row T-AI-10). (2) Field Spell, AI, Token.
//!   Base:    "End of turn: Buff a random card in your hand."
//!   Radiant: "End of turn: Buff 2 random cards in your hand."
//!
//! At its controller's end of turn (R62): Upgrade (R386) of different random hand cards (R60); a card no
//! change fits, or an Immutable one, is unchanged; an empty hand draws nothing (R129); the other player
//! learns only that a card of that hand changed (R97, R440). AI cards declare no numbers (B8).

use jackioh_engine::effects::upgrade;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-t-ai-10";

const CARDS: i32 = 1;
const RADIANT_CARDS: i32 = 2;

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            end_of_turn: Some(hook(|_ctx| {
                vec![upgrade(json_as(json!({ "scope": { "zones": ["hand"] }, "random": CARDS })))]
            })),
            ..Script::default()
        },
        radiant: Script {
            end_of_turn: Some(hook(|_ctx| {
                vec![upgrade(json_as(json!({ "scope": { "zones": ["hand"] }, "random": RADIANT_CARDS })))]
            })),
            ..Script::default()
        },
    }
}

// T-AI-10 Fine-Tuning — SPEC §8.7 row T-AI-10, BUILD M9 row T-AI-10: a Field Spell that, at each end of
// its controller's turn, Upgrades one random card in their hand (R386: one draw; an Immutable card or one
// nothing fits unchanged); an empty hand, nothing and no random draw (R129); the `upgraded` event names
// nothing to the opponent (R97); nothing at the opponent's end; radiant 2 different random cards.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const TUNING: &str = "classicplus-t-ai-10";
    const UNIT: &str = "core-008";
    const MENACE: &str = "core-019"; // Radiant: Immutable.
    const FILLER: &str = "core-005";

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    use crate::js;

    /// The `upgraded` events of a list, as JSON.
    fn upgrades(events: &[GameEvent]) -> Vec<Value> {
        events.iter().map(js).filter(|event| event["type"] == "upgraded").collect()
    }

    /// The `upgraded` events a view carries.
    fn view_upgrades(s: &Scenario, player: PlayerId) -> Vec<Value> {
        js(&s.view(player))["events"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|event| event["type"] == "upgraded")
            .collect()
    }

    fn instance_of(event: &Value) -> String {
        event["instanceId"].as_str().unwrap_or_default().to_string()
    }

    fn fine_tuning(hand: Vec<Value>, radiant: bool, seed: Option<&str>) -> Scenario {
        crate::register_all();
        let mut opts = json!({
            "p1": { "hand": hand, "backrow": [{ "def": TUNING, "radiant": radiant }], "field": [UNIT] },
            "p2": { "hand": [FILLER, FILLER], "field": [UNIT] },
        });
        if let Some(seed) = seed {
            opts["seed"] = json!(seed);
        }
        scenario(opts)
    }

    mod t_ai_10_fine_tuning {
        use super::*;

        #[test]
        fn is_a_2_field_spell_ai_token_with_an_end_of_turn_hook_on_each_face() {
            crate::register_all();
            let def = js(&crate::card_def(super::super::ID));
            assert_eq!(def["cost"], json!(2));
            assert_eq!(def["type"], json!("Field Spell"));
            assert_eq!(def["tags"], json!(["AI", "Token"]));
            assert_eq!(def["token"], json!(true));
            let scripts = super::super::script();
            assert!(scripts.base.end_of_turn.is_some());
            assert!(scripts.radiant.end_of_turn.is_some());
            assert!(scripts.base.start_of_turn.is_none());
        }

        mod base {
            use super::*;

            #[test]
            fn r62_r386_at_the_end_of_your_turn_it_upgrades_one_card_in_your_hand() {
                let mut s = fine_tuning(vec![json!(UNIT), json!(FILLER), json!(FILLER)], false, None);
                let hand: Vec<String> = s.hand(P1).iter().map(|card| card.id.clone()).collect();
                s.end_turn();
                let events = upgrades(s.last_events());
                assert_eq!(events.len(), 1);
                assert!(hand.contains(&instance_of(&events[0])));
                assert_ne!(events[0]["change"]["kind"], json!("none"));
            }

            #[test]
            fn r60_the_card_is_a_random_one_of_the_hand_the_seeds() {
                let mut seen: IndexSet<String> = IndexSet::new();
                for n in 0..16 {
                    let seed = format!("fine-tuning-{n}");
                    let mut s = fine_tuning(vec![json!(UNIT), json!(FILLER), json!("core-047")], false, Some(&seed));
                    s.end_turn();
                    let event = upgrades(s.last_events()).into_iter().next().expect("one upgrade");
                    seen.insert(s.card(instance_of(&event).as_str()).def_id.clone());
                }
                assert_eq!(seen.len(), 3);
            }

            #[test]
            fn nothing_at_the_opponents_end_of_turn_nor_at_the_start_of_yours() {
                let mut s = fine_tuning(vec![json!(UNIT), json!(FILLER)], false, None);
                s.end_turn();
                assert_eq!(s.state().active, P2);
                s.end_turn();
                assert_eq!(s.state().active, P1);
                // p2's end and p1's start: no Upgrade.
                assert!(upgrades(s.last_events()).is_empty());
            }

            #[test]
            fn r386_r440_an_immutable_card_is_unchanged_cued_with_the_change_none() {
                let mut s = fine_tuning(vec![json!({ "def": MENACE, "radiant": true })], false, None);
                let menace = s.hand(P1).into_iter().next().expect("the Menace in hand");
                s.end_turn();
                let events = upgrades(s.last_events());
                assert_eq!(events.len(), 1);
                assert_eq!(events[0]["change"], json!({ "kind": "none" }));
                assert!(s.card(&menace).tuning.is_none());
                assert_eq!(s.card(&menace).cost_mod, 0);
            }

            #[test]
            fn r97_the_upgraded_event_names_nothing_to_the_opponent() {
                let mut s = fine_tuning(vec![json!(UNIT), json!(FILLER)], false, None);
                s.end_turn();
                let theirs = view_upgrades(&s, P2);
                assert_eq!(theirs.len(), 1);
                assert_eq!(theirs[0]["instanceId"], json!(HIDDEN_ID));
                assert_eq!(theirs[0]["defId"], json!(HIDDEN_ID));
                assert_ne!(view_upgrades(&s, P1).first().map(|event| event["instanceId"].clone()), Some(json!(HIDDEN_ID)));
            }

            #[test]
            fn r129_an_empty_hand_nothing_and_no_random_draw() {
                let mut s = fine_tuning(vec![], false, None);
                let source = s.backrow(P1, 1);
                let end_of_turn = super::super::super::script().base.end_of_turn.expect("the end-of-turn hook");
                let mut events: Vec<GameEvent> = Vec::new();
                let state = s.state_mut();
                let cursor = state.rng_cursor;
                let mut rng = Rng::new(&state.seed, state.rng_cursor);
                {
                    let mut sink = EngineSink::new(state, &mut events, &mut rng);
                    let mut ctx = make_context(
                        &mut sink,
                        source.as_ref(),
                        HookOptions {
                            controller: Some(P1),
                            ..Default::default()
                        },
                    );
                    let effects = end_of_turn(&mut ctx);
                    apply_effects(&effects, &mut ctx);
                }
                assert!(events.is_empty());
                assert_eq!(rng.cursor(), cursor);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r60_upgrades_2_different_random_cards_in_your_hand() {
                for n in 0..6 {
                    let seed = format!("fine-tuning-r-{n}");
                    let mut s = fine_tuning(vec![json!(UNIT), json!(FILLER), json!("core-047")], true, Some(&seed));
                    s.end_turn();
                    let ids: Vec<String> = upgrades(s.last_events()).iter().map(instance_of).collect();
                    assert_eq!(ids.len(), 2);
                    assert_eq!(ids.iter().collect::<IndexSet<_>>().len(), 2);
                }
            }

            #[test]
            fn r129_with_one_card_in_hand_that_card_only_with_no_draw_for_the_pick() {
                let mut s = fine_tuning(vec![json!(UNIT)], true, None);
                s.end_turn();
                let ids: Vec<String> = upgrades(s.last_events()).iter().map(instance_of).collect();
                let held = s.pile(P1, "hand").first().expect("the card in hand").id.clone();
                assert_eq!(ids, vec![held]);
            }
        }
    }
}
