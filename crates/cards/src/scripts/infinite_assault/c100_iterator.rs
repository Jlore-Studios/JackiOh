//! SPEC §8.13 IA #100 Iterator ((2) Unit, Common, 3/3 → 6/6), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Death: Shuffle a copy of this with +{buff}/+{buff} into your deck." (buff 2)
//!   Radiant: "Death: Shuffle a copy of this with +{buff}/+{buff} into your deck." (buff 4)
//!
//! IA #2 Scrap Prototype's shape with one copy on both faces: the Death shuffles a fresh copy of the
//! face that died into its controller's deck with the declared `buff` (R4577002), so a copy of a copy
//! is 5/5 again, never more. A full library refuses it (R80, R316).

use jackioh_engine::effects::shuffle_into;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-100";

/// "a copy", on both faces.
const COPIES: i32 = 1;

fn iterator() -> Script {
    Script {
        death: Some(hook(move |ctx| {
            let buff = param(&*ctx, "buff");
            let mut args = json!({
                "defId": ID,
                "count": COPIES,
                "player": "self",
                "radiant": ctx.radiant,
                "buff": { "attack": buff, "health": buff },
            });
            if let Some(this) = ctx.self_.as_ref() {
                args["copyOf"] = json!(this.id);
            }
            vec![shuffle_into(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: iterator(),
        radiant: iterator(),
    }
}

// IA #100 Iterator — SPEC §8.13 row 100.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill
    const FILLER: &str = "stand-in (0) Spell";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [HIT_JOB, HIT_JOB],
                "field": [{ "def": ID, "radiant": radiant }],
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }))
    }

    fn kill(s: &mut Scenario, unit: &CardInstance) {
        s.play(HIT_JOB, pick::targets(pick::instance(unit)));
    }

    fn copies(s: &Scenario) -> Vec<CardInstance> {
        s.pile(P1, "library").into_iter().filter(|card| card.def_id == ID).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn death_shuffles_one_copy_with_2_2_into_the_deck() {
            let mut s = board(false);
            let iterator = s.unit_at(P1, 1);
            kill(&mut s, &iterator);
            let copies = copies(&s);
            assert_eq!(copies.len(), 1);
            for copy in nonempty(copies, "the copy") {
                assert_eq!((copy.buffs.attack, copy.buffs.health), (2, 2));
                assert!(!copy.radiant);
                assert_ne!(copy.id, iterator.id, "a fresh copy (R57)");
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn death_shuffles_one_radiant_copy_with_4_4_into_the_deck() {
            let mut s = board(true);
            let iterator = s.unit_at(P1, 1);
            kill(&mut s, &iterator);
            let copies = copies(&s);
            assert_eq!(copies.len(), 1);
            for copy in nonempty(copies, "the copy") {
                assert_eq!((copy.buffs.attack, copy.buffs.health), (4, 4));
                assert!(copy.radiant);
            }
        }
    }
}
