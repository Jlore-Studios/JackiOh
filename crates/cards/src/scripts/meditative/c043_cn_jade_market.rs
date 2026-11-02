//! Meditative #43 CN Jade Market (SPEC §8.8 row 43). (X) Spell, CN, Rare.
//!
//!   Base:    "Add X Auspicious Rocks to your hand."
//!   Radiant: "Add X Radiant Auspicious Rocks to your hand."
//!
//! Engine: X is chosen with the play and is between 1 and the caster's current mana (R348); cost
//! modifiers never apply (R65). Then `add_to_hand` the Rock (M #39.1) X times, Radiant on the
//! Radiant face; the hand cap burns extras (§2.4). A Nerf or Buff moves X by 1 at resolution
//! (§6.3's X row).
//! Tunes: none (X is chosen).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-043";

/// The Rock this adds (M #39.1).
pub const ROCK: &str = "meditative-039-1";

fn market(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            // R348: X is at least 1 (the play validator refused 0 and above-mana); `ctx.x` is the
            // X paid as Nerf and Buff tuned it (§6.3's X row, `tuning::x_of`), and `max(0)` keeps
            // a tuned-down X from underflowing.
            let x = ctx.x.max(0);
            (0..x)
                .map(|_| add_to_hand(json_as(json!({ "defId": ROCK, "radiant": radiant }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: market(false),
        radiant: market(true),
    }
}

// Meditative #43 CN Jade Market — SPEC §8.8 row 43, BUILD M10 row M 43.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::def_ids;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-008";

    #[test]
    fn x_3_adds_three_base_rocks() {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "hand": [ID, FILLER] }, "p2": { "hand": [FILLER] } }));

        s.play(ID, json!({ "x": 3 }));

        assert_eq!(def_ids(&s.hand(P1)), vec![FILLER, ROCK, ROCK, ROCK]);
        assert!(s.hand(P1).iter().skip(1).all(|card| !card.radiant));
    }

    #[test]
    fn r348_x_0_and_x_above_mana_are_refused() {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "hand": [ID, FILLER] }, "p2": { "hand": [FILLER] } }));
        let mana = s.view(P1).you.mana.current;

        s.expect_refused(|s| s.play(ID, json!({ "x": 0 })));
        s.expect_refused(|s| s.play(ID, json!({ "x": mana + 1 })));
        assert_eq!(def_ids(&s.hand(P1)), vec![ID, FILLER]);
    }

    #[test]
    fn the_hand_cap_burns_the_rest() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [ID, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER] },
            "p2": { "hand": [FILLER] },
        }));

        // Nine after the play: the first Rock makes ten, the cap (§2.4), so the other two burn.
        s.play(ID, json!({ "x": 3 }));

        assert_eq!(s.hand(P1).len(), 10);
        assert_eq!(def_ids(&s.hand(P1)).iter().filter(|def| *def == ROCK).count(), 1);
        let burned = def_ids(&s.pile(P1, "graveyard"));
        assert_eq!(burned.iter().filter(|def| *def == ROCK).count(), 2);
    }

    #[test]
    fn radiant_rocks_are_radiant() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": true }, FILLER] },
            "p2": { "hand": [FILLER] },
        }));

        s.play(ID, json!({ "x": 2 }));

        let hand = s.hand(P1);
        let rocks: Vec<&CardInstance> =
            hand.iter().filter(|card| card.def_id == ROCK).collect();
        assert_eq!(rocks.len(), 2);
        assert!(rocks.iter().all(|card| card.radiant));
    }
}
