//! SPEC §7 Stinkyscape T-gellium Gellium, a Resource token of the Stinkyscape set (§7's token table).
//!   Base:    "Heal your hero {heal}." (4)
//!   Radiant: the same (8).
//!
//! A Resource (R1840000), tagged Resource and Token (§5's tags, issue #2521), played from its owner's
//! hand at (0) for its Credits (R2250000, R2250001). "Heal your hero" is §6.3 Heal on a hero, which
//! has no cap (§3), so it heals past 30. The number is the declared `heal` (R386).

use jackioh_engine::effects::heal;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-gellium";

fn face() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            vec![heal(json_as(
                json!({ "target": { "of": "selfHero" }, "amount": param(&*ctx, "heal") }),
            ))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(),
        radiant: face(),
    }
}

// S T-gellium — SPEC §7, BUILD M12 row S T-gellium. Tagged Resource and Token (issue #2521).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, play_resource, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";

    fn in_hand(radiant: bool, health: i32) -> Scenario {
        scenario(
            json!({ "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "health": health } }),
        )
    }

    #[test]
    fn tagged_resource_and_token() {
        assert_eq!(card_def(ID).tags, vec![Tag::Resource, Tag::Token]);
    }

    #[test]
    fn heals_past_30() {
        let mut s = in_hand(false, 20);
        play_resource(&mut s, P1, ID, Value::Null);
        s.expect_health(P1, 24);
        let mut s = in_hand(false, 30);
        play_resource(&mut s, P1, ID, Value::Null);
        s.expect_health(P1, 34);
    }

    #[test]
    fn heal_reads_through_param() {
        let mut s = in_hand(false, 20);
        let id = crate::resources_in_hand(&s, P1)[0].id.clone();
        set_param(s.card_mut(id.as_str()), "heal", 6);
        play_resource(&mut s, P1, ID, Value::Null);
        s.expect_health(P1, 26);
    }

    #[test]
    fn radiant_heals_8() {
        let mut s = in_hand(true, 20);
        play_resource(&mut s, P1, ID, Value::Null);
        s.expect_health(P1, 28);
    }
}
