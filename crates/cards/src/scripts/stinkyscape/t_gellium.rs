//! SPEC §7 Stinkyscape T-gellium Gellium, a Resource token of the held Stinkyscape set (R1420).
//!   Base:    "Heal your hero {heal}." (4)
//!   Radiant: the same (8).
//!
//! A Resource (R1840000), played from its owner's tray at (0) (R1840003) or sold (R1853003). "Heal your
//! hero" is §6.3 Heal on a hero, which has no cap (§3), so it heals past 30. The number is the
//! declared `heal` (R386).

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

// S T-gellium — SPEC §7, BUILD M12 row S T-gellium: "played from the tray, heals its caster's hero 4,
// past 30 too; heal reads through `param()`; radiant 8".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{play_from_tray, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";

    fn tray(radiant: bool, health: i32) -> Scenario {
        scenario(
            json!({ "p1": { "tray": [{ "def": ID, "radiant": radiant }], "hand": [FILLER], "health": health } }),
        )
    }

    #[test]
    fn heals_past_30() {
        let mut s = tray(false, 20);
        play_from_tray(&mut s, P1, ID, Value::Null);
        s.expect_health(P1, 24);
        let mut s = tray(false, 30);
        play_from_tray(&mut s, P1, ID, Value::Null);
        s.expect_health(P1, 34);
    }

    #[test]
    fn heal_reads_through_param() {
        let mut s = tray(false, 20);
        let id = s.pile(P1, "tray")[0].id.clone();
        set_param(s.card_mut(id.as_str()), "heal", 6);
        play_from_tray(&mut s, P1, ID, Value::Null);
        s.expect_health(P1, 26);
    }

    #[test]
    fn radiant_heals_8() {
        let mut s = tray(true, 20);
        play_from_tray(&mut s, P1, ID, Value::Null);
        s.expect_health(P1, 28);
    }
}
