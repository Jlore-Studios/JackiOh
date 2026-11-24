//! IA #44 The Lie Detector (SPEC §8.13 row 44), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Cry: Destroy every enemy face-down card. Set a Figment for each."
//!   Radiant: "Cry: Destroy every enemy face-down card. Set two Figments for each."
//!
//! Every face-down card of the enemy's is marked first, then one plain Figment is set per card
//! the destroy marked — two on the Radiant face — as many as fit (§3.2, R4586001). A ward the
//! opponent's destroy passes over leaves no mark, so no Figment (R4586004); unmarked cards are
//! passed by.
//!
//! Engine: `destroy`, a Figment `summon` (R4586001). Built by C2 (#4586).

use jackioh_engine::prelude::*;

use crate::scripts::infinite_assault::c025_1_figment::ID as FIGMENT_ID;

pub const ID: &str = "infiniteassault-044";

/// The enemy's face-down cards, by id.
fn facedown_enemy(c: &mut EffectContext<'_>) -> Vec<String> {
    let enemy = opponent_of(c.controller);
    face_down_cards(&*c.state, enemy)
        .into_iter()
        .map(|card| card.id)
        .collect()
}

/// The enemy's face-down cards the destroy marked, by id.
fn marked_enemy(c: &mut EffectContext<'_>) -> Vec<String> {
    let enemy = opponent_of(c.controller);
    face_down_cards(&*c.state, enemy)
        .into_iter()
        .filter(|card| card.marked_destroyed == Some(true))
        .map(|card| card.id)
        .collect()
}

fn summon_figment() -> Effect {
    summon(json_as(json!({ "defId": FIGMENT_ID })))
}

fn detector(figments_per: i32) -> Script {
    Script {
        cry: Some(hook(move |_| {
            let mut effects = vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(facedown_enemy),
                each: Arc::new(|instance_id: &str| {
                    destroy(json_as(json!({
                        "target": { "of": "instance", "instanceId": instance_id },
                    })))
                }),
            })];
            for _ in 0..figments_per.max(0) {
                effects.push(for_each_card(ForEachCardArgs {
                    cards: Arc::new(marked_enemy),
                    each: Arc::new(|_| summon_figment()),
                }));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: detector(1),
        radiant: detector(2),
    }
}

// IA #44 The Lie Detector — SPEC §8.13 row 44 (R4586001).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const FIGMENT: &str = "infiniteassault-025-1";
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.
    const TRAP: &str = "core-060"; // Bear Honeypot: a Trap.

    fn cried(radiant: bool, seed: &str, enemy_backrow: Value) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, STOCKPILE],
                "library": crate::filler(4),
            },
            "p2": {
                "backrow": enemy_backrow,
                "hand": [STOCKPILE],
                "library": crate::filler(4),
            },
        }));
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    fn p1_figments(s: &Scenario) -> Vec<CardInstance> {
        s.state().players[P1]
            .backrow
            .iter()
            .filter_map(|slot| slot.clone())
            .filter(|card| card.def_id == FIGMENT)
            .cloned()
            .collect()
    }

    fn p2_backrow_defs(s: &Scenario) -> Vec<String> {
        s.state().players[P2]
            .backrow
            .iter()
            .filter_map(|slot| slot.clone())
            .map(|card| card.def_id)
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4586001_every_enemy_face_down_card_is_destroyed_one_figment_each() {
            let s = cried(false, "detector-base", json!([TRAP, TRAP]));
            assert!(p2_backrow_defs(&s).is_empty(), "both destroyed");
            assert_eq!(p1_figments(&s).len(), 2, "one Figment per marked card");
        }

        #[test]
        fn r4586001_face_up_cards_are_passed_by() {
            const WELL: &str = "core-006"; // Mana Well: a Field Spell.
            let s = cried(false, "detector-faceup", json!([WELL, TRAP]));
            assert_eq!(p2_backrow_defs(&s), vec![WELL], "the face-up Field Spell stands");
            assert_eq!(p1_figments(&s).len(), 1, "one Figment for the marked Trap");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4586001_two_figments_for_each_destroyed_card() {
            let s = cried(true, "detector-radiant", json!([TRAP]));
            assert!(p2_backrow_defs(&s).is_empty());
            assert_eq!(p1_figments(&s).len(), 2, "two Figments for the one card");
        }
    }
}
