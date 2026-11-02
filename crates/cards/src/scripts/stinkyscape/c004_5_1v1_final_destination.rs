//! S #4.5 1v1 Final Destination (SPEC §8.10 row S 4.5, SPEC §7; §2.4, §3.2, §5.1, §6.3 Lock, Set
//! health, Discard, R18, R33, R62, R688, R800, R1857002, R1870005): (4) Trap, Terminus, Token (printed
//! Mythic), one of Olivian Expanse's picks.
//!   Base:    "Reveals when your opponent's turn ends: Lock every lane but one, picked at random. Set
//!            each hero's health to 1. Both players discard every Spell in their hand."
//!   Radiant: "… Then fill your hand with Book of Buffs. They cost (0)."
//!
//! Engine: a face-down Trap in the end-of-turn trap window of its controller's opponent's turn (R62),
//! never its controller's own (C #65 Ace in the Hole's `turnEnded` trigger, the other player's), going
//! to the graveyard as it fires. One lane drawn uniformly from the match rng stays open and
//! `lock_lane` Locks the other four whole, both sides' unit and backrow zones (C #71's; a Lock evicts
//! nothing, R688).

use jackioh_engine::effects::{add_to_hand, discard, lock_lane, set_health};
use jackioh_engine::draw::discard_guarded;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-004-5";

/// C+ #71 Book of Buff, the Radiant face's fill.
const BOOK_OF_BUFF: &str = "classicplus-071";

/// "Set each hero's health to 1."
const HEALTH: i32 = 1;

/// The Spells in `player`'s hand, in hand order.
fn spells_in_hand(state: &GameState, player: PlayerId) -> Vec<String> {
    zone_cards(state, player, OffFieldZone::Hand)
        .into_iter()
        .filter(|card| card_type_of(state, card) == CardType::Spell)
        .map(|card| card.id)
        .collect()
}

fn final_destination(ctx: &mut EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    let open = ctx.rng.int(UNIT_ZONES) + 1;
    let mut effects: Vec<Effect> = (1..=UNIT_ZONES)
        .filter(|lane| *lane != open)
        .map(|lane| lock_lane(json_as(json!({ "lane": lane }))))
        .collect();
    effects.extend(
        ["selfHero", "enemyHero"].map(|hero| set_health(json_as(json!({ "to": { "of": hero }, "value": HEALTH })))),
    );
    let controller = ctx.controller;
    let mut discarded = 0;
    for player in [controller, opponent_of(controller)] {
        let spells = spells_in_hand(ctx.state, player);
        // R800: it fires on the opponent's turn, so a discard guard of its controller's keeps their
        // Spells, and the fill counts them as held.
        if player == controller && !discard_guarded(ctx.state, player) {
            discarded = spells.len() as i32;
        }
        effects.extend(
            spells
                .into_iter()
                .map(|id| discard(json_as(json!({ "target": { "of": "instance", "instanceId": id } })))),
        );
    }
    if radiant {
        let kept = zone_count(ctx.state, controller, OffFieldZone::Hand) - discarded;
        let room = (hand_cap_of(ctx.state, controller) - kept).max(0);
        effects.extend(
            (0..room).map(|_| add_to_hand(json_as(json!({ "defId": BOOK_OF_BUFF, "costOverride": 0 })))),
        );
    }
    effects
}

/// "Reveals when your opponent's turn ends", in that turn's end-of-turn trap window (R62).
fn fire(radiant: bool) -> TriggerDef {
    TriggerDef::new("final-destination", &[GameEventType::TurnEnded], move |ctx, _event| {
        final_destination(ctx, radiant)
    })
    .with_when(|ctx, event| {
        matches!(event, GameEvent::TurnEnded { player, .. } if *player == opponent_of(ctx.controller))
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            triggers: vec![fire(false)],
            ..Script::default()
        },
        radiant: Script {
            triggers: vec![fire(true)],
            ..Script::default()
        },
    }
}

// S #4.5 1v1 Final Destination — SPEC §8.10 row S 4.5, BUILD M12 row S 4.5 (R1870005).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FILLER: &str = "core-005"; // (1) Spell Stockpile.
    const LUNAR: &str = "core-035"; // (1) Spell.
    const WELL: &str = "core-006"; // (3) Field Spell.
    const GAMBIT: &str = "classic-052"; // (2) Trap.
    const BOOK_OF_BUFF: &str = "classicplus-071";
    const DISRUPTOR: &str = "meditative-001"; // (2) Field Spell, a discard guard (R800).
    const DECK: [&str; 6] = [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA];

    fn board(seed: &str, active: &str, radiant: bool) -> Scenario {
        board_with(seed, active, radiant, json!([]))
    }

    fn board_with(seed: &str, active: &str, radiant: bool, backrow: Value) -> Scenario {
        let mut backrow = backrow.as_array().cloned().unwrap_or_default();
        backrow.push(json!({ "def": ID, "radiant": radiant, "faceUp": false, "lane": 3 }));
        let mut s = scenario(json!({
            "seed": seed,
            "active": active,
            "p1": {
                "hand": [FILLER, VANILLA, LUNAR, WELL, GAMBIT],
                "backrow": backrow,
                "library": DECK,
            },
            "p2": { "hand": [LUNAR, VANILLA, FILLER, GAMBIT], "library": DECK },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    fn fired(seed: &str, radiant: bool) -> Scenario {
        let mut s = board(seed, "p2", radiant);
        s.end_turn();
        s
    }

    fn trap_fired(s: &Scenario) -> bool {
        s.events().iter().any(|event| event.event_type() == GameEventType::TrapFired)
    }

    fn locked_lanes(s: &Scenario) -> Vec<i32> {
        (1..=UNIT_ZONES)
            .filter(|lane| {
                [P1, P2].into_iter().all(|player| {
                    [Row::Units, Row::Backrow]
                        .into_iter()
                        .all(|row| is_locked(s.state(), ZoneSlot { player, row, lane: *lane }))
                })
            })
            .collect()
    }

    fn any_locked(s: &Scenario, lane: i32) -> bool {
        [P1, P2].into_iter().any(|player| {
            [Row::Units, Row::Backrow]
                .into_iter()
                .any(|row| is_locked(s.state(), ZoneSlot { player, row, lane }))
        })
    }

    fn hand_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.hand(player).into_iter().map(|card| card.def_id).collect()
    }

    mod r1870005_final_destination {
        use super::*;

        #[test]
        fn r1870005_fires_at_opponents_end_never_its_controllers() {
            let mut s = board("final-destination-own", "p1", false);
            s.end_turn();
            assert!(!trap_fired(&s), "its controller's turn end passes it by");
            assert!(locked_lanes(&s).is_empty());
            s.end_turn();
            assert!(trap_fired(&s), "the opponent's turn end fires it");
            let card = s
                .pile(P1, "graveyard")
                .into_iter()
                .find(|card| card.def_id == ID)
                .expect("it went to the graveyard");
            assert_eq!(card.def_id, ID);
        }

        #[test]
        fn r1870005_four_lanes_locked_open_one_varies_over_seeds() {
            let mut open: Vec<i32> = Vec::new();
            for i in 0..20 {
                let s = fired(&format!("final-destination-lanes-{i}"), false);
                let locked = locked_lanes(&s);
                assert_eq!(locked.len(), 4, "four lanes locked whole");
                let free: Vec<i32> = (1..=UNIT_ZONES).filter(|lane| !locked.contains(lane)).collect();
                assert_eq!(free.len(), 1);
                assert!(!any_locked(&s, free[0]), "the open lane has no Locked zone");
                open.push(free[0]);
            }
            open.sort_unstable();
            open.dedup();
            assert!(open.len() > 1, "the open lane varies: {open:?}");
        }

        #[test]
        fn r1870005_both_heroes_at_one() {
            let mut s = fired("final-destination-health", false);
            s.expect_health(P1, 1).expect_health(P2, 1);
        }

        #[test]
        fn r1870005_spells_discarded_others_kept() {
            let s = fired("final-destination-discards", false);
            let mut mine = hand_defs(&s, P1);
            mine.sort();
            assert_eq!(mine, vec![GAMBIT, WELL, VANILLA, VANILLA]);
            assert_eq!(hand_defs(&s, P2), vec![VANILLA, GAMBIT]);
            let discards: Vec<(PlayerId, String)> = s
                .events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Discarded { owner, def_id, .. } => Some((*owner, def_id.clone())),
                    _ => None,
                })
                .collect();
            assert_eq!(
                discards,
                vec![
                    (P1, FILLER.to_string()),
                    (P1, LUNAR.to_string()),
                    (P2, LUNAR.to_string()),
                    (P2, FILLER.to_string()),
                ],
                "its controller's hand first, each in hand order"
            );
        }

        #[test]
        fn r1870005_controller_turn_follows() {
            let s = fired("final-destination-next", false);
            assert_eq!(s.state().active, P1, "its controller strikes first");
            assert!(s.state().result.is_none());
        }

        #[test]
        fn r1870005_radiant_fills_hand_with_free_book_of_buff() {
            let s = fired("final-destination-radiant", true);
            let hand = s.hand(P1);
            assert_eq!(hand.len(), HAND_CAP as usize, "filled");
            // Vanilla, Well and Gambit kept and seven Books; p1's turn's draw then burns (§2.4).
            let books: Vec<&CardInstance> = hand.iter().filter(|card| card.def_id == BOOK_OF_BUFF).collect();
            assert_eq!(books.len(), HAND_CAP as usize - 3);
            for book in books {
                assert_eq!(book.cost_override, Some(0));
            }
            let s = fired("final-destination-base", false);
            assert!(s.hand(P1).iter().all(|card| card.def_id != BOOK_OF_BUFF));
        }

        #[test]
        fn r1870005_radiant_fill_counts_a_guarded_hand_as_held() {
            let mut s = board_with(
                "final-destination-guarded",
                "p2",
                true,
                json!([{ "def": DISRUPTOR, "faceUp": true, "lane": 1 }]),
            );
            s.end_turn();
            let mut kept = hand_defs(&s, P1);
            kept.retain(|def| def != BOOK_OF_BUFF);
            kept.sort();
            assert_eq!(kept, vec![GAMBIT, FILLER, WELL, VANILLA, LUNAR]);
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize, "filled");
            assert!(
                !s.events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::Burned { def_id, .. } if def_id == BOOK_OF_BUFF)),
                "no Book burned"
            );
            assert_eq!(hand_defs(&s, P2), vec![VANILLA, GAMBIT], "the unguarded hand still discards");
        }
    }
}
