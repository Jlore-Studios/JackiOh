//! FvH #112.6 Power Module (SPEC §8.14 row 112.6; R62, R4079000): (2) Unit, Token (printed Mythic), 1/3 →
//! 2/6, of the held Felinors vs Humans set. A module FvH #112 Jilliax Deluxe offers and fuses (FV2 #112).
//!   Base and Radiant: "Start of turn: Double this's Attack."
//! Engine: a `start_of_turn` hook (its controller's, R62): a permanent `buff` of its own Attack by its Attack
//! now, read through the layers as the hook resolves (§10.4), IA #202 Exponent's base shape, capped so the
//! doubled Attack never passes `MULTIPLIED_STAT_CAP` (R4079000). Fused, "this" is the fused card.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-112-6";

pub fn script() -> CardScripts {
    let base = Script {
        start_of_turn: Some(hook(|ctx| {
            let Some(me) = ctx.live_self() else {
                return vec![];
            };
            let attack = unit_view(ctx.state, me).attack.max(0);
            let gain = attack.min(MULTIPLIED_STAT_CAP - attack).max(0);
            if gain == 0 {
                return vec![];
            }
            vec![buff(json_as(json!({ "target": { "of": "self" }, "attack": gain })))]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 2/6 is catalog data.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #112.6 Power Module — SPEC §8.14 row 112.6.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, filler, scenario};

    fn doubled(radiant: bool) -> (i32, i32) {
        let mut s = scenario(json!({
            "p1": { "field": [{ "def": ID, "lane": 1, "radiant": radiant }], "hand": ["core-005"], "library": filler(4) },
            "p2": { "hand": ["core-005"], "library": filler(4) },
        }));
        s.end_turn();
        s.end_turn();
        let view = unit_view(s.state(), &s.unit_at(P1, 1));
        (view.attack, view.health)
    }

    #[test]
    fn its_card_declares_no_numbers() {
        crate::register_all();
        assert!(card_def(ID).params.is_none());
    }

    #[test]
    fn r62_the_base_face_doubles_its_attack_at_the_start_of_your_turn() {
        assert_eq!(doubled(false), (2, 3));
    }

    #[test]
    fn r62_the_radiant_face_doubles_its_attack_at_the_start_of_your_turn() {
        assert_eq!(doubled(true), (4, 6));
    }
}
