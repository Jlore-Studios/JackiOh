//! IA #36 Deepfake (SPEC §8.13 row 36), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Choose an enemy Unit. Summon a copy of it for yourself with Brittle 2."
//!   Radiant: "Choose an enemy Unit. Summon a copy of it for yourself."
//!
//! The chosen Unit is a declared target; the copy lands for its controller (R57), then the base
//! face gives it Brittle 2 once it stands. The Radiant face gives no Brittle (R4586009).
//!
//! Engine: `summon_copy`, `give_brittle` (R4586009). Built by C2 (#4586).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-036";

/// "Brittle N" on the base face's copy.
/// A Spell declares no number it prints unless the cross test names why it is fixed
/// (`tests/cross/params.rs`, R1748003): the base face's 2 is that file's `FIXED` Brittle row.
const BASE_BRITTLE: i32 = 2;

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"] }))]
}

fn deepfake(brittle: Option<i32>) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let Some(unit) = instance_of(ctx, &TargetSpec::Chosen { index: None }) else {
                return vec![];
            };
            let mut effects = vec![summon_copy(json_as(json!({
                "of": { "of": "instance", "instanceId": unit.id },
                "player": "self",
            })))];
            if let Some(n) = brittle {
                effects.push(for_each_card(ForEachCardArgs {
                    cards: Arc::new(|ctx: &mut EffectContext<'_>| summoned_so_far(ctx)),
                    each: Arc::new(move |instance_id: &str| {
                        give_brittle(json_as(json!({ "instanceId": instance_id, "n": n })))
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
        base: deepfake(Some(BASE_BRITTLE)),
        radiant: deepfake(None),
    }
}

// IA #36 Deepfake — SPEC §8.13 row 36 (R4586009).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.
    const VICTIM: &str = "core-008"; // A 2/4.

    fn cast(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, STOCKPILE],
                "library": crate::filler(4),
            },
            "p2": { "field": [VICTIM], "hand": [STOCKPILE], "library": crate::filler(4) },
        }));
        let victim = s.unit_at(PlayerId::P2, 1).id.clone();
        s.play(ID, pick::targets(pick::id(&victim)));
        s
    }

    fn copies(s: &Scenario) -> Vec<CardInstance> {
        s.state().players[P1]
            .units
            .iter()
            .flatten()
            .flatten()
            .filter(|card| card.def_id == VICTIM)
            .cloned()
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4586009_the_chosen_enemy_is_copied_with_brittle_2() {
            let s = cast(false, "deepfake-base");
            let landed = copies(&s);
            assert_eq!(landed.len(), 1, "one copy of the chosen Unit");
            assert_eq!(landed[0].brittle.map(|brittle| brittle.count), Some(2));
            assert_eq!(s.state().players[P2].units.iter().flatten().flatten().count(), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4586009_the_copy_has_no_brittle() {
            let s = cast(true, "deepfake-radiant");
            let landed = copies(&s);
            assert_eq!(landed.len(), 1);
            assert_eq!(landed[0].brittle, None, "no Brittle on the Radiant copy");
        }
    }
}
