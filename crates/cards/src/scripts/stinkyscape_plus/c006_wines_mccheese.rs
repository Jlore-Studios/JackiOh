//! SPEC §8.12 S+ #6 Wines McCheese, a card of the Stinkyscape+ set (R1420, R3535000). (2) Unit, Human, Epic.
//!   Base:    4/5. "Start of turn: Add a Fine Fermented Grape, Stinky Cheese, or The Coin to your hand at random."
//!   Radiant: 8/10. The same, each of the three Radiant.
//!
//! R3566002: at its controller's Start of turn (R62) one fair pick of the three by the match rng, then
//! `add_to_hand` of the one picked, Radiant on the Radiant face (the Coin too). All three are named
//! cards, so no pool and no R387. No prompt opens; the opponent sees a hidden card arrive. Declares no
//! numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-006";

/// Core's The Coin.
const COIN: &str = "core-t-coin";

/// R3566002: the three cards one pick is made among, in the order the text names them.
const MENU: [&str; 3] = [
    super::c006_1_fine_fermented_grape::ID,
    super::c006_2_stinky_cheese::ID,
    COIN,
];

fn wines(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |ctx| {
            let Some(id) = ctx.rng.pick(&MENU).copied() else {
                return vec![];
            };
            vec![add_to_hand(json_as(json!({ "defId": id, "radiant": radiant })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: wines(false),
        radiant: wines(true),
    }
}

// S+ #6 Wines McCheese — SPEC §8.12 row 6 (R3566002).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::{ID, MENU};
    use crate::{filler, js, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";

    fn standing(seed: &str, radiant: bool) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [FILLER], "field": [{ "def": ID, "radiant": radiant, "lane": 1 }], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// The menu cards p1's hand holds that it did not hold before.
    fn added(s: &Scenario, before: &[String]) -> Vec<CardInstance> {
        s.hand(P1)
            .into_iter()
            .filter(|card| !before.contains(&card.id) && MENU.contains(&card.def_id.as_str()))
            .collect()
    }

    fn hand_ids(s: &Scenario) -> Vec<String> {
        s.hand(P1).iter().map(|card| card.id.clone()).collect()
    }

    #[test]
    fn r3566002_start_of_turn_adds_one_of_the_three() {
        let mut seen: Vec<String> = Vec::new();
        for n in 0..24 {
            let mut s = standing(&format!("r3566002-{n}"), false);
            let before = hand_ids(&s);
            s.start_turn();
            assert!(s.state().pending.is_none(), "{n}: no prompt");
            let made = added(&s, &before);
            assert_eq!(made.len(), 1, "{n}: one card of the menu");
            assert!(!made[0].radiant, "{n}");
            assert_eq!(s.hand(P1).len(), before.len() + 2, "{n}: the draw and the pick");
            if !seen.contains(&made[0].def_id) {
                seen.push(made[0].def_id.clone());
            }
            // Hidden from the opponent: a count, never the card.
            let view: Value = js(&s.view(P2));
            assert_eq!(
                view["opponent"]["hand"],
                json!({ "count": s.hand(P1).len() }),
                "{n}"
            );
        }
        for id in MENU {
            assert!(
                seen.iter().any(|seen| seen == id),
                "{id} came up over the seeds: {seen:?}"
            );
        }
    }

    #[test]
    fn r3566002_the_opponents_turn_adds_nothing() {
        let mut s = standing("r3566002-theirs", false);
        s.start_turn();
        let before = s.hand(P1).len();
        s.end_turn();
        assert_eq!(s.hand(P1).len(), before);
    }

    #[test]
    fn r3566002_radiant_adds_radiant() {
        for n in 0..10 {
            let mut s = standing(&format!("r3566002-radiant-{n}"), true);
            let before = hand_ids(&s);
            s.start_turn();
            let made = added(&s, &before);
            assert_eq!(made.len(), 1, "{n}");
            assert!(made[0].radiant, "{n}: {} is Radiant", made[0].def_id);
        }
    }
}
