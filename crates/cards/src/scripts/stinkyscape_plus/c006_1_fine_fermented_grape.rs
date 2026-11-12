//! SPEC §8.12 S+ #6.1 Fine Fermented Grape (1) Spell, Fruit, Token, a token of the Stinkyscape+ set
//! (R1420; R3558005), made by #6 Wines McCheese.
//!   Base:    "Tipsy 2 / End of turn: Fuse a different Grape into this."
//!   Radiant: "Tipsy 2 / End of turn: Fuse a different Radiant Grape into this."
//!
//! Engine: Tipsy 2 is the catalog's keyword (R3548000). The End of turn is a hand trigger (it is a
//! Spell, so a hand is where it waits): at its controller's turn end, E8's `fuseRandomInto` fuses one
//! random Grape — a card whose name holds "Grape", the `nameContains` filter (R3558005) — into it,
//! never this card nor what is already in it (B4.1), so each End of turn adds a different one until
//! none is left. It keeps its instance and its price (R470); the Radiant face's Grape goes in Radiant
//! (R469).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-006-1";

/// R3558005: "a Grape is a card whose name holds 'Grape'".
const GRAPE: &str = "grape";

fn fermented(radiant: bool) -> Script {
    Script {
        hand_triggers: vec![
            TriggerDef::new(
                "s+6.1-ferment",
                &[GameEventType::TurnEnded],
                move |_ctx, _event| {
                    vec![fuse_random_into(json_as(json!({
                        "into": { "target": { "of": "self" } },
                        "query": { "nameContains": GRAPE },
                        "radiant": radiant,
                    })))]
                },
            )
            .with_when(|ctx, event| {
                matches!(event, GameEvent::TurnEnded { player, .. } if *player == ctx.controller)
            }),
        ],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: fermented(false),
        radiant: fermented(true),
    }
}

// S+ #6.1 Fine Fermented Grape — SPEC §8.12 row 6.1 (R3558005).
#[cfg(test)]
mod tests {
    use jackioh_engine::subsystems::fuse::fused_ingredients;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    /// p1 holds the Grape (`radiant` its face) and ends the turn: the Grape after it.
    fn fermented(seed: &str, radiant: bool) -> (Scenario, CardInstance) {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, "core-005"], "library": filler(4) },
            "p2": { "hand": ["core-005"], "library": filler(4) },
        }));
        let id = s.hand(P1)[0].id.clone();
        s.end_turn();
        let card = s.card(&id).clone();
        (s, card)
    }

    fn grapes_in(s: &Scenario, card: &CardInstance) -> Vec<String> {
        let ingredients = fused_ingredients(s.state(), &card.def_id).expect("a Grape was fused in");
        assert!(ingredients.contains(&ID.to_string()), "{ingredients:?}");
        let others: Vec<String> = ingredients.into_iter().filter(|def| def != ID).collect();
        for other in &others {
            let name = &def_of(Some(s.state()), other).name;
            assert!(name.to_lowercase().contains("grape"), "{other}: {name}");
        }
        others
    }

    #[test]
    fn r3558005_at_its_controllers_end_of_turn_a_different_grape_is_fused_in() {
        for n in 0..6 {
            let (s, card) = fermented(&format!("r3558005-fermented-{n}"), false);
            assert_eq!(grapes_in(&s, &card).len(), 1, "{n}");
            assert!(matches!(card.zone, Zone::Hand { .. }), "it stays in hand");
            assert_eq!(effective_cost(s.state(), &card, Default::default()), 1, "its price stays");
        }
    }

    #[test]
    fn r3558005_the_opponents_end_of_turn_fuses_nothing() {
        let mut s = scenario(json!({
            "seed": "r3558005-fermented-p2",
            "p1": { "hand": ["core-005"], "library": filler(4) },
            "p2": { "hand": [ID, "core-005"], "library": filler(4) },
        }));
        let id = s.hand(PlayerId::P2)[0].id.clone();
        s.end_turn();
        assert_eq!(s.card(&id).def_id, ID, "p1's turn end is not its controller's");
    }

    #[test]
    fn r3558005_radiant_fuses_a_radiant_grape() {
        let (s, card) = fermented("r3558005-fermented-radiant", true);
        let fused = def_of(Some(s.state()), &card.def_id).clone();
        for grape in grapes_in(&s, &card) {
            assert!(fused.base.text.contains(&crate::card_def(&grape).radiant.text), "{grape}");
        }
    }
}
