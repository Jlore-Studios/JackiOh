//! #59 Unbiased Immigration (SPEC §8.3): Field Spell, cost 2 embiggen 4, Rare. "Activate: add a random card
//! to your hand (Embiggened (4): it costs (0))" / radiant "A random Radiant card (Embiggened (4): it costs
//! (0))". Engine cell: "Non-token pool excluding #59", every set's (R380; §8).
//! Embiggen travels in play action (R81, §10.6). Pool query excludes def (#59, §5.1, R387).
//! Drawn via RNG (R60; Radiant R74); embiggened discounts costMod (R766, R65; burns if full, R4, §2.4, R78).
//!
//! An Activate since issue #3139 (R384), once a turn on its controller's turn with no price: it no longer
//! fires on its own at the start of a turn. The Embiggened price is read as the Activate runs
//! (`ctx.embiggened`, built from the instance, R65), so it is remembered turns after the play. Through
//! v0.5.11p (`script_v0_5_11o`, kept, R767000) it was a start-of-turn trigger.

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-059";

/// §5.1's "a random card": the non-token catalog of every set (R380), minus this card (R387).
fn card_pool() -> Value {
    json!({ "excludeDefId": ID })
}

/// One add. `ctx.embiggened` is the price this Field Spell was played for (R65), read at the moment
/// the hook resolves rather than remembered by the script, so the two faces differ only in the
/// radiant flag they put on the created card (§5.2, R74).
fn add_random_card(ctx: &EffectContext, as_radiant: bool) -> Effect {
    let mut args = json!({ "query": card_pool(), "count": 1 });
    if as_radiant {
        args["radiant"] = json!(true);
    }
    // "(Embiggened (4): it costs (0))" — the Embiggened version, not the mana actually spent after discounts. The
    // (0) is the declared number `setCost` (R386).
    if ctx.embiggened {
        args["costOverride"] = json!(param(ctx, "setCost"));
    }
    add_random_from_catalog(json_as(args))
}

fn start_of_turn_base() -> Hook {
    hook(|ctx| vec![add_random_card(ctx, false)])
}

fn start_of_turn_radiant() -> Hook {
    hook(|ctx| vec![add_random_card(ctx, true)])
}

/// "Activate: Add a random card to your hand." (R384): once a turn, no price.
fn immigrate(as_radiant: bool) -> ActivationDecl {
    ActivationDecl {
        id: "immigrate".to_string(),
        label: "Add a random card to your hand".to_string(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(move |ctx| vec![add_random_card(ctx, as_radiant)]),
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            activations: vec![immigrate(false)],
            ..Script::default()
        },
        radiant: Script {
            activations: vec![immigrate(true)],
            ..Script::default()
        },
    }
}

/// R767000: the script v0.5.11p and the versions before it ran, a start-of-turn trigger; `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_5_11o() -> CardScripts {
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

// #59 Unbiased Immigration (SPEC §8.3, BUILD M4-T4 row 59). Rulings proved here: R65 (the chosen
// embiggen price is the cost, and `costOverride` starts the calculation), R81 (embiggen travels in
// the `play` action and opens no prompt), R384 (an Activate: once a turn, on its controller's turn, and no
// longer at the start of a turn), R60 and §5.1 (the pool: no tokens, never #59), R74 (the added card is Radiant
// by flag).
#[cfg(test)]
mod tests {
    use super::script_v0_5_11o;
    use crate::merged;
    use jackioh_engine::testkit::*;

    use crate::scenario;

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

    mod the_pool {
        use super::*;

        #[test]
        fn r380_s5_1_the_pool_is_every_set_s_non_token_cards_without_n59() {
            crate::register_all();
            let pool = card_pool();
            // Every non-token card of every set that ships (R1420), less #59 itself and the Dormant
            // Starship, which only a rule makes (R1857001).
            let open = |def: &&CardDef| {
                !def.token
                    && !jackioh_engine::catalog::made_by_a_rule(def)
                    && jackioh_engine::catalog::set_is_open(def.set)
            };
            assert_eq!(pool.len(), crate::CATALOG.values().filter(open).count() - 1);
            let has = |id: &str| pool.iter().any(|entry| entry == id);
            assert!(!has("core-059"));
            assert!(!has("core-t-rush"));
            assert!(!has("core-051-1"));
            assert!(!has("classicplus-065-1"));
            assert!(has("classic-001"));
            assert!(has("classicplus-078"));
        }
    }

    /// A Field Spell on p1's backrow (`radiant`'s face), p1 with cards enough to keep its turn open.
    fn standing(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": merged(busy(), json!({ "backrow": [{ "def": "core-059", "radiant": radiant, "lane": 1 }], "hand": ["core-005"] })),
            "p2": merged(busy(), json!({ "hand": ["core-005"] })),
        }))
    }

    /// p1 holds the Field Spell to play (embiggened or not) with a spare card.
    fn in_hand(radiant: bool) -> Scenario {
        let mut g = scenario(json!({
            "p1": merged(busy(), json!({ "hand": ["core-059", "core-005"] })),
            "p2": merged(busy(), json!({ "hand": ["core-005"] })),
        }));
        // The setup builder takes `radiant` on the field and the backrow only, so a radiant card that has to
        // be PLAYED is flagged on the hand instance (reported as a harness gap).
        g.card_mut("core-059").radiant = radiant;
        g
    }

    mod base {
        use super::*;

        #[test]
        fn r384_its_activate_adds_a_random_card_to_your_hand() {
            let mut g = standing(false);

            g.activate("core-059", json!({}));

            let added = only_added(&g);
            assert!(in_pool(&added.def_id));
            assert!(!added.radiant);
            assert_eq!(added.cost_override, None);
        }

        /// Issue #3139: "Start of turn" is gone, the Field Spell adds a card only when it is activated.
        #[test]
        fn r384_it_adds_nothing_at_the_start_of_a_turn_on_its_own() {
            let mut g = standing(false);

            g.start_turn();
            assert_eq!(added_cards(&g).len(), 0);
            g.end_turn().end_turn();
            assert_eq!(g.state().active, P1);
            assert_eq!(added_cards(&g).len(), 0);
        }

        #[test]
        fn r384_once_a_turn_a_second_activate_is_refused_and_the_next_turn_serves_again() {
            let mut g = standing(false);
            g.activate("core-059", json!({}));
            let after_one = g.hand(P1).len();

            g.expect_refused(|g| g.activate("core-059", json!({})));
            assert_eq!(g.hand(P1).len(), after_one);

            g.end_turn().end_turn();
            assert_eq!(g.state().active, P1);
            let before = g.hand(P1).len();
            g.activate("core-059", json!({}));
            assert_eq!(g.hand(P1).len(), before + 1);
        }

        #[test]
        fn r384_only_its_controller_activates_it_on_their_own_turn() {
            let mut g = scenario(json!({
                "p1": merged(busy(), json!({ "hand": ["core-005"] })),
                "p2": merged(busy(), json!({ "backrow": [{ "def": "core-059", "lane": 1 }], "hand": ["core-005"] })),
            }));
            g.expect_refused(|g| g.activate("core-059", json!({})));
            g.end_turn();
            assert_eq!(g.state().active, PlayerId::P2);
            let before = g.hand(PlayerId::P2).len();
            g.activate("core-059", json!({}));
            assert_eq!(g.hand(PlayerId::P2).len(), before + 1);
        }

        /// It can be used the turn it lands (summoning sickness does not apply to an Activate).
        #[test]
        fn r384_it_serves_the_turn_it_is_played() {
            let mut g = in_hand(false);

            g.play("core-059", json!({ "zone": 1 }));
            g.activate("core-059", json!({}));

            assert!(in_pool(&only_added(&g).def_id));
        }

        #[test]
        fn r65_paid_2_the_added_card_keeps_its_own_cost() {
            let mut g = in_hand(false);

            g.play("core-059", json!({ "zone": 1 })).expect_mana(P1, 2);
            g.activate("core-059", json!({}));

            let added = only_added(&g);
            assert_eq!(added.cost_override, None);
        }

        /// R65: `ctx.embiggened` reaches the Activate's run: the instance's embiggened flag, as the old hook.
        #[test]
        fn r65_paid_4_the_added_card_costs_0() {
            let mut g = in_hand(false);

            // R81: the embiggen price travels in the `play` action; §10.6 says no Core card prompts for it.
            g.play("core-059", json!({ "zone": 1, "embiggen": true })).expect_mana(P1, 0);
            g.activate("core-059", json!({}));

            let added = only_added(&g);
            assert_eq!(added.cost_override, Some(0));
        }

        #[test]
        fn r386_a_degrade_makes_the_paid_card_cost_1_and_an_upgrade_finds_the_cost_at_its_floor_of_0() {
            let mut g = in_hand(false);
            assert!(!crate::can_upgrade_number(&g, "core-059", "setCost"));
            assert_eq!(crate::degrade_number(&mut g, "core-059", "setCost"), 1);

            g.play("core-059", json!({ "zone": 1, "embiggen": true }));
            g.activate("core-059", json!({}));

            assert_eq!(only_added(&g).cost_override, Some(1));
        }

        #[test]
        fn r65_the_0_is_the_cost_the_view_shows_whatever_the_card_was_printed_at() {
            let mut g = in_hand(false);

            g.play("core-059", json!({ "zone": 1, "embiggen": true }));
            g.activate("core-059", json!({}));

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
            let mut g = in_hand(false);

            g.play("core-059", json!({ "zone": 1, "embiggen": true }));
            let mut added = Vec::new();
            for _ in 0..3 {
                g.activate("core-059", json!({}));
                added.extend(added_cards(&g));
                g.end_turn().end_turn();
                assert_eq!(g.state().active, P1);
            }

            // Every card this Field Spell has added is free, on this turn and on the ones after it.
            assert_eq!(added.len(), 3);
            for card in added {
                assert_eq!(card.cost_override, Some(0));
            }
        }

        #[test]
        fn s9_3_the_pick_is_seeded_the_same_seed_adds_the_same_card() {
            crate::register_all();
            let build = |seed: &str| -> String {
                let mut g = scenario(json!({
                    "seed": seed,
                    "p1": merged(busy(), json!({ "backrow": [{ "def": "core-059", "lane": 1 }], "hand": ["core-005"] })),
                    "p2": busy(),
                }));
                g.activate("core-059", json!({}));
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
                    "p1": merged(busy(), json!({ "backrow": [{ "def": "core-059", "lane": 1 }], "hand": ["core-005"] })),
                    "p2": busy(),
                }));
                g.activate("core-059", json!({}));
                assert!(in_pool(&only_added(&g).def_id));
            }
        }

        /// R767000: through v0.5.11p it added a card at the start of its controller's turn, with no Activate.
        #[test]
        fn r767000_the_script_kept_for_v0_5_11o_adds_a_card_at_the_start_of_its_controllers_turn() {
            let kept = script_v0_5_11o();
            assert!(kept.base.activations.is_empty() && kept.radiant.activations.is_empty());
            crate::under_kept("core-059", kept, || {
                let mut g = standing(false);
                g.expect_refused(|g| g.activate("core-059", json!({})));
                g.start_turn();
                assert!(in_pool(&only_added(&g).def_id));
            });
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r74_r384_its_activate_adds_a_random_radiant_card() {
            let mut g = standing(true);

            g.activate("core-059", json!({}));

            let added = only_added(&g);
            assert!(added.radiant);
            assert!(in_pool(&added.def_id));
        }

        #[test]
        fn s8_conventions_the_restated_clause_replaces_the_base_one_one_card_not_two() {
            let mut g = standing(true);

            g.activate("core-059", json!({}));

            assert_eq!(added_cards(&g).len(), 1);
        }

        #[test]
        fn r65_paid_4_on_the_radiant_face_radiant_and_costing_0() {
            let mut g = in_hand(true);

            g.play("core-059", json!({ "zone": 1, "embiggen": true }));
            g.activate("core-059", json!({}));

            let added = only_added(&g);
            assert!(added.radiant);
            assert_eq!(added.cost_override, Some(0));
        }

        #[test]
        fn r384_the_radiant_face_adds_nothing_on_its_own_and_serves_once_a_turn() {
            let mut g = standing(true);

            g.start_turn();
            assert_eq!(added_cards(&g).len(), 0);
            g.activate("core-059", json!({}));
            assert_eq!(added_cards(&g).len(), 1);
            g.expect_refused(|g| g.activate("core-059", json!({})));
            g.end_turn();

            assert_eq!(g.state().active, PlayerId::P2);
            assert_eq!(added_cards(&g).len(), 0);
        }
    }
}
