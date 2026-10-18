//! M+ #11 Living Mana Potion (SPEC §8.9 row 11, BUILD M11 row M+ 11): (1) Unit, Potion, Rare,
//! 5/1 → 10/2.
//!   Base:    "Activate: Tribute this. You have Spell Damage +{spellDamage} this turn." (5)
//!   Radiant: "Activate: You have Spell Damage +{spellDamage} this turn." (5)
//!
//! An Activate (§6.2, R384): in its controller's main phase, the turn it lands included (no
//! summoning sickness), once a turn. The base face's cost is the card's own Tribute (`tribute_self`,
//! C #84, M #54), paid first; the Radiant face's has none, so it stays and activates again each turn.
//! Either way the activator gets a "Spell Damage +N this turn" player modifier (R1180001,
//! `ModifierKind::SpellDamage`, expiry `thisTurn`), summed with their Units' Spell Damage and any
//! Spell Damage aura (M+ #9) in the one place §4.4 step 0 reads (`damage::spell_damage_of`); it is the
//! activator's, the opponent's Spells untouched, it stays when the potion has left, two stack, and the
//! turn's cleanup ends it. The number is the declared `spellDamage`, read as the ability runs (R386).
//! Tunes: spellDamage 5 ↑.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-011";

fn spell_damage_this_turn() -> Hook {
    hook(|ctx| {
        vec![add_player_modifier(json_as(json!({
            "mod": {
                "expiry": { "until": "thisTurn", "turn": ctx.state.turn },
                "kind": "spellDamage",
                "amount": param(&*ctx, "spellDamage"),
            },
        })))]
    })
}

fn potion(radiant: bool) -> Script {
    Script {
        activations: vec![ActivationDecl {
            id: "spellDamage".to_string(),
            label: "Spell Damage this turn".to_string(),
            uses: ActivationUses::Count(1),
            cost: (!radiant).then(|| ActivationCost {
                tribute_self: Some(true),
                ..ActivationCost::default()
            }),
            targets: Vec::new(),
            modes: Vec::new(),
            can_activate: None,
            has: None,
            run: spell_damage_this_turn(),
        }],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: potion(false),
        radiant: potion(true),
    }
}

// M+ #11 Living Mana Potion — SPEC §8.9 row 11, BUILD M11 row M+ 11: "Activate in your main phase,
// the turn it lands included (no sickness, R384): it is tributed and your Spells deal 5 more per hit
// until the turn's cleanup, then not; the opponent's Spells are untouched; it sums with Units' Spell
// Damage and M+ 9; spellDamage reads through `param()`; radiant 10/2, the Activate keeps it on the
// field, once a turn, +5 each turn".
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
    /// M+ #9, "Aura: Both players have Spell Damage +2".
    const AMPLIFIER: &str = "meditativeplus-009";
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
    fn it_activates_the_turn_it_lands_no_sickness_and_is_tributed() {
        let mut s = game("m+011-lands", json!({ "hand": [ID, TRUE_STRIKE] }), json!({}));
        s.play(ID, json!({ "zone": 1 }));
        let potion = s.unit(P1, 1).expect("the potion");
        s.activate(potion.id.as_str(), json!({}));
        s.expect_in_zone(potion.id.as_str(), "graveyard");
        assert_eq!(spell_damage_of(s.state(), P1), 5);
    }

    #[test]
    fn r1180001_your_spells_deal_5_more_per_hit_this_turn() {
        let mut s = game("m+011-hits", json!({ "field": [ID], "hand": [TRUE_STRIKE] }), json!({}));
        s.activate(ID, json!({}));
        s.play(TRUE_STRIKE, json!({ "targets": hero(P2) }));
        assert_eq!(health(&s, P2), 21);
    }

    #[test]
    fn r1180001_the_turns_cleanup_ends_it() {
        let mut s = game(
            "m+011-cleanup",
            json!({ "field": [ID] }),
            json!({ "hand": [FILLER, FILLER] }),
        );
        s.activate(ID, json!({}));
        s.end_turn();
        assert_eq!(spell_damage_of(s.state(), P1), 0);
        assert!(s.state().players.p1.mods.is_empty());
    }

    #[test]
    fn r1180001_the_opponents_spells_are_untouched() {
        let mut s = game("m+011-theirs", json!({ "field": [ID], "hand": [FILLER] }), json!({}));
        s.activate(ID, json!({}));
        assert_eq!(spell_damage_of(s.state(), P2), 0);
    }

    #[test]
    fn r1180001_it_sums_with_units_spell_damage_and_magic_amplifier() {
        let mut s = game(
            "m+011-sum",
            json!({ "field": [ID, SOLARIUS], "backrow": [AMPLIFIER], "hand": [TRUE_STRIKE] }),
            json!({}),
        );
        s.activate(ID, json!({}));
        assert_eq!(spell_damage_of(s.state(), P1), 9);
        s.play(TRUE_STRIKE, json!({ "targets": hero(P2) }));
        assert_eq!(health(&s, P2), 17);
    }

    #[test]
    fn spell_damage_reads_through_param() {
        // A card in hand, so the turn does not end itself once the potion is gone (§2.5).
        let mut s = game("m+011-param", json!({ "field": [ID], "hand": [FILLER] }), json!({}));
        let potion = s.unit(P1, 1).expect("the potion").id.clone();
        set_param(s.card_mut(&potion), "spellDamage", 3);
        s.activate(potion.as_str(), json!({}));
        assert_eq!(spell_damage_of(s.state(), P1), 3);
    }

    #[test]
    fn radiant_it_stays_once_a_turn_and_again_next_turn() {
        let mut s = game(
            "m+011-radiant",
            json!({ "field": [{ "def": ID, "radiant": true }], "hand": [FILLER] }),
            json!({ "hand": [FILLER, FILLER] }),
        );
        s.activate(ID, json!({}));
        s.expect_in_zone(ID, "field");
        assert_eq!(spell_damage_of(s.state(), P1), 5);
        s.expect_refused(|s| s.activate(ID, json!({})));
        s.end_turn();
        s.end_turn();
        assert_eq!(spell_damage_of(s.state(), P1), 0);
        s.activate(ID, json!({}));
        assert_eq!(spell_damage_of(s.state(), P1), 5);
    }
}
