//! SPEC §8.12 S+ #T-tears-of-the-kwzki-tree Tears of the Kwzki Tree (4) Spell, Terminus, Mythic, a
//! Terminus token of the Stinkyscape+ set (R1420, R3535000; R3569000, R3569001).
//!   Base:    "All cards for the rest of the game are Radiant, at the end of any turn buff all cards."
//!   Radiant: "… Gain {mana} mana."
//!
//! Engine: `make_all_radiant` sets the game's Radiant flag, which the settled state check keeps true
//! of every card of both players in every zone, a card made later at the next check (R3569000); a
//! played Radiant card is therefore never base again, and a Radiant card never turns back but for
//! De-Radiant. The Buff is `forRestOfGame` for each player at the end of their turn (R458, R1861002), so
//! it runs at the end of every turn of the game, both players': `upgrade` over every card of both sides
//! in the field, hands, libraries, graveyards and exile piles, once each, the hidden piles cued whole
//! (R440; R3569001). The Radiant face adds `mana` temporary mana. The badge says so on both sides (R169).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-t-tears-of-the-kwzki-tree";

/// The rest-of-game effects' step (§10.6).
const STEP: &str = "buff";
/// The badge each side carries for the rest of the game.
const BADGE: &str = "Tears of the Kwzki Tree: all cards are Radiant; end of turn: Buff all cards.";

fn tears(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut effects = vec![
                make_all_radiant(),
                for_rest_of_game(json_as(json!({ "step": STEP, "label": BADGE, "atEnd": true }))),
                for_rest_of_game(json_as(json!({
                    "step": STEP,
                    "label": BADGE,
                    "atEnd": true,
                    "player": "enemy",
                }))),
            ];
            if radiant {
                effects.push(gain_mana(json_as(json!({ "amount": param(ctx, "mana") }))));
            }
            effects
        })),
        delayed: Some(hook(|_ctx| {
            vec![upgrade(json_as(json!({
                "scope": { "side": "any", "zones": ["field", "hand", "library", "graveyard", "exile"] },
                "times": 1,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: tears(false),
        radiant: tears(true),
    }
}

// S+ #T-tears-of-the-kwzki-tree — SPEC §8.12 (R3569000, R3569001; R458, R1861002, R440).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::{filler, run_as, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005"; // Stockpile: a Spell (§2.5).
    const UNIT: &str = "stand-in (1) 2/3";

    fn game(seed: &str, radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": filler(6),
                "field": [UNIT],
                "graveyard": [UNIT],
                "exile": [UNIT],
                "mana": 4,
            },
            "p2": {
                "hand": [FILLER],
                "library": filler(6),
                "field": [UNIT],
                "graveyard": [UNIT],
                "exile": [UNIT],
            },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    /// Every card of `player`, in every pile and lane.
    fn every_card(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        let mut cards: Vec<CardInstance> = Vec::new();
        for pile in ["hand", "library", "graveyard", "exile"] {
            cards.extend(s.pile(player, pile));
        }
        for lane in 1..=5 {
            cards.extend(s.unit(player, lane));
            cards.extend(s.backrow(player, lane));
        }
        cards
    }

    /// The cards the Buff reached since `from`, by instance id.
    fn buffed(s: &Scenario, from: usize) -> Vec<String> {
        s.events()[from..]
            .iter()
            .filter_map(|event| match event {
                GameEvent::Upgraded { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r3569000_every_card_of_both_players_is_radiant_now_and_later() {
            let mut s = game("tears-radiant-world", false);
            for player in [P1, P2] {
                assert!(every_card(&s, player).iter().all(|card| !card.radiant));
            }
            s.play(ID, json!({}));
            for player in [P1, P2] {
                let cards = every_card(&s, player);
                assert!(!cards.is_empty());
                assert!(
                    cards.iter().all(|card| card.radiant),
                    "every card of {player:?} is Radiant"
                );
            }
            // A card made later is Radiant at the next settled check: the library draws, the made card
            // is in a hand, and Tears itself came to rest in the graveyard.
            run_as(
                &mut s,
                jackioh_engine::effects::add_to_hand(json_as(json!({ "defId": FILLER, "player": "enemy" }))),
                P1,
            );
            s.end_turn();
            for player in [P1, P2] {
                assert!(
                    every_card(&s, player).iter().all(|card| card.radiant),
                    "{player:?} after a turn"
                );
            }
            assert!(s.pile(P2, "hand").len() >= 2);
        }

        #[test]
        fn r3569001_every_card_is_buffed_at_the_end_of_each_turn() {
            let mut s = game("tears-buff", false);
            s.play(ID, json!({}));
            let from = s.events().len();
            s.end_turn();
            let at_p1_end = buffed(&s, from);
            // Both sides, every zone: p1's library and p2's library are hidden piles (cued whole),
            // the Units on the field and in the graveyards and exile piles are public ones.
            let p1_library: Vec<String> = s.pile(P1, "library").iter().map(|card| card.id.clone()).collect();
            let p2_library: Vec<String> = s.pile(P2, "library").iter().map(|card| card.id.clone()).collect();
            assert!(p1_library.iter().any(|id| at_p1_end.contains(id)), "p1's library");
            assert!(p2_library.iter().any(|id| at_p1_end.contains(id)), "p2's library");
            for (player, pile) in [(P1, "graveyard"), (P1, "exile"), (P2, "graveyard"), (P2, "exile")] {
                let unit = s
                    .pile(player, pile)
                    .into_iter()
                    .find(|card| card.def_id.starts_with("stand-in"));
                let unit = unit.unwrap_or_else(|| panic!("{player:?}'s {pile} holds its Unit"));
                assert!(at_p1_end.contains(&unit.id), "{player:?}'s {pile}");
            }
            for player in [P1, P2] {
                assert!(at_p1_end.contains(&s.unit_at(player, 1).id), "{player:?}'s Unit");
            }
            let mut once = at_p1_end.clone();
            once.sort();
            once.dedup();
            assert_eq!(once.len(), at_p1_end.len(), "each card is Buffed once");
            // And at the end of the other player's turn too: any turn.
            assert_eq!(s.state().active, P2);
            let from = s.events().len();
            s.end_turn();
            let at_p2_end = buffed(&s, from);
            assert!(at_p2_end.contains(&s.unit_at(P1, 1).id));
            assert!(at_p2_end.contains(&s.unit_at(P2, 1).id));
            let mut once = at_p2_end.clone();
            once.sort();
            once.dedup();
            assert_eq!(once.len(), at_p2_end.len(), "each card is Buffed once");
        }

        #[test]
        fn r3569000_the_base_face_gains_no_mana() {
            let mut s = game("tears-base-mana", false);
            s.play(ID, json!({}));
            s.expect_mana(P1, 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3569001_radiant_tears_gains_mana() {
            let mut s = game("tears-radiant-mana", true);
            s.play(ID, json!({}));
            // Cast for (4) from 4, then gains 4.
            s.expect_mana(P1, 4);
            for player in [P1, P2] {
                assert!(every_card(&s, player).iter().all(|card| card.radiant));
            }
        }
    }
}
