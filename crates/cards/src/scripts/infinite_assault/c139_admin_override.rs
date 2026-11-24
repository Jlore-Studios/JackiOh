//! SPEC §8.13 IA #139 Admin Override ((4) Spell, Epic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Steal an enemy Unit."
//!   Radiant: "Steal an enemy Unit. It has Rush."
//!
//! #49 Snom Bunny's shape without a cap (R15): a declared enemy Unit is stolen as it is picked; on
//! the Radiant face it is granted Rush as well, so it may attack a Unit this turn (§4.2, R83).

use jackioh_engine::effects::{grant_keyword, steal};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-139";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Harm),
        ..TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"] }))
    }]
}

fn override_(rush: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |_ctx| {
            let mut effects = vec![steal(json_as(json!({ "target": { "of": "chosen" }})))];
            if rush {
                effects.push(grant_keyword(json_as(json!({
                    "target": { "of": "chosen" },
                    "keyword": { "kind": "Rush" },
                }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: override_(false),
        radiant: override_(true),
    }
}

// IA #139 Admin Override — SPEC §8.13 row 139.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "admin-override",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 },
            "p2": { "hand": ["core-005"], "field": [VANILLA, VANILLA] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn steals_the_chosen_enemy_unit() {
            let mut s = board(false);
            let unit = s.unit_at(P2, 1);
            s.play(ID, pick::targets(pick::instance(&unit)));
            let stolen = s.card(&unit);
            assert_eq!(stolen.controller, P1, "stolen");
            assert!(
                !has_keyword(&s.stats(stolen).keywords, KeywordKind::Rush),
                "no Rush on the base face"
            );
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn steals_the_chosen_enemy_unit_and_gives_it_rush() {
            let mut s = board(true);
            let unit = s.unit_at(P2, 1);
            s.play(ID, pick::targets(pick::instance(&unit)));
            let stolen = s.card(&unit).id.clone();
            assert_eq!(s.card(stolen.as_str()).controller, P1, "stolen");
            assert!(
                has_keyword(&s.stats(stolen.as_str()).keywords, KeywordKind::Rush),
                "Rush to attack this turn"
            );
            let other = s.unit_at(P2, 2);
            s.attack(&stolen, &other);
            assert!(s.card(other).damage > 0, "it attacked a Unit this turn");
        }
    }
}
