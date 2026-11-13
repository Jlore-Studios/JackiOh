//! SPEC §8.12 S+ #61 Jayaad, a card of the Stinkyscape+ set (R1420, R3535000). (2) Unit, Acclaimed, Epic.
//!   Base:    2/6. "Cry: Summon Shawarma Law."
//!   Radiant: 4/12. "Cry: Summon Radiant Shawarma Law."
//!
//! Cry (R1): a summon of S+ #61.1 Shawarma Law, a Field Spell, into its controller's leftmost open
//! backrow zone (R64); with no open zone nothing, as S #147.1 IN33DH3 Prime's is. The Radiant face
//! summons a Radiant Law. Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-061";

fn jayaad(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![summon(json_as(json!({
                "defId": super::c061_1_shawarma_law::ID,
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: jayaad(false),
        radiant: jayaad(true),
    }
}

// S+ #61 Jayaad — SPEC §8.12 row 61.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const LAW: &str = super::super::c061_1_shawarma_law::ID;
    const FILLER: &str = "core-005";

    fn played(radiant: bool, backrow: Value) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "backrow": backrow },
            "p2": { "hand": [FILLER] },
        }));
        s.play(ID, json!({}));
        s
    }

    #[test]
    fn the_cry_summons_shawarma_law() {
        let s = played(false, json!([]));
        let law = s
            .backrow(P1, 1)
            .expect("the Law stands in the first backrow zone");
        assert_eq!(law.def_id, LAW);
        assert!(!law.radiant);
    }

    #[test]
    fn radiant_summons_a_radiant_law() {
        let s = played(true, json!([]));
        let law = s
            .backrow(P1, 1)
            .expect("the Law stands in the first backrow zone");
        assert_eq!(law.def_id, LAW);
        assert!(law.radiant);
    }

    #[test]
    fn r64_a_full_backrow_summons_none() {
        let s = played(
            false,
            json!(["core-041", "core-041", "core-041", "core-041", "core-041"]),
        );
        let laws = (1..=5)
            .filter_map(|lane| s.backrow(P1, lane))
            .filter(|card| card.def_id == LAW)
            .count();
        assert_eq!(laws, 0);
    }
}
