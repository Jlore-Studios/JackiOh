//! FvH #112.12 Debuff Module (SPEC §8.14 row 112.12; R4783001, R5239007): (5) Unit, Token (printed Mythic),
//! 6/2 → 12/4, of the held Felinors vs Humans set. A module FvH #112 Jilliax Deluxe offers and fuses (FV2
//! #112).
//!   Base:    "Damage dealt also Burns."
//!   Radiant: "Damage dealt also applies Toxic."
//! Engine: base `StaticFlags.damage_also_burns`, S+ #151 Volcanic Rounds' (R4783001): each hit it deals lands
//! as damage and then gives a target that survived it Burn equal to what it dealt. Radiant
//! `StaticFlags.damage_also_toxic` (R5239007): the same with Toxic. A fusion carries the flag (R77).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-112-12";

pub fn script() -> CardScripts {
    // R4783001: "Damage dealt also Burns".
    let base = Script {
        static_flags: Some(StaticFlags {
            damage_also_burns: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    // R5239007: "Damage dealt also applies Toxic".
    let radiant = Script {
        static_flags: Some(StaticFlags {
            damage_also_toxic: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// FvH #112.12 Debuff Module — SPEC §8.14 row 112.12, R4783001, R5239007.
#[cfg(test)]
mod tests {
    use jackioh_engine::query::{burn_on, toxic_on};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, scenario};

    /// A target that takes the hit and lives, striking nothing back.
    const TARGET: &str = "stand-in (1) 0/30";
    /// The printed Attack, and the Radiant face's.
    const ATTACK: i32 = 6;
    const RADIANT_ATTACK: i32 = 12;

    /// The module attacks `TARGET`; the target as it stands after.
    fn struck(radiant: bool) -> CardInstance {
        let mut s = scenario(json!({
            "p1": { "hand": ["core-005"], "field": [{ "def": ID, "radiant": radiant }] },
            "p2": { "hand": ["core-005"], "field": [TARGET] },
        }));
        let module = s.unit_at(P1, 1);
        let target = s.unit_at(P2, 1);
        s.attack(&module, &target);
        s.card(target.id.as_str()).clone()
    }

    #[test]
    fn its_card_declares_no_numbers() {
        crate::register_all();
        assert!(card_def(ID).params.is_none());
    }

    #[test]
    fn r4783001_the_base_face_s_hit_also_burns_as_much_as_it_dealt() {
        let target = struck(false);
        assert_eq!(target.damage, ATTACK);
        assert_eq!(burn_on(&target), ATTACK);
        assert_eq!(toxic_on(&target), 0);
    }

    #[test]
    fn r5239007_the_radiant_face_s_hit_also_gives_toxic_as_much_as_it_dealt() {
        let target = struck(true);
        assert_eq!(target.damage, RADIANT_ATTACK);
        assert_eq!(toxic_on(&target), RADIANT_ATTACK);
        assert_eq!(burn_on(&target), 0);
    }
}
