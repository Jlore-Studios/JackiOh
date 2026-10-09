//! M #27 Clip-Farming Lawyer (SPEC §8.8 row 27): (2) Unit, Human, Common, 6/5 → 12/10.
//!
//! Base:    "End of turn: Unlock a random zone. If you do, add {coins|Coin|Coins} to your hand."
//! Radiant: "End of turn: Unlock a random zone. If you do, add {coins|Coin|Coins} to your hand. If
//! no zone is Locked, Lock a random enemy zone."
//! Engine: the end-of-turn hook first reads the Locked zones on both sides and in both rows, with
//! `zones::is_locked` over `slots_of` (both in the prelude).
//! - If at least one is Locked, it runs NEW `unlock_random_zone(ZoneScope)`, then
//!   `add_to_hand({ defId: "core-t-coin" })` `coins` times.
//! - If none is Locked, the base face does nothing. The Radiant face runs
//!   `lock_random_zone({ side: enemy })`.
//! The card's `refs` are `[core-t-coin]`; "Coin" names The Coin ([[R903]]).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-027";

/// The Coin (§7), added to the hand for each Unlock.
const COIN: &str = "core-t-coin";

/// Whether any zone both sides and both rows hold is Locked now.
fn any_zone_locked(state: &GameState) -> bool {
    [PlayerId::P1, PlayerId::P2].into_iter().any(|player| {
        [Row::Units, Row::Backrow]
            .into_iter()
            .any(|row| slots_of(player, row).into_iter().any(|slot| is_locked(state, &slot)))
    })
}

fn lawyer(lock_when_none: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| {
            if any_zone_locked(ctx.state) {
                let mut effects = vec![unlock_random_zone(json_as(json!({})))];
                for _ in 0..param(&*ctx, "coins") {
                    effects.push(add_to_hand(json_as(json!({ "defId": COIN }))));
                }
                effects
            } else if lock_when_none {
                vec![lock_random_zone(json_as(json!({ "side": "enemy" })))]
            } else {
                vec![]
            }
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: lawyer(false),
        radiant: lawyer(true),
    }
}

// M #27 Clip-Farming Lawyer — SPEC §8.8 row 27, BUILD M10 row M 27: "At your end of turn only,
// with a Locked zone on either side and in either row, one Locked zone is unlocked at random
// (`unlocked`) and The Coin is added to your hand (MD-B4); with no Locked zone nothing happens and
// no random number is drawn (R129); the hand cap burns the Coin; coins reads through `param()`;
// radiant 12/10, 2 Coins, and with no Locked zone a random unlocked enemy zone of either row is
// Locked".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const LAWYER: &str = "meditative-027";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4, no text: the R129 comparison.
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    /// p1 plays the Lawyer (base unless `radiant_face`).
    fn lawyered(seed: &str, radiant_face: bool) -> Scenario {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": LAWYER, "radiant": radiant_face }, FILLER],
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(LAWYER, json!({}));
        s
    }

    /// Every Locked zone, as "p1:units:2", in R68's walk.
    fn locked_zones(s: &Scenario) -> Vec<String> {
        let mut out = Vec::new();
        for player in [P1, P2] {
            for (row, lanes) in
                [("units", &s.state().players[player].locks.units), ("backrow", &s.state().players[player].locks.backrow)]
            {
                for (at, locked) in lanes.iter().enumerate() {
                    if *locked {
                        out.push(format!("{}:{row}:{}", player.as_str(), at + 1));
                    }
                }
            }
        }
        out
    }

    /// The coins in p1's hand.
    fn coins_in_hand(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id == COIN).collect()
    }

    fn unlocked(events: &[GameEvent]) -> Vec<Value> {
        events.iter().filter(|event| matches!(event, GameEvent::Unlocked { .. })).map(crate::js).collect()
    }

    fn locked(events: &[GameEvent]) -> Vec<Value> {
        events.iter().filter(|event| matches!(event, GameEvent::Locked { .. })).map(crate::js).collect()
    }

    mod m27_clip_farming_lawyer {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r900_at_your_end_of_turn_a_locked_zone_is_unlocked_and_a_coin_is_added() {
                let mut s = lawyered("lawyer-one-lock", false);
                s.state_mut().players.p2.locks.backrow[0] = true;

                s.end_turn();

                assert_eq!(locked_zones(&s), Vec::<String>::new());
                assert_eq!(
                    unlocked(s.events()),
                    vec![json!({ "type": "unlocked", "player": "p2", "row": "backrow", "lane": 1 })]
                );
                assert_eq!(coins_in_hand(&s).len(), 1);
            }

            #[test]
            fn r900_one_of_several_locked_zones_is_picked_at_random_the_same_for_the_same_seed() {
                let locked = |seed: &str| -> (Vec<String>, Vec<Value>, usize) {
                    let mut s = lawyered(seed, false);
                    s.state_mut().players.p1.locks.units[1] = true;
                    s.state_mut().players.p2.locks.backrow[4] = true;
                    s.state_mut().players.p2.locks.units[3] = true;
                    let cursor = s.state().rng_cursor;
                    s.end_turn();
                    let draws = (s.state().rng_cursor - cursor) as usize;
                    (locked_zones(&s), unlocked(s.events()), draws)
                };
                let (first, first_events, first_draws) = locked("lawyer-pick");
                // Exactly one of the three opened, by one event and one draw.
                assert_eq!(first.len(), 2);
                assert_eq!(first_events.len(), 1);
                assert_eq!(first_draws, 1);
                // The same seed replays to the same zone.
                assert_eq!(locked("lawyer-pick").0, first);
                // Another seed may open another one.
                let mut opened = vec![first];
                for n in 0..8 {
                    let (zones, _, _) = locked(&format!("lawyer-pick-{n}"));
                    if !opened.contains(&zones) {
                        opened.push(zones);
                    }
                }
                assert!(opened.len() > 1, "the pick varies with the seed");
            }

            #[test]
            fn r900_r129_with_no_locked_zone_nothing_happens_and_no_number_is_drawn() {
                // The same game with a textless vanilla in the Lawyer's place: the rng cursor and
                // the event types match it exactly.
                let mut s = lawyered("lawyer-no-lock", false);
                crate::register_all();
                let mut plain = scenario(json!({
                    "seed": "lawyer-no-lock",
                    "p1": { "hand": [VANILLA, FILLER], "library": filler(4) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                plain.play(VANILLA, json!({}));

                s.end_turn();
                plain.end_turn();

                assert_eq!(s.state().rng_cursor, plain.state().rng_cursor);
                let types = |s: &Scenario| -> Vec<String> {
                    s.events().iter().map(|event| event.event_type().to_string()).collect()
                };
                assert_eq!(types(&s), types(&plain));
                assert!(unlocked(s.events()).is_empty());
                assert!(coins_in_hand(&s).is_empty());
            }

            #[test]
            fn does_nothing_at_the_opponent_s_end_of_turn() {
                let mut s = lawyered("lawyer-foe-turn", false);

                s.end_turn();
                assert!(unlocked(s.events()).is_empty());
                s.state_mut().players.p2.locks.units[2] = true;

                s.end_turn();

                assert_eq!(locked_zones(&s), vec!["p2:units:3".to_string()]);
                assert!(unlocked(s.events()).is_empty());
                assert!(coins_in_hand(&s).is_empty());
            }

            #[test]
            fn a_full_hand_burns_the_coin() {
                crate::register_all();
                let hand: Vec<Value> =
                    std::iter::once(json!(LAWYER)).chain((0..10).map(|_| json!(FILLER))).collect();
                let mut s = scenario(json!({
                    "seed": "lawyer-burn",
                    "p1": { "hand": hand, "library": filler(4) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(LAWYER, json!({}));
                s.state_mut().players.p2.locks.backrow[0] = true;

                s.end_turn();

                assert_eq!(unlocked(s.events()).len(), 1);
                assert!(coins_in_hand(&s).is_empty());
                assert_eq!(s.hand(P1).len(), 10);
                assert!(
                    s.events()
                        .iter()
                        .any(|event| matches!(event, GameEvent::Burned { def_id, .. } if def_id == COIN))
                );
                assert!(s.pile(P1, "graveyard").iter().any(|card| card.def_id == COIN));
            }

            #[test]
            fn coins_reads_through_param() {
                let mut s = lawyered("lawyer-param", false);
                assert_eq!(crate::upgrade_number(&mut s, LAWYER, "coins"), 2);
                s.state_mut().players.p2.locks.backrow[0] = true;

                s.end_turn();

                assert_eq!(coins_in_hand(&s).len(), 2);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn adds_two_coins_and_has_12_10() {
                let mut s = lawyered("lawyer-radiant", true);
                s.state_mut().players.p2.locks.backrow[0] = true;

                s.end_turn();

                assert_eq!(coins_in_hand(&s).len(), 2);
                assert_eq!(locked_zones(&s), Vec::<String>::new());
                let me = s.unit(P1, 1).expect("the Lawyer on the field");
                s.expect_stats(&me, json!({ "attack": 12, "health": 10, "maxHealth": 10 }));
            }

            #[test]
            fn r900_with_no_locked_zone_locks_one_random_unlocked_enemy_zone() {
                let mut s = lawyered("lawyer-radiant-lock", true);

                s.end_turn();

                let zones = locked_zones(&s);
                assert_eq!(zones.len(), 1);
                assert!(zones[0].starts_with("p2:"));
                assert_eq!(locked(s.events()).len(), 1);
                assert!(coins_in_hand(&s).is_empty());
            }

            #[test]
            fn with_a_locked_zone_it_unlocks_and_locks_nothing() {
                let mut s = lawyered("lawyer-radiant-unlock", true);
                s.state_mut().players.p1.locks.units[0] = true;

                s.end_turn();

                assert_eq!(locked_zones(&s), Vec::<String>::new());
                assert_eq!(unlocked(s.events()).len(), 1);
                assert!(locked(s.events()).is_empty());
                // The Radiant face adds two Coins per Unlock.
                assert_eq!(coins_in_hand(&s).len(), 2);
            }
        }
    }
}
