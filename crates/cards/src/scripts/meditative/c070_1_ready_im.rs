//! M #70.1 Ready… I'm (SPEC §8.8 row 70.1): (1) Unit, Token (printed Common), 2/1 → 4/2.
//!   Base:    "Poisonous"
//!   Radiant: "Poisonous, Divine Shield"
//! Engine: keywords only; a unit token, gone when it leaves the field (R11).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-070-1";

pub fn script() -> CardScripts {
    // Keywords only: Poisonous (Radiant also Divine Shield) are catalog data.
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// M #70.1 Ready… I'm — SPEC §8.8 row 70.1, BUILD M10 row M 70.1.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const READY: &str = "meditative-070-1";
    const MENACE: &str = "core-019";
    const FILLER: &str = "core-005";

    #[test]
    fn keywords_only_poisonous_destroys_the_unit_it_damages() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "field": [{ "def": READY, "lane": 1 }], "hand": [FILLER] },
            "p2": { "field": [{ "def": MENACE, "lane": 1 }], "hand": [FILLER] },
        }));
        s.end_turn();
        let attacker = s.unit(PlayerId::P2, 1).expect("the menace").id.clone();
        let victim = s.unit(PlayerId::P1, 1).expect("ready").id.clone();
        s.attack(&attacker, &victim);
        s.expect_in_zone(&victim, "gone");
    }

    #[test]
    fn r11_gone_when_it_leaves_the_field() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "field": [{ "def": READY, "lane": 1 }], "hand": ["core-016", FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        let id = s.unit(PlayerId::P1, 1).expect("ready").id.clone();
        s.play("core-016", json!({ "targets": [{ "pick": "instance", "instanceId": id }] }));
        s.expect_in_zone(&id, "gone");
    }

    #[test]
    fn radiant_4_2_divine_shield() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "field": [{ "def": READY, "lane": 1, "radiant": true }] },
            "p2": { "hand": [FILLER] },
        }));
        s.expect_stats(READY, json!({ "attack": 4, "health": 2 }));
        let kinds: Vec<String> = s
            .stats(s.unit(PlayerId::P1, 1).expect("ready"))
            .keywords
            .iter()
            .map(|keyword| keyword.kind().as_str().to_string())
            .collect();
        assert!(kinds.contains(&"Poisonous".to_string()));
        assert!(kinds.contains(&"Divine Shield".to_string()));
    }
}
