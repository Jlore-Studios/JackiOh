//! SPEC §8.14 FvH #176 Plagiarized: Privilege Escalation, a frozen copy of IA #131 Privilege Escalation
//! (R2195003, FV4), a card of the held Felinors vs Humans set, built by #5235.
//!
//! The code is a copy of the IA card's as it stood when the set was founded, not a re-export, so a later IA
//! patch does not move it (R2195003). Its `ID` is its own, so every read of it names this card; it is named
//! `Plagiarized: Privilege Escalation`, tagged Plagiarized only, and a token it makes stays the IA token. Its
//! change from IA #131: none. The IA file's notes, as they stood:
//!
//! SPEC §8.13 IA #131 Privilege Escalation ((3) Spell, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Steal an enemy Unit that costs (2) or less."
//!   Radiant: "Steal an enemy Unit that costs (3) or less."
//!
//! A declared enemy Unit under the face's cost cap (#49 Snom Bunny's shape, §6.3 Steal, R15, R65),
//! stolen as it is picked. The caps are fixed prices (`tests/cross/params.rs`, R1748003); the text is
//! reworded to "(N) Cost or less" (R432, IA11 rule 7).

use jackioh_engine::effects::steal;
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-176";

/// "Costs (2) or less" (base) or "(3) or less" (Radiant): R65's cost where the Unit stands.
const CAP: i32 = 2;
const RADIANT_CAP: i32 = 3;

fn targets(cap: i32) -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Harm),
        ..TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"], "costRange": { "max": cap } }))
    }]
}

fn escalation(cap: i32) -> Script {
    Script {
        targets: targets(cap),
        cry: Some(hook(|_ctx| vec![steal(json_as(json!({ "target": { "of": "chosen" } })))])),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: escalation(CAP),
        radiant: escalation(RADIANT_CAP),
    }
}

// FvH #176 Plagiarized: Privilege Escalation — SPEC §8.14 row 176; IA #131's tests (SPEC §8.13 row 131,
// R432), on this card.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const CHEAP: &str = "stand-in (2) 3/3 R6/6";
    const PRICEY: &str = "stand-in (3) 6/6 R12/12";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "privilege-escalation",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 },
            "p2": { "field": [CHEAP, PRICEY] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r432_a_3_cost_unit_is_no_legal_pick_a_2_cost_unit_is_stolen() {
            let mut s = board(false);
            let cheap = s.unit_at(P2, 1).id.clone();
            let pricey = s.unit_at(P2, 2).id.clone();
            let offered = pick::instance_ids(&s.play_targets(ID));
            assert_eq!(offered, vec![cheap.clone()]);
            s.play(ID, pick::targets(pick::id(&offered[0])));
            assert_eq!(s.card(cheap.as_str()).controller, P1, "stolen");
            assert_eq!(s.card(pricey.as_str()).controller, P2, "left alone");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r432_a_3_cost_unit_is_stolen() {
            let mut s = board(true);
            let pricey = s.unit_at(P2, 2).id.clone();
            let offered = pick::instance_ids(&s.play_targets(ID));
            assert!(offered.contains(&pricey), "offered on Radiant: {offered:?}");
            s.play(ID, pick::targets(pick::id(&pricey)));
            assert_eq!(s.card(pricey.as_str()).controller, P1, "stolen");
        }
    }
}
