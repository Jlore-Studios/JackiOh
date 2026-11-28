//! SPEC §8.14 FvH #96 Magic Inclined Felinor ((2) Unit, Felinor, Rare, 3/5 → 6/10), a card of the held Felinors
//! vs Humans set (R5235000).
//!   Base:    "Spell Damage +1\nCry: Add {spells|random (1) Cost Spell|random (1) Cost Spells} to your hand."
//!   Radiant: "Spell Damage +2\nCry: Add {spells|random Radiant (1) Cost Spell|random Radiant (1) Cost Spells}
//!            to your hand." (spells 2 on both faces)
//!
//! The Cry is `add_random_from_catalog` of the declared `spells` Spells printed at (1), the non-token cards of
//! the sets that ship (R380, R1420), different cards while the pool lasts (R3563000), Radiant on the Radiant
//! face; a full hand burns the rest. Spell Damage is catalog data. The (1) is a filter's price, no declared
//! number. Built by S07 (#5257).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-096";

/// The printed price of the Spells the Cry adds.
const SPELL_COST: i32 = 1;

fn magic_inclined(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "type": "Spell", "cost": SPELL_COST },
                "count": param(ctx, "spells"),
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: magic_inclined(false),
        radiant: magic_inclined(true),
    }
}

// FvH #96 Magic Inclined Felinor — SPEC §8.14 row 96.
#[cfg(test)]
mod tests {
    use super::{ID, SPELL_COST};
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005"; // Stockpile: a card to hold

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": crate::filler(4) },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }))
    }

    /// The cards p1's hand gained in the last step, by `addedToHand`.
    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.last_events_of(GameEventType::AddedToHand)
            .into_iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player: P1, instance_id, .. } => Some(s.card(instance_id.as_str()).clone()),
                _ => None,
            })
            .collect()
    }

    fn felinor(s: &Scenario) -> CardInstance {
        must(
            active_units_of(s.state(), P1)
                .into_iter()
                .find(|unit| unit.def_id == ID)
                .cloned(),
            "the Magic Inclined Felinor on p1's field",
        )
    }

    fn adds_two_different_one_cost_spells(radiant: bool) {
        let mut s = board(radiant);
        s.play(ID, json!({}));
        let spells = nonempty(added(&s), "the Spells the Cry added");
        assert_eq!(spells.len(), 2);
        for spell in &spells {
            let def = def_of(Some(s.state()), &spell.def_id);
            assert_eq!(def.type_, CardType::Spell, "{} is a Spell", def.id);
            assert_eq!(query_cost(def), SPELL_COST, "{} is printed at (1)", def.id);
            assert!(!def.token, "{} is no token", def.id);
            assert_eq!(spell.radiant, radiant);
        }
        assert_ne!(spells[0].def_id, spells[1].def_id, "different cards while the pool lasts (R3563000)");
    }

    mod base {
        use super::*;

        #[test]
        fn its_cry_adds_two_different_random_one_cost_spells() {
            adds_two_different_one_cost_spells(false);
        }

        #[test]
        fn spell_damage_1_reaches_play() {
            let mut s = board(false);
            s.play(ID, json!({}));
            let keywords = s.stats(felinor(&s)).keywords;
            assert!(keywords.contains(&Keyword::SpellDamage { n: 1 }), "{keywords:?}");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn its_cry_adds_two_different_random_radiant_one_cost_spells() {
            adds_two_different_one_cost_spells(true);
        }

        #[test]
        fn spell_damage_2_reaches_play() {
            let mut s = board(true);
            s.play(ID, json!({}));
            let keywords = s.stats(felinor(&s)).keywords;
            assert!(keywords.contains(&Keyword::SpellDamage { n: 2 }), "{keywords:?}");
        }
    }
}
