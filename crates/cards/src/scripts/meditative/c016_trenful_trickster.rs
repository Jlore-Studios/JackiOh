//! M #16 Trenful Trickster (SPEC §8.8 row 16, R33, R60, R64, R129, R351, R386): (2) Unit, Rare, 1/5 → 2/10.
//!
//! Base:    "End of turn: Summon {traps|random Trap|random Traps}."
//! Radiant: the same, traps 2.
//! Engine: `summon_random` over the Trap types, as Core #67 Zoomerbin Oomen's verb, once per trap: a
//! non-token Trap or Field Trap of every set that ships (R380, R1420), each face-down into the leftmost
//! open backrow zone of its controller (R64; no lane is named). The opponent sees only a back and its
//! cost (R33, R351). With no open zone nothing is summoned and no random number is drawn (R129). The
//! Radiant's two picks may repeat (R60).

use crate::query::TRAP_TYPES;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-016";

fn trenful_trickster() -> Script {
    Script {
        end_of_turn: Some(hook(|ctx| {
            (0..param(&*ctx, "traps").max(0))
                .map(|_| summon_random(json_as(json!({ "query": { "type": TRAP_TYPES }, "player": "self" }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: trenful_trickster(),
        radiant: trenful_trickster(),
    }
}

// M #16 Trenful Trickster — SPEC §8.8 row 16, BUILD M10 row M 16: "At your end of turn, summons one random
// Trap or Field Trap (a shipped non-token card of any set, R380) face-down into your leftmost open backrow
// zone (R64); nothing at the opponent's end of turn; a full backrow summons nothing and draws no random
// number (R129); the opponent sees a back and its cost (R33, R351); the trap it sets fires as any set trap
// does; traps reads through param() (R386); the Radiant face makes two picks, which may repeat (R60)".
//
// Every game gives p1 no mana, so a Bread and Butter pick stays quiet at its end of turn, and p2 a Unit, so
// a Siphon Squad pick has something to Tribute but itself. A pick is read off the `summoned` events of the
// end-of-turn step. The pick is random, so no seed is pinned: a test that needs a particular trap loops
// seeds until the Trickster sets it.
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::js;
    use crate::query::TRAP_TYPES;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008";
    const FILLER: &str = "core-010";
    const STOCKPILE: &str = "core-005";
    const MANA_WELL: &str = "core-006";
    const COUNTERSPELL: &str = "classic-017";

    /// p1 owns the Trickster with `wells` Mana Wells in its first backrow zones and no mana; p2 holds a Unit.
    fn board(seed: &str, radiant: bool, wells: usize, stock: bool) -> Scenario {
        let mut p2_hand = vec![FILLER];
        if stock {
            p2_hand.push(STOCKPILE);
        }
        crate::scenario(json!({
            "seed": seed,
            "p1": {
                "field": [{ "def": ID, "radiant": radiant }],
                "backrow": vec![MANA_WELL; wells],
                "hand": [FILLER],
                "mana": 0,
                "library": [VANILLA, VANILLA, VANILLA],
            },
            "p2": { "field": [VANILLA], "hand": p2_hand, "library": [VANILLA, VANILLA, VANILLA] },
        }))
    }

    /// The traps p1's turn end set: each pick's def and backrow lane, in order.
    fn picks(s: &Scenario) -> Vec<(String, i32)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { player, def_id, row, lane, .. } if *player == P1 && *row == Row::Backrow => {
                    Some((def_id.clone(), *lane))
                }
                _ => None,
            })
            .collect()
    }

    fn trap_pool() -> Vec<String> {
        crate::query::query(&json_as(json!({ "type": TRAP_TYPES })))
            .into_iter()
            .map(|def| def.id.clone())
            .collect()
    }

    /// A game whose Trickster's end-of-turn pick is a Counterspell, found by seed.
    fn counterspell_board(stock: bool) -> Scenario {
        for n in 0..300 {
            let mut s = board(&format!("trickster-{n}"), false, 0, stock);
            s.end_turn();
            if picks(&s).first().is_some_and(|(def_id, _)| def_id == COUNTERSPELL) {
                return board(&format!("trickster-{n}"), false, 0, stock);
            }
        }
        panic!("no seed in 0..300 set a Counterspell");
    }

    #[test]
    fn is_a_2_cost_1_5_declaring_traps() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.cost, CardCost::Fixed(2));
        assert_eq!(def.type_, CardType::Unit);
        assert_eq!(def.rarity, Rarity::Rare);
        assert!(def.tags.is_empty());
        assert_eq!(
            [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
            [Some(1), Some(5), Some(2), Some(10)]
        );
        assert_eq!(
            js(&def.params),
            json!([{ "key": "traps", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1 }])
        );
        let scripts = super::script();
        assert!(scripts.base.end_of_turn.is_some());
        assert!(scripts.radiant.end_of_turn.is_some());
    }

    mod base {
        use super::*;

        #[test]
        fn r64_at_your_end_of_turn_sets_one_shipped_non_token_trap_in_the_leftmost_open_zone() {
            let pool = trap_pool();
            assert!(pool.len() > 1);
            for n in 0..20 {
                let mut s = board(&format!("trickster-{n}"), false, 1, false);
                s.end_turn();
                let set = picks(&s);
                assert_eq!(set.len(), 1, "seed {n}: {set:?}");
                // Mana Well holds lane 1, so the pick goes to lane 2.
                assert_eq!(set[0].1, 2, "seed {n}");
                assert!(pool.contains(&set[0].0), "seed {n}: {}", set[0].0);
                let def = crate::card_def(&set[0].0);
                assert!(!def.token, "{}", def.id);
                assert!(s.backrow(P1, 2).is_some_and(|card| card.def_id == set[0].0));
            }
        }

        #[test]
        fn nothing_at_the_opponents_end_of_turn() {
            let mut s = crate::scenario(json!({
                "active": "p2",
                "p1": { "field": [ID], "hand": [FILLER], "library": [VANILLA, VANILLA] },
                "p2": { "field": [VANILLA], "hand": [FILLER], "library": [VANILLA, VANILLA] },
            }));
            s.end_turn();
            assert!(picks(&s).is_empty());
            assert!(s.backrow(P1, 1).is_none());
        }

        #[test]
        fn r129_a_full_backrow_summons_nothing_and_draws_no_random_number() {
            let mut full = board("trickster-full", false, 5, false);
            full.end_turn();
            assert!(picks(&full).is_empty());
            // The same game with a Vanilla in its place draws exactly as many random numbers.
            let mut vanilla = crate::scenario(json!({
                "seed": "trickster-full",
                "p1": {
                    "field": [VANILLA],
                    "backrow": vec![MANA_WELL; 5],
                    "hand": [FILLER],
                    "mana": 0,
                    "library": [VANILLA, VANILLA, VANILLA],
                },
                "p2": { "field": [VANILLA], "hand": [FILLER], "library": [VANILLA, VANILLA, VANILLA] },
            }));
            vanilla.end_turn();
            assert_eq!(full.state().rng_cursor, vanilla.state().rng_cursor);
        }

        #[test]
        fn r33_r351_the_opponent_sees_a_back_and_its_cost() {
            let mut s = counterspell_board(false);
            s.end_turn();
            let view = js(&s.view(P2));
            let card = &view["opponent"]["backrow"][0];
            assert_eq!(card["faceDown"], json!(true));
            assert_eq!(card["cost"], json!(2));
            assert!(card.get("defId").is_none(), "{card}");
            assert!(card.get("name").is_none(), "{card}");
        }

        #[test]
        fn the_trap_it_sets_fires_as_any_set_trap_does() {
            let mut s = counterspell_board(true);
            s.end_turn();
            assert_eq!(s.state().active, P2);
            let hand = s.hand(P2).len();
            s.play(STOCKPILE, json!({}));
            assert!(s.last_events().iter().any(|event| event.event_type() == GameEventType::TrapFired));
            // Countered: the Stockpile drew nothing, so the hand only lost the card played.
            assert_eq!(s.hand(P2).len(), hand - 1);
        }

        #[test]
        fn r386_a_buff_sets_two() {
            let mut s = board("trickster-buff", false, 0, false);
            assert_eq!(crate::upgrade_number(&mut s, ID, "traps"), 2);
            s.end_turn();
            let set = picks(&s);
            assert_eq!(set.iter().map(|(_, lane)| *lane).collect::<Vec<_>>(), vec![1, 2]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_2_10_and_sets_two_traps_in_the_two_leftmost_open_zones() {
            let pool = trap_pool();
            let mut s = board("trickster-radiant", true, 1, false);
            s.expect_stats(ID, json!({ "attack": 2, "health": 10, "maxHealth": 10 }));
            s.end_turn();
            let set = picks(&s);
            assert_eq!(set.iter().map(|(_, lane)| *lane).collect::<Vec<_>>(), vec![2, 3]);
            assert!(set.iter().all(|(def_id, _)| pool.contains(def_id)));
        }

        #[test]
        fn with_one_open_zone_it_sets_one() {
            let mut s = board("trickster-one-zone", true, 4, false);
            s.end_turn();
            let set = picks(&s);
            assert_eq!(set.len(), 1);
            assert_eq!(set[0].1, 5);
        }

        #[test]
        fn r60_the_two_picks_may_repeat() {
            let repeated = (0..400).any(|n| {
                let mut s = board(&format!("trickster-repeat-{n}"), true, 0, false);
                s.end_turn();
                let set = picks(&s);
                set.len() == 2 && set[0].0 == set[1].0
            });
            assert!(repeated, "no seed in 0..400 set the same Trap twice");
        }
    }
}
