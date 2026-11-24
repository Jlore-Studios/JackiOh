//! SPEC §8.13 IA #2 Scrap Prototype ((0) Unit, Common, 1/2 → 2/4), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Death: Shuffle a copy of this with +{buff}/+{buff} into your deck." (buff 1)
//!   Radiant: "Death: Shuffle two copies of this with +{buff}/+{buff} into your deck." (buff 2)
//!
//! Core #90.1 Super-Virus's copies (R57: fresh copies of the face that died, with its riders), each
//! with `shuffleInto`'s `buff` (R4577002): a fresh copy carries no buff of the card that died, so a
//! copy of a copy is no bigger than the first. A full library refuses a copy (R80, R316).

use jackioh_engine::effects::shuffle_into;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-002";

/// "a copy", Radiant "two copies".
const COPIES: i32 = 1;
const RADIANT_COPIES: i32 = 2;

fn scrap(copies: i32) -> Script {
    Script {
        death: Some(hook(move |ctx| {
            let buff = param(&*ctx, "buff");
            let mut args = json!({
                "defId": ID,
                "count": copies,
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
        base: scrap(COPIES),
        radiant: scrap(RADIANT_COPIES),
    }
}

// IA #2 Scrap Prototype — SPEC §8.13 row 2 (R4577002).
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
        fn death_shuffles_one_copy_with_1_1_into_the_deck() {
            let mut s = board(false);
            let scrap = s.unit_at(P1, 1);
            kill(&mut s, &scrap);
            let copies = copies(&s);
            assert_eq!(copies.len(), 1);
            for copy in nonempty(copies, "the copy") {
                assert_eq!((copy.buffs.attack, copy.buffs.health), (1, 1));
                assert!(!copy.radiant);
                assert_ne!(copy.id, scrap.id, "a fresh copy (R57)");
            }
        }

        #[test]
        fn r4577002_the_copy_plays_as_a_2_3_and_its_own_copy_is_no_bigger() {
            let mut s = board(false);
            let scrap = s.unit_at(P1, 1);
            kill(&mut s, &scrap);
            s.end_turn().end_turn();
            assert_eq!(s.hand(P1).iter().filter(|card| card.def_id == ID).count(), 1, "the copy is drawn");
            s.state_mut().players[P1].mana.current = 10;
            s.play(ID, json!({ "zone": 1 }));
            let second = s.unit_at(P1, 1);
            s.expect_stats(&second, json!({ "attack": 2, "health": 3 }));
            kill(&mut s, &second);
            let copies = copies(&s);
            assert_eq!(copies.len(), 1);
            for copy in nonempty(copies, "the copy's copy") {
                assert_eq!((copy.buffs.attack, copy.buffs.health), (1, 1), "nothing compounds");
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn death_shuffles_two_radiant_copies_with_2_2_into_the_deck() {
            let mut s = board(true);
            let scrap = s.unit_at(P1, 1);
            s.expect_stats(&scrap, json!({ "attack": 2, "health": 4 }));
            kill(&mut s, &scrap);
            let copies = copies(&s);
            assert_eq!(copies.len(), 2);
            for copy in nonempty(copies, "the copies") {
                assert_eq!((copy.buffs.attack, copy.buffs.health), (2, 2));
                assert!(copy.radiant, "the face that died");
            }
        }
    }
}
