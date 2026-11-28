//! SPEC §8.14 FvH #45 Subject 9 ((3) Unit, Epic, 5/5 → 10/10), a card of the held Felinors vs Humans set
//! (R5235000). Built by S09 (#5259).
//!   Base:    "Cry: Draw {traps|Trap|Traps} from your deck. If your deck has fewer, add random Traps to your
//!            hand to make up the rest." (5)
//!   Radiant: "Cry: Recruit {traps|Trap|Traps}. If your deck has fewer, summon random Traps to make up the
//!            rest." (5)
//!
//! Engine: the Cry (R1), FV2 #45's reading. "Trap" is a Trap or a Field Trap card. The base face draws up to
//! the declared `traps` different Trap cards of its controller's deck, picked at random by the match rng (R60),
//! each a real draw (`draw_from_library`: the hand cap, cast on draw, R55), then adds one random Trap of the
//! shipped sets (R380) for each one the deck lacked (`add_random_from_catalog`). The Radiant face Recruits
//! Traps top down, face-down into the backrow (R64), then summons a random Trap for each one the deck lacked;
//! a full backrow stops both alike (R129). What the deck lacks is counted once, as the Cry reaches it, before
//! any Trap leaves the deck, and the picks are drawn with it (`for_each_card`, R113), so a pause in between
//! resumes over the same list. Tunes: traps 5 ↑, read by its Cry (R1748000).

use jackioh_engine::effects::{add_random_from_catalog, draw_from_library, recruit, summon_random};
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-045";

/// The key of one Trap the deck lacked, beside the drawn Traps' instance ids (which never read so).
const SHORTFALL: &str = "shortfall";
/// The key of one Recruit of a Trap the deck holds.
const RECRUIT: &str = "recruit";

/// "Trap": a Trap or a Field Trap card.
fn is_trap(kind: CardType) -> bool {
    matches!(kind, CardType::Trap | CardType::FieldTrap)
}

/// The Trap cards of the controller's deck, top first.
fn deck_traps(ctx: &EffectContext<'_>) -> Vec<String> {
    let state: &GameState = ctx.state;
    zone_cards(state, ctx.controller, OffFieldZone::Library)
        .iter()
        .filter(|card| is_trap(card_type_of(state, card)))
        .map(|card| card.id.clone())
        .collect()
}

/// The catalog pool of the random Traps: every shipped Trap and Field Trap, no token (§5.1).
fn trap_pool() -> Value {
    json!({ "type": ["Trap", "Field Trap"] })
}

/// `wanted` keys: the `taken` ones, then `SHORTFALL` for each the deck lacked.
fn with_shortfall(mut taken: Vec<String>, wanted: usize) -> Vec<String> {
    taken.truncate(wanted);
    taken.resize(wanted, SHORTFALL.to_string());
    taken
}

fn draw_or_add(key: &str) -> Effect {
    if key == SHORTFALL {
        return add_random_from_catalog(json_as(json!({ "query": trap_pool(), "count": 1 })));
    }
    draw_from_library(json_as(json!({ "instanceId": key })))
}

fn recruit_or_summon(key: &str) -> Effect {
    if key == RECRUIT {
        return recruit(json_as(json!({ "filter": trap_pool() })));
    }
    summon_random(json_as(json!({ "query": trap_pool() })))
}

fn subject(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let wanted = param(ctx, "traps").max(0) as usize;
            if radiant {
                return vec![for_each_card(ForEachCardArgs {
                    cards: Arc::new(move |c: &mut EffectContext<'_>| {
                        let held = deck_traps(c).len().min(wanted);
                        with_shortfall(vec![RECRUIT.to_string(); held], wanted)
                    }),
                    each: Arc::new(recruit_or_summon),
                })];
            }
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(move |c: &mut EffectContext<'_>| {
                    let traps = deck_traps(c);
                    let drawn: Vec<String> = c.rng.shuffle(&traps);
                    with_shortfall(drawn, wanted)
                }),
                each: Arc::new(draw_or_add),
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: subject(false),
        radiant: subject(true),
    }
}

// FvH #45 Subject 9 — SPEC §8.14 row 45 (R60, R64, R129, R380, R1748000).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005"; // Stockpile: a Spell kept in hand and the deck's filler
    const TRAP: &str = "stand-in (1) Trap";
    const FIELD_SPELL: &str = "core-006"; // Mana Well: a Field Spell, to fill a backrow zone

    fn board(radiant: bool, library: Value, backrow: Value) -> Scenario {
        scenario(json!({
            "seed": "subject-9",
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": library,
                "backrow": backrow,
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(3) },
        }))
    }

    fn deck(trap_count: usize, filler_count: usize) -> Value {
        let mut cards = vec![json!(TRAP); trap_count];
        cards.extend(vec![json!(FILLER); filler_count]);
        json!(cards)
    }

    fn is_trap(s: &Scenario, card: &CardInstance) -> bool {
        matches!(card_type_of(s.state(), card), CardType::Trap | CardType::FieldTrap)
    }

    fn of_def(cards: &[CardInstance], def_id: &str) -> usize {
        cards.iter().filter(|card| card.def_id == def_id).count()
    }

    fn backrow_cards(s: &Scenario) -> Vec<CardInstance> {
        (1..=BACKROW_ZONES).filter_map(|lane| s.backrow(P1, lane)).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r60_it_draws_5_of_the_decks_traps_and_adds_none_when_the_deck_holds_enough() {
            let mut s = board(false, deck(6, 2), json!([]));
            s.play(ID, json!({}));
            let hand = s.hand(P1);
            assert_eq!(of_def(&hand, TRAP), 5, "five drawn");
            assert_eq!(hand.iter().filter(|card| is_trap(&s, card)).count(), 5, "and nothing added");
            assert_eq!(of_def(&s.pile(P1, "library"), TRAP), 1, "one left in the deck");
            assert_eq!(s.count("drawn"), 5);
        }

        #[test]
        fn it_adds_random_traps_for_the_ones_the_deck_lacked() {
            let mut s = board(false, deck(2, 3), json!([]));
            s.play(ID, json!({}));
            let hand = s.hand(P1);
            assert_eq!(of_def(&hand, TRAP), 2, "the deck's two are drawn");
            let traps: Vec<&CardInstance> = hand.iter().filter(|card| is_trap(&s, card)).collect();
            assert_eq!(traps.len(), 5, "three random Traps make up the rest");
            assert_eq!(of_def(&s.pile(P1, "library"), TRAP), 0);
            assert_eq!(of_def(&s.pile(P1, "library"), FILLER), 3, "no other card is drawn");
            let added: Vec<&CardInstance> = traps.into_iter().filter(|card| card.def_id != TRAP).collect();
            for card in nonempty(added, "the added Traps") {
                assert!(!crate::card_def(&card.def_id).token, "{}: no token", card.def_id);
            }
        }

        #[test]
        fn a_deck_with_no_trap_adds_five() {
            let mut s = board(false, deck(0, 3), json!([]));
            s.play(ID, json!({}));
            let hand = s.hand(P1);
            assert_eq!(hand.iter().filter(|card| is_trap(&s, card)).count(), 5);
            assert_eq!(s.count("drawn"), 0);
        }

        #[test]
        fn r1748000_the_count_reads_through_param() {
            let mut s = board(false, deck(8, 0), json!([]));
            let wanted = crate::upgrade_number(&mut s, ID, "traps");
            assert_eq!(wanted, 6);
            s.play(ID, json!({}));
            assert_eq!(of_def(&s.hand(P1), TRAP), 6);
            assert_eq!(of_def(&s.pile(P1, "library"), TRAP), 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r64_it_recruits_5_traps_face_down_into_the_backrow() {
            let mut s = board(true, deck(6, 2), json!([]));
            s.play(ID, json!({}));
            let backrow = backrow_cards(&s);
            assert_eq!(backrow.len(), 5);
            assert_eq!(of_def(&backrow, TRAP), 5, "all five from the deck");
            assert_eq!(of_def(&s.pile(P1, "library"), TRAP), 1);
            assert_eq!(of_def(&s.hand(P1), TRAP), 0, "nothing is drawn");
        }

        #[test]
        fn it_summons_random_traps_for_the_ones_the_deck_lacked() {
            let mut s = board(true, deck(2, 3), json!([]));
            s.play(ID, json!({}));
            let backrow = backrow_cards(&s);
            assert_eq!(backrow.len(), 5, "two recruited, three random");
            assert_eq!(of_def(&backrow, TRAP), 2);
            assert!(backrow.iter().all(|card| is_trap(&s, card)));
            assert_eq!(of_def(&s.pile(P1, "library"), TRAP), 0);
        }

        #[test]
        fn r129_a_full_backrow_stops_both_alike() {
            let mut s = board(true, deck(1, 3), json!([FIELD_SPELL, FIELD_SPELL, FIELD_SPELL]));
            s.play(ID, json!({}));
            let backrow = backrow_cards(&s);
            assert_eq!(backrow.len(), 5, "two zones were open");
            assert_eq!(of_def(&backrow, TRAP), 1, "the deck's one is recruited");
            assert_eq!(of_def(&backrow, FIELD_SPELL), 3);
        }
    }
}
