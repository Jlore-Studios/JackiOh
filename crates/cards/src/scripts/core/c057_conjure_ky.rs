//! #57 Conjure KY (SPEC §8.3): Spell, cost 2, tag KY. "Add 3 random KY cards to your hand" /
//! radiant "2 random plus 2 random Radiant KY cards". Engine cell: "KY pool = #31, #51, #82 (no
//! tokens, not #57); repeats allowed" — and since patch v0.2.0 every set's KY cards (R380).
//!
//! The pool is §5.1's one query and nothing else: `{ tags: ["KY"] }` already leaves out the Token-
//! tagged KY card (#51.1 KY's Empty Notebook), because "random pools never include Token-tagged
//! cards", and `excludeDefId: def.id` is §5.1's other half — "never include the generating card's own
//! definition" (R387). In Core that leaves #31 KY's Math Equation, #51 KY's Private Tutor and #82
//! KY's Trial, which is the Engine cell verbatim and BUILD M4-T4's must-pass row.
//!
//! R60: "Cards generated from the catalog may repeat unless the card says 'different'". This card
//! does not say different, so three picks may land on one def — that is `count`, not a shuffle.
//! R4: the hand caps at 10 and extra adds are burned to the graveyard; the add-to-hand pipeline owns
//! that (engine/src/draw.rs), so neither face counts hand space here.
//! R74/§5.2: a card generated "Radiant" is Radiant — an instance flag on the created card, not a
//! separate def, which is why the radiant face is two calls rather than one with a ratio.
//!
//! A hook may not roll dice itself — `ctx.rng.*` advances `rngCursor`, which is state — so the picks
//! happen inside `add_random_from_catalog` (engine/src/effects/add_to_hand.rs), shared with #54 Straaza
//! and #59 Unbiased Immigration. It picks `count` defs from the pool with `ctx.rng`, repeats allowed
//! (R60), and creates each in the hand through the same pipeline `add_to_hand` uses (§2.4, R4).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-057";

/// §5.1's KY pool: the KY tag minus tokens (automatic) minus this card (`excludeDefId`, R387).
/// `add_random_from_catalog` already excludes the running card's own id, as `discover_from_catalog`
/// does (`catalog.excludingDefId`), so naming it here repeats it and the pool comes out the same.
/// (TS `const KY_POOL: CatalogQueryArgs`, written as the literal the effect's argument takes.)
fn ky_pool() -> Value {
    json!({ "tags": ["KY"], "excludeDefId": ID })
}

pub fn script() -> CardScripts {
    CardScripts {
        // A spell's script is its `cry` hook (§10.9; `run_hook` in engine/src/resolve.rs).
        base: Script {
            cry: Some(hook(|_ctx| {
                vec![add_random_from_catalog(json_as(json!({ "query": ky_pool(), "count": 3 })))]
            })),
            ..Script::default()
        },
        // "2 random plus 2 random Radiant KY cards": a restated clause replaces the base one (§8
        // Conventions), so the radiant face adds 4 cards, not 3 + 4. The plain pair is rolled first so the
        // radiant flag lands on exactly the last two adds.
        radiant: Script {
            cry: Some(hook(|_ctx| {
                vec![
                    add_random_from_catalog(json_as(json!({ "query": ky_pool(), "count": 2 }))),
                    add_random_from_catalog(json_as(json!({ "query": ky_pool(), "count": 2, "radiant": true }))),
                ]
            })),
            ..Script::default()
        },
    }
}

// #57 Conjure KY (SPEC §8.3, BUILD M4-T4 row 57: "Pool exactly #31, #51, #82 with repeats allowed;
// radiant 2 base + 2 radiant"). The Engine cell is "KY pool = #31, #51, #82 (no tokens, not #57);
// repeats allowed" — and, since patch v0.2.0's one format (R380), the KY cards of every set — so the
// pool itself is asserted twice: once against §5.1's query directly, and once through seeded play.
//
// Rulings proved here: R60 (catalog-generated cards may repeat), R4 (hand cap 10, extras burned),
// R74/§5.2 (a generated Radiant card carries the instance flag), §5.1 (no tokens, never the
// generating card).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// The def ids in p1's hand, in hand order.
    fn hand_defs(g: &Scenario) -> Vec<String> {
        g.hand(P1).iter().map(|card| card.def_id.clone()).collect()
    }

    /// R380: the KY tag of every set — Core #31, #51, #82 and Classic+ #41, #42, #62 — no token, never #57.
    const KY_POOL: [&str; 6] = [
        "core-031",
        "core-051",
        "core-082",
        "classicplus-041",
        "classicplus-042",
        "classicplus-062",
    ];

    /// TS `pool(ownId, args).map((def) => def.id)` (`crate::query`, SPEC §5.1).
    fn pool_ids(own_id: &str, args: Value) -> Vec<String> {
        crate::query::pool(own_id, &json_as(args)).iter().map(|def| def.id.clone()).collect()
    }

    /// TS `query(args).map((def) => def.id)`.
    fn query_ids(args: Value) -> Vec<String> {
        crate::query::query(&json_as(args)).iter().map(|def| def.id.clone()).collect()
    }

    fn in_ky_pool(def_id: &str) -> bool {
        KY_POOL.contains(&def_id)
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
        fn r380_s5_1_the_ky_pool_is_exactly_core_n31_n51_n82_and_classic_n41_n42_n62() {
            crate::register_all();
            assert_eq!(pool_ids("core-057", json!({ "tags": ["KY"] })), KY_POOL);
        }

        #[test]
        fn s5_1_the_pool_never_offers_the_token_tagged_ky_card_n51_1_nor_n57_itself() {
            crate::register_all();
            let ids = pool_ids("core-057", json!({ "tags": ["KY"] }));

            assert!(!ids.iter().any(|id| id == "core-051-1"));
            assert!(!ids.iter().any(|id| id == "classicplus-042-1"));
            assert!(!ids.iter().any(|id| id == "core-057"));
            // The KY tokens (Core #51.1, Classic+ #42.1 KY's Gift) are only reachable by naming the token
            // pool, which this card never does.
            assert_eq!(query_ids(json!({ "tags": ["KY", "Token"] })), ["core-051-1", "classicplus-042-1"]);
        }
    }

    mod base {
        use super::*;

        #[test]
        fn adds_3_random_ky_cards_to_your_hand() {
            crate::register_all();
            let mut g = scenario(json!({ "p1": { "hand": ["core-057"] } }));

            g.play("core-057", json!({}));

            assert_eq!(hand_defs(&g).len(), 3);
            for def_id in hand_defs(&g) {
                assert!(in_ky_pool(&def_id), "{def_id} is not in the KY pool");
            }
            g.expect_events(json!(["cardPlayed", "addedToHand", "addedToHand", "addedToHand"]));
        }

        #[test]
        fn adds_them_to_the_caster_s_hand_and_nothing_to_the_opponent_s() {
            crate::register_all();
            let mut g = scenario(json!({ "p1": { "hand": ["core-057"] } }));

            g.play("core-057", json!({}));

            assert_eq!(g.hand(P2).len(), 0);
        }

        #[test]
        fn r60_three_picks_from_a_pool_of_three_may_repeat_nothing_forces_them_to_differ() {
            crate::register_all();
            let seeds = ["s1", "s2", "s3", "s4", "s5", "s6", "s7", "s8"];
            let hands: Vec<Vec<String>> = seeds
                .iter()
                .map(|seed| {
                    let mut g = scenario(json!({ "seed": seed, "p1": { "hand": ["core-057"] } }));
                    g.play("core-057", json!({}));
                    hand_defs(&g)
                })
                .collect();

            // Every hand is 3 cards from the pool, and at least one seed repeats a def — which a
            // "3 different cards" implementation (R60's Discover rule) could never produce.
            for hand in &hands {
                assert_eq!(hand.len(), 3);
            }
            assert!(hands.iter().any(|hand| hand.iter().collect::<IndexSet<_>>().len() < 3));
        }

        #[test]
        fn adds_no_radiant_card_on_the_base_face() {
            crate::register_all();
            let mut g = scenario(json!({ "p1": { "hand": ["core-057"] } }));

            g.play("core-057", json!({}));

            let flags: Vec<bool> = g.hand(P1).iter().map(|card| card.radiant).collect();
            assert_eq!(flags, [false, false, false]);
        }

        #[test]
        fn r4_the_hand_caps_at_10_and_the_extra_add_is_burned_to_the_graveyard() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": {
                    // 8 other cards plus Conjure KY: playing it leaves 8, and 3 adds would make 11.
                    "hand": [
                        "core-057",
                        "core-005",
                        "core-008",
                        "core-010",
                        "core-011",
                        "core-016",
                        "core-019",
                        "core-020",
                        "core-025",
                    ],
                },
            }));

            g.play("core-057", json!({}));

            assert_eq!(g.hand(P1).len(), 10);
            g.expect_events(json!(["burned"]));
        }

        #[test]
        fn s10_5_step_7_the_spell_itself_ends_in_the_graveyard() {
            crate::register_all();
            let mut g = scenario(json!({ "p1": { "hand": ["core-057"] } }));

            g.play("core-057", json!({})).expect_in_zone("core-057", "graveyard");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn adds_2_random_plus_2_random_radiant_ky_cards_the_flag_on_exactly_the_last_two() {
            crate::register_all();
            let mut g = scenario(json!({ "p1": { "hand": ["core-057"] } }));

            // The setup builder takes `radiant` on the field and the backrow only, so a radiant card that
            // has to be PLAYED is flagged on the hand instance (reported as a harness gap).
            flag_radiant(&mut g, "core-057");
            g.play("core-057", json!({}));

            assert_eq!(hand_defs(&g).len(), 4);
            for def_id in hand_defs(&g) {
                assert!(in_ky_pool(&def_id), "{def_id} is not in the KY pool");
            }
            let flags: Vec<bool> = g.hand(P1).iter().map(|card| card.radiant).collect();
            assert_eq!(flags, [false, false, true, true]);
        }

        #[test]
        fn s8_conventions_the_restated_clause_replaces_the_base_one_so_it_is_4_cards_and_not_7() {
            crate::register_all();
            let mut g = scenario(json!({ "p1": { "hand": ["core-057"] } }));

            flag_radiant(&mut g, "core-057");
            g.play("core-057", json!({}));

            assert_eq!(g.hand(P1).len(), 4);
        }

        #[test]
        fn r74_the_two_radiant_adds_are_the_radiant_form_of_a_real_card_not_a_separate_def() {
            crate::register_all();
            let mut g = scenario(json!({ "p1": { "hand": ["core-057"] } }));

            flag_radiant(&mut g, "core-057");
            g.play("core-057", json!({}));

            for card in g.hand(P1).iter().skip(2) {
                assert!(in_ky_pool(&card.def_id), "{} is not in the KY pool", card.def_id);
                assert!(card.radiant);
            }
        }

        #[test]
        fn r4_the_hand_cap_burns_the_extras_on_the_radiant_face_too() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": {
                    "hand": ["core-057", "core-005", "core-008", "core-010", "core-011", "core-016", "core-019", "core-020"],
                },
            }));

            flag_radiant(&mut g, "core-057");
            g.play("core-057", json!({}));

            assert_eq!(g.hand(P1).len(), 10);
            g.expect_events(json!(["burned"]));
        }
    }
}
