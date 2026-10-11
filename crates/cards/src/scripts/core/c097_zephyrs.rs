//! #97 Zephyrs (SPEC §8.5, §10.7's scorer bullet, §6.3 Discover and Exile, R29). Spell, cost 0.
//!   Base:    "Discover the 'perfect' card from the Core set; exile this"
//!   Radiant: "A perfect Radiant card" — the cell restates only which card the Discover hands over,
//!            so "exile this" is kept and the pick arrives Radiant (§8 Conventions).
//!   Engine:  "Scorer in 10.7 ranks every non-token Core card except #97 for the current state;
//!            Discover offers the top 3 (R29)".
//! The ranking is the subsystem's (§10.7): `subsystems/scorer.rs` owns every weight and filter, so
//! this card only asks it for three ids and passes `{ radiant }`, so the radiant face ranks radiant
//! faces (`ScorerOptions`, §5.2). The ids are a `defId` pool (§5.1), so only those three are offered;
//! tokens and #97 are out already (R29, B2.6, R387). The exile is the second effect, as §8.5 prints
//! it: `apply_resumable` parks it on `state.work` and drains it after the answer (§10.6). §10.5
//! step 7 sends a resolved Spell to the graveyard "or exile": `exile` bumps R55's counter.

use jackioh_engine::effects::{add_to_hand, chosen_options, discover_from_catalog, exile};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-097";

/// The one resume step: the Discover's answer comes back here (§10.6, `prompts::RESUME_HOOK`).
const PICKED: &str = "picked";

/// R29: "Discover shows the top 3" — §6.3's Discover is 1 of 3 either way, said explicitly.
const OFFERED: i32 = 3;

/// The two faces differ only in which face the scorer ranks and which face reaches the hand.
fn zephyrs(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            let mut discover: DiscoverFromCatalogArgs = json_as(json!({
                "step": PICKED,
                "count": OFFERED,
                "prompt": if radiant { "Discover a perfect Radiant card" } else { "Discover the perfect card" },
            }));
            // R29: the scorer's top three, by id, for the state as the Discover meets it. A function, so
            // the ranking is made when the Discover applies, and not again when the hook is rebuilt to
            // resume "exile this" after the answer (§10.7's dry run plays every candidate).
            discover.query_fn = Some(Arc::new(move |at| {
                let ids: Vec<String> = subsystems::top_three(
                    at.state,
                    at.controller,
                    &subsystems::ScorerOptions {
                        radiant: Some(radiant),
                    },
                )
                .iter()
                .map(|scored| scored.def.id.clone())
                .collect();
                json_as::<CatalogQueryArgs>(json!({ "defId": ids }))
            }));
            vec![
                discover_from_catalog(discover),
                exile(json_as(json!({ "target": { "of": "self" } }))),
            ]
        })),
        resume: IndexMap::from([(
            PICKED,
            // A Discover's pick arrives as a `mode` selection carrying a catalog id (§10.6).
            hook(move |ctx| {
                // The prompt only ever offers three real ids, so this is the "no answer at all" case.
                let Some(picked) = chosen_options(ctx).into_iter().next() else {
                    return vec![];
                };
                vec![add_to_hand(json_as(json!({ "defId": picked, "radiant": radiant })))]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: zephyrs(false),
        radiant: zephyrs(true),
    }
}

// BUILD M4-T4 row 97: "Scorer deterministic; a lethal-enabling card ranks first when lethal
// exists; Discover offers the top 3 (R29); exiled; radiant picks are radiant". §10.7's dry run plays
// each candidate on a copy of the state and reads only what its player may read (R222).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const ZEPHYRS: &str = "core-097";
    /// #45 Deft Duelist, 4/3 Charge for 2: the one base face the scorer can read as "enables lethal".
    const CHARGER: &str = "core-045";
    /// #11 Tempo Timmy, 3/3: board damage that is already pointed at the hero.
    const BOARD: &str = "core-011";
    /// #19 Midrange Menace, a second hand card, so no play empties the hand (R82).
    const MENACE_FILLER: &str = "core-019";

    use crate::js;

    fn open(state: &GameState) -> Value {
        match &state.pending {
            Some(pending) => js(pending),
            None => panic!("#97 opened no prompt"),
        }
    }

    /// A Discover's options are `mode` selections carrying catalog ids (§10.6).
    fn option_ids(pending: &Value) -> Vec<String> {
        pending["options"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter(|option| option["selection"]["pick"] == "mode")
            .map(|option| option["selection"]["option"].as_str().unwrap_or_default().to_string())
            .collect()
    }

    fn sorted(ids: &[String]) -> Vec<String> {
        let mut out = ids.to_vec();
        out.sort();
        out
    }

    fn options(radiant: Option<bool>) -> subsystems::ScorerOptions {
        subsystems::ScorerOptions { radiant }
    }

    fn priority_of(scored: &subsystems::Scored) -> Value {
        js(&scored.priority)
    }

    mod n97_zephyrs_base {
        use super::*;

        #[test]
        fn r29_offers_exactly_the_scorer_s_top_3_and_nothing_else() {
            crate::register_all();
            let mut s = scenario(json!({ "seed": "zephyrs-top3", "p1": { "hand": [ZEPHYRS] } }));
            s.play(ZEPHYRS, json!({}));

            let pending = open(s.state());
            assert_eq!(pending["kind"], "discover");
            assert_eq!(pending["playerId"], "p1");
            assert_eq!(pending["min"], 1);
            assert_eq!(pending["max"], 1);
            assert_eq!(pending["options"].as_array().map(Vec::len), Some(3));

            // The state the card read is this one: the spell is resolving and the prompt is open.
            let top: Vec<String> = subsystems::top_three(s.state(), P1, &options(None))
                .iter()
                .map(|scored| scored.def.id.clone())
                .collect();
            assert_eq!(sorted(&option_ids(&pending)), sorted(&top));
        }

        #[test]
        fn r29_never_offers_zephyrs_itself_and_never_offers_a_token_s5_1() {
            crate::register_all();
            let mut s = scenario(json!({ "seed": "zephyrs-self", "p1": { "hand": [ZEPHYRS] } }));
            s.play(ZEPHYRS, json!({}));

            let ids = option_ids(&open(s.state()));
            assert!(!ids.contains(&ZEPHYRS.to_string()));
            for id in &ids {
                let candidates = subsystems::candidate_defs();
                let def = candidates.iter().find(|candidate| &candidate.id == id);
                assert!(def.is_some());
                let def = js(&def);
                assert_eq!(def["token"], false);
                assert_eq!(def["set"], "Core");
            }
        }

        #[test]
        fn s10_7_the_ranking_is_deterministic_the_same_board_offers_the_same_3_under_any_seed() {
            crate::register_all();
            let board = |seed: &str| -> Value {
                json!({
                    "seed": seed,
                    "p1": { "hand": [ZEPHYRS], "field": [BOARD] },
                    "p2": { "field": [BOARD], "health": 20 },
                })
            };
            let mut a = scenario(board("zephyrs-a"));
            let mut b = scenario(board("zephyrs-b"));
            a.play(ZEPHYRS, json!({}));
            b.play(ZEPHYRS, json!({}));

            // The scorer takes no rng at all, so only the order the three are OFFERED in can differ.
            assert_eq!(sorted(&option_ids(&open(a.state()))), sorted(&option_ids(&open(b.state()))));
        }

        #[test]
        fn s10_7_lethal_available_max_a_lethal_enabling_card_ranks_first_when_lethal_exists() {
            crate::register_all();
            // p1's 3/3 can already swing at the hero for 3; #45 adds 4 with Charge on its summon turn, and
            // costs 2 of p1's 4 mana, so 3 + 4 ≥ p2's 7 health is lethal available this turn.
            let mut s = scenario(json!({
                "seed": "zephyrs-lethal",
                "p1": { "hand": [ZEPHYRS], "field": [BOARD] },
                "p2": { "health": 7 },
            }));
            s.play(ZEPHYRS, json!({}));

            let ranked = subsystems::rank(s.state(), P1, &options(None));
            assert_eq!(ranked.first().map(|scored| scored.def.id.clone()).as_deref(), Some(CHARGER));
            assert_eq!(ranked.first().map(priority_of), Some(json!("lethal")));
            // …and the card offers it, which is the whole of what #97 contributes.
            assert!(option_ids(&open(s.state())).contains(&CHARGER.to_string()));
        }

        #[test]
        fn s8_5_the_pick_goes_to_the_caster_s_hand_not_radiant_on_the_base_face() {
            crate::register_all();
            let mut s = scenario(json!({ "seed": "zephyrs-hand", "p1": { "hand": [ZEPHYRS] } }));
            s.play(ZEPHYRS, json!({}));
            let picked = option_ids(&open(s.state())).into_iter().next().unwrap_or_default();
            s.answer(json!(picked));

            assert!(s.state().pending.is_none());
            let held: Vec<CardInstance> = s.hand(P1).into_iter().filter(|card| card.def_id == picked).collect();
            assert_eq!(held.len(), 1);
            assert!(!held[0].radiant);
            s.expect_events(json!(["cardPlayed", "promptOpened", "addedToHand"]));
        }

        #[test]
        fn s8_5_exile_this_the_spell_reaches_the_exile_pile_not_the_graveyard() {
            crate::register_all();
            let mut s = scenario(json!({ "seed": "zephyrs-exile", "p1": { "hand": [ZEPHYRS] } }));
            s.play(ZEPHYRS, json!({}));
            let first = option_ids(&open(s.state())).into_iter().next().unwrap_or_default();
            s.answer(json!(first));

            s.expect_in_zone(ZEPHYRS, "exile");
            assert!(!s.pile(P1, "graveyard").iter().any(|card| card.def_id == ZEPHYRS));
            assert!(s.events().iter().any(|event| js(event)["type"] == "exiled"));
            // R55: an exile is one of Ceaseless Void's four game counters.
            assert!(s.state().counters.exiled >= 1);
        }

        #[test]
        fn s8_5_costs_0_it_is_castable_with_no_mana_at_all() {
            crate::register_all();
            let mut s = scenario(json!({ "seed": "zephyrs-free", "p1": { "hand": [ZEPHYRS], "mana": 0 } }));
            s.play(ZEPHYRS, json!({}));
            s.expect_mana(P1, 0);
        }
    }

    mod n97_zephyrs_radiant {
        use super::*;

        #[test]
        fn s8_5_a_perfect_radiant_card_the_pick_arrives_radiant() {
            crate::register_all();
            let mut s = scenario(json!({ "seed": "zephyrs-radiant", "p1": { "hand": [ZEPHYRS] } }));
            s.card_mut(ZEPHYRS).radiant = true;
            s.play(ZEPHYRS, json!({}));

            let picked = option_ids(&open(s.state())).into_iter().next().unwrap_or_default();
            s.answer(json!(picked));

            let held: Vec<CardInstance> = s.hand(P1).into_iter().filter(|card| card.def_id == picked).collect();
            assert_eq!(held.len(), 1);
            assert!(held[0].radiant);
            s.expect_events(json!(["cardPlayed", "promptOpened", "addedToHand"]));
        }

        #[test]
        fn s5_2_the_radiant_face_ranks_the_radiant_faces_so_the_top_3_is_the_radiant_top_3() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "zephyrs-radiant-rank",
                "p1": { "hand": [ZEPHYRS], "field": [BOARD] },
                "p2": { "field": [BOARD], "health": 20 },
            }));
            s.card_mut(ZEPHYRS).radiant = true;
            s.play(ZEPHYRS, json!({}));

            let top: Vec<String> = subsystems::top_three(s.state(), P1, &options(Some(true)))
                .iter()
                .map(|scored| scored.def.id.clone())
                .collect();
            assert_eq!(sorted(&option_ids(&open(s.state()))), sorted(&top));
        }

        #[test]
        fn s8_5_exile_this_is_kept_the_radiant_cell_restates_only_which_card_is_discovered() {
            crate::register_all();
            let mut s = scenario(json!({ "seed": "zephyrs-radiant-exile", "p1": { "hand": [ZEPHYRS] } }));
            s.card_mut(ZEPHYRS).radiant = true;
            s.play(ZEPHYRS, json!({}));
            let first = option_ids(&open(s.state())).into_iter().next().unwrap_or_default();
            s.answer(json!(first));

            s.expect_in_zone(ZEPHYRS, "exile");
        }
    }

    // §10.7's priorities for cards whose claim lives in their text.

    /// A Discover's options are `mode` selections carrying catalog ids (§10.6).
    fn offered_ids(s: &Scenario) -> Vec<String> {
        option_ids(&open(s.state()))
    }

    /// Every Core card whose own text heals its controller's hero: #5 Stockpile, #24 Efficiency Dividend,
    /// #47 Fig of Life, #53 Reno, #56 Jilliax (Lifesteal), #74 Adaptive UI and #95 Call to Chaos. #19 Midrange
    /// Menace and #93 Combo-Index are allowed too, so a scorer that reads "heals" more loosely still passes.
    const HEALERS: &[&str] = &[
        "core-005", "core-024", "core-047", "core-053", "core-056", "core-074", "core-095", "core-019", "core-093",
    ];

    /// Every Core card whose own text removes a whole enemy board of ordinary units: #17 Flood, #43 Big
    /// Felinor, #88 Twisting Nether and #100 Ceaseless Void. #87 Pocket Chaos, #16 Hit Job and #55 Lava
    /// Golem are allowed too; Lava Golem's Tribute 3 may take enemy units (R101), so it is paid with
    /// the three 7/7s.
    const CLEARERS: &[&str] = &["core-017", "core-043", "core-088", "core-100", "core-087", "core-016", "core-055"];

    /// Every Core card that, played from p1's 4 mana, deals 3 or more to the enemy hero this turn:
    /// #24 Efficiency Dividend (X damage to a target), #35 Lunar Eclipse (3 damage), #44 True Strike (4,
    /// ignoring Armor), #45 Deft Duelist (a 4/3 with Charge), #68 Twisted Sorcerer (Cry: 4 damage) and
    /// #74 Adaptive UI (X damage).
    const LETHAL_AT_3: &[&str] = &["core-024", "core-035", "core-044", "core-045", "core-068", "core-074"];

    fn outside(offered: &[String], allowed: &[&str]) -> Vec<String> {
        offered
            .iter()
            .filter(|id| !allowed.contains(&id.as_str()))
            .cloned()
            .collect()
    }

    mod n97_zephyrs_s10_7_s_dry_run_reads_what_a_card_s_text_does {
        use super::*;

        #[test]
        fn s10_7_hero_below_10_and_card_heals_high_at_5_health_all_three_offers_are_cards_that_heal() {
            crate::register_all();
            // Nothing is lethal (p2 is at 30 with no board on either side) and there is no enemy board to
            // clear, so the heal criterion is the highest one that applies. Seven Core cards heal their
            // controller's hero, so a scorer that ranks every one of them "high" offers three of them.
            let mut s = scenario(json!({
                "seed": "r5-zephyrs-heal",
                "p1": { "hand": [ZEPHYRS, MENACE_FILLER], "health": 5 },
            }));
            s.play(ZEPHYRS, json!({}));
            let offered = offered_ids(&s);
            assert_eq!(offered.len(), 3);
            assert_eq!(outside(&offered, HEALERS), Vec::<String>::new(), "offers that do not heal");
        }

        #[test]
        fn s10_7_can_clear_the_enemy_board_high_facing_three_7_7s_all_three_offers_clear_the_board() {
            crate::register_all();
            // Three #25 4-mana 7/7s (Armor 7): no unit in Core answers one alone, let alone three, while
            // Flood, Big Felinor, Twisting Nether and Ceaseless Void each remove the whole board by text.
            let mut s = scenario(json!({
                "seed": "r5-zephyrs-clear",
                "p1": { "hand": [ZEPHYRS, MENACE_FILLER] },
                "p2": { "field": ["core-025", "core-025", "core-025"] },
            }));
            s.play(ZEPHYRS, json!({}));
            let offered = offered_ids(&s);
            assert_eq!(offered.len(), 3);
            assert_eq!(outside(&offered, CLEARERS), Vec::<String>::new(), "offers that do not clear the board");
        }

        #[test]
        fn s10_7_lethal_available_max_with_the_enemy_hero_at_3_all_three_offers_deal_the_last_3() {
            crate::register_all();
            // p1 has 4 mana (Zephyrs costs 0) and p2 has no board, so any card that puts 3 damage on the
            // enemy hero this turn is lethal: a burn spell as much as a Charge unit.
            let mut s = scenario(json!({
                "seed": "r5-zephyrs-lethal",
                "p1": { "hand": [ZEPHYRS, MENACE_FILLER] },
                "p2": { "health": 3 },
            }));
            s.play(ZEPHYRS, json!({}));
            let offered = offered_ids(&s);
            assert_eq!(offered.len(), 3);
            assert_eq!(outside(&offered, LETHAL_AT_3), Vec::<String>::new(), "offers that are not lethal");
        }
    }

    /// #8 Mr. Vanilla: a plain body that keeps each side's turn from auto-ending (R82).
    const VANILLA: &str = "core-008";
    const LIBRARY: [&str; 6] = [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA];
    /// #25 4-mana 7/7 with Armor 7: no Core unit's printed attack answers it.
    const SEVEN_SEVEN: &str = "core-025";
    /// #1 Big D-fender, a 0/8: a friendly body that never attacks, so nothing on it is lethal.
    const D_FENDER: &str = "core-001";

    fn must_have<T>(value: Option<T>, what: &str) -> T {
        value.unwrap_or_else(|| panic!("setup: {what} is missing"))
    }

    /// The catalog ids a Discover of #97's offers, sorted.
    fn offered_defs(g: &Scenario) -> Vec<String> {
        sorted(&option_ids(&open(g.state())))
    }

    /// §10.7's ranking for the active player now, the one #97's Discover reads (R29).
    fn scored_on(g: &Scenario, def_id: &str) -> subsystems::Scored {
        must_have(
            subsystems::rank(g.state(), P1, &options(None))
                .into_iter()
                .find(|entry| entry.def.id == def_id),
            &format!("{def_id}'s score"),
        )
    }

    mod n97_zephyrs_r222_the_dry_run_reads_only_what_its_player_may_read {
        use super::*;

        #[test]
        fn r222_the_cards_n97_offers_do_not_depend_on_which_face_down_trap_the_opponent_holds_s9_1_s10_8_r33() {
            crate::register_all();
            // Two games p1 cannot tell apart: p2 holds one face-down trap in the same zone, a My Pawn in
            // one and a Sheepish in the other. The dry run plays each candidate on a copy of the real
            // state, so the Sheepish turns Big Felinor and Bigot into Sheep before their Cries and they
            // stop "clearing" p2's board: the offers name the trap.
            let offers_with = |trap: &str| -> Vec<String> {
                let mut g = scenario(json!({
                    "seed": "zephyrs-hidden-trap",
                    "p1": { "hand": [ZEPHYRS, VANILLA], "library": LIBRARY },
                    "p2": {
                        "hand": [VANILLA],
                        "field": [{ "def": SEVEN_SEVEN, "lane": 3 }],
                        "backrow": [{ "def": trap, "lane": 2, "faceUp": false }],
                        "library": LIBRARY,
                    },
                }));
                g.play(ZEPHYRS, json!({}));
                offered_defs(&g)
            };

            assert_eq!(offers_with("core-041"), offers_with("core-096"));
        }

        #[test]
        fn r222_the_cards_n97_offers_do_not_depend_on_the_order_of_the_viewer_s_own_library_s3_s9_1() {
            crate::register_all();
            // p1 is at 5 health, so a heal is the priority, and #5 Stockpile ("draw 2; heal your hero 2")
            // is one. With two #27 Blood Ridden Glowy Jelly Beans on top of the library the dry run's draw
            // casts them, p1 loses 10 and Stockpile stops "healing": the offers say what is on top.
            let offers_with = |library: &[&str]| -> Vec<String> {
                let mut g = scenario(json!({
                    "seed": "zephyrs-library-order",
                    "p1": { "hand": [ZEPHYRS, VANILLA], "library": library, "health": 5 },
                    "p2": { "hand": [VANILLA], "library": LIBRARY },
                }));
                g.play(ZEPHYRS, json!({}));
                offered_defs(&g)
            };

            let beans_below = offers_with(&[VANILLA, VANILLA, "core-027", "core-027", VANILLA, VANILLA]);
            let beans_on_top = offers_with(&["core-027", "core-027", VANILLA, VANILLA, VANILLA, VANILLA]);
            assert_eq!(beans_on_top, beans_below);
        }
    }

    mod n97_zephyrs_s10_7_lethal_available_and_can_clear_the_enemy_board_as_the_board_has_them {
        use super::*;

        #[test]
        fn s10_7_a_charge_unit_s_swing_is_read_through_the_viewer_s_auras_both_ways_s10_4_layer_5() {
            crate::register_all();
            // #45 Deft Duelist is a printed 4/3 with Charge. Under p1's #14 Jlockeed's Weapons it swings
            // for 8 the turn it lands, which is lethal on a hero at 8; under p1's own #65.1 Spikey Pillow
            // it swings for 2, which is not lethal on a hero at 4. The scorer reads the printed 4 both
            // times, and the Duelist has no text for the dry run to play.
            let mut under_weapons = scenario(json!({
                "seed": "zephyrs-weapons",
                "p1": { "hand": [ZEPHYRS, VANILLA], "backrow": ["core-014"], "library": LIBRARY },
                "p2": { "hand": [VANILLA], "health": 8, "library": LIBRARY },
            }));
            let under_pillow = scenario(json!({
                "seed": "zephyrs-pillow",
                "p1": { "hand": [ZEPHYRS, VANILLA], "field": [{ "def": "core-065-1", "lane": 1 }], "library": LIBRARY },
                "p2": { "hand": [VANILLA], "health": 4, "library": LIBRARY },
            }));

            assert_eq!(priority_of(&scored_on(&under_weapons, "core-045")), json!("lethal"));
            assert_ne!(priority_of(&scored_on(&under_pillow, "core-045")), json!("lethal"));

            // …and what p1 is offered follows: the lethal card is on the table.
            under_weapons.play(ZEPHYRS, json!({}));
            assert!(offered_defs(&under_weapons).contains(&"core-045".to_string()));
        }

        #[test]
        fn s10_7_the_dry_run_reaches_the_lone_enemy_unit_however_many_permanents_the_viewer_has() {
            crate::register_all();
            // #34 Collateral Damage exiles one target permanent, and p2 has one unit: it clears p2's board.
            // With nine permanents of p1's own ahead of it among the targets, the dry run's eight plays
            // never aim at p2's unit, and the clear is missed. Four fewer friendly permanents and it is seen.
            let g = scenario(json!({
                "seed": "zephyrs-crowded",
                "p1": {
                    "hand": [ZEPHYRS, VANILLA],
                    "field": [D_FENDER, D_FENDER, D_FENDER, D_FENDER, D_FENDER],
                    "backrow": ["core-006", "core-006", "core-006", "core-006"],
                    "library": LIBRARY,
                },
                "p2": { "hand": [VANILLA], "field": [{ "def": SEVEN_SEVEN, "lane": 5 }], "library": LIBRARY },
            }));

            assert_eq!(priority_of(&scored_on(&g, "core-034")), json!("clear"));
        }

        #[test]
        fn s10_7_the_dry_run_sees_a_radiant_n55_lava_golem_clear_the_enemy_board_by_tributing_the_enemy_s_units_s8_n55_r101() {
            crate::register_all();
            // Lava Golem "can use opposing Units as Tributes": paid with p2's three 7/7s the Radiant face
            // leaves p2 no board.
            let g = scenario(json!({
                "seed": "zephyrs-golem",
                "p1": { "hand": [ZEPHYRS, VANILLA], "field": [D_FENDER, D_FENDER, D_FENDER], "library": LIBRARY },
                "p2": { "hand": [VANILLA], "field": [SEVEN_SEVEN, SEVEN_SEVEN, SEVEN_SEVEN], "library": LIBRARY },
            }));

            let radiant_golem = subsystems::rank(g.state(), P1, &options(Some(true)))
                .into_iter()
                .find(|entry| entry.def.id == "core-055");
            assert_eq!(radiant_golem.as_ref().map(priority_of), Some(json!("clear")));
        }

        #[test]
        fn r360_the_dry_run_sees_the_base_lava_golem_clear_nothing_paid_with_the_enemy_s_units_it_is_summoned_for_them() {
            crate::register_all();
            let g = scenario(json!({
                "seed": "zephyrs-golem",
                "p1": { "hand": [ZEPHYRS, VANILLA], "field": [D_FENDER, D_FENDER, D_FENDER], "library": LIBRARY },
                "p2": { "hand": [VANILLA], "field": [SEVEN_SEVEN, SEVEN_SEVEN, SEVEN_SEVEN], "library": LIBRARY },
            }));

            assert_ne!(priority_of(&scored_on(&g, "core-055")), json!("clear"));
        }
    }
}
