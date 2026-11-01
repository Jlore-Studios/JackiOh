//! C+ #78 Claude's Datacenter (SPEC §8.7 row 78, B7). (3) Field Spell, Legendary.
//!   Base:    "Activate: Add a random AI generated card to your hand. It costs ({setCost})."
//!   Radiant: "… random Radiant AI generated card …"
//!   Engine:  "Activate (§6.2, R384): once per turn, its controller's, in their main phase, and no play.
//!            The pool is the AI generated cards (T-AI-1 to T-AI-10, and M+ #1.1 to #1.4 since
//!            Meditative+ shipped and #1563, §7), named by the text, so tokens reach it; `costOverride`
//!            setCost; the hand cap burns it (§2.4). Tunes: setCost 0 ↓ (R1748003); the count is fixed
//!            at one (balance patch 1)."
//!
//! Each card is its own pick (R60) through §6.3's Add to hand, so a full hand burns it, and the opponent
//! sees the sentinel (R97).
//!
//! Issue #2179 made it an Activate; through v0.5.2 it added the card at its controller's end of turn
//! (R62), which `script_v0_5_2` keeps, and through v0.4.4b that end of turn's (0) was printed, not
//! declared, which `script_v0_4_4b` keeps (R767000).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-078";

/// §8.7: "a random AI generated card" — the AI tag's tokens (fourteen since #1563, twenty-four since the Stinkyscape release, twenty-five since #2521 added M+ #1.5 Star Chart), which a pool names by asking for tokens.
fn ai_pool() -> Value {
    json!({ "tags": ["AI"], "token": true })
}

/// §8.7: "It costs (0)", as v0.4.4b printed it before it was the declared `setCost` (R1748003).
const FREE: i32 = 0;

/// The printed "a random AI generated card": one card per Activate (per end of turn through v0.5.2).
const CARDS: i32 = 1;

/// The add; the card's price is the declared `setCost`, or the printed (0) when `declared` is off.
fn add_ai_card(ctx: &EffectContext<'_>, radiant: bool, declared: bool) -> Vec<Effect> {
    let cost = if declared { param(ctx, "setCost") } else { FREE };
    let mut args = json!({ "query": ai_pool(), "count": CARDS, "costOverride": cost });
    if radiant {
        args["radiant"] = json!(radiant);
    }
    vec![add_random_from_catalog(json_as(args))]
}

/// One face since #2179: the add is its Activate, once per turn (R384).
fn datacenter(radiant: bool) -> Script {
    Script {
        activations: vec![ActivationDecl {
            id: "datacenter".to_string(),
            label: if radiant {
                "Add a random Radiant AI generated card to your hand".to_string()
            } else {
                "Add a random AI generated card to your hand".to_string()
            },
            uses: ActivationUses::Count(1),
            cost: None,
            targets: vec![],
            modes: vec![],
            can_activate: None,
            has: None,
            run: hook(move |ctx| add_ai_card(ctx, radiant, true)),
        }],
        ..Script::default()
    }
}

/// One face through v0.5.2: the add at its controller's end of turn (R62).
fn datacenter_at_end_of_turn(radiant: bool, declared: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| add_ai_card(ctx, radiant, declared))),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: datacenter(false),
        radiant: datacenter(true),
    }
}

/// R767000: through v0.5.2 it added its card at the end of its controller's turn; `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_5_2() -> CardScripts {
    CardScripts {
        base: datacenter_at_end_of_turn(false, true),
        radiant: datacenter_at_end_of_turn(true, true),
    }
}

/// R767000: through v0.4.4b its card's (0) was printed; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_4_4b() -> CardScripts {
    CardScripts {
        base: datacenter_at_end_of_turn(false, false),
        radiant: datacenter_at_end_of_turn(true, false),
    }
}

// C+ #78 Claude's Datacenter — SPEC §8.7 row 78, BUILD M9 Classic+ row C+ 78: "Field Spell: its Activate,
// once per turn, adds a random AI generated card (T-AI-1 to T-AI-10 and M+ #1.1 to #1.4, the pool its
// text names) that costs (0) to your hand; nothing at either end of turn; a full hand burns it; hidden
// from the opponent (R97); the count is fixed at 1 with no tunable (balance patch 1); radiant the card is
// Radiant".

#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const DATACENTER: &str = "classicplus-078";
    const VANILLA: &str = "core-008"; // (1) Unit
    const TIMMY: &str = "core-011"; // (1) Unit

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    use crate::js;

    /// The AI generated cards of the sets that ship, sorted (§7, R1421): T-AI-1 to T-AI-10 and M+ #1.1
    /// Base Camp and #1.2 Spa Day. Re-pinned for v0.3.15 (Meditative+ ships, #1187): Bob's Plan's two
    /// AI tokens join the pool, which is now twelve; #1563's #1.3 and #1.4 make it fourteen; and the
    /// Stinkyscape release (#1884) its T-AI-1 to T-AI-10, twenty-four; #2521 M+ #1.5 Star Chart, twenty-five.
    fn ai_ids() -> Vec<String> {
        let mut ids: Vec<String> = (1..=10).map(|at| format!("classicplus-t-ai-{at:02}")).collect();
        ids.extend(
            [
                "meditativeplus-001-1",
                "meditativeplus-001-2",
                "meditativeplus-001-3",
                "meditativeplus-001-4",
                "meditativeplus-001-5",
            ]
            .map(String::from),
        );
        ids.extend((1..=10).map(|at| format!("stinkyscape-t-ai-{at:02}")));
        ids
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
        serde_json::to_string(&s.view(player)).expect("serializes")
    }

    /// Its Activate (R384).
    fn activate(s: &mut Scenario) -> &mut Scenario {
        s.activate(DATACENTER, json!({}))
    }

    mod c_n78_claudes_datacenter {
        use super::*;

        #[test]
        fn r1748003_declares_the_card_s_price_and_no_count() {
            // Balance patch 1 fixed the count at one: the price is its one declared number.
            crate::register_all();
            let keys: Vec<String> = crate::card_def(DATACENTER)
                .params
                .unwrap_or_default()
                .into_iter()
                .map(|param| param.key)
                .collect();
            assert_eq!(keys, ["setCost"]);
        }

        #[test]
        fn r1748003_set_cost_moves_with_a_buff_or_a_nerf() {
            let mut s = setup(false, 1, None);
            // On the field it is no Cry's number (R1748000): a Buff finds it at its floor of (0), a
            // Nerf raises it.
            assert!(!crate::can_upgrade_number(&s, DATACENTER, "setCost"));
            assert_eq!(crate::degrade_number(&mut s, DATACENTER, "setCost"), 1);
            activate(&mut s);
            let added = ai_cards_in(&s);
            assert_eq!(added.len(), 1);
            assert_eq!(added[0].cost_override, Some(1));
        }

        #[test]
        fn r1748003_r767000_the_v0_4_4b_kept_script_keeps_its_printed_set_cost() {
            crate::under_kept(DATACENTER, super::super::script_v0_4_4b(), || {
                let mut s = setup(false, 1, None);
                crate::degrade_number(&mut s, DATACENTER, "setCost");
                s.end_turn();
                let added = ai_cards_in(&s);
                assert_eq!(added.len(), 1);
                assert_eq!(added[0].cost_override, Some(0), "the kept script prints (0)");
            });
        }

        #[test]
        fn s7_the_pool_its_text_names_is_the_twenty_five_ai_generated_cards_tokens_all() {
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
            fn r384_its_activate_adds_a_random_ai_generated_card_that_costs_0() {
                let mut s = setup(false, 1, None);
                activate(&mut s);
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
            fn r60_every_seed_draws_from_the_twenty_five_ai_cards_and_nothing_else_and_more_than_one_of_them_turns_up() {
                let mut seen: IndexSet<String> = IndexSet::new();
                for seed in 0..24 {
                    let seed = format!("datacenter-{seed}");
                    let mut s = setup(false, 1, Some(&seed));
                    activate(&mut s);
                    let added = ai_cards_in(&s);
                    assert_eq!(added.len(), 1);
                    seen.insert(added.first().map(|card| card.def_id.clone()).unwrap_or_default());
                }
                assert!(seen.len() > 3);
            }

            #[test]
            fn r384_once_per_turn_and_never_at_an_end_of_turn() {
                let mut s = setup(false, 1, None);
                activate(&mut s);
                s.expect_refused(|s| s.activate(DATACENTER, json!({})));
                assert_eq!(ai_cards_in(&s).len(), 1);
                // Neither end of turn adds one; it is back on p1's next turn.
                s.end_turn().end_turn();
                assert_eq!(s.state().active, P1);
                assert_eq!(ai_cards_in(&s).len(), 1);
                activate(&mut s);
                assert_eq!(ai_cards_in(&s).len(), 2);
                let ai = ai_ids();
                assert!(!s.hand(P2).iter().any(|card| ai.contains(&card.def_id)));
            }

            #[test]
            fn s2_4_a_full_hand_burns_it() {
                let mut s = setup(false, 10, None);
                activate(&mut s);
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
                activate(&mut s);
                let added = ai_cards_in(&s).into_iter().next();
                let theirs = view_text(&s, P2);
                assert!(!theirs.contains(added.as_ref().map(|card| card.id.as_str()).unwrap_or("?")));
                assert!(!theirs.contains(added.as_ref().map(|card| card.def_id.as_str()).unwrap_or("?")));
            }

            #[test]
            fn r386_the_count_is_fixed_at_1_an_upgrade_still_adds_exactly_one_card() {
                let mut s = setup(false, 1, None);
                step_param(s.card_mut(DATACENTER), "cards", 1);
                activate(&mut s);
                assert_eq!(ai_cards_in(&s).len(), 1);
                assert!(ai_cards_in(&s).iter().all(|card| card.cost_override == Some(0)));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r384_the_card_it_adds_is_radiant_and_costs_0() {
                let mut s = setup(true, 1, None);
                activate(&mut s);
                let added = ai_cards_in(&s);
                assert_eq!(added.len(), 1);
                assert!(added[0].radiant);
                assert_eq!(added[0].cost_override, Some(0));
            }

            #[test]
            fn r97_hidden_from_the_opponent() {
                let mut s = setup(true, 1, None);
                activate(&mut s);
                let added = ai_cards_in(&s).into_iter().next().map(|card| card.id).unwrap_or_else(|| "?".to_string());
                assert!(!view_text(&s, P2).contains(&added));
            }
        }

        /// R767000: the script kept for the replays of v0.5.2 and before, which added at its controller's
        /// end of turn.
        mod v0_5_2 {
            use super::*;

            #[test]
            fn r767000_r62_a_game_on_v0_5_2_adds_at_the_end_of_your_turn_only() {
                crate::under_kept(DATACENTER, super::super::super::script_v0_5_2(), || {
                    let mut s = setup(false, 1, None);
                    s.end_turn();
                    assert_eq!(ai_cards_in(&s).len(), 1);
                    s.end_turn();
                    assert_eq!(ai_cards_in(&s).len(), 1);
                    s.end_turn();
                    assert_eq!(ai_cards_in(&s).len(), 2);
                });
            }
        }
    }
}
