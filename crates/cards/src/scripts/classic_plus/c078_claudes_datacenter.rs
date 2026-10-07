//! C+ #78 Claude's Datacenter (SPEC §8.7 row 78, B7). (2) Field Spell, Legendary.
//!   Base:    "End of Turn: Add a random AI Generated card to your hand. It costs (0)."
//!   Radiant: "… random Radiant AI generated card …"
//!   Engine:  "Its controller's end of turn (R62). The pool is the ten AI generated cards (T-AI-1 to
//!            T-AI-10, §7), named by the text, so tokens reach it; `costOverride` 0; the hand cap burns
//!            it (§2.4). Tunes: none (balance patch 1: the count is fixed at one)."
//!
//! An `endOfTurn` hook runs on its controller's turn only (§6.2). Each card is its own pick (R60) through
//! §6.3's Add to hand, so a full hand burns it, and the opponent sees the sentinel (R97).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-078";

/// §8.7: "a random AI generated card" — the AI tag's ten tokens, which a pool names by asking for tokens.
fn ai_pool() -> Value {
    json!({ "tags": ["AI"], "token": true })
}

/// §8.7: "It costs (0)".
const FREE: i32 = 0;

/// The printed "a random AI generated card": one card per end of turn.
const CARDS: i32 = 1;

fn datacenter(radiant: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |_ctx| {
            let mut args = json!({ "query": ai_pool(), "count": CARDS, "costOverride": FREE });
            if radiant {
                args["radiant"] = json!(radiant);
            }
            vec![add_random_from_catalog(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: datacenter(false),
        radiant: datacenter(true),
    }
}

// C+ #78 Claude's Datacenter — SPEC §8.7 row 78, BUILD M9 Classic+ row C+ 78: "Field Spell: at each end
// of your turn adds a random AI generated card (T-AI-1 to T-AI-10, the pool its text names) that costs
// (0) to your hand; nothing at the opponent's end; a full hand burns it; hidden from the opponent (R97);
// the count is fixed at 1 with no tunable (balance patch 1); radiant the card is Radiant".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const DATACENTER: &str = "classicplus-078";
    const VANILLA: &str = "core-008"; // (1) Unit
    const TIMMY: &str = "core-011"; // (1) Unit

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialises")
    }

    fn ai_ids() -> Vec<String> {
        (1..=10).map(|at| format!("classicplus-t-ai-{at:02}")).collect()
    }

    /// TS's live `s.card(ref)`, written through: the card under that id in the state.
    fn card_mut<'a>(s: &'a mut Scenario, card: &str) -> &'a mut CardInstance {
        let id = s.card(card).id.clone();
        find_instance_mut(s.state_mut(), &id).expect("the card is in the state")
    }

    fn setup(radiant_face: bool, hand: usize, seed: Option<&str>) -> Scenario {
        crate::register_all();
        let hand: Vec<Value> = (0..hand).map(|_| json!(VANILLA)).collect();
        let mut opts = json!({
            "p1": {
                "hand": hand,
                "backrow": [{ "def": DATACENTER, "radiant": radiant_face, "faceUp": true }],
                "library": [TIMMY, TIMMY, TIMMY],
            },
            "p2": { "hand": [VANILLA], "library": [TIMMY, TIMMY, TIMMY] },
        });
        if let Some(seed) = seed {
            opts["seed"] = json!(seed);
        }
        scenario(opts)
    }

    fn ai_cards_in(s: &Scenario) -> Vec<CardInstance> {
        let ai = ai_ids();
        s.hand(P1).into_iter().filter(|card| ai.contains(&card.def_id)).collect()
    }

    fn view_text(s: &Scenario, player: PlayerId) -> String {
        serde_json::to_string(&s.view(player)).expect("serialises")
    }

    mod c_n78_claudes_datacenter {
        use super::*;

        #[test]
        fn is_a_2_legendary_field_spell_with_no_tunable_count() {
            crate::register_all();
            let def = crate::card_def(super::super::ID);
            assert_eq!(def.type_, CardType::FieldSpell);
            assert_eq!(js(&def.cost), json!(2));
            assert_eq!(def.rarity, Rarity::Legendary);
            assert!(def.params.is_none());
            let scripts = super::super::script();
            assert!(scripts.base.end_of_turn.is_some());
            assert!(scripts.radiant.end_of_turn.is_some());
        }

        #[test]
        fn s7_the_pool_its_text_names_is_the_ten_ai_generated_cards_tokens_all() {
            crate::register_all();
            let mut pool: Vec<String> = crate::query::query(&json_as(json!({ "tags": ["AI"], "token": true })))
                .iter()
                .map(|card| card.id.clone())
                .collect();
            pool.sort();
            assert_eq!(pool, ai_ids());
            for id in ai_ids() {
                assert!(crate::card_def(&id).token);
            }
        }

        mod base {
            use super::*;

            #[test]
            fn r62_at_the_end_of_your_turn_adds_a_random_ai_generated_card_that_costs_0() {
                let mut s = setup(false, 1, None);
                s.end_turn();
                let added = ai_cards_in(&s);
                assert_eq!(added.len(), 1);
                assert_eq!(added[0].cost_override, Some(0));
                assert!(!added[0].radiant);
                assert_eq!(added[0].owner, P1);
                let shown = js(&s.view(P1))["you"]["hand"]
                    .as_array()
                    .and_then(|cards| cards.iter().find(|card| card["instanceId"] == added[0].id.as_str()).cloned());
                assert_eq!(shown.map(|card| card["cost"].clone()), Some(json!(0)));
            }

            #[test]
            fn r60_every_seed_draws_from_the_ten_ai_cards_and_nothing_else_and_more_than_one_of_them_turns_up() {
                let mut seen: IndexSet<String> = IndexSet::new();
                for seed in 0..24 {
                    let seed = format!("datacenter-{seed}");
                    let mut s = setup(false, 1, Some(&seed));
                    s.end_turn();
                    let added = ai_cards_in(&s);
                    assert_eq!(added.len(), 1);
                    seen.insert(added.first().map(|card| card.def_id.clone()).unwrap_or_default());
                }
                assert!(seen.len() > 3);
            }

            #[test]
            fn s6_2_nothing_at_the_opponents_end_of_turn() {
                let mut s = setup(false, 1, None);
                s.end_turn();
                assert_eq!(ai_cards_in(&s).len(), 1);
                s.end_turn();
                assert_eq!(s.state().active, P1);
                // p2's end of turn added nothing; p1 drew one Timmy at the start of its turn.
                assert_eq!(ai_cards_in(&s).len(), 1);
                let ai = ai_ids();
                assert!(!s.hand(P2).iter().any(|card| ai.contains(&card.def_id)));
            }

            #[test]
            fn adds_again_at_each_end_of_your_turn() {
                let mut s = setup(false, 1, None);
                s.end_turn().end_turn().end_turn();
                assert_eq!(ai_cards_in(&s).len(), 2);
            }

            #[test]
            fn s2_4_a_full_hand_burns_it() {
                let mut s = setup(false, 10, None);
                s.end_turn();
                assert!(ai_cards_in(&s).is_empty());
                let burned: Vec<&GameEvent> =
                    s.events().iter().filter(|event| event.event_type() == GameEventType::Burned).collect();
                assert_eq!(burned.len(), 1);
                let def_id = match burned.first() {
                    Some(GameEvent::Burned { def_id, .. }) => def_id.clone(),
                    _ => String::new(),
                };
                assert!(ai_ids().contains(&def_id));
            }

            #[test]
            fn r97_hidden_from_the_opponent_their_view_never_names_the_card() {
                let mut s = setup(false, 1, None);
                s.end_turn();
                let added = ai_cards_in(&s).into_iter().next();
                let theirs = view_text(&s, P2);
                assert!(!theirs.contains(added.as_ref().map(|card| card.id.as_str()).unwrap_or("?")));
                assert!(!theirs.contains(added.as_ref().map(|card| card.def_id.as_str()).unwrap_or("?")));
            }

            #[test]
            fn r386_the_count_is_fixed_at_1_an_upgrade_still_adds_exactly_one_card() {
                let mut s = setup(false, 1, None);
                step_param(card_mut(&mut s, DATACENTER), "cards", 1);
                s.end_turn();
                assert_eq!(ai_cards_in(&s).len(), 1);
                assert!(ai_cards_in(&s).iter().all(|card| card.cost_override == Some(0)));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r62_the_card_it_adds_is_radiant_and_costs_0() {
                let mut s = setup(true, 1, None);
                s.end_turn();
                let added = ai_cards_in(&s);
                assert_eq!(added.len(), 1);
                assert!(added[0].radiant);
                assert_eq!(added[0].cost_override, Some(0));
            }

            #[test]
            fn r97_hidden_from_the_opponent() {
                let mut s = setup(true, 1, None);
                s.end_turn();
                let added = ai_cards_in(&s).into_iter().next().map(|card| card.id).unwrap_or_else(|| "?".to_string());
                assert!(!view_text(&s, P2).contains(&added));
            }
        }
    }
}
