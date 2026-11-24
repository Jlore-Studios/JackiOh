//! SPEC §8.13 IA #136 DDoS ((3) Spell, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Freeze every enemy Unit."
//!   Radiant: "Freeze every enemy Unit and deal {damage} damage to them."
//!
//! S #1 Freeze Ray's Radiant shape: `freezeAll` over the enemy's Units (§6.3 Freeze, R1174000);
//! on the Radiant face the same Units then take the card's declared `damage` (M+ T-AI-3's shape,
//! §6.3 Damage).

use jackioh_engine::effects::{damage_all, freeze_all};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-136";

fn ddos(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut effects = vec![freeze_all(json_as(json!({ "side": "enemy" })))];
            if radiant {
                effects.push(damage_all(json_as(json!({
                    "side": "enemy",
                    "amount": param(&*ctx, "damage"),
                }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: ddos(false),
        radiant: ddos(true),
    }
}

// IA #136 DDoS — SPEC §8.13 row 136.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P2;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "ddos",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 },
            "p2": { "field": [VANILLA, VANILLA], "hand": ["core-005"] },
        }))
    }

    fn frozen(s: &Scenario, card: &CardInstance) -> bool {
        s.card(card).frozen.is_some()
    }

    mod base {
        use super::*;

        #[test]
        fn freezes_every_enemy_unit_and_deals_no_damage() {
            let mut s = board(false);
            let first = s.unit_at(P2, 1);
            let second = s.unit_at(P2, 2);
            s.play(ID, json!({}));
            assert!(frozen(&s, &first));
            assert!(frozen(&s, &second));
            assert_eq!(s.card(&first).damage, 0, "no damage on the base face");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn freezes_every_enemy_unit_and_deals_2_damage_to_them() {
            let mut s = board(true);
            let first = s.unit_at(P2, 1);
            let second = s.unit_at(P2, 2);
            s.play(ID, json!({}));
            assert!(frozen(&s, &first));
            assert!(frozen(&s, &second));
            assert_eq!(s.card(&first).damage, 2, "the declared damage lands");
            assert_eq!(s.card(&second).damage, 2, "the declared damage lands");
        }
    }
}
