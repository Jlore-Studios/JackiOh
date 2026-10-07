//! A card's return to its hand: the price it returns with, and §5.1's end-of-turn return (SPEC §2.4,
//! §5.1, §10.5 step 7, R4, R78, R153, R155). Found by the polish-4 edge-case hunt, round 3
//! (docs/polish/4-edge-cases.md, lenses L2 and L8); every case here failed before its fix.
//!
//!  - R4: a price a card is given as it returns to a hand — #31's "+1", #37r's "costs 1 less" — is
//!    its price in that hand. A full hand burns the card instead (§2.4), and the burned card keeps
//!    its cost, as radiant #52's "costing 0" already did (re-entry.test.ts).
//!  - R155: the end-of-turn return belongs to the Spell its own play landed in the graveyard, so a
//!    card that left the graveyard and came back some other way the same turn stays there (R153).
//!  - R215 (round 4, lens L2): a hand card that reaches a graveyard is reset as a card leaving the
//!    field is (R78), so it comes back as the printed card; and #99's crafted card, like every price
//!    a card is given as it reaches a hand, takes its "costs 0" only in that hand.
//!  - R215 (round 5, lenses "card by card" and "engine invariants"): a card landing from the resolving
//!    zone is reset too, so a #95 an earlier Call to Chaos cast carries no link of that chain (R28)
//!    into a play of its own once Reminisce has brought it back.
//!  - R155 (round 7, lens L8): a return Spell cast on the other player's turn (a cast on draw, R70) is
//!    flagged and cleared at that turn's cleanup — §6.2's "End of turn" is its controller's own — so it
//!    does not come back at the end of a later turn it was not played on.
//!  - R215 (round 8, lens "engine invariants"): radiant #52's "costing 0" is announced with a
//!    `costChanged` once the card has landed, as #31's +1 and #72r's 0 are (§10.3).
//!
//! Port of `packages/cards/test/hand-returns.test.ts` (SURFACE §4.1, §8). TS's live card objects are
//! owned copies here, read back from the state by id after every step and written through
//! `find_instance_mut`.

use jackioh_engine::effects::bounce;
use jackioh_engine::testkit::*;
use jackioh_engine::PlayerId::{P1, P2};

const VANILLA: &str = "core-008";
const STOCKPILE: &str = "core-005";
const TIMMY: &str = "core-011";
const DREAM: &str = "core-023";
const SEVEN_SEVEN: &str = "core-025";
const KY_MATH: &str = "core-031";
const GRAVEDIGGER: &str = "core-037";
const REMINISCE: &str = "core-072";
const FIELD_OF_DREAMS: &str = "core-076";
const POINTMASTER: &str = "core-020";
const TWINSPELL: &str = "core-079";
const ZAO_GAO: &str = "core-080";
const CORPSE_EATER: &str = "core-089";
const CRAFT: &str = "core-099";
const MENACE: &str = "core-019";
const CHAOS: &str = "core-095";

use super::scenario;

/// A cursor at which a base #95's single roll is `effect`, the pick 095's own tests use (R28).
fn chaos_cursor(seed: &str, effect: &str) -> u32 {
    for cursor in 0..500u32 {
        if subsystems::roll_chaos_effects(&mut Rng::new(seed, cursor), false, None)
            .first()
            .is_some_and(|rolled| rolled.name == effect)
        {
            return cursor;
        }
    }
    panic!("no cursor below 500 rolls \"{effect}\" from seed \"{seed}\"");
}

/// TS `AT_P2`: the target list naming p2's hero.
fn at_p2() -> Value {
    json!([{ "pick": "hero", "player": "p2" }])
}

/// `[...head, ...fillers]` for a setup's hand.
fn with_fillers(head: &[&'static str], count: usize) -> Vec<&'static str> {
    head.iter().copied().chain(std::iter::repeat_n(VANILLA, count)).collect()
}

mod r4_a_card_a_full_hand_burns_keeps_its_cost_without_the_price_of_a_return_it_never_made {
    use super::*;

    #[test]
    fn r4_r78_kys_math_equation_that_a_full_hand_burns_at_end_of_turn_stays_at_its_cost_without_the_1_8_31() {
        let hand = with_fillers(&[KY_MATH, STOCKPILE], 8);
        let mut g = scenario(json!({
            "p1": { "hand": hand, "field": [TIMMY], "library": [VANILLA, VANILLA, VANILLA, VANILLA] },
            "p2": { "field": [TIMMY], "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA] },
        }));
        let equation = g.card(KY_MATH).clone();

        g.play(&equation.id, json!({ "targets": at_p2() }));
        g.play(STOCKPILE, json!({})); // hand 8 → draws 2 → 10: the hand is full when the turn ends
        assert_eq!(g.hand(P1).len(), 10);

        g.end_turn();

        // "Return to hand with cost +1": the hand is full, so the card is burned back to the graveyard
        // (§2.4) and never returns, and the +1 was the price of that return.
        g.expect_in_zone(&equation.id, "graveyard");
        assert_eq!(g.card(&equation.id).cost_mod, 0);
    }

    #[test]
    fn r4_r62_r78_radiant_gravediggers_start_of_turn_pick_that_a_full_hand_burns_stays_at_its_cost_without_the_1_less_8_37() {
        let fillers: Vec<&str> = vec![VANILLA; 10];
        let mut g = scenario(json!({
            "p1": {
                "hand": fillers,
                "field": [{ "def": GRAVEDIGGER, "radiant": true }],
                "graveyard": [SEVEN_SEVEN],
                "library": [VANILLA, VANILLA, VANILLA],
            },
            "p2": { "field": [TIMMY], "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA] },
        }));
        let seven = g.card(SEVEN_SEVEN).clone();
        assert_eq!(g.hand(P1).len(), 10);

        g.end_turn(); // p2's turn
        g.end_turn(); // p1's start of turn: the Discover opens before the draw (R62)
        assert_eq!(g.state().pending.as_ref().map(|pending| pending.player_id), Some(P1));
        g.answer(json!(seven.id));

        g.expect_in_zone(&seven.id, "graveyard");
        assert_eq!(g.card(&seven.id).cost_mod, 0);
    }
}

mod r155_the_end_of_turn_return_belongs_to_the_landing_the_spells_own_play_made {
    use super::*;

    #[test]
    fn r155_r153_reoccurring_dream_played_taken_back_to_hand_and_then_discarded_into_the_graveyard_the_same_turn_does_not_return_at_end_of_turn_5_1() {
        let mut g = scenario(json!({
            "p1": {
                "hand": [DREAM, REMINISCE, FIELD_OF_DREAMS, VANILLA],
                "field": [TIMMY],
                "mana": 10,
                "library": [VANILLA, VANILLA, VANILLA, VANILLA],
            },
            "p2": { "field": [TIMMY], "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA] },
        }));
        let dream = g.card(DREAM).clone();

        g.play(&dream.id, json!({})); // §10.5 step 7 lands it in the graveyard, flagged to return (R155)
        g.expect_in_zone(&dream.id, "graveyard");
        g.play(REMINISCE, json!({}));
        g.answer(json!(dream.id)); // back to hand, never played again
        g.expect_in_zone(&dream.id, "hand");
        g.play(FIELD_OF_DREAMS, json!({})); // the hand is replaced: the Dream is discarded to the graveyard (R31)
        g.expect_in_zone(&dream.id, "graveyard");

        g.end_turn();

        // What lies in the graveyard now arrived by a discard, not by its own play's step 7.
        g.expect_in_zone(&dream.id, "graveyard");
    }
}

mod r215_a_hand_card_that_reaches_a_graveyard_is_the_printed_card_again {
    use super::*;

    #[test]
    fn r215_r78_a_corpse_eater_that_fed_in_hand_was_discarded_by_zao_gao_and_came_back_by_reminisce_is_a_fresh_2_2_8_89() {
        let mut g = scenario(json!({
            "p1": {
                // Zao Gao's discard is random (R354), so the hand it discards from is the Eater and one
                // other card, and both go; Reminisce comes from the library afterwards.
                "hand": [CORPSE_EATER, ZAO_GAO, STOCKPILE],
                "field": [{ "def": POINTMASTER, "lane": 1 }],
                "library": [REMINISCE, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA],
            },
            "p2": { "hand": [STOCKPILE], "field": [{ "def": VANILLA, "lane": 1 }], "library": [VANILLA, VANILLA, VANILLA] },
        }));
        let eater = g.card(CORPSE_EATER).clone();
        let Some(prey) = g.unit(P2, 1) else {
            panic!("setup: p2's lane-1 unit");
        };

        // A 4/4 dies while the Eater is in hand: it gains +4/+4 (§8 #89).
        g.attack(POINTMASTER, &prey.id);
        g.expect_in_zone(&prey.id, "graveyard");
        assert_eq!(g.stats(&eater.id).attack, 6);

        // Zao Gao discards it; back on p1's next turn, Reminisce brings it back from the graveyard.
        g.play(ZAO_GAO, json!({}));
        g.expect_in_zone(&eater.id, "graveyard");
        g.end_turn();
        g.end_turn();
        g.play(REMINISCE, json!({}));
        g.answer(json!(eater.id));
        g.expect_in_zone(&eater.id, "hand");

        // The card that went to the graveyard and came back is the printed card, not the one that fed.
        assert_eq!(g.card(&eater.id).buffs, AttackHealth { attack: 0, health: 0 });
        assert_eq!(g.stats(&eater.id).attack, 2);
        assert_eq!(g.stats(&eater.id).max_health, 2);
    }

    #[test]
    fn r215_r4_r77_a_crafted_card_a_full_hand_burns_does_not_keep_the_cost_0_it_was_to_have_in_hand_8_99() {
        let hand = with_fillers(&[TWINSPELL, CRAFT], 8);
        let mut g = scenario(json!({
            "p1": { "hand": hand, "library": [VANILLA, VANILLA, VANILLA] },
            "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA] },
        }));
        g.play(TWINSPELL, json!({ "zone": 1 }));
        g.end_turn();
        g.end_turn();
        assert_eq!(g.state().active, P1);
        assert_eq!(g.hand(P1).len(), 10);
        g.play(CRAFT, json!({}));
        // Twinspell's Echo +1: two crafts. The first lands in the hand, which is then full again.
        let mut i = 0;
        while i < 4 {
            let Some(pending) = g.state().pending.as_ref() else {
                break;
            };
            let key = pending.options.first().map(|option| option.key.clone()).unwrap_or_default();
            g.answer(json!(key));
            i += 1;
        }
        let burned: Vec<String> = g
            .events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Burned { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(burned.len(), 1);
        let crafted = g.card(burned.first().map(String::as_str).unwrap_or("")).clone();
        assert_eq!(crafted.zone.z(), ZoneName::Graveyard);
        assert_eq!(crafted.cost_override, None);
    }
}

mod r215_a_card_that_lands_from_the_resolving_zone_is_the_printed_card_again {
    use super::*;

    #[test]
    fn r215_r28_r87_a_call_to_chaos_cast_at_the_end_of_a_chain_taken_back_from_the_graveyard_and_played_starts_a_chain_of_its_own() {
        // The played #95 stands in for the 19th link of a chain, which is how 095's own tests pin R28's
        // counter. Its roll casts the 20th link, which R87 sends to the graveyard as it resolves, and
        // #72 Reminisce takes that card back. Played from hand, it is a new play, so a new chain from
        // nothing — and its "cast a random Call to Chaos" casts one, where the old link at the cap cast
        // nothing. Hearthstone likewise returns a card from the graveyard without what its last trip
        // left on it.
        let seed = "inv-r5-chaos-chain";
        let mut s = scenario(json!({ "seed": seed, "p1": { "hand": [CHAOS, REMINISCE, MENACE], "mana": 20 }, "p2": { "hand": [MENACE] } }));
        let chaos = s.card(CHAOS).id.clone();
        s.card_mut(&chaos)
            .memory
            .insert(subsystems::CHAOS_CHAIN_KEY.to_string(), json!(CALL_TO_CHAOS_CHAIN_CAP - 1));
        s.state_mut().rng_cursor = chaos_cursor(seed, "recast");
        s.play(CHAOS, json!({}));

        let plays: Vec<String> = s
            .events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::CardPlayed { instance_id, def_id, .. } if def_id == CHAOS => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(plays.len(), 2); // the play and the chain's 20th cast
        let last_link = match plays.get(1) {
            Some(id) => id.clone(),
            None => panic!("the chain's 20th cast"),
        };
        s.expect_in_zone(&last_link, "graveyard");
        // It landed as the printed card: the chain's count stayed with the chain.
        assert!(s.card(&last_link).memory.get(subsystems::CHAOS_CHAIN_KEY).is_none());

        s.play(REMINISCE, json!({}));
        s.answer(json!(last_link));
        s.expect_in_zone(&last_link, "hand");

        s.state_mut().rng_cursor = chaos_cursor(seed, "recast");
        s.play(&last_link, json!({}));
        let played = s
            .last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id == CHAOS))
            .count();
        assert_eq!(played, 2);
    }
}

// ---------------------------------------------------------------------------
// Round 7 (lens L8): a return Spell cast on the other player's turn.
// ---------------------------------------------------------------------------

const MOTHS: &str = "core-009"; // 1/14; start of turn: every enemy Unit attacks this
const PREM_PANTHER: &str = "core-032"; // 5/4 Rush; after it attacks and survives, draw 2 per Unit destroyed
const RENO: &str = "core-053";

/// A fixture card: a transient def in the match state and its script in the registry. TS
/// `fixture(s, id, type, script, stats = { attack: 2, health: 2 })`.
fn fixture(s: &mut Scenario, id: &str, type_: CardType, script: Script, stats: Option<AttackHealth>) {
    let stats = stats.unwrap_or(AttackHealth { attack: 2, health: 2 });
    let face = if type_ == CardType::Unit {
        json!({ "attack": stats.attack, "health": stats.health, "keywords": [], "text": id })
    } else {
        json!({ "keywords": [], "text": id })
    };
    let def: CardDef = json_as(json!({
        "id": id,
        "index": id,
        "name": id,
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face.clone(),
        "radiant": face,
    }));
    s.state_mut().transient_defs.insert(id.to_string(), def);
    let mut scripts = registered_scripts().clone();
    scripts.insert(id.to_string(), CardScripts { base: script.clone(), radiant: script });
    register_scripts(scripts);
}

mod r155_5_1_an_end_of_turn_return_belongs_to_the_turn_the_spell_was_played_on {
    use super::*;

    #[test]
    fn r155_r70_a_return_spell_cast_on_the_opponents_turn_does_not_come_back_at_the_end_of_its_casters_next_turn_5_1_6_2() {
        // At p2's start of turn p2's #9 Moths to the Flame (worn to 4 health) makes p1's Prem Panther
        // (5/4) attack it: the Panther kills it and survives, so p1 draws 2 on p2's turn (R426). The top
        // card is a cast-on-draw Spell carrying #23 Reoccurring Dream's "End of turn: returns from the GY
        // to your hand" (the flag R155 writes is what the return reads), so p1 casts it on p2's turn
        // (§2.4, R70).
        let mut s = scenario(json!({
            "p1": { "field": [PREM_PANTHER], "hand": [RENO], "library": [RENO, RENO, RENO, RENO] },
            "p2": { "field": [{ "def": MOTHS, "damage": 10 }], "hand": [RENO], "library": [RENO, RENO, RENO, RENO] },
        }));
        fixture(
            &mut s,
            "edge-r7-dream-cod",
            CardType::Spell,
            Script {
                static_flags: Some(json_as(json!({ "castOnDraw": true }))),
                cry: Some(hook(|_ctx| vec![])),
                end_of_turn: Some(hook(|ctx| {
                    if ctx.self_.as_ref().is_some_and(|me| me.return_to_hand_at_end_of_turn == Some(true)) {
                        vec![bounce(json_as(json!({ "target": { "of": "self" } })))]
                    } else {
                        vec![]
                    }
                })),
                ..Script::default()
            },
            None,
        );
        let cod = new_instance(s.state_mut(), "edge-r7-dream-cod", P1, Zone::Library { player: P1 });
        s.state_mut().players.p1.library.insert(0, cod.clone());

        s.end_turn();
        assert_eq!(s.state().active, P2);
        // The cast happened on p2's turn, and the Spell landed in p1's graveyard (§10.5 step 7).
        assert!(s
            .events()
            .iter()
            .any(|event| matches!(event, GameEvent::CardPlayed { instance_id, .. } if *instance_id == cod.id)));
        s.expect_in_zone(&cod.id, "graveyard");

        // p2's turn ends. §5.1: the Spell returns "at the end of that turn", and R155's cleanup clears
        // the flag "at the end of that turn" — whichever reading, nothing of that return is left once
        // the turn it was played on is over.
        s.end_turn();
        assert_eq!(s.state().active, P1);
        let after_its_turn = s.card(&cod.id).clone();
        if after_its_turn.zone.z() == ZoneName::Graveyard {
            assert_ne!(
                after_its_turn.return_to_hand_at_end_of_turn,
                Some(true),
                "the return flag outlived the cleanup of the turn the Spell was cast on"
            );
        }

        // p1's own next turn ends: a Spell p1 did not play on this turn does not come back now.
        s.end_turn();
        let returned = s.last_events().iter().any(|event| match event {
            GameEvent::Bounced { instance_id, .. } | GameEvent::AddedToHand { instance_id, .. } => *instance_id == cod.id,
            _ => false,
        });
        assert!(!returned, "the Spell came back at the end of a turn it was not played on");
    }
}

mod r215_10_3_a_price_given_as_a_card_reaches_a_hand_is_announced {
    use super::*;

    #[test]
    fn r215_radiant_52s_bounce_at_cost_0_emits_costchanged_for_the_card_it_prices_as_31s_1_and_72rs_0_do_10_3() {
        let silas = "core-052";
        let mut s = scenario(json!({
            "p1": { "hand": [silas], "field": [{ "def": RENO, "lane": 5 }] },
        }));
        let silas_id = s.card(silas).id.clone();
        s.card_mut(&silas_id).radiant = true;
        let Some(reno) = s.unit(P1, 5).map(|card| card.id.clone()) else {
            panic!("expected p1's Reno in lane 5");
        };

        // Rotating right would move Reno from p1's lane 5 to p2's, so radiant #52 bounces it to its
        // owner's hand costing 0 instead (§8 #52, R14).
        s.play(&silas_id, json!({ "zone": 1, "modes": ["right"] }));

        s.expect_in_zone(&reno, "hand");
        assert_eq!(s.card(&reno).cost_override, Some(0));
        // Reno's price in its owner's hand went from its printed 3 to 0, a visible change (§10.3), which
        // is announced once the card has landed, as #31's +1 and #72r's 0 are.
        let bounced = s
            .last_events()
            .iter()
            .position(|event| matches!(event, GameEvent::Bounced { instance_id, .. } if *instance_id == reno));
        let Some(bounced) = bounced else {
            panic!("no bounced event for {reno}");
        };
        let announced = GameEvent::CostChanged { instance_id: reno.clone(), cost: 0, hidden_from: None };
        assert!(s.last_events()[bounced..].contains(&announced));
    }
}
