//! SPEC §8.13 IA #114 Ouroboros Engine ((3) Unit, Epic, 5/5 → 10/10), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Death: Shuffle your graveyard into your deck."
//!   Radiant: "Death: Shuffle your graveyard into your deck and draw two cards."
//!
//! C #30 Recycle's shape as a Death: every card of the controller's graveyard as the Death runs —
//! this card included (R4589002: it is already there) — is shuffled into its deck at a random
//! position, up to the library cap (R80, R316). The Radiant face then draws two cards (§2.4's
//! pipeline).

use jackioh_engine::effects::{draw, shuffle_card_into};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-114";

/// "draw two cards", the Radiant face's.
const RADIANT_DRAW: i32 = 2;

fn ouroboros(radiant: bool) -> Script {
    Script {
        death: Some(hook(move |ctx| {
            let view: &EffectContext = ctx;
            let buried: Vec<String> =
                zone_cards(&*view.state, view.controller, OffFieldZone::Graveyard)
                    .iter()
                    .map(|card| card.id.clone())
                    .collect();
            let mut effects = Vec::new();
            {
                let buried = buried.clone();
                effects.push(for_each_card(ForEachCardArgs {
                    cards: Arc::new(move |_ctx| buried.clone()),
                    each: Arc::new(|instance_id| {
                        shuffle_card_into(json_as(json!({ "instanceId": instance_id })))
                    }),
                }));
            }
            if radiant {
                effects.push(draw(json_as(json!({ "count": RADIANT_DRAW }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: ouroboros(false),
        radiant: ouroboros(true),
    }
}

// IA #114 Ouroboros Engine — SPEC §8.13 row 114 (R4589002).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill
    const FILLER: &str = "stand-in (0) Spell";

    /// p1's graveyard holds two Spells; its Engine dies to p2's Hit Job.
    fn ground(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "field": [{ "def": ID, "radiant": radiant }],
                "graveyard": [FILLER, FILLER],
                "library": crate::filler(4),
            },
            "p2": { "hand": [HIT_JOB], "library": crate::filler(3), "mana": 10 },
        }));
        s.end_turn();
        let engine = s.unit_at(P1, 1);
        s.play(HIT_JOB, pick::targets(pick::instance(&engine)));
        s
    }

    fn library_of(s: &Scenario) -> Vec<CardInstance> {
        s.pile(P1, "library")
    }

    mod base {
        use super::*;

        #[test]
        fn r4589002_shuffles_itself_too() {
            let s = ground(false);
            let library = library_of(&s);
            assert!(
                library.iter().any(|card| card.def_id == ID),
                "the Engine shuffles its own grave with the rest"
            );
            assert_eq!(
                s.pile(P1, "graveyard").len(),
                0,
                "the graveyard is empty"
            );
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4589002_radiant_shuffles_and_draws_two() {
            let s = ground(true);
            assert!(library_of(&s).iter().any(|card| card.def_id == ID));
            assert_eq!(s.hand(P1).len(), 2, "nothing spent, two drawn");
        }
    }
}
