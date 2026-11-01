//! SPEC §7 Stinkyscape T-axnit Axnit, a Resource token of the Stinkyscape set (§7's token table).
//!   Base:    "Deal {damage} damage to an enemy, with Lifesteal." (3)
//!   Radiant: "Echo 1\nDeal {damage} damage to an enemy, with Lifesteal." (3)
//!
//! A Resource (R1840000), tagged Resource and Token (§5's tags, issue #2521), played from its owner's
//! hand at (0) for its Credits (R2250000, R2250001). "An enemy" is a declared target, the enemy hero
//! or an enemy Unit, picked with the play (R81). Lifesteal is the effect's (R85, Core #93.1
//! Combo-Fodder's shape): its caster's hero heals by the damage dealt. The Radiant face prints Echo 1
//! (`StaticFlags.echo`, §6.3): it resolves a second time, its target asked afresh. The number is the
//! declared `damage` (R386).

use jackioh_engine::effects::damage;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-axnit";

/// "Echo 1", the Radiant face's.
const RADIANT_ECHO: i32 = 1;

fn face(radiant: bool) -> Script {
    Script {
        static_flags: radiant.then(|| StaticFlags {
            echo: Some(RADIANT_ECHO),
            ..StaticFlags::default()
        }),
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "enemy", "of": ["unit", "hero"] }),
        )],
        cry: Some(hook(|ctx| {
            vec![damage(json_as(json!({
                "to": { "of": "chosen" },
                "amount": param(&*ctx, "damage"),
                "lifesteal": true,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S T-axnit — SPEC §7, BUILD M12 row S T-axnit: "played from the hand at a chosen enemy, 3 damage and
// its caster's hero healed 3; damage reads through `param()`; radiant it resolves twice, a target asked
// each time". Tagged Resource and Token (issue #2521).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, play_resource, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const BODY: &str = "core-019"; // 9/9
    const FILLER: &str = "core-005";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "health": 20 },
            "p2": { "field": [BODY] },
        }))
    }

    #[test]
    fn tagged_resource_and_token() {
        assert_eq!(card_def(ID).tags, vec![Tag::Resource, Tag::Token]);
    }

    #[test]
    fn damage_and_lifesteal() {
        let mut s = board(false);
        play_resource(&mut s, P1, ID, json!([{ "pick": "hero", "player": "p2" }]));
        s.expect_health(P2, 27).expect_health(P1, 23);

        let mut s = board(false);
        let body = s.unit_at(P2, 1);
        play_resource(&mut s, P1, ID, pick::instance(&body));
        s.expect_stats(&body, json!({ "health": 6 }));
        s.expect_health(P1, 23);
    }

    #[test]
    fn an_own_hero_is_no_target() {
        let mut s = board(false);
        s.expect_refused(|s| play_resource(s, P1, ID, json!([{ "pick": "hero", "player": "p1" }])));
        s.expect_health(P1, 20);
    }

    #[test]
    fn damage_reads_through_param() {
        let mut s = board(false);
        let id = crate::resources_in_hand(&s, P1)[0].id.clone();
        set_param(s.card_mut(id.as_str()), "damage", 5);
        play_resource(&mut s, P1, ID, json!([{ "pick": "hero", "player": "p2" }]));
        s.expect_health(P2, 25).expect_health(P1, 25);
    }

    #[test]
    fn radiant_asks_again() {
        let mut s = board(true);
        let body = s.unit_at(P2, 1);
        play_resource(&mut s, P1, ID, json!([{ "pick": "hero", "player": "p2" }]));
        s.expect_health(P2, 27);
        assert!(s.state().pending.is_some(), "the Echo repeat asks its target");
        s.answer(pick::instance(&body));
        s.expect_stats(&body, json!({ "health": 6 }));
        s.expect_health(P1, 26);
    }
}
