//! Meditative #39.4 Red Jade (SPEC §8.8 row 39.4). (0) Spell, CN, Token (printed Mythic).
//!
//!   Base:    "Add {jade} to your Jade Counter." — jade 5
//!   Radiant: "Add {jade} to your Jade Counter." — jade 10
//!
//! Engine: ME-JADE `add_jade`. From 0, the Radiant face's 10 crosses both thresholds in one add:
//! the summon at 5 happens first, then the ascension at 10 (MD-C3).
//! Tunes: jade 5 ↑.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-039-4";

fn red_jade() -> Script {
    Script {
        cry: Some(hook(|ctx| vec![add_jade(json_as(json!({ "amount": param(&*ctx, "jade") })))])),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: the Radiant face's 10 is its declared `jade`, which `param` reads off the
    // running face.
    let base = red_jade();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Meditative #39.4 Red Jade — SPEC §8.8 row 39.4, BUILD M10 row M 39.4: "Raises its caster's Jade
// Counter by 5: from 0 a Jade Beauty is summoned (MD-C3); jade reads through `param()`; radiant 10:
// from 0 the Beauty is summoned at 5, then made Radiant at 10, in that order".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BEAUTY: &str = "meditative-039-5";
    const FILLER: &str = "core-008";

    fn beauties(s: &Scenario) -> Vec<CardInstance> {
        (0..4).filter_map(|lane| s.unit(P1, lane)).filter(|unit| unit.def_id == BEAUTY).collect()
    }

    #[test]
    fn r962_from_0_summons_a_jade_beauty() {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "hand": [ID, FILLER] }, "p2": { "hand": [FILLER] } }));

        s.play(ID, json!({}));

        assert_eq!(s.view(P1).you.jade, Some(5));
        let summoned = beauties(&s);
        assert_eq!(summoned.len(), 1);
        assert!(!summoned[0].radiant);
    }

    #[test]
    fn r962_radiant_from_0_summons_then_makes_it_radiant_in_that_order() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": true }, FILLER] },
            "p2": { "hand": [FILLER] },
        }));

        s.play(ID, json!({}));

        assert_eq!(s.view(P1).you.jade, Some(10));
        // One Beauty: summoned at 5, then made Radiant at 10, in that order.
        let summoned = beauties(&s);
        assert_eq!(summoned.len(), 1);
        assert!(summoned[0].radiant);
        let order: Vec<String> = s
            .events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { def_id, .. } if def_id == BEAUTY => Some("summoned".to_string()),
                GameEvent::RadiantSet { .. } => Some("radiantSet".to_string()),
                _ => None,
            })
            .collect();
        assert_eq!(order, vec!["summoned", "radiantSet"]);
    }
}
