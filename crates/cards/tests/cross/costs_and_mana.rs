//! Port of `packages/cards/test/costs-and-mana.test.ts` (part 27.1).
//!
//! What a card costs outside play, and what the refresh gives (SPEC §2.3, §6.3 Cost and Mana, R24,
//! R48, R65, R66, R78). Found by the polish-4 edge-case hunt, round 4 (docs/polish/4-edge-cases.md,
//! lenses "keywords and layers" and L8); every case here failed before its fix.
//!
//!  - R65 is one cost calculation for an instance, and it applies outside play too ("library, hand,
//!    GY, pools, filters, comparisons"): #69's Recruit filter and #51's brackets read it off the
//!    library card, as #30 Archivist and #94 Genn's Greed already did (R24, R66), so a `costMod` —
//!    kept in every zone, R78 — moves a card between them. Two live Professor Curvatures both test the
//!    cost the flat discounts leave, and neither reads what the other has already lowered.
//!  - §2.3: max mana is min(turns, 4) plus persistent modifiers. The next refresh's one-shot rider
//!    (#24's next-turn mana, #21's lower refresh) moves current mana only, as #6 Mana Well's gain does.
//!  - Round 9, lens "keywords and layers". R65 (amended): a player's discounts (#35, #77, #78) are
//!    prices for a play from the hand, so a library or a graveyard card is read at its own cost: #30
//!    Archivist's "highest" and #94 Genn's Greed's parity do not see the turn's discounts.

use jackioh_engine::testkit::*;

const STOCKPILE: &str = "core-005";
const VANILLA: &str = "core-008";
const STAB: &str = "core-070"; // Spiteful Stab, a printed 3-cost Spell (Flood costs (4) since patch v0.2.0)
const SHREDDER: &str = "core-013"; // Jlockeed Shredder-10, a printed 3-cost Unit
const MENACE: &str = "core-019";
const RAPID: &str = "core-010";
const TIMMY: &str = "core-011"; // a unit on each board, so no turn auto-ends (R82)
const HIT_JOB: &str = "core-016";
const HINDER: &str = "core-021"; // cast on draw: the opponent's next refresh is 1 lower
const DIVIDEND: &str = "core-024"; // X-cost Spell; the "mana" mode gains floor(X/2) next turn
const COST_4: &str = "core-025"; // 4-mana 7/7
const TUTOR: &str = "core-051";
const CALL_TO_ARMS: &str = "core-069";
const CURVATURE: &str = "core-077";
const LIBRARY: [&str; 4] = [VANILLA, VANILLA, VANILLA, VANILLA];

/// The harness with the real catalog and every card script registered (TS's `_harness.ts` import
/// ran `registerAll()`; the engine's testkit cannot name the cards crate, so the cards test does).
fn setup(opts: Value) -> Scenario {
    jackioh_cards::register_all();
    scenario(opts)
}

fn cost_in_hand(s: &Scenario, def_id: &str) -> i32 {
    let card = s
        .state()
        .players
        .p1
        .hand
        .iter()
        .find(|c| c.def_id == def_id)
        .unwrap_or_else(|| panic!("{def_id} is not in p1's hand"));
    mana::effective_cost(s.state(), card, Default::default())
}

fn def_ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.def_id.clone()).collect()
}

/// R65: one cost calculation, in play and out of it
mod r65_one_cost_calculation_in_play_and_out_of_it {
    use super::*;

    #[test]
    fn r65_r48_two_live_professor_curvature_discounts_both_read_the_cost_after_the_flat_discounts_whichever_was_played_first()
     {
        /// Play a base and a radiant Curvature in the given order, then read a 4-cost card next turn.
        fn next_turn_cost(radiant_first: bool) -> i32 {
            let mut s = setup(json!({
                "p1": {
                    "hand": [CURVATURE, { "def": CURVATURE, "radiant": true }, COST_4, STOCKPILE],
                    "library": LIBRARY,
                },
                "p2": { "hand": [STOCKPILE], "field": [MENACE], "library": LIBRARY },
            }));
            let order = if radiant_first { [true, false] } else { [false, true] };
            for radiant in order {
                let card = s
                    .hand("p1")
                    .into_iter()
                    .find(|c| c.def_id == CURVATURE && c.radiant == radiant)
                    .expect("setup");
                s.play(&card, json!({}));
            }
            s.end_turn().end_turn();
            assert_eq!(s.state().active, PlayerId::P1);
            cost_in_hand(&s, COST_4)
        }
        // R65: "add player discounts; apply Professor Curvature if the result is then 4". The result
        // of the discounts is 4 for both modifiers, so both apply, 4 - 1 - 2 = 1, and the order the
        // two Curvatures were played in cannot change what a card costs.
        assert_eq!([next_turn_cost(false), next_turn_cost(true)], [1, 1]);
    }

    #[test]
    fn r65_r24_r66_r78_call_to_arms_recruits_a_library_unit_whose_cost_is_1_as_archivist_and_genns_greed_read_it() {
        let mut s = setup(json!({
            "p1": {
                "hand": [CALL_TO_ARMS, RAPID],
                // #95's "every card in your hand and library costs 2 less" leaves exactly this costMod.
                "library": [{ "def": SHREDDER, "costMod": -2 }, STOCKPILE, STOCKPILE],
            },
            "p2": { "hand": [STOCKPILE], "field": [MENACE], "library": LIBRARY },
        }));
        let shredder = s
            .pile("p1", "library")
            .into_iter()
            .find(|c| c.def_id == SHREDDER)
            .expect("setup");
        // R65's one calculation for an instance, which #30 and #94 read for library cards: 3 - 2 = 1.
        assert_eq!(mana::effective_cost(s.state(), &shredder, Default::default()), 1);

        s.play(CALL_TO_ARMS, json!({}));
        // "Recruit 3 Units costing 1 or less": the Shredder costs 1, so it is recruited.
        s.expect_in_zone(&shredder, "field");
    }

    #[test]
    fn r65_r24_kys_private_tutor_offers_the_bracket_a_library_cards_cost_puts_it_in_s8_c51() {
        let mut s = setup(json!({
            "p1": {
                "hand": [TUTOR, RAPID],
                "library": [{ "def": STAB, "costMod": -2 }],
            },
            "p2": { "hand": [STOCKPILE], "field": [MENACE], "library": LIBRARY },
        }));
        let stab = s.pile("p1", "library").into_iter().next().expect("setup");
        assert_eq!(mana::effective_cost(s.state(), &stab, Default::default()), 1);

        s.play(TUTOR, json!({}));
        s.answer(json!("Spell"));
        let brackets = s
            .state()
            .pending
            .clone()
            .expect("the bracket prompt should be open");
        // The Stab costs 1, so the one bracket with a match is "0-1".
        let offered: Vec<String> = brackets
            .options
            .iter()
            .map(|o| match &o.selection {
                Selection::Mode { option } => option.clone(),
                _ => o.key.clone(),
            })
            .collect();
        assert_eq!(offered, ["0-1"]);
    }
}

/// §2.3: a one-shot refresh rider moves current mana, not max mana
mod s2_3_a_one_shot_refresh_rider_moves_current_mana_not_max_mana {
    use super::*;

    #[test]
    fn s2_3_efficiency_dividends_next_turn_mana_and_hinders_lower_refresh_leave_max_mana_at_min_turns_4_as_mana_wells_gain_does_s6_3_mana()
     {
        // Efficiency Dividend, X = 4, mana mode: floor(4/2) = 2 more at p1's next refresh.
        let mut dividend = setup(json!({
            "seed": "edge-l8-dividend-max",
            "p1": { "hand": [DIVIDEND, HIT_JOB], "field": [TIMMY] },
            "p2": { "hand": [STOCKPILE], "field": [TIMMY] },
        }));
        dividend.play(DIVIDEND, json!({ "x": 4, "modes": ["mana"] }));
        dividend.end_turn(); // p2's turn.
        dividend.end_turn(); // p1's turn: the refresh the rider was stored for.

        assert_eq!(dividend.state().active, PlayerId::P1);
        // The rider is temporary mana on top of the refresh (§2.3, §6.3)...
        dividend.expect_mana("p1", 6);
        // ...and max mana is still min(turns started, 4), exactly as it is after Mana Well's gain.
        assert_eq!(dividend.view("p1").you.mana, ManaView { current: 6, max: 4 });

        // Hinder, drawn and cast at p1's start of turn: p2's next refresh is 1 lower.
        let mut hinder = setup(json!({
            "seed": "edge-l8-hinder-max",
            "p1": { "hand": [HIT_JOB], "field": [TIMMY], "library": [HINDER, TIMMY, TIMMY, TIMMY] },
            "p2": { "hand": [STOCKPILE], "field": [TIMMY], "library": [STOCKPILE, STOCKPILE, STOCKPILE] },
        }));
        hinder.end_turn(); // p2's turn.
        hinder.end_turn(); // p1's turn: the draw casts Hinder.
        // R682: the base face's "Discard 1" is random, so no prompt opens and the one card held goes.
        assert!(hinder.state().pending.is_none());
        hinder.expect_in_zone(HIT_JOB, "graveyard");
        hinder.end_turn(); // p2's turn: the lowered refresh.

        assert_eq!(hinder.state().active, PlayerId::P2);
        hinder.expect_mana("p2", 3);
        assert_eq!(hinder.view("p2").you.mana, ManaView { current: 3, max: 4 });
    }
}

// ---------------------------------------------------------------------------
// Round 9: a play-time discount is not a library or graveyard card's cost (R65, R48, §8 #35, #77, #78)
// ---------------------------------------------------------------------------

const RAPID_REPLENISH: &str = "core-010"; // Spell, 0
const SEVEN_SEVEN: &str = "core-025"; // Unit, 4
const ARCHIVIST: &str = "core-030";
const LUNAR_ECLIPSE: &str = "core-035";
const FULLSEND: &str = "core-078"; // Spell, 4
const GREED: &str = "core-094";

fn at_p2() -> Value {
    json!([{ "pick": "hero", "player": "p2" }])
}

/// A play-time discount is not a library or graveyard card's cost (§6.3 Cost, R48, R65, §8 #35, #77, #78)
mod a_play_time_discount_is_not_a_library_or_graveyard_cards_cost {
    use super::*;

    #[test]
    fn r65_r48_r24_archivist_on_the_curvature_turn_draws_the_4_cost_card_as_the_librarys_highest_not_a_3_cost_one_nearer_the_top()
     {
        let mut g = setup(json!({
            "p1": {
                "hand": [CURVATURE, ARCHIVIST, HINDER],
                "field": [{ "def": VANILLA, "lane": 1 }],
                // The turn's draw takes the top Vanilla; Archivist then reads a 3-cost Unit above a 4-cost one.
                "library": [VANILLA, SHREDDER, SEVEN_SEVEN],
            },
            "p2": { "hand": [HINDER, HINDER], "field": [{ "def": VANILLA, "lane": 1 }], "library": LIBRARY },
        }));
        g.play(CURVATURE, json!({ "zone": 2 }));
        g.end_turn().end_turn();
        assert_eq!(g.state().active, PlayerId::P1);
        assert_eq!(def_ids(&g.pile("p1", "library")), [SHREDDER, SEVEN_SEVEN]);

        g.play(ARCHIVIST, json!({ "zone": 3, "modes": ["highest"] }));

        // R24: "highest" is the 7/7 at 4. Curvature prices a 4-cost card PLAYED this turn (R48); it does
        // not make the 7/7 in the library a 3 that ties with Shredder and loses to its place in the pile.
        assert!(def_ids(&g.hand("p1")).contains(&SEVEN_SEVEN.to_string()));
        assert_eq!(def_ids(&g.pile("p1", "library")), [SHREDDER]);
    }

    #[test]
    fn r65_r24_archivist_after_lunar_eclipse_still_reads_a_library_spell_at_its_own_cost_a_4_cost_spell_is_higher_than_a_3_cost_unit_s8_c35()
     {
        let mut g = setup(json!({
            "p1": {
                "hand": [LUNAR_ECLIPSE, ARCHIVIST, HINDER],
                "library": [SHREDDER, FULLSEND],
            },
            "p2": { "hand": [HINDER], "library": LIBRARY },
        }));
        g.play(LUNAR_ECLIPSE, json!({ "targets": at_p2() }));
        g.play(ARCHIVIST, json!({ "zone": 1, "modes": ["highest"] }));

        // "The next Spell you play this turn costs 1 less" is a price for playing /fullsend, not what it
        // costs lying in the library, so /fullsend at 4 is the highest card there.
        assert!(def_ids(&g.hand("p1")).contains(&FULLSEND.to_string()));
        assert_eq!(def_ids(&g.pile("p1", "library")), [SHREDDER]);
    }

    #[test]
    fn r65_r66_genns_greed_played_under_fullsend_exiles_no_even_cost_card_from_the_library_or_the_graveyard_fullsend_itself_included_s8_c78_c94()
     {
        let mut g = setup(json!({
            "p1": {
                // The Radiant /fullsend, the face that still grants "Combo: Draw 1" (patch v0.1.1).
                "hand": [{ "def": FULLSEND, "radiant": true }, GREED],
                // A unit that can still switch keeps §2.5's auto-end from passing the turn mid-test.
                "field": [VANILLA],
                // /fullsend's granted "Combo: draw 1" takes the 0-cost Spell on top before Genn's text runs;
                // a 0 reads 0 in the hand whatever the discount, so the hand's parity is not in question.
                "library": [RAPID_REPLENISH, SEVEN_SEVEN],
            },
            "p2": { "hand": [HINDER], "field": [VANILLA], "library": LIBRARY },
        }));
        g.play(FULLSEND, json!({}));
        g.play(GREED, json!({}));

        assert_eq!(def_ids(&g.hand("p1")), [RAPID_REPLENISH]);
        // "Exile every odd-cost card in your library, hand and GY": the 7/7 costs 4 in the library, and
        // /fullsend, which step 7 landed in the graveyard, costs 4 there. "This turn your cards cost 1
        // less" is what they would cost to PLAY this turn; it does not make either of them a 3.
        assert_eq!(def_ids(&g.pile("p1", "exile")), Vec::<String>::new());
        assert_eq!(def_ids(&g.pile("p1", "library")), [SEVEN_SEVEN]);
        assert!(def_ids(&g.pile("p1", "graveyard")).contains(&FULLSEND.to_string()));
    }
}
