//! SPEC §8.13 IA #107 Recursive Call ((2) Spell, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Deal {damage} damage to a random enemy, once more for each Recursive Call you've played this game. Shuffle a copy of this into your deck." (damage 1)
//!   Radiant: "Deal {damage} damage to a random enemy, once more for each Recursive Call you've played this game. Shuffle a copy of this into your deck." (damage 2)
//!
//! IA #11 Quickgen Turret's `damage_split`: one hit of the declared `damage` for this resolution
//! plus each earlier Recursive Call (R4589002: the count is of plays with this id, minus this one,
//! read off the `played_this_game` log, R4589000), each at a random enemy still standing, the hero
//! included, drawn again for every hit. Then a fresh copy of the face cast is shuffled into its
//! controller's deck (R4577002).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-107";

fn recursive_call() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let view: &EffectContext = ctx;
            let earlier = played_this_game(&*view.state, view.controller)
                .iter()
                .filter(|record| record.def_id == ID)
                .count()
                .saturating_sub(1);
            let per_hit = param(view, "damage");
            let hits = 1 + i32::try_from(earlier).unwrap_or(i32::MAX);
            let mut effects = vec![damage_split(json_as(json!({
                "amount": per_hit.saturating_mul(hits),
                "perHit": per_hit,
                "among": "enemies",
            })))];
            let mut copy = json!({
                "defId": ID,
                "count": 1,
                "player": "self",
                "radiant": ctx.radiant,
            });
            if let Some(this) = ctx.self_.as_ref() {
                copy["copyOf"] = json!(this.id);
            }
            effects.push(shuffle_into(json_as(copy)));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: recursive_call(),
        radiant: recursive_call(),
    }
}

// IA #107 Recursive Call — SPEC §8.13 row 107 (R4589002).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "stand-in (0) Spell";

    /// p1 casts Recursive Call `casts` times, with IA #97 in the library to keep the log.
    fn cast(radiant: bool, casts: usize) -> Scenario {
        let hand: Vec<Value> =
            (0..casts).map(|_| json!({ "def": ID, "radiant": radiant })).collect();
        let mut s = scenario(json!({
            "p1": {
                "hand": hand,
                "library": ["infiniteassault-097", FILLER, FILLER, FILLER],
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(3) },
        }));
        for _ in 0..casts {
            s.play(ID, json!({}));
        }
        s
    }

    fn shuffled(s: &Scenario) -> usize {
        s.pile(P1, "library").into_iter().filter(|card| card.def_id == ID).count()
    }

    mod base {
        use super::*;

        #[test]
        fn r4589002_hits_once_more_per_earlier_copy() {
            let mut s = cast(false, 2);
            // One hit of 1, then two hits of 1, all at the enemy hero.
            s.expect_health(P2, HERO_HEALTH - 3);
            assert_eq!(shuffled(&s), 2, "each cast shuffles a copy back");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4589002_radiant_hits_twice_more_per_earlier_copy() {
            let mut s = cast(true, 2);
            // One hit of 2, then two hits of 2.
            s.expect_health(P2, HERO_HEALTH - 6);
            assert_eq!(shuffled(&s), 2, "each cast shuffles a copy back");
        }
    }
}
