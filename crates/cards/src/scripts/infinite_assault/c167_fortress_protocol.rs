//! SPEC §8.13 IA #167 Fortress Protocol ((4) Spell, Legendary), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Switch your Units to Defense Position. Give them +0/+{health} and Divine Shield." (health 4)
//!   Radiant: "Switch your Units to Defense Position. Give them +0/+{health} and Divine Shield." (health 8)
//!
//! Engine: each of the controller's Units in turn (as Felinor Feeler's Cry reads them once, R113)
//! switches to Defense Position — spending no exertion (R20) — then one `buffAllUnits` gives them
//! +0/+{health} (R4453000: a Unit Immune to Spells is passed by), then each gains Divine Shield.
//! One script: the Radiant face's 8 is its declared param.

use jackioh_engine::effects::{buff_all_units, for_each_card, grant_keyword, switch_position_of};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-167";

fn fortress() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let health = param(&*ctx, "health");
            let own: Vec<String> = active_units_of(&*ctx.state, ctx.controller)
                .iter()
                .map(|card| card.id.clone())
                .collect();
            let switched = own.clone();
            let shielded = own.clone();
            vec![
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(move |_: &mut EffectContext<'_>| switched.clone()),
                    each: Arc::new(|instance_id: &str| {
                        switch_position_of(json_as(json!({
                            "to": "DEF",
                            "target": { "of": "instance", "instanceId": instance_id },
                        })))
                    }),
                }),
                buff_all_units(json_as(json!({ "side": "self", "health": health }))),
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(move |_: &mut EffectContext<'_>| shielded.clone()),
                    each: Arc::new(|instance_id: &str| {
                        grant_keyword(json_as(json!({
                            "target": { "of": "instance", "instanceId": instance_id },
                            "keyword": { "kind": "Divine Shield" },
                        })))
                    }),
                }),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // One script: the Radiant face's 8 is its declared `health`, read off the running face.
    let base = fortress();
    CardScripts {
        base: base.clone(),
        radiant: base,
    }
}

// IA #167 Fortress Protocol — SPEC §8.13 row 167.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";
    const FOE: &str = "stand-in (1) 3/3";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": [UNIT, UNIT],
                "library": crate::filler(3),
                "mana": 8,
            },
            "p2": { "field": [FOE], "library": crate::filler(3) },
        }))
    }

    fn defended(s: &Scenario, card: &CardInstance) -> bool {
        card.position == Some(Position::Def)
            && has_keyword(&s.stats(card).keywords, KeywordKind::DivineShield)
    }

    mod base {
        use super::*;

        #[test]
        fn your_units_switch_give_0_4_and_divine_shield() {
            let mut s = board(false);
            s.play(ID, json!({}));
            for lane in 1..=2 {
                let unit = s.unit_at(P1, lane);
                s.expect_stats(&unit, json!({ "attack": 2, "health": 6 }));
                assert!(defended(&s, &s.card(unit.id.as_str())));
            }
            // The enemy's Units stand as they stood.
            assert_eq!(s.unit_at(P2, 1).position, None);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn your_units_switch_give_0_8_and_divine_shield() {
            let mut s = board(true);
            s.play(ID, json!({}));
            for lane in 1..=2 {
                let unit = s.unit_at(P1, lane);
                s.expect_stats(&unit, json!({ "attack": 2, "health": 10 }));
                assert!(defended(&s, &s.card(unit.id.as_str())));
            }
        }
    }
}
