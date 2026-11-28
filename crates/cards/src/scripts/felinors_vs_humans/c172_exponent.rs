//! SPEC §8.14 FvH #172 Plagiarized: Exponent, a frozen copy of IA #202 Exponent (R2195003, FV4), a card of
//! the held Felinors vs Humans set, built by #5235.
//!
//! The code is a copy of the IA card's as it stood when the set was founded, not a re-export, so a later IA
//! patch does not move it (R2195003). Its `ID` is its own, so every read of it names this card; it is named
//! `Plagiarized: Exponent`, tagged Plagiarized only, and a token it makes stays the IA token. Its change from
//! IA #202: none. The IA file's notes, as they stood:
//!
//! IA #202 Exponent (SPEC §8.13 row 202), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Start of turn: Double this's Attack."
//!   Radiant: "Start of turn: Double this's Attack and Health."
//!
//! Engine: a `start_of_turn` hook (its controller's, R68): a permanent `buff` of its own Attack
//! now — the Radiant face's, its Health now too — read through the layers as the hook resolves
//! (§10.4), capped so no multiplied stat passes `MULTIPLIED_STAT_CAP` (R4079000). Built by C9
//! (#4593).

use jackioh_engine::effects::buff;
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-172";

fn exponent(radiant_face: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |ctx| {
            let Some(me) = ctx.live_self() else {
                return vec![];
            };
            let view = unit_view(ctx.state, me);
            let attack = view.attack.max(0);
            let gain_attack = attack.min(MULTIPLIED_STAT_CAP - attack).max(0);
            let gain_health = if radiant_face {
                let health = view.health.max(0);
                health.min(MULTIPLIED_STAT_CAP - health).max(0)
            } else {
                0
            };
            if gain_attack == 0 && gain_health == 0 {
                return vec![];
            }
            vec![buff(json_as(json!({
                "target": { "of": "self" },
                "attack": gain_attack,
                "health": gain_health,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: exponent(false),
        radiant: exponent(true),
    }
}

// FvH #172 Plagiarized: Exponent — SPEC §8.14 row 172; IA #202's tests (SPEC §8.13 row 202), on this card.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    fn doubled(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "field": [{ "def": ID, "lane": 1, "radiant": radiant }],
                "hand": [FILLER],
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        s.end_turn();
        s.end_turn();
        s
    }

    fn stats(s: &Scenario) -> (i32, i32) {
        let unit = s.unit_at(P1, 1);
        let view = unit_view(s.state(), &unit);
        (view.attack, view.health)
    }

    mod base {
        use super::*;

        #[test]
        fn doubles_attack_only() {
            let s = doubled(false, "exponent-base");
            assert_eq!(stats(&s), (4, 2));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn doubles_attack_and_health() {
            let s = doubled(true, "exponent-radiant");
            assert_eq!(stats(&s), (8, 8));
        }
    }
}
