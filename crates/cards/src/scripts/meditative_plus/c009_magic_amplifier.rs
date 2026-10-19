//! M+ #9 Magic Amplifier (SPEC §8.9 row 9, BUILD M11 row M+ 9): (1) Field Spell, Common.
//!   Base:    "Aura: Both players have Spell Damage +{spellDamage}." (2)
//!   Radiant: "Aura: Both players have Spell Damage +{spellDamage}." (4)
//!
//! Spell Damage beyond Units (R1180000): the engine's `spellDamageAura` static flag, for both players.
//! While the Amplifier acts on the field — face-up, in its controller's backrow — each player's Spell
//! Damage, the one sum §4.4 step 0 raises every hit of their Spells by (`damage::spell_damage_of`),
//! gains its number, beside their Units' keyword and any "this turn" modifier (M+ #11). A Field
//! Spell's or a Trap's hit is not a Spell's and is never raised; face-down or gone it gives nothing.
//! The number is the declared `spellDamage`, read through `params::declared_or` as each hit lands, so
//! a Nerf or a Buff moves it (R386). Tunes: spellDamage 2 ↑ (Radiant 4).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-009";

/// The catalog's declared Spell Damage, read off the face that is up (R386).
fn declared_spell_damage(radiant: bool) -> i32 {
    match crate::card_def(ID)
        .params
        .as_ref()
        .and_then(|params| params.iter().find(|entry| entry.key == "spellDamage"))
    {
        Some(param) if radiant => param.radiant,
        Some(param) => param.base,
        None => panic!("{ID} declares spellDamage (catalog params)"),
    }
}

fn amplifier(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            spell_damage_aura: Some(SpellDamageAura {
                amount: declared_spell_damage(radiant),
                both: Some(true),
            }),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: amplifier(false),
        radiant: amplifier(true),
    }
}

// M+ #9 Magic Amplifier — SPEC §8.9 row 9, BUILD M11 row M+ 9: "while it acts face-up, every hit of
// either player's Spells deals 2 more, added to Units' Spell Damage (C+ 38, M+ 8); a Field Spell's and
// a Trap's hits do not; face-down or gone it gives nothing; spellDamage reads through `param()`;
// radiant +4".
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::damage::spell_damage_of;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// (1) Spell, Pierce: 4 damage to a declared Unit or hero.
    const TRUE_STRIKE: &str = "core-044";
    /// (2) 3/2, Spell Damage +2.
    const SOLARIUS: &str = "classicplus-038";
    /// (2) Field Spell: start of turn, damage to the enemy hero equal to the cards in your exile.
    const ECHOES: &str = "core-040";
    /// (2) Field Trap: reveals when the opponent summons a Unit, 4 damage to it.
    const TESLA: &str = "classic-005";
    /// (4) Spell: exile a permanent, and a random card from the opponent's deck.
    const COLLATERAL: &str = "core-034";
    /// A (1) 4/4 with no text.
    const FILLER: &str = "core-008";

    fn game(seed: &str, p1: Value, p2: Value) -> Scenario {
        let mut sides = json!({
            "seed": seed,
            "p1": { "library": [FILLER, FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
        });
        for (seat, extra) in [("p1", p1), ("p2", p2)] {
            if let Value::Object(entries) = extra {
                for (key, value) in entries {
                    sides[seat][key.as_str()] = value;
                }
            }
        }
        crate::scenario(sides)
    }

    fn hero(player: PlayerId) -> Value {
        json!([{ "pick": "hero", "player": player.as_str() }])
    }

    fn health(s: &Scenario, player: PlayerId) -> i32 {
        s.state().players[player].hero.health
    }

    #[test]
    fn r1180000_your_spells_deal_2_more_per_hit() {
        let mut s = game(
            "m+009-yours",
            json!({ "backrow": [ID], "hand": [TRUE_STRIKE] }),
            json!({}),
        );
        s.play(TRUE_STRIKE, json!({ "targets": hero(P2) }));
        assert_eq!(health(&s, P2), 24);
    }

    #[test]
    fn r1180000_the_opponents_spells_deal_2_more_too() {
        let mut s = game(
            "m+009-theirs",
            json!({ "backrow": [ID] }),
            json!({ "hand": [TRUE_STRIKE, FILLER] }),
        );
        s.end_turn();
        s.play(TRUE_STRIKE, json!({ "targets": hero(P1) }));
        assert_eq!(health(&s, P1), 24);
    }

    #[test]
    fn r1180000_it_adds_to_units_spell_damage() {
        let mut s = game(
            "m+009-solarius",
            json!({ "backrow": [ID], "field": [SOLARIUS], "hand": [TRUE_STRIKE] }),
            json!({}),
        );
        assert_eq!(spell_damage_of(s.state(), P1), 4);
        s.play(TRUE_STRIKE, json!({ "targets": hero(P2) }));
        assert_eq!(health(&s, P2), 22);
    }

    #[test]
    fn r1180000_a_field_spells_hit_is_not_raised() {
        let mut s = game(
            "m+009-field-spell",
            json!({ "backrow": [ID, ECHOES], "exile": [FILLER] }),
            json!({}),
        );
        s.start_turn();
        assert_eq!(health(&s, P2), 29);
    }

    #[test]
    fn r1180000_a_traps_hit_is_not_raised() {
        let mut s = game(
            "m+009-trap",
            json!({ "backrow": [ID, { "def": TESLA, "faceUp": false, "lane": 3 }] }),
            json!({ "hand": [FILLER, FILLER] }),
        );
        s.end_turn();
        s.play(FILLER, json!({ "zone": 2 }));
        let hits: Vec<i32> = s
            .events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { amount, .. } => Some(*amount),
                _ => None,
            })
            .collect();
        assert_eq!(hits, vec![4]);
    }

    #[test]
    fn r1180000_face_down_it_gives_nothing() {
        let mut s = game("m+009-face-down", json!({ "backrow": [ID] }), json!({}));
        let amp = s.backrow(P1, 1).expect("the amplifier");
        s.card_mut(&amp.id).set_as = Some(SetAs {
            reveal: RevealAt::Cloaked,
            set_turn: 1,
            echo: None,
            revealing: None,
        });
        s.card_mut(&amp.id).face_up = Some(false);
        assert_eq!(spell_damage_of(s.state(), P1), 0);
        assert_eq!(spell_damage_of(s.state(), P2), 0);
    }

    #[test]
    fn r1180000_gone_it_gives_nothing() {
        let mut s = game(
            "m+009-gone",
            json!({ "backrow": [ID] }),
            json!({ "hand": [COLLATERAL, FILLER], "mana": 9 }),
        );
        s.end_turn();
        let amp = s.backrow(P1, 1).expect("the amplifier");
        s.play(COLLATERAL, json!({ "targets": [{ "pick": "instance", "instanceId": amp.id }] }));
        s.expect_in_zone(amp.id.as_str(), "exile");
        assert_eq!(spell_damage_of(s.state(), P1), 0);
        assert_eq!(spell_damage_of(s.state(), P2), 0);
    }

    #[test]
    fn spell_damage_reads_through_param() {
        let mut s = game("m+009-param", json!({ "backrow": [ID] }), json!({}));
        let amp = s.backrow(P1, 1).expect("the amplifier").id.clone();
        set_param(s.card_mut(&amp), "spellDamage", 3);
        assert_eq!(spell_damage_of(s.state(), P1), 3);
        assert_eq!(spell_damage_of(s.state(), P2), 3);
    }

    #[test]
    fn radiant_both_players_have_spell_damage_4() {
        let mut s = game(
            "m+009-radiant",
            json!({ "backrow": [{ "def": ID, "radiant": true }], "hand": [TRUE_STRIKE] }),
            json!({}),
        );
        assert_eq!(spell_damage_of(s.state(), P2), 4);
        s.play(TRUE_STRIKE, json!({ "targets": hero(P2) }));
        assert_eq!(health(&s, P2), 22);
    }
}
