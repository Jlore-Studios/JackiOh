//! SPEC §8.12 S+ #79 A Glimpse into the Endless Sea (4) Spell, Wincon, Terminus, Mythic, a card of
//! the Stinkyscape+ set (R1420, R3535000; R3569005, R3569006).
//!   Base:    "Exile your deck. Start a {count} turn countdown. When it ends, win the game." (10)
//!   Radiant: "Exile your deck. Start an {count} turn countdown. When it ends, win the game." (8)
//!
//! Engine: the whole library is exiled with no draw, so no fatigue (§2.4), as S+ #80 does. The
//! countdown is `delay` "in N turns" (R1181000): a badge on its owner's side counts down the turns they
//! start, and at the start of their `count`th turn from now, before the draw (§2.2), the delayed step
//! wins them the game (R850; a hero at 0 still loses first). Only the owner's turns tick it, one made
//! on the other player's turn counting from their next (R3569005). The card is a Wincon, so Bob's Plan
//! may pick it: its plan is Core #75 Infinite Reserves, which keeps the empty deck from costing its
//! owner the game before the countdown ends (R3569006; `c001_bobs_plan.rs`).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-079";

/// The countdown's badge (R169).
const BADGE: &str = "A Glimpse into the Endless Sea: you win";
/// The delayed step, which `delayed` runs.
const STEP: &str = "win";

fn glimpse() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            vec![
                exile_bottom_of_library(json_as(json!({
                    "count": zone_cards(&*ctx.state, ctx.controller, OffFieldZone::Library).len(),
                }))),
                delay(json_as(json!({
                    "at": { "phase": "start", "player": "self" },
                    "step": STEP,
                    "turns": param(ctx, "count"),
                    "label": BADGE,
                }))),
            ]
        })),
        delayed: Some(hook(|_ctx| vec![win_game(json_as(json!({})))])),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: glimpse(),
        radiant: glimpse(),
    }
}

// S+ #79 A Glimpse into the Endless Sea — SPEC §8.12 row 79 (R3569005, R3569006; R1181000, R850).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, next_turn_of, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    fn game(seed: &str, radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "library": filler(5),
                // The empty deck is fatigue each turn (§2.4): a hero that outlasts a dozen of them.
                "health": 200,
                "mana": 4,
            },
            "p2": { "library": filler(20), "health": 200 },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    /// The countdown badges on p1's side, as `viewer` reads them (R169).
    fn badges(s: &Scenario, viewer: PlayerId) -> Vec<String> {
        let view = s.view(viewer);
        let modifiers = if viewer == P1 {
            view.you.modifiers
        } else {
            view.opponent.modifiers
        };
        modifiers.iter().map(|modifier| modifier.label.clone()).collect()
    }

    /// How many of p1's own turns pass after the cast before the game is won, and the winner.
    fn turns_to_win(s: &mut Scenario, bound: usize) -> (usize, Option<Winner>) {
        for turns in 1..=bound {
            next_turn_of(s, P1);
            if let Some(result) = s.state().result {
                return (turns, Some(result.winner));
            }
        }
        (bound, None)
    }

    mod base {
        use super::*;

        #[test]
        fn r3569005_the_deck_is_exiled_and_the_owner_wins_when_it_ends() {
            let mut s = game("glimpse-base", false);
            let deck: Vec<String> = s.pile(P1, "library").iter().map(|card| card.id.clone()).collect();
            assert_eq!(deck.len(), 5);
            s.play(ID, json!({}));
            assert!(s.pile(P1, "library").is_empty(), "the deck is exiled");
            assert_eq!(s.pile(P1, "exile").len(), 5);
            assert!(s.pile(P2, "library").len() == 20, "only the owner's deck");
            // The countdown is 10 of p1's turns; none of the first nine ends it.
            let (turns, winner) = turns_to_win(&mut s, 12);
            assert_eq!((turns, winner), (10, Some(Winner::P1)));
            let result = s.state().result.expect("the game is over");
            assert_eq!(result.reason, GameOverReason::WonByEffect);
        }

        #[test]
        fn r3569005_the_badge_counts_down() {
            let mut s = game("glimpse-badge", false);
            s.play(ID, json!({}));
            let at = |turns: i32| vec![format!("{BADGE} in {turns} turns")];
            assert_eq!(badges(&s, P1), at(10));
            assert_eq!(badges(&s, P2), at(10), "the other seat reads it too");
            // The opponent's turn is no tick of it; p1's own is.
            s.end_turn();
            assert_eq!(badges(&s, P1), at(10), "the other player's turn does not tick it");
            s.end_turn();
            assert_eq!(badges(&s, P1), at(9));
            assert_eq!(badges(&s, P2), at(9));
        }

        #[test]
        fn r3569005_an_empty_deck_has_nothing_to_exile_and_the_countdown_still_starts() {
            let mut s = game("glimpse-empty", false);
            s.state_mut().players[P1].library.clear();
            s.play(ID, json!({}));
            let (turns, winner) = turns_to_win(&mut s, 12);
            // With no deck, fatigue (1+2+…) is what the countdown races: 200 health holds.
            assert_eq!((turns, winner), (10, Some(Winner::P1)));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3569005_radiant_counts_8() {
            let mut s = game("glimpse-radiant", true);
            s.play(ID, json!({}));
            assert!(s.pile(P1, "library").is_empty());
            let (turns, winner) = turns_to_win(&mut s, 12);
            assert_eq!((turns, winner), (8, Some(Winner::P1)));
        }
    }
}
