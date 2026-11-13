//! SPEC §8.12 S+ #85 Body Snatch, a card of the Stinkyscape+ set (R1420, R3535000). (2) Spell, Rare.
//!   Base:    "Transform a Unit into an Ayy in Defense Position."
//!   Radiant: "Transform a Unit into an Ayy."
//!
//! A declared target: any Unit on either side (R81). `transform` rewrites it in place into S+ #76.1 Ayy,
//! the base token, no Cry (R691), and an Immutable Unit is no target (R781001, R23). A Transform keeps the
//! position of the card it replaces, so the base face then switches the new Ayy, found in the zone the
//! old Unit stood in, to Defense Position (`switch_position_of`, no exertion spent, R20); the Radiant face
//! leaves the position as the Unit had it (R3557003's reading: both faces make the base Ayy, the Radiant
//! face's gain is the stance).

use std::sync::Arc;

use jackioh_engine::effects::{switch_position_of, transform};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-085";

/// S+ #76.1 Ayy.
const AYY: &str = "stinkyscapeplus-076-1";

fn snatch(defense: bool) -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))],
        cry: Some(hook(move |ctx| {
            let mut effects = vec![transform(json_as(json!({ "target": { "of": "chosen" }, "defId": AYY })))];
            let old = match ctx.targets.first() {
                Some(Selection::Instance { instance_id }) => find_instance(&*ctx.state, instance_id).cloned(),
                _ => None,
            };
            if let (true, Some(old)) = (defense, old) {
                let slot = slot_of(&*ctx.state, &old);
                effects.push(for_each_card(ForEachCardArgs {
                    cards: Arc::new(move |now: &mut EffectContext<'_>| {
                        slot.and_then(|slot| card_at(&*now.state, slot).cloned())
                            .filter(|card| card.def_id == AYY && card.id != old.id)
                            .map(|card| vec![card.id])
                            .unwrap_or_default()
                    }),
                    each: Arc::new(|instance_id: &str| {
                        switch_position_of(json_as(json!({
                            "target": { "of": "instance", "instanceId": instance_id },
                            "to": "DEF",
                        })))
                    }),
                }));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: snatch(true),
        radiant: snatch(false),
    }
}

// S+ #85 Body Snatch — SPEC §8.12 row 85.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::{AYY, ID};
    use crate::{filler, scenario};

    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";
    const BODY: &str = "core-019"; // Midrange Menace
    const ROCK: &str = "core-066"; // The Rock: Immutable on its Radiant face.

    fn board(radiant: bool, target: Value) -> Scenario {
        scenario(json!({
            "seed": "body-snatch",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": filler(3), "mana": 10 },
            "p2": { "hand": [FILLER], "field": [target], "library": filler(4) },
        }))
    }

    #[test]
    fn r3557003_base_turns_the_unit_into_an_ayy_in_defense_position() {
        crate::register_all();
        let mut s = board(false, json!({ "def": BODY, "lane": 1 }));
        let old = s.unit_at(P2, 1).id;
        s.play(ID, pick::targets(pick::id(&old)));
        let ayy = s.unit(P2, 1).expect("an Ayy stands there");
        assert_eq!(ayy.def_id, AYY);
        assert_ne!(ayy.id, old.clone());
        assert_eq!(ayy.position, Some(Position::Def));
        assert!(!ayy.radiant);
    }

    #[test]
    fn r3557003_radiant_keeps_the_position_the_unit_had() {
        crate::register_all();
        let mut s = board(true, json!({ "def": BODY, "lane": 1 }));
        let old = s.unit_at(P2, 1).id;
        let before = s.card(&old).position;
        s.play(ID, pick::targets(pick::id(&old)));
        let ayy = s.unit(P2, 1).expect("an Ayy stands there");
        assert_eq!(ayy.def_id, AYY);
        assert_eq!(ayy.position, before);
    }

    #[test]
    fn r781001_an_immutable_unit_is_no_target() {
        crate::register_all();
        let mut s = board(false, json!({ "def": ROCK, "lane": 1, "radiant": true }));
        let rock = s.card(ROCK).clone();
        s.expect_refused(|s| s.play(ID, pick::targets(pick::id(&rock.id))));
        assert_eq!(s.unit(P2, 1).map(|unit| unit.id), Some(rock.id));
    }
}
