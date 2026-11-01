//! S #136 Magic Module (SPEC §8.10 row 136; R380, R1086, R1748001, R1852000, R1852003, R1859001,
//! R1859005): (2) Unit, Common, 4/3 → 8/6, Starship Piece, Equip, Spell Damage +2 (Radiant +4), a card
//! of the Stinkyscape set.
//!
//! Base and Radiant: "Starship Piece, Equip, Spell Damage +N\nWhen launched, add {spells|random Spell
//! that deals damage|random Spells that deal damage} to your hand." Spells 1, Radiant 2.
//! Engine: its keywords are catalog data — Equip fuses it into a Unit of yours (R1086), its Spell
//! Damage going with it into a host and into the Starship — and its death fuses a copy into its

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-136";

fn magic_module(radiant: bool) -> Script {
    Script {
        triggers: vec![when_launched("magic-module", move |ctx, _event| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "type": "Spell", "dealsDamage": true },
                "count": piece_number(ctx, ID, radiant, "spells"),
            })))]
        })],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: magic_module(false),
        radiant: magic_module(true),
    }
}

// S #136 Magic Module — SPEC §8.10 row 136, BUILD M12 row S 136 (R1859001).
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::{built_starship, card_def, launch_starship};
    use jackioh_engine::testkit::*;

    const LUNAR: &str = "core-035"; // (1) Spell: deal 3 damage.
    const AP_ROUNDS: &str = "stinkyscape-100"; // (1) 3/1 Starship Piece, Equip, Pierce: keywords only.

    fn added_on_launch(seed: &str, radiant: bool) -> Vec<CardDef> {
        let (mut s, ship) = built_starship(
            seed,
            json!([{ "def": ID, "lane": 1, "radiant": radiant }]),
            json!({}),
            json!({}),
        );
        let before: Vec<String> = s.state().players.p1.hand.iter().map(|card| card.id.clone()).collect();
        launch_starship(&mut s, &ship);
        s.state()
            .players
            .p1
            .hand
            .iter()
            .filter(|card| !before.contains(&card.id))
            .map(|card| card_def(&card.def_id))
            .collect()
    }

    fn assert_damage_spells(added: &[CardDef]) {
        for def in added {
            assert_eq!(def.type_, CardType::Spell, "{}", def.id);
            assert_eq!(def.deals_damage, Some(true), "{}", def.id);
            assert!(!def.token, "{}", def.id);
        }
    }

    #[test]
    fn r1859005_when_launched_one_random_spell_that_deals_damage_goes_to_the_hand() {
        let added = added_on_launch("r1859005-base", false);
        assert_eq!(added.len(), 1);
        assert_damage_spells(&added);
    }

    #[test]
    fn r1859001_a_radiant_piece_adds_two_read_on_the_face_it_fused_on() {
        let added = added_on_launch("r1859005-radiant", true);
        assert_eq!(added.len(), 2);
        assert_damage_spells(&added);
    }

    #[test]
    fn r1859005_on_the_field_its_spell_damage_raises_its_controllers_spell_hits_and_launches_add_nothing() {
        for (radiant, spell_damage) in [(false, 2), (true, 4)] {
            let (mut s, ship) = built_starship(
                "r1859005-field",
                json!([{ "def": AP_ROUNDS, "lane": 1 }]),
                json!({
                    "hand": [LUNAR],
                    "field": [{ "def": ID, "lane": 2, "radiant": radiant }],
                }),
                json!({}),
            );
            let health = s.state().players.p2.hero.health;
            s.play(LUNAR, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            assert_eq!(s.state().players.p2.hero.health, health - 3 - spell_damage);
            let hand = s.state().players.p1.hand.len();
            launch_starship(&mut s, &ship);
            assert_eq!(s.state().players.p1.hand.len(), hand, "a Module on the field adds nothing");
            assert_eq!(s.state().players.p2.hero.health, health - 3 - spell_damage);
        }
    }
}
