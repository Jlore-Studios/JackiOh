//! #94 Genn's Greed (SPEC §8.4, R4, R26, R55, R65, R66, R135, BUILD M4-T4 row 94).
//!
//! Base: "Draw every 2-cost card from your library; exile every odd-cost card in your library, hand
//! and GY (X-cost cards exempt); gain 2 mana". Radiant: "Gain 6" — a cell that changes only a number
//! changes only that number (§8 Conventions), so the draw and the exile are kept verbatim and only
//! the mana moves from 2 to 6.
//!
//! R26 (decide, `GENN_GREED_EXILES = "odd"`) settles the garbled source line as "exile all odd-cost
//! cards". R66 settles what "cost" means in both halves: "both the 2-cost draw and the odd-cost
//! exile read each card's cost per R65 AT RESOLUTION; X-cost cards are exempt from both". So the
//! number both clauses read is `effectiveCost(state, instance)` — R65's one calculation for an
//! instance: `costOverride`, else the printed cost, plus the instance's `costMod` (which persists in
//! every zone, R78). The player's discounts are prices for a play from the hand, so they reach neither
//! a library card nor a graveyard one (R65): /fullsend's "this turn your cards cost 1 less" does not
//! make a 4 in the library a 3. `queryCost` is the other half of R65 and is wrong
//! here because it reads a DEFINITION and so cannot see the `costMod` #7 Jewelosco Scarab left on a
//! card or the discount #95 Call to Chaos put across a whole library. A printed 3 discounted to 2 is
//! therefore drawn, and a printed 2 pushed to 3 is odd and exiled instead.
//!
//! THE ORDER IS §8's AND R135's. The draw runs first, so the 2-cost cards are in hand before the
//! exile looks at hands — and 2 is even, so nothing this card drew is then exiled. R4's hand cap
//! applies to the draw, and a card burned to the graveyard by the cap is likewise even and survives
//! the exile. Inside the exile the zones go library, then hand, then graveyard, and each card is its
//! own exile, so R55's counter moves once per card and anything watching an exile sees them one at a
//! time: `exileMatching` is written to R135 and this file only names the order it already keeps.
//!
//! The draw clause reads its set once, as it begins, and keeps it (`forEachCard`): a drawn card that is
//! cast on draw and asks has left the library by the answer, and the draws still owed are the ones
//! the clause began with (R113). The exile reads its set once too (R135).
//!
//! The two clauses are one effect per card and one sweep, not a loop in this file that touches state:
//! `drawFromLibrary` is the §6.3 Draw of a card a script named (the verb #30 Archivist asks for too),
//! and `exileMatching` is §6.3 Exile over the three off-field zones by cost. Reading the library to
//! name the cards is a read, not a mutation (CLAUDE.md rule 5), and it goes through `zoneCards`
//! (engine/src/query.ts), which answers with a copy, so this file never names a field of PlayerState.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-094";

/// §8 #94: the one cost the draw clause names.
const DRAWN_COST: i32 = 2;

/// R66: every card in your library that costs 2 RIGHT NOW, X-cost cards exempt, in library order
/// (top down), which is the order they are drawn in. By instance id, which is what `forEachCard`
/// keeps (TS took the instances and read their ids).
fn drawn_cards(ctx: &EffectContext<'_>) -> Vec<String> {
    zone_cards(ctx.state, ctx.controller, OffFieldZone::Library)
        .iter()
        .filter(|card| {
            !is_x_cost(ctx.state, card) && effective_cost(ctx.state, card, Default::default()) == DRAWN_COST
        })
        .map(|card| card.id.clone())
        .collect()
}

/// §8: base "gain 2 mana"; radiant "Gain 6" — the declared number `mana` (R386).
fn greed() -> Hook {
    hook(|ctx| {
        let mana = param(&*ctx, "mana");
        vec![
            // §8 "Draw every 2-cost card from your library": one draw per card, so each counts on the draw
            // counter, emits its own `drawn` event and meets the hand cap on its own (§2.4, R4, R55). The
            // set is read once, as the clause begins (R66), so a drawn card that is cast and asks does not
            // reshape the draws still owed after the answer (R113, `forEachCard`).
            for_each_card(ForEachCardArgs {
                cards: Arc::new(|ctx: &mut EffectContext<'_>| drawn_cards(ctx)),
                each: Arc::new(|instance_id: &str| {
                    draw_from_library(json_as(json!({ "instanceId": instance_id })))
                }),
            }),
            // §8 "exile every odd-cost card in your library, hand and GY (X-cost cards exempt)". R135's
            // order is `exileMatching`'s default — library, then hand, then graveyard — and it runs after
            // the draws, so a card this play just drew is in hand, even, and spared.
            exile_matching(json_as(json!({ "parity": "odd", "exemptXCost": true }))),
            // §2.3: temporary mana, which may take current above max.
            gain_mana(json_as(json!({ "amount": mana }))),
        ]
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(greed()),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(greed()),
            ..Script::default()
        },
    }
}

// #94 Genn's Greed (SPEC §8 row 94, §2.3, §2.4; R4, R26, R65, R66).
//
// BUILD M4-T4 row 94: "Draws every 2-cost card; odd current-cost cards exiled from library, hand
// and GY, X-cost exempt (R26, R66); +2 mana (radiant +6)".
//
// §8: "Draw every 2-cost card from your library; exile every odd-cost card in your library, hand
// and GY (X-cost cards exempt); gain 2 mana", radiant "Gain 6" — a cell that changes only a number
// changes only that number (§8 Conventions), so the draw and the exile are identical on both faces
// and only the mana moves.
//
// R26 settles the garbled source line as "exile all ODD-cost cards". R66 settles what "cost" means
// in both halves: each card's cost per R65 read AT RESOLUTION, with X-cost cards exempt from both.
// §8's own order matters and is asserted: the draw runs first, so a drawn 2-cost card is in hand
// before the exile looks at hands — and 2 is even, so nothing this card drew is then exiled.
//
// STATUS: the script (`packages/cards/src/scripts/094-genns-greed.ts`) implements only the mana
// clause and documents both other clauses as BLOCKED on missing engine verbs. The tests below are
// written as the card SHOULD behave, so the draw and exile cases FAIL and name the gap rather than
// being weakened or skipped. The mana cases pass. See the report.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const GREED: &str = "core-094";

    /// Even, cost 2 — what the draw clause names.
    const TWO_COST: [&str; 3] = ["core-020", "core-045", "core-056"]; // Pointmaster, Deft Duelist, Jilliax
    /// Odd — what the exile clause names.
    const ODD_COST: [&str; 4] = ["core-008", "core-005", "core-053", "core-019"]; // 1, 1, 3, 3
    /// A (2) Cost Spell on top of a library the test does not look at: Greed draws it. (#16 Hit Job was it
    /// until patch v0.2.0 made it (3), which Greed exiles.)
    const FILLER_SPELL: &str = "core-069"; // #69 Call to Arms
    /// Even and not 2, so neither clause touches it.
    const FOUR_COST: &str = "core-025";
    /// X-cost, exempt from both clauses (R66).
    const X_COST: &str = "core-074"; // #74 Adaptive UI
    /// A keyword-only unit parked on the board. §2.5 auto-ends a turn with nothing meaningful left, and
    /// an auto-end starts the opponent's turn — with their draw — inside the same `play` step, which
    /// would move cards these tests are watching. A unit that could still switch position prevents it.
    const KEEP_TURN: &str = "core-008"; // #8 Mr. Vanilla

    fn def_ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
    }

    /// TS `zones(s)`: p1's four off-field piles, by def id.
    struct Zones {
        hand: Vec<String>,
        library: Vec<String>,
        graveyard: Vec<String>,
        exile: Vec<String>,
    }

    fn zones(s: &Scenario) -> Zones {
        Zones {
            hand: def_ids(&s.pile(P1, "hand")),
            library: def_ids(&s.pile(P1, "library")),
            graveyard: def_ids(&s.pile(P1, "graveyard")),
            exile: def_ids(&s.pile(P1, "exile")),
        }
    }

    /// `ids.includes(id)` over def ids.
    fn has(ids: &[String], id: &str) -> bool {
        ids.iter().any(|candidate| candidate == id)
    }

    /// The board every clause is read against: a 2-cost, an odd, an even non-2 and an X in each pile.
    /// TS `greedBoard(seed, radiant = false)`.
    fn greed_board(seed: &str, radiant: bool) -> Scenario {
        let greed = if radiant { json!({ "def": GREED, "radiant": true }) } else { json!(GREED) };
        scenario(json!({
            "seed": seed,
            "p1": {
                // A unit on the board keeps §2.5 from auto-ending the turn once the hand is spent or
                // exiled — an auto-end hands p2 a draw, which would move cards this test is watching.
                "field": [KEEP_TURN],
                "hand": [greed, ODD_COST[1], FOUR_COST],
                "library": [TWO_COST[0], ODD_COST[0], ODD_COST[2], X_COST, FOUR_COST],
                "graveyard": [ODD_COST[3], TWO_COST[1]],
            },
            "p2": { "hand": ["core-005"], "library": [FILLER_SPELL] },
        }))
    }

    // =========================================================================================
    // the mana clause (§2.3) — implemented
    // =========================================================================================

    mod n94_genn_s_greed_mana {
        use super::*;

        #[test]
        fn s2_3_base_gains_2_temporary_mana_after_paying_its_own_4() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-094-mana-base",
                "p1": { "hand": [GREED, "core-005"], "library": [FILLER_SPELL] },
                "p2": { "hand": ["core-005"] },
            }));
            s.expect_mana(P1, 4);

            s.play(GREED, json!({}));

            // 4 − 4 cost + 2 gained.
            s.expect_mana(P1, 2);
            assert_eq!(s.state().players[P1].mana.max, 4);
        }

        #[test]
        fn r386_an_upgrade_gains_3_and_a_radiant_degrade_5() {
            for (radiant, upgrade, mana) in [(false, true, 3), (true, false, 5)] {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "core-094-mana-tuned",
                    "p1": { "hand": [{ "def": GREED, "radiant": radiant }, "core-005"], "library": [FILLER_SPELL] },
                    "p2": { "hand": ["core-005"] },
                }));
                let moved = if upgrade {
                    crate::upgrade_number(&mut s, GREED, "mana")
                } else {
                    crate::degrade_number(&mut s, GREED, "mana")
                };
                assert_eq!(moved, mana);
                s.play(GREED, json!({}));
                s.expect_mana(P1, mana);
            }
        }

        #[test]
        fn s2_3_the_gain_may_take_current_mana_above_max() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-094-mana-above-max",
                "p1": { "hand": [GREED, "core-005"], "library": [FILLER_SPELL], "mana": 4 },
                "p2": { "hand": ["core-005"] },
            }));

            s.play(GREED, json!({}));

            s.expect_mana(P1, 2);
            // Radiant's 6 is the case that really exceeds max; the base face proves `mana.max` is untouched.
            assert_eq!(s.state().players[P1].mana.max, 4);
        }

        #[test]
        fn s3_2_the_spell_reaches_the_graveyard_and_counts_as_a_card_played() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-094-graveyard",
                "p1": { "hand": [GREED, "core-005"], "library": [FILLER_SPELL] },
                "p2": { "hand": ["core-005"] },
            }));

            s.play(GREED, json!({}));

            s.expect_in_zone(GREED, "graveyard");
            assert_eq!(s.state().players[P1].turn_log.cards_played, 1);
            s.expect_events(json!(["cardPlayed", "manaChanged", "enteredGraveyard"]));
        }

        #[test]
        fn the_mana_goes_to_the_caster_not_the_opponent() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-094-mana-side",
                "p1": { "hand": [GREED, "core-005"], "library": [FILLER_SPELL] },
                "p2": { "hand": ["core-005"], "mana": 4 },
            }));

            s.play(GREED, json!({}));

            s.expect_mana(P1, 2);
            s.expect_mana(P2, 4);
        }
    }

    mod n94_genn_s_greed_radiant_mana {
        use super::*;

        #[test]
        fn s8_gain_6_only_the_number_changes_and_6_takes_current_mana_past_max_mana() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-094-mana-radiant",
                "p1": { "hand": [{ "def": GREED, "radiant": true }, "core-005"], "library": [FILLER_SPELL] },
                "p2": { "hand": ["core-005"] },
            }));

            s.play(GREED, json!({}));

            // 4 − 4 cost + 6 gained; §2.3 lets current sit above the max of 4.
            s.expect_mana(P1, 6);
            assert_eq!(s.state().players[P1].mana.max, 4);
        }

        #[test]
        fn radiant_keeps_the_same_cost_of_4_s8_conventions_only_the_gain_moved() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-094-radiant-cost",
                "p1": { "hand": [{ "def": GREED, "radiant": true }, "core-005"], "library": [FILLER_SPELL], "mana": 3 },
                "p2": { "hand": ["core-005"] },
            }));

            s.expect_refused_with(|s| s.play(GREED, json!({})), "costs 4");
        }
    }

    // =========================================================================================
    // the draw clause — §8 "Draw every 2-cost card from your library"
    // =========================================================================================

    mod n94_genn_s_greed_the_2_cost_draw_r66 {
        use super::*;

        #[test]
        fn draws_every_2_cost_card_out_of_the_library_and_leaves_every_other_cost_behind() {
            crate::register_all();
            let mut s = greed_board("core-094-draw", false);

            s.play(GREED, json!({}));

            let after = zones(&s);
            // The one 2-cost card in the library is now in hand; the library keeps the rest.
            assert!(has(&after.hand, TWO_COST[0]));
            assert!(!has(&after.library, TWO_COST[0]));
            assert!(has(&after.library, FOUR_COST));
        }

        #[test]
        fn it_is_a_draw_so_each_card_emits_drawn_and_counts_on_the_draw_counter_s2_4() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-094-draw-events",
                "p1": {
                    // §2.5: a unit that could still switch keeps the turn from auto-ending, whose draw would
                    // otherwise land on the global draw counter this test reads.
                    "field": [KEEP_TURN],
                    "hand": [GREED],
                    "library": [TWO_COST[0], TWO_COST[1], FOUR_COST],
                },
                "p2": { "hand": ["core-005"], "library": [FILLER_SPELL] },
            }));
            let drawn_before = s.state().counters.drawn;

            s.play(GREED, json!({}));

            assert_eq!(
                def_ids(&s.pile(P1, "hand"))
                    .iter()
                    .filter(|id| *id == TWO_COST[0] || *id == TWO_COST[1])
                    .count(),
                2
            );
            assert_eq!(s.state().counters.drawn, drawn_before + 2);
            assert_eq!(s.pile(P1, "library").len(), 1);
        }

        #[test]
        fn r66_the_cost_is_read_at_resolution_so_a_card_discounted_to_2_is_drawn_and_a_2_pushed_to_3_is_not() {
            crate::register_all();
            // HARNESS GAP (reported): no `SideSetup` key seeds `costMod`, and R66's whole point is that the
            // number is `effectiveCost` at resolution rather than the printed cost, so the test writes it.
            let mut s = scenario(json!({
                "seed": "core-094-r66",
                "p1": {
                    "field": [KEEP_TURN],
                    "hand": [GREED],
                    // A printed 3 discounted to 2, and a printed 2 pushed to 3.
                    "library": [ODD_COST[2], TWO_COST[0]],
                },
                "p2": { "hand": ["core-005"], "library": [FILLER_SPELL] },
            }));
            let library = s.pile(P1, "library");
            let discounted = library.first().cloned();
            let inflated = library.get(1).cloned();
            assert_eq!(discounted.as_ref().map(|card| card.def_id.as_str()), Some(ODD_COST[2]));
            assert_eq!(inflated.as_ref().map(|card| card.def_id.as_str()), Some(TWO_COST[0]));
            if let Some(card) = &discounted {
                find_instance_mut(s.state_mut(), &card.id).expect("in the library").cost_mod = -1;
            }
            if let Some(card) = &inflated {
                find_instance_mut(s.state_mut(), &card.id).expect("in the library").cost_mod = 1;
            }

            s.play(GREED, json!({}));

            // The printed-3 card now costs 2, so it is drawn; the printed-2 card now costs 3, so it is odd
            // and exiled instead.
            assert!(has(&def_ids(&s.pile(P1, "hand")), ODD_COST[2]));
            assert!(has(&def_ids(&s.pile(P1, "exile")), TWO_COST[0]));
        }

        #[test]
        fn r66_an_x_cost_card_is_exempt_it_is_never_drawn_by_the_2_cost_clause() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-094-x-draw",
                "p1": { "field": [KEEP_TURN], "hand": [GREED], "library": [X_COST, "core-024"] },
                "p2": { "hand": ["core-005"], "library": [FILLER_SPELL] },
            }));

            s.play(GREED, json!({}));

            // Both X-cost cards are still in the library; R65 reads them as 0 out of play, R66 exempts them.
            let mut library = def_ids(&s.pile(P1, "library"));
            library.sort();
            let mut expected = vec![X_COST.to_string(), "core-024".to_string()];
            expected.sort();
            assert_eq!(library, expected);
            assert_eq!(s.pile(P1, "hand").len(), 0);
        }

        #[test]
        fn r4_the_hand_cap_applies_to_the_draw_and_the_burned_card_is_a_2_cost_so_the_exile_spares_it() {
            crate::register_all();
            // Nine held cards plus Genn's Greed is a full hand; playing it leaves nine, so the first drawn
            // 2-cost fills the tenth slot and the second is burned to the graveyard (R4).
            let mut s = scenario(json!({
                "seed": "core-094-hand-cap",
                "p1": {
                    "field": [KEEP_TURN],
                    "hand": [
                        GREED,
                        FOUR_COST,
                        FOUR_COST,
                        FOUR_COST,
                        FOUR_COST,
                        FOUR_COST,
                        FOUR_COST,
                        FOUR_COST,
                        FOUR_COST,
                        FOUR_COST,
                    ],
                    "library": [TWO_COST[0], TWO_COST[1]],
                },
                "p2": { "hand": ["core-005"], "library": [FILLER_SPELL] },
            }));

            s.play(GREED, json!({}));

            assert_eq!(s.pile(P1, "hand").len(), 10);
            s.expect_events(json!(["burned"]));
            // The burned card is a 2-cost, which is even, so the exile clause leaves it in the graveyard.
            assert!(has(&def_ids(&s.pile(P1, "graveyard")), TWO_COST[1]));
            assert!(!has(&def_ids(&s.pile(P1, "exile")), TWO_COST[1]));
        }
    }

    // =========================================================================================
    // the exile clause — §8 "exile every odd-cost card in your library, hand and GY"
    // =========================================================================================

    mod n94_genn_s_greed_the_odd_cost_exile_r26_r66 {
        use super::*;

        #[test]
        fn r26_every_odd_cost_card_in_the_library_is_exiled() {
            crate::register_all();
            let mut s = greed_board("core-094-exile-library", false);

            s.play(GREED, json!({}));

            let after = zones(&s);
            assert!(has(&after.exile, ODD_COST[0])); // cost 1
            assert!(has(&after.exile, ODD_COST[2])); // cost 3
            assert!(!has(&after.library, ODD_COST[0]));
            assert!(!has(&after.library, ODD_COST[2]));
        }

        #[test]
        fn r26_every_odd_cost_card_in_the_hand_is_exiled() {
            crate::register_all();
            let mut s = greed_board("core-094-exile-hand", false);

            s.play(GREED, json!({}));

            let after = zones(&s);
            assert!(has(&after.exile, ODD_COST[1])); // the cost-1 card that was held
            assert!(!has(&after.hand, ODD_COST[1]));
        }

        #[test]
        fn r26_every_odd_cost_card_in_the_graveyard_is_exiled() {
            crate::register_all();
            let mut s = greed_board("core-094-exile-graveyard", false);

            s.play(GREED, json!({}));

            let after = zones(&s);
            assert!(has(&after.exile, ODD_COST[3])); // cost 3, in the graveyard
            assert!(!has(&after.graveyard, ODD_COST[3]));
        }

        #[test]
        fn an_even_cost_card_is_spared_in_all_three_zones() {
            crate::register_all();
            let mut s = greed_board("core-094-exile-spares-even", false);

            s.play(GREED, json!({}));

            let after = zones(&s);
            // The 4-cost cards in hand and library, and the 2-cost card in the graveyard, all stay put.
            assert!(has(&after.hand, FOUR_COST));
            assert!(has(&after.library, FOUR_COST));
            assert!(has(&after.graveyard, TWO_COST[1]));
            assert!(!has(&after.exile, FOUR_COST));
            assert!(!has(&after.exile, TWO_COST[1]));
        }

        #[test]
        fn r66_an_x_cost_card_is_exempt_from_the_exile_as_well() {
            crate::register_all();
            let mut s = greed_board("core-094-exile-x", false);

            s.play(GREED, json!({}));

            let after = zones(&s);
            assert!(has(&after.library, X_COST));
            assert!(!has(&after.exile, X_COST));
        }

        #[test]
        fn s8_s_order_the_draw_runs_before_the_exile_so_a_drawn_2_cost_card_is_never_exiled() {
            crate::register_all();
            let mut s = greed_board("core-094-order", false);

            s.play(GREED, json!({}));

            let after = zones(&s);
            // The 2-cost card left the library, reached the hand, and the exile — which looks at hands —
            // left it alone because 2 is even.
            assert!(has(&after.hand, TWO_COST[0]));
            assert!(!has(&after.exile, TWO_COST[0]));
        }

        #[test]
        fn the_exile_reaches_your_three_zones_only_the_opponent_s_cards_are_untouched() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-094-own-zones",
                // §2.5: a unit that could still switch keeps the turn from auto-ending and handing p2 a
                // draw off the library this test is watching.
                "p1": { "field": [KEEP_TURN], "hand": [GREED], "library": [ODD_COST[0]] },
                "p2": { "hand": [ODD_COST[1]], "library": [ODD_COST[2]], "graveyard": [ODD_COST[3]] },
            }));

            s.play(GREED, json!({}));

            assert_eq!(s.pile(P2, "exile").len(), 0);
            assert!(has(&def_ids(&s.pile(P2, "hand")), ODD_COST[1]));
            assert!(has(&def_ids(&s.pile(P2, "library")), ODD_COST[2]));
            assert!(has(&def_ids(&s.pile(P2, "graveyard")), ODD_COST[3]));
        }

        #[test]
        fn r55_the_exiles_count_and_the_card_itself_is_a_4_cost_so_it_survives_its_own_clause() {
            crate::register_all();
            let mut s = greed_board("core-094-self", false);

            s.play(GREED, json!({}));

            // Genn's Greed costs 4 (even) and resolves from the `resolving` zone anyway.
            s.expect_in_zone(GREED, "graveyard");
            assert!(s.state().counters.exiled > 0);
        }
    }

    mod n94_genn_s_greed_radiant_keeps_the_draw_and_the_exile {
        use super::*;

        #[test]
        fn s8_conventions_gain_6_restates_the_mana_only_so_both_other_clauses_still_run() {
            crate::register_all();
            let mut s = greed_board("core-094-radiant-clauses", true);

            s.play(GREED, json!({}));

            let after = zones(&s);
            s.expect_mana(P1, 6);
            assert!(has(&after.hand, TWO_COST[0]));
            assert!(has(&after.exile, ODD_COST[0]));
            assert!(has(&after.exile, ODD_COST[1]));
            assert!(has(&after.exile, ODD_COST[3]));
            assert!(has(&after.library, X_COST));
        }
    }

    mod n94_genn_s_greed_which_cards_are_odd_cost_is_decided_once_r66_r135 {
        use super::*;

        #[test]
        fn r135_exiling_one_card_does_not_flip_ceaseless_void_s_parity_halfway_through_the_clause_r66_r55() {
            crate::register_all();
            // Found by the polish-4 edge-case hunt, round 7 (lens "card by card"). Genn's Greed and a
            // Ceaseless Void in hand; one odd-cost card on top of the library, which the exile clause meets
            // before the hand (R135: library, then hand, then graveyard).
            let mut s = scenario(json!({
                "seed": "r7-card-genn-void",
                "p1": { "field": ["core-008"], "hand": [GREED, "core-100"], "library": ["core-005"] },
                "p2": { "hand": ["core-005"], "library": [FILLER_SPELL], "field": ["core-008"] },
            }));
            let before = {
                let c = &s.state().counters;
                c.drawn + c.played + c.destroyed + c.exiled
            };
            // The play counts itself before its Cry runs (§10.5 step 4), and nothing here costs 2, so no
            // draw moves the counters before the exile clause reads the Void's cost (R55).
            let void_cost_at_clause = 100 - (before + 1);
            s.play(GREED, json!({}));
            let void_card = s
                .pile(P1, "hand")
                .into_iter()
                .chain(s.pile(P1, "exile"))
                .find(|card| card.def_id == "core-100");
            assert!(void_card.is_some());
            // The Stockpile (cost 1) is odd and goes; the Void goes exactly when its cost as the clause
            // resolves is odd (R66), not when it is odd after the Stockpile's exile has moved R55's counter.
            s.expect_in_zone("core-005", "exile");
            assert_eq!(
                void_card.is_some_and(|card| matches!(card.zone, Zone::Exile { .. })),
                void_cost_at_clause % 2 == 1
            );
        }
    }
}
