//! SPEC §8.13 IA #155 Two-Factor Gate ((2) Unit, Rare, 3/6 → 6/12), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Divine Shield. Start of turn: If this has no Divine Shield, it gains Divine Shield."
//!   Radiant: "Taunt, Divine Shield. Start of turn: If this has no Divine Shield, it gains Divine Shield."
//!
//! Engine: at the start of its controller's turn, when its layered keywords (§10.4) hold no Divine
//! Shield, it grants itself Divine Shield. Taunt and the doubled stats are the catalog's data.

use jackioh_engine::effects::grant_keyword;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-155";

fn gate() -> Script {
    Script {
        start_of_turn: Some(hook(|ctx| {
            let Some(me) = ctx.live_self().cloned() else {
                return vec![];
            };
            if unit_has(ctx.state, &me, KeywordKind::DivineShield) {
                return vec![];
            }
            vec![grant_keyword(json_as(json!({
                "target": { "of": "self" },
                "keyword": { "kind": "Divine Shield" },
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // One script: both faces regain the shield the same way; the stats are the catalog's.
    let base = gate();
    CardScripts {
        base: base.clone(),
        radiant: base,
    }
}

// IA #155 Two-Factor Gate — SPEC §8.13 row 155.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HITTER: &str = "stand-in (2) 4/4";
    const FILLER: &str = "stand-in (0) Spell";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "library": crate::filler(2), "mana": 8 },
            "p2": { "hand": [FILLER], "field": [HITTER], "library": crate::filler(2) },
        }))
    }

    fn shielded(s: &Scenario, card: &CardInstance) -> bool {
        has_keyword(&s.stats(card).keywords, KeywordKind::DivineShield)
    }

    mod base {
        use super::*;

        #[test]
        fn it_regains_divine_shield_at_its_turn_start() {
            let mut s = board(false);
            s.play(ID, json!({ "zone": 1 }));
            let gate = s.unit_at(P1, 1);
            assert!(shielded(&s, &gate));
            s.end_turn();
            // The hitter pops the shield; the gate survives at 6 - 4.
            let hitter = s.unit_at(P2, 1);
            s.attack(&hitter, &gate);
            assert!(!shielded(&s, &s.unit_at(P1, 1)));
            s.end_turn();
            assert!(shielded(&s, &s.unit_at(P1, 1)), "regained at its turn start");
        }

        #[test]
        fn an_unbroken_shield_stays_through_its_turn_start() {
            let mut s = board(false);
            s.play(ID, json!({ "zone": 1 }));
            s.end_turn();
            s.end_turn();
            assert!(shielded(&s, &s.unit_at(P1, 1)));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn a_6_12_taunt_that_regains_divine_shield() {
            let mut s = board(true);
            s.play(ID, json!({ "zone": 1 }));
            let gate = s.unit_at(P1, 1);
            s.expect_stats(&gate, json!({ "attack": 6, "health": 12 }));
            assert!(has_keyword(&s.stats(&gate).keywords, KeywordKind::Taunt));
            s.end_turn();
            let hitter = s.unit_at(P2, 1);
            s.attack(&hitter, &gate);
            s.end_turn();
            assert!(shielded(&s, &s.unit_at(P1, 1)), "regained at its turn start");
        }
    }
}
