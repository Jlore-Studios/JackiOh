//! SPEC §8.12 S+ #T-AI-2 Data-Poisoned Plum, a card of the Stinkyscape+ set (R1420, R3535000). Built by S25 (#3571).
//!   Base:    "Give an enemy Unit Toxic 1."
//!   Radiant: "Give an enemy Unit Toxic 2."
//! Engine: a (1) Spell, Fruit, AI, Token (no printed rarity, R482). The enemy Unit is a declared target (R81),
//!   never a hero; with none on the board the Spell plays and fizzles (R90). The `toxic` verb (as #146 Scourge
//!   Mortar v3's) adds the Toxic and deals no damage (R3540000); it ticks at its controller's turn start
//!   (R3540001). Declares no numbers (R482: its numbers are the script's).

use jackioh_engine::effects::{AfflictArgs, TargetSpec, toxic};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-t-ai-02";

/// "Toxic 1".
const TOXIC: i32 = 1;

/// "Toxic 2".
const RADIANT_TOXIC: i32 = 2;

fn face(amount: i32) -> Script {
    Script {
        // "an enemy Unit": enemy Units only, never a hero. Not `required`: with none, it fizzles (R90).
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "enemy", "of": ["unit"] }),
        )],
        cry: Some(hook(move |_ctx| {
            vec![toxic(AfflictArgs {
                to: TargetSpec::Chosen { index: None },
                amount,
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(TOXIC),
        radiant: face(RADIANT_TOXIC),
    }
}

// S+ #T-AI-2 Data-Poisoned Plum — SPEC §8.12 row T-AI-2 (R81, R90, R3540000, R3540001).
#[cfg(test)]
mod tests {
    use super::{ID, RADIANT_TOXIC, TOXIC};
    use crate::{hits, weapon_board};
    use jackioh_engine::query::toxic_on;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A durable Unit: it survives any Toxic's tick.
    const TOUGH: &str = "stand-in (1) 2/30";

    fn plum(radiant: bool) -> Value {
        json!({ "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 })
    }

    fn amount(radiant: bool) -> i32 {
        if radiant { RADIANT_TOXIC } else { TOXIC }
    }

    #[test]
    fn gives_the_chosen_enemy_unit_its_toxic_and_no_damage() {
        for radiant in [false, true] {
            let toxic = amount(radiant);
            let mut s = weapon_board("s-t-ai-02", plum(radiant), json!({ "field": [TOUGH, TOUGH] }));
            let target = s.unit_at(P2, 1).id;
            let beside = s.unit_at(P2, 2).id;
            s.play(ID, json!({ "targets": pick::id(&target) }));
            assert_eq!(toxic_on(s.card(target.as_str())), toxic, "radiant {radiant}");
            assert_eq!(toxic_on(s.card(beside.as_str())), 0, "radiant {radiant}");
            // A verb, no damage.
            assert!(hits(s.last_events()).is_empty(), "radiant {radiant}");
            // It ticks at its controller's turn start.
            s.end_turn();
            assert_eq!(s.stats(target.as_str()).health, 30 - toxic, "radiant {radiant}");
            assert_eq!(s.stats(beside.as_str()).health, 30, "radiant {radiant}");
        }
    }

    #[test]
    fn offers_only_enemy_units() {
        for radiant in [false, true] {
            let mut s = weapon_board(
                "s-t-ai-02-offers",
                json!({ "hand": [{ "def": ID, "radiant": radiant }], "field": [TOUGH], "mana": 10 }),
                json!({ "field": [TOUGH, TOUGH] }),
            );
            let plum = s.card(ID).id.clone();
            let offers = legal_actions(s.state(), P1)
                .iter()
                .filter(
                    |action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == plum),
                )
                .count();
            // One play for each enemy Unit, none for the friendly one and none for a hero.
            assert_eq!(offers, 2, "radiant {radiant}");
            s.expect_refused(|s| s.play(ID, json!({ "targets": [{ "pick": "hero", "player": "p2" }] })));
        }
    }

    #[test]
    fn with_no_enemy_unit_it_fizzles() {
        for radiant in [false, true] {
            let mut s = weapon_board("s-t-ai-02-fizzle", plum(radiant), json!({}));
            s.play(ID, json!({}));
            s.expect_in_zone(ID, "graveyard");
            s.expect_health(P2, HERO_HEALTH);
        }
    }
}
