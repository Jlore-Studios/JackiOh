//! SPEC §8.13 IA #105 Callback ((2) Unit, Common, 3/4 → 6/8), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Death: Add a copy of the last Spell you played to your hand."
//!   Radiant: "Death: Add two copies of the last Spell you played to your hand."
//!
//! The Death takes the newest entry of the controller's `played_this_game` log (R4589000) whose
//! type is Spell — a Field Spell is none (R4589002) — and adds it on the face it was played with,
//! twice on the Radiant face. With no such entry it does nothing (R129).

use jackioh_engine::effects::add_to_hand;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-105";

/// "a copy", the base face's.
const COPIES: i32 = 1;
/// "two copies", the Radiant face's.
const RADIANT_COPIES: i32 = 2;

fn callback(copies: i32) -> Script {
    Script {
        death: Some(hook(move |ctx| {
            let last = played_this_game(&*ctx.state, ctx.controller)
                .into_iter()
                .rev()
                .find(|record| record.type_ == CardType::Spell);
            let Some(last) = last else {
                return vec![];
            };
            (0..copies)
                .map(|_| {
                    add_to_hand(json_as(json!({
                        "defId": last.def_id.clone(),
                        "radiant": last.radiant,
                    })))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: callback(COPIES),
        radiant: callback(RADIANT_COPIES),
    }
}

// IA #105 Callback — SPEC §8.13 row 105 (R4589002).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill
    const SPELL: &str = "stand-in (0) Spell";
    const FILLER: &str = "stand-in (0) Spell";

    /// p1 casts a Spell and plays Callback, then p2 kills it, with IA #97 in the library to keep
    /// the log. p2 does the killing: a Hit Job of p1's own would be the last Spell (R4589002).
    fn called_back(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [SPELL, { "def": ID, "radiant": radiant }],
                "library": ["infiniteassault-097", FILLER, FILLER],
                "mana": 10,
            },
            "p2": { "hand": [HIT_JOB], "library": crate::filler(3), "mana": 10 },
        }));
        s.play(SPELL, json!({}));
        s.play(ID, json!({ "zone": 1 }));
        s.end_turn();
        let callback = s.unit_at(P1, 1);
        s.play(HIT_JOB, pick::targets(pick::instance(&callback)));
        s
    }

    fn spells_back(s: &Scenario) -> usize {
        s.hand(P1).iter().filter(|card| card.def_id == SPELL).count()
    }

    mod base {
        use super::*;

        #[test]
        fn r4589002_adds_the_last_spell() {
            let s = called_back(false);
            assert_eq!(spells_back(&s), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4589002_adds_two_copies_of_the_last_spell() {
            let s = called_back(true);
            assert_eq!(spells_back(&s), 2);
        }
    }
}
