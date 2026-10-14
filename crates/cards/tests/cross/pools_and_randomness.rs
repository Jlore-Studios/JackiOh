//! Random pools and the draws they take (SPEC §5.1, §10.7, R60, R129). Found by the polish-4
//! edge-case hunt, round 4 (docs/polish/4-edge-cases.md, lens "card by card"); every case here failed
//! before its fix.
//!
//!  - §5.1: a random pool never offers the card that generated it, unless the card names the pool
//!    itself. #95's "add 3 random cards" named no pool; and a card's own exclusion (§8 #54's "pool
//!    excluding #54") holds when a fused card runs that text, whose own definition is a transient id.
//!  - R129: an effect that finds nothing to do draws no random numbers — #23 with no non-Radiant hand
//!    card, #83 over an Immutable board card, #67 into an occupied lane, #95's Units into a full row.
//!  - §10.7: the Zephyrs scorer's "lethal available" needs a Charge unit that can reach the field and
//!    then the hero.

use jackioh_cards::{card_def, register_all};
use jackioh_engine::testkit::{Scenario, cursor_where, scenario};
use jackioh_engine::{GameEvent, PlayerId, Rng, Row, subsystems};
use serde_json::json;

use super::craft_fused;

const MANA_WELL: &str = "core-006";
const VANILLA: &str = "core-008";
const BIG_D: &str = "core-001";
const MENACE: &str = "core-019";
const DREAM: &str = "core-023";
const DUELIST: &str = "core-045";
const STRAAZA: &str = "core-054";
const JILLIAX: &str = "core-056";
const OOMEN: &str = "core-067";
const TRANSMOGULATE: &str = "core-083";
const CALL_TO_CHAOS: &str = "core-095";
const ZEPHYRS: &str = "core-097";
const CRAFT: &str = "core-099";

/// The def ids of the `addedToHand` events among `events`.
fn added_to_hand(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::AddedToHand { def_id, .. } => Some(def_id.clone()),
            _ => None,
        })
        .collect()
}

/// The `(defId, row)` of the `summoned` events among `events`.
fn summoned(events: &[GameEvent]) -> Vec<(String, Row)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Summoned { def_id, row, .. } => Some((def_id.clone(), *row)),
            _ => None,
        })
        .collect()
}

/// The name of the first Call to Chaos effect the rng rolls at `cursor` of `seed` (base face).
fn chaos_roll(seed: &str, cursor: u32) -> Option<String> {
    subsystems::roll_chaos_effects(&mut Rng::new(seed, cursor), false, None)
        .first()
        .map(|effect| effect.name.to_string())
}

fn last_added(s: &Scenario) -> Vec<String> {
    added_to_hand(s.last_events())
}

mod s5_1_a_random_pool_never_offers_the_card_that_generated_it {
    use super::*;

    #[test]
    fn s5_1_call_to_chaos_s_add_3_random_cards_never_adds_a_call_to_chaos_8_95_10_7() {
        register_all();
        // The seed is arbitrary: the sweep plays #95 at every cursor whose roll is "add".
        const SEED: &str = "chaos-pool-probe";
        let mut added: Vec<String> = Vec::new();
        let mut plays = 0;
        let mut cursor: u32 = 0;
        while cursor < 6000 && plays < 150 {
            let rolled = chaos_roll(SEED, cursor);
            if rolled.as_deref() == Some("add") {
                plays += 1;
                let mut s =
                    scenario(json!({ "seed": SEED, "p1": { "hand": [CALL_TO_CHAOS, MENACE], "mana": 8 } }));
                s.state_mut().rng_cursor = cursor;
                s.play(CALL_TO_CHAOS, json!({}));
                added.extend(last_added(&s));
            }
            cursor += 1;
        }
        assert!(plays > 50);
        assert!(added.len() > 150);
        assert!(!added.contains(&CALL_TO_CHAOS.to_string()));
    }

    #[test]
    fn s5_1_a_crafted_card_carrying_54_straaza_s_text_still_never_adds_a_straaza_8_54_pool_excluding_54_r102()
    {
        register_all();
        // Craft a Card's result for #54 Straaza and #56 Jilliax (a keyword-only body), made directly (R77):
        // the test is about the fused card, not about which two Discover offers the pools of every set
        // (R380) make. The seed is arbitrary: the cursor is set before the play, whatever the fuse drew.
        const SEED: &str = "craft-straaza";
        let mut added: Vec<String> = Vec::new();
        for cursor in 0..200 {
            let mut s = scenario(json!({ "seed": SEED, "p1": { "hand": [MENACE], "mana": 8 } }));
            craft_fused(&mut s, PlayerId::P1, &[STRAAZA, JILLIAX]);
            let crafted = s
                .hand(PlayerId::P1)
                .iter()
                .find(|card| card.def_id.contains(STRAAZA))
                .map(|card| card.id.clone());
            let Some(crafted) = crafted else {
                panic!("the crafted Straaza + Jilliax should be in hand");
            };
            s.state_mut().rng_cursor = cursor;
            s.play(&crafted, json!({ "zone": 1 }));
            added.extend(last_added(&s));
        }
        assert_eq!(added.len(), 400);
        assert!(!added.contains(&STRAAZA.to_string()));
    }
}

mod r129_an_effect_that_finds_nothing_to_do_draws_no_random_numbers {
    use super::*;

    #[test]
    fn r129_reoccurring_dream_with_no_hand_card_left_to_make_radiant_draws_no_randomness_r60() {
        register_all();
        for radiant in [false, true] {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": DREAM, "radiant": radiant }], "field": [VANILLA], "library": [VANILLA] },
                "p2": { "hand": [VANILLA], "library": [VANILLA] },
            }));
            let before = s.state().rng_cursor;
            s.play(DREAM, json!({}));
            // Nothing is left in the hand, so "a random card in your hand becomes Radiant" finds nothing.
            assert_eq!(s.hand(PlayerId::P1).len(), 0);
            assert_eq!(s.state().rng_cursor, before);
        }
    }

    #[test]
    fn r129_transmogulate_draws_no_randomness_for_an_immutable_board_card_it_leaves_standing_r35_r23() {
        register_all();
        // A Radiant #19 Midrange Menace is Immutable (R23, R35).
        let mut s = scenario(json!({
            "p1": { "hand": [TRANSMOGULATE], "field": [{ "def": "core-019", "radiant": true }], "mana": 4 },
        }));
        let menace = s.unit(PlayerId::P1, 1).map(|card| card.id.clone());
        let before = s.state().rng_cursor;
        s.play(TRANSMOGULATE, json!({}));
        // The Menace is Immutable and the hand, library, graveyard and exile are empty: nothing is
        // replaced, so the effect found nothing to do.
        assert_eq!(s.unit(PlayerId::P1, 1).map(|card| card.id.clone()), menace);
        assert_eq!(s.state().rng_cursor, before);
    }

    #[test]
    fn r129_zoomerbin_oomen_whose_lane_s_backrow_zone_is_occupied_draws_no_randomness_for_the_trap_it_cannot_summon_r47()
     {
        register_all();
        for radiant in [false, true] {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": OOMEN, "radiant": radiant }, VANILLA],
                    "backrow": [{ "def": MANA_WELL, "lane": 2 }],
                    "mana": 4,
                },
            }));
            let before = s.state().rng_cursor;
            s.play(OOMEN, json!({ "zone": 2 }));
            // §8 #67: "zone occupied or Locked → fizzles", so the Cry found nothing to do.
            assert_eq!(
                s.backrow(PlayerId::P1, 2).map(|card| card.def_id.clone()),
                Some(MANA_WELL.to_string())
            );
            assert_eq!(
                summoned(s.last_events())
                    .into_iter()
                    .filter(|(_, row)| *row == Row::Backrow)
                    .count(),
                0
            );
            assert_eq!(s.state().rng_cursor, before);
        }
    }

    #[test]
    fn r129_call_to_chaos_s_summon_3_random_3_cost_units_into_a_full_unit_row_draws_no_randomness_past_the_roll_r64()
     {
        register_all();
        // The seed is arbitrary: the cursor is searched for the roll that summons Units.
        const SEED: &str = "chaos-full-row";
        let cursor = cursor_where(SEED, |cursor| {
            chaos_roll(SEED, cursor).as_deref() == Some("units")
        });
        let mut s = scenario(json!({
            "seed": SEED,
            "p1": { "hand": [CALL_TO_CHAOS, MENACE], "field": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA], "mana": 8 },
        }));
        s.state_mut().rng_cursor = cursor;
        s.play(CALL_TO_CHAOS, json!({}));
        // The roll is the one draw the card makes; every summon then fails on the full row (§3.2).
        assert_eq!(
            summoned(s.last_events())
                .into_iter()
                .filter(|(def_id, _)| def_id != CALL_TO_CHAOS)
                .count(),
            0
        );
        assert_eq!(s.state().rng_cursor, cursor + 1);
    }
}

mod s10_7_the_zephyrs_scorer_s_lethal_available {
    use super::*;

    #[test]
    fn s10_7_zephyrs_does_not_score_a_charge_unit_as_lethal_when_an_enemy_taunt_stands_in_its_way_or_when_it_has_no_zone_to_enter_4_2_step_3_3_2_r29()
     {
        register_all();
        // p2's hero is at 3 behind #19 Midrange Menace (9/9 Taunt). #45 Deft Duelist (4/3 Charge, cost
        // 2) would be lethal on an open board, but §4.2 step 3 makes it attack the Menace.
        let s = scenario(json!({
            "p1": { "hand": [ZEPHYRS], "mana": 4 },
            "p2": { "field": [MENACE], "health": 3 },
        }));
        assert_ne!(
            subsystems::score_def(
                s.state(),
                PlayerId::P1,
                &card_def(DUELIST),
                &subsystems::ScorerOptions::default(),
                None
            )
            .priority,
            subsystems::ScorePriority::Lethal
        );

        // The same card on an open board but with no free unit zone to play it into (§3.2) is no
        // lethal either: it cannot reach the field this turn at all.
        let full = scenario(json!({
            "p1": { "hand": [ZEPHYRS], "field": [BIG_D, BIG_D, BIG_D, BIG_D, BIG_D], "mana": 4 },
            "p2": { "health": 3 },
        }));
        assert_ne!(
            subsystems::score_def(
                full.state(),
                PlayerId::P1,
                &card_def(DUELIST),
                &subsystems::ScorerOptions::default(),
                None
            )
            .priority,
            subsystems::ScorePriority::Lethal
        );

        // And on an open board with room it is lethal, which is what the priority is for.
        let open = scenario(json!({ "p1": { "hand": [ZEPHYRS], "mana": 4 }, "p2": { "health": 3 } }));
        assert_eq!(
            subsystems::score_def(
                open.state(),
                PlayerId::P1,
                &card_def(DUELIST),
                &subsystems::ScorerOptions::default(),
                None
            )
            .priority,
            subsystems::ScorePriority::Lethal
        );
    }
}

mod seed_search_survives_a_pool_change {
    //! Issue #848: a test that needs a pool's roll to land one way searches for the seed at run time
    //! (`seed_where`) or builds the position directly, so a card joining a pool moves nothing it
    //! depends on. This one changes a pool on purpose and runs the converted Reborn and Craft positions.
    use super::*;
    use jackioh_engine::testkit::{clear_overrides, register_catalog, seed_where};

    const BIGOT: &str = "core-002";
    const PREM_PANTHER: &str = "core-032";
    const RENO: &str = "core-053";
    const SURGERY: &str = "core-063";
    const TWISTED_SORCERER: &str = "core-068";
    const RUSH_TOKEN: &str = "core-t-rush";
    /// Core #1 Big D: a Unit no converted test names, first in every Unit pool, so removing it moves
    /// the picks a pool of Units makes.
    const UNINVOLVED_UNIT: &str = "core-001";
    /// How many seeds' first Discover the test compares across the two catalogs.
    const SEEDS_COMPARED: usize = 50;

    /// Back to the production catalog when it drops, so a failure mid-test cannot leak the changed
    /// pool (a thread-local override) onto whatever test runs next on this thread.
    struct ClearOnDrop;
    impl Drop for ClearOnDrop {
        fn drop(&mut self) {
            clear_overrides();
        }
    }

    /// The def ids the open Discover offers.
    fn offered(s: &Scenario) -> Vec<String> {
        let pending = serde_json::to_value(s.state().pending.as_ref()).unwrap_or_default();
        pending["options"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|option| option["selection"]["option"].as_str().map(str::to_string))
            .collect()
    }

    /// What Craft a Card's first Discover offers under each of the seeds, on the catalog registered now.
    fn first_offers() -> Vec<Vec<String>> {
        (0..SEEDS_COMPARED)
            .map(|n| {
                let mut s = scenario(json!({
                    "seed": format!("craft-pool-{n}"),
                    "p1": { "hand": [CRAFT, RENO], "mana": 4 },
                    "p2": { "hand": [RENO], "field": [PREM_PANTHER] },
                }));
                s.play(CRAFT, json!({}));
                offered(&s)
            })
            .collect()
    }

    #[test]
    fn converted_reborn_and_craft_scenarios_pass_on_a_changed_pool_while_the_old_craft_pins_move() {
        register_all();
        let _clear = ClearOnDrop;
        let shipped = first_offers();

        // The shipped catalog with one uninvolved Unit removed: a pool that changed.
        let mut defs = (*jackioh_cards::CATALOG).clone();
        assert!(defs.shift_remove(UNINVOLVED_UNIT).is_some());
        register_catalog(defs);

        // A seed pinned for what Craft a Card's Discover offers (what the old tests answered by name)
        // stops offering it: most seeds' first Discover is not the one it was.
        let changed = first_offers();
        let moved = shipped
            .iter()
            .zip(&changed)
            .filter(|(before, after)| before != after)
            .count();
        assert!(
            moved * 2 > SEEDS_COMPARED,
            "{moved} of {SEEDS_COMPARED} seeds' offers moved"
        );

        // The converted Craft position needs no Discover: Bigot and Twisted Sorcerer are fused directly (R77).
        let mut s = scenario(json!({
            "p1": { "hand": [RENO], "mana": 4 },
            "p2": { "hand": [RENO], "field": [PREM_PANTHER] },
        }));
        craft_fused(&mut s, PlayerId::P1, &[BIGOT, TWISTED_SORCERER]);
        let crafted = s
            .hand(PlayerId::P1)
            .iter()
            .find(|card| card.def_id.starts_with("t-"))
            .map(|card| card.id.clone());
        let Some(crafted) = crafted else {
            panic!("the crafted card should be in hand");
        };
        let Some(panther) = s.unit(PlayerId::P2, 1) else {
            panic!("p2's Prem Panther");
        };
        s.play(
            &crafted,
            json!({
                "zone": 2,
                "targets": [{ "pick": "instance", "instanceId": panther.id }, { "pick": "hero", "player": "p2" }],
            }),
        );
        assert_eq!(s.card(&panther.id).zone.z().as_str(), "graveyard");
        assert_eq!(s.state().players.p2.hero.health, 26);

        // The converted Reborn position searches for its seed. Plastic Surgery's pool is
        // `RANDOM_KEYWORD_POOL` (R21), which no catalog change moves, so this part holds either way.
        let setup = |seed: &str| {
            let s = scenario(json!({
                "seed": seed,
                "p1": { "hand": [SURGERY], "field": [{ "def": RUSH_TOKEN, "lane": 1 }] },
            }));
            let token = s.card(RUSH_TOKEN).id.clone();
            (s, token)
        };
        let reborn = |s: &Scenario, token: &str| {
            s.stats(token)
                .keywords
                .iter()
                .any(|keyword| keyword.kind() == jackioh_engine::KeywordKind::Reborn)
        };
        let seed = seed_where("surgery-reborn-token", |seed| {
            let (mut s, token) = setup(seed);
            s.play(
                SURGERY,
                json!({ "targets": [{ "pick": "instance", "instanceId": token }] }),
            );
            reborn(&s, &token)
        });
        let (mut s, token) = setup(&seed);
        s.play(
            SURGERY,
            json!({ "targets": [{ "pick": "instance", "instanceId": token }] }),
        );
        assert!(reborn(&s, &token));
    }
}
