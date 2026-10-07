//! #59 Unbiased Immigration (SPEC §8.3): Field Spell, cost 2 embiggen 4, Rare. "Start of turn: add a
//! random card to your hand (paid 4: it costs 0)" / radiant "A random Radiant card (paid 4: it costs
//! 0)". Engine cell: "Non-token pool excluding #59" — every set's since patch v0.2.0 (R380).
//!
//! §8 Conventions: the radiant cell restates the whole clause, so it replaces it — one random card
//! per start of turn either way, Radiant on the radiant face. The parenthesis is restated too and
//! means the same thing on both faces.
//!
//! THE EMBIGGEN PRICE IS NOT A PROMPT (R81, §10.6): "Zone, X, embiggen, Tribute … travel in the
//! `play` action", and "No Core card opens an `x`, `embiggen`, `zone`, `tribute` or `direction`
//! prompt, since all five are play choices". So this card declares NOTHING — no `targets`, no
//! `modes`. `reduce`'s `play_card` writes the answer onto the instance (`card.embiggened`) and
//! `make_context` hands it to every hook of that instance as `ctx.embiggened`, which is why a trigger
//! that fires turns later still knows what was paid (R65: the chosen embiggen price is the cost).
//!
//! R65 is also why the discount is a `cost_override` and not a `cost_mod`: "start from `costOverride`,
//! else the printed cost … floor at 0" — an override of 0 makes the added card free whatever it was
//! printed at, and it persists in every zone (R78).
//!
//! R60: the pool may repeat across turns; nothing here says "different". §5.1: a random pool never
//! offers a Token-tagged card nor the generating card, which `excludeDefId` spells out (R387). R380:
//! "a random card" names no set, so it draws from every set. R4: an eleventh card is burned to the
//! graveyard by the add-to-hand pipeline.
//!
//! A hook may not roll dice — `ctx.rng.*` advances `rngCursor`, which is state — so the pick happens
//! inside `add_random_from_catalog` (engine/src/effects/add_to_hand.rs), the verb #54 Straaza and #57
//! Conjure KY use too.

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-059";

/// §5.1's "a random card": the non-token catalog of every set (R380), minus this card (R387).
/// (TS `const CARD_POOL: CatalogQueryArgs`, written as the literal the effect's argument takes.)
fn card_pool() -> Value {
    json!({ "excludeDefId": ID })
}

/// One add. `ctx.embiggened` is the price this Field Spell was played for (R65), read at the moment
/// the trigger resolves rather than remembered by the script, so the two faces differ only in the
/// radiant flag they put on the created card (§5.2, R74).
fn add_random_card(ctx: &EffectContext, as_radiant: bool) -> Effect {
    let mut args = json!({ "query": card_pool(), "count": 1 });
    if as_radiant {
        args["radiant"] = json!(true);
    }
    // "(paid 4: it costs 0)" — the embiggen price, not the mana actually spent after discounts.
    if ctx.embiggened {
        args["costOverride"] = json!(0);
    }
    add_random_from_catalog(json_as(args))
}

fn start_of_turn_base() -> Hook {
    hook(|ctx| vec![add_random_card(ctx, false)])
}

fn start_of_turn_radiant() -> Hook {
    hook(|ctx| vec![add_random_card(ctx, true)])
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            start_of_turn: Some(start_of_turn_base()),
            ..Script::default()
        },
        radiant: Script {
            start_of_turn: Some(start_of_turn_radiant()),
            ..Script::default()
        },
    }
}

// #59 Unbiased Immigration (SPEC §8.3, BUILD M4-T4 row 59: "Random non-token card each start of
// turn; paid 4 → cost 0; radiant gives a radiant card"). Engine cell: "Non-token Core pool
// excluding #59".
//
// Rulings proved here: R65 (the chosen embiggen price is the cost, and `costOverride` starts the
// calculation), R81 (embiggen travels in the `play` action and opens no prompt), R62 and §6.2 (the
// controller's turn only, trigger before the draw), R60 and §5.1 (the pool: no tokens, never #59),
// R74 (the added card is Radiant by flag).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    /// The card the Field Spell ADDED, told apart from the card the same start of turn DREW: a draw also
    /// goes through `add_to_hand` and so emits `addedToHand` too, but only after its own `drawn` event
    /// (engine/src/draw.rs).
    fn added_cards(g: &Scenario) -> Vec<CardInstance> {
        let drawn: IndexSet<String> = g
            .last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Drawn { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        g.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { instance_id, .. } if !drawn.contains(instance_id) => {
                    Some(g.card(instance_id).clone())
                }
                _ => None,
            })
            .collect()
    }

    fn only_added(g: &Scenario) -> CardInstance {
        let added = added_cards(g);
        assert_eq!(added.len(), 1);
        match added.into_iter().next() {
            Some(card) => card,
            None => panic!("no card was added this step"),
        }
    }

    /// Keeps a side's turn open past a play (R82's auto-end) and gives the start-of-turn draw a card.
    fn busy() -> Value {
        json!({ "field": [{ "def": "core-008", "lane": 1 }], "library": ["core-011", "core-016"] })
    }

    /// TS `{ ...base, ...extra }` over two JSON objects: the later keys win.
    fn spread(base: Value, extra: Value) -> Value {
        let mut all = base.as_object().cloned().unwrap_or_default();
        for (key, value) in extra.as_object().cloned().unwrap_or_default() {
            all.insert(key, value);
        }
        Value::Object(all)
    }

    /// R380: "a random card" names no set, so the pool is every set's non-token cards but #59 itself.
    fn card_pool() -> Vec<String> {
        crate::query::pool("core-059", &json_as(json!({})))
            .iter()
            .map(|def| def.id.clone())
            .collect()
    }

    fn in_pool(def_id: &str) -> bool {
        card_pool().iter().any(|id| id == def_id)
    }

    /// `g.card(ref).radiant = true`: the setup builder takes `radiant` on the field and the backrow only,
    /// so a radiant card that has to be PLAYED is flagged on the hand instance (reported as a harness gap).
    fn flag_radiant(g: &mut Scenario, card: &str) {
        let id = g.card(card).id.clone();
        match find_instance_mut(g.state_mut(), &id) {
            Some(instance) => instance.radiant = true,
            None => panic!("no instance {id}"),
        }
    }

    mod the_pool {
        use super::*;

        #[test]
        fn r380_s5_1_the_pool_is_every_set_s_non_token_cards_without_n59() {
            crate::register_all();
            let pool = card_pool();
            // Every non-token card of every set, less #59 itself.
            assert_eq!(pool.len(), crate::CATALOG.values().filter(|def| !def.token).count() - 1);
            let has = |id: &str| pool.iter().any(|entry| entry == id);
            assert!(!has("core-059"));
            assert!(!has("core-t-rush"));
            assert!(!has("core-051-1"));
            assert!(!has("classicplus-065-1"));
            assert!(has("classic-001"));
            assert!(has("classicplus-078"));
        }
    }

    mod base {
        use super::*;

        #[test]
        fn s6_2_adds_a_random_card_to_your_hand_at_your_start_of_turn() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": spread(busy(), json!({ "backrow": [{ "def": "core-059", "lane": 1 }] })),
                "p2": busy(),
            }));

            g.start_turn();

            let added = only_added(&g);
            assert!(in_pool(&added.def_id));
            assert!(!added.radiant);
        }

        #[test]
        fn r62_and_s6_2_nothing_is_added_on_the_opponent_s_turn() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": spread(busy(), json!({ "backrow": [{ "def": "core-059", "lane": 1 }], "hand": ["core-005"] })),
                "p2": spread(busy(), json!({ "hand": ["core-005"] })),
            }));

            g.start_turn();
            let after_mine = g.hand(P1).len();

            g.end_turn();
            assert_eq!(g.hand(P1).len(), after_mine);
            assert_eq!(added_cards(&g).len(), 0);

            g.end_turn();
            assert!(g.hand(P1).len() > after_mine);
        }

        #[test]
        fn r65_paid_2_the_added_card_keeps_its_own_cost() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": spread(busy(), json!({ "hand": ["core-059", "core-005"] })),
                "p2": busy(),
            }));

            g.play("core-059", json!({ "zone": 1 })).expect_mana(P1, 2);
            g.start_turn();

            let added = only_added(&g);
            assert_eq!(added.cost_override, None);
        }

        #[test]
        fn r65_paid_4_the_added_card_costs_0() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": spread(busy(), json!({ "hand": ["core-059", "core-005"] })),
                "p2": busy(),
            }));

            // R81: the embiggen price travels in the `play` action; §10.6 says no Core card prompts for it.
            g.play("core-059", json!({ "zone": 1, "embiggen": true })).expect_mana(P1, 0);
            g.start_turn();

            let added = only_added(&g);
            assert_eq!(added.cost_override, Some(0));
        }

        #[test]
        fn r65_the_0_is_the_cost_the_view_shows_whatever_the_card_was_printed_at() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": spread(busy(), json!({ "hand": ["core-059", "core-005"] })),
                "p2": busy(),
            }));

            g.play("core-059", json!({ "zone": 1, "embiggen": true }));
            g.start_turn();

            let added = only_added(&g);
            let hand = g.view(P1).you.hand;
            let HandView::Cards(cards) = hand else {
                panic!("p1's own hand is a list of cards in p1's view");
            };
            let shown = cards.iter().find(|card| card.instance_id == added.id);
            assert_eq!(shown.map(|card| card.cost), Some(0));
        }

        #[test]
        fn r65_the_embiggen_price_is_remembered_turns_later_not_just_on_the_play() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": spread(busy(), json!({ "hand": ["core-059", "core-005"] })),
                "p2": spread(busy(), json!({ "hand": ["core-005"] })),
            }));

            g.play("core-059", json!({ "zone": 1, "embiggen": true }));
            g.start_turn();
            g.end_turn().end_turn();

            // Every card this Field Spell has added is free, on this turn and on the ones after it.
            for card in added_cards(&g) {
                assert_eq!(card.cost_override, Some(0));
            }
        }

        #[test]
        fn s9_3_the_pick_is_seeded_the_same_seed_adds_the_same_card() {
            crate::register_all();
            let build = |seed: &str| -> String {
                let mut g = scenario(json!({
                    "seed": seed,
                    "p1": spread(busy(), json!({ "backrow": [{ "def": "core-059", "lane": 1 }] })),
                    "p2": busy(),
                }));
                g.start_turn();
                only_added(&g).def_id
            };

            assert_eq!(build("seed-a"), build("seed-a"));
        }

        #[test]
        fn r60_and_s5_1_every_card_it_can_add_is_a_non_token_card_of_any_set_that_is_not_itself_r380() {
            crate::register_all();
            let seeds = ["s1", "s2", "s3", "s4", "s5", "s6", "s7", "s8"];

            for seed in seeds {
                let mut g = scenario(json!({
                    "seed": seed,
                    "p1": spread(busy(), json!({ "backrow": [{ "def": "core-059", "lane": 1 }] })),
                    "p2": busy(),
                }));
                g.start_turn();
                assert!(in_pool(&only_added(&g).def_id));
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r74_adds_a_random_radiant_card() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": spread(busy(), json!({ "backrow": [{ "def": "core-059", "radiant": true, "lane": 1 }] })),
                "p2": busy(),
            }));

            g.start_turn();

            let added = only_added(&g);
            assert!(added.radiant);
            assert!(in_pool(&added.def_id));
        }

        #[test]
        fn s8_conventions_the_restated_clause_replaces_the_base_one_one_card_not_two() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": spread(busy(), json!({ "backrow": [{ "def": "core-059", "radiant": true, "lane": 1 }] })),
                "p2": busy(),
            }));

            g.start_turn();

            assert_eq!(added_cards(&g).len(), 1);
        }

        #[test]
        fn r65_paid_4_on_the_radiant_face_radiant_and_costing_0() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": spread(busy(), json!({ "hand": ["core-059", "core-005"] })),
                "p2": busy(),
            }));

            // The setup builder takes `radiant` on the field and the backrow only, so a radiant card that
            // has to be PLAYED is flagged on the hand instance (reported as a harness gap).
            flag_radiant(&mut g, "core-059");
            g.play("core-059", json!({ "zone": 1, "embiggen": true }));
            g.start_turn();

            let added = only_added(&g);
            assert!(added.radiant);
            assert_eq!(added.cost_override, Some(0));
        }

        #[test]
        fn r62_the_radiant_face_is_just_as_silent_on_the_opponent_s_turn() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": spread(busy(), json!({ "backrow": [{ "def": "core-059", "radiant": true, "lane": 1 }], "hand": ["core-005"] })),
                "p2": spread(busy(), json!({ "hand": ["core-005"] })),
            }));

            g.start_turn();
            g.end_turn();

            assert_eq!(added_cards(&g).len(), 0);
        }
    }
}
