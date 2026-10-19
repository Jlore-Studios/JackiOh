//! M+ #7 Life Mage (SPEC §8.9 row 7, BUILD M11 row M+ 7): (1) Unit, Human, Common, 1/2 → 2/4.
//!   Base:    "Divine Shield\nAura: Your Spells have Lifesteal."
//!   Radiant: "Divine Shield\nAura: Your Spells have Lifesteal, and healing your hero and Units is twice
//!            as effective."
//!
//! Divine Shield is catalog data. The aura is the engine's Spell-wide keyword aura (R1180002,
//! `layers::spells_have_aura`): while Life Mage acts, each Spell its controller is resolving — not a
//! Field Spell or a Trap, nor a Spell set face-down — has Lifesteal, so damage step 8 heals their hero
//! by each hit (R85); two Life Mages give it once, and it ends when the Mage leaves. The Radiant face
//! adds the healing multiplier (R1180003, `doublesHealing`): every heal of its controller's hero and
//! Units is doubled after any replacement (a heal Blood Moon turns into damage is not), Lifesteal's
//! included, two Radiant Mages quadrupling; the opponent's heals are untouched. Declares no numbers:
//! the doubling is the rule's `HEAL_MULTIPLIER`.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-007";

fn life_mage(radiant: bool) -> Script {
    Script {
        aura: Some(spells_have_aura(Keyword::Lifesteal)),
        static_flags: radiant.then(|| StaticFlags {
            doubles_healing: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: life_mage(false),
        radiant: life_mage(true),
    }
}

// M+ #7 Life Mage — SPEC §8.9 row 7, BUILD M11 row M+ 7: "Divine Shield; while it acts, each of your
// Spells that deals damage heals your hero by the damage dealt, once per hit (R85), a Field Spell, a
// Trap and the opponent's Spells not; leaving the field ends it; two give Lifesteal once; radiant 2/4,
// and every heal of your hero and Units is doubled (a Lifesteal heal included), two Radiant Life Mages
// quadrupling, the opponent's heals untouched".
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// (1) Spell, Pierce: 4 damage to a declared Unit or hero (8 Radiant).
    const TRUE_STRIKE: &str = "core-044";
    /// (2) Field Spell: your next Spell gains Echo +1.
    const TWINSPELL: &str = "core-079";
    /// (1) Spell: draw 2, heal your hero 2.
    const STOCKPILE: &str = "core-005";
    /// (1) Spell: heal a target 9.
    const BOOK_OF_HEAL: &str = "classic-003";
    /// (3) Spell: destroy a Unit.
    const HIT_JOB: &str = "core-016";
    /// (2) Field Spell: start of turn, damage to the enemy hero equal to the cards in your exile.
    const ECHOES: &str = "core-040";
    /// (4) 7/7 Armor 7 (14/14 Radiant).
    const BIG: &str = "core-025";
    /// A (1) 4/4 with no text.
    const FILLER: &str = "core-008";
    /// (2) Trap: a lethal hit is redirected to the enemy hero, then heal and draw.
    const TRAP: &str = "classic-052";

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

    fn health(s: &Scenario, player: PlayerId) -> i32 {
        s.state().players[player].hero.health
    }

    fn hero(player: PlayerId) -> Value {
        json!([{ "pick": "hero", "player": player.as_str() }])
    }

    fn instance(card: &CardInstance) -> Value {
        json!([{ "pick": "instance", "instanceId": card.id }])
    }

    /// `amount` damage to p2's hero with p1's face-up Trap in backrow zone 1 as the running card, then
    /// the state check.
    fn trap_hits_enemy_hero(s: &mut Scenario, amount: i32) {
        let trap = s.backrow(P1, 1).expect("the trap");
        let mut rng = create_rng(&s.state().seed, s.state().rng_cursor);
        let mut events: Vec<GameEvent> = Vec::new();
        {
            let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
            {
                let mut ctx = make_context(
                    &mut sink,
                    Some(&trap),
                    HookOptions {
                        controller: Some(P1),
                        ..HookOptions::default()
                    },
                );
                apply_effects(
                    &[jackioh_engine::effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))],
                    &mut ctx,
                );
            }
            settle(&mut sink, SettleOptions::default());
        }
        s.state_mut().rng_cursor = rng.cursor();
    }

    fn heals_on(s: &Scenario, target_id: &str) -> Vec<i32> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Healed { target_id: healed, amount } if healed == target_id => Some(*amount),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn divine_shield_takes_the_first_hit() {
        let mut s = game(
            "m+007-shield",
            json!({ "field": [ID] }),
            json!({ "hand": [TRUE_STRIKE, FILLER] }),
        );
        s.end_turn();
        let mage = s.unit(P1, 1).expect("the mage");
        s.play(TRUE_STRIKE, json!({ "targets": instance(&mage) }));
        assert_eq!(s.card(&mage.id).damage, 0);
        s.expect_in_zone(mage.id.as_str(), "field");
    }

    #[test]
    fn r1180002_your_spell_heals_your_hero_by_its_damage() {
        let mut s = game(
            "m+007-lifesteal",
            json!({ "field": [ID], "hand": [TRUE_STRIKE], "health": 20 }),
            json!({}),
        );
        s.play(TRUE_STRIKE, json!({ "targets": hero(P2) }));
        assert_eq!(health(&s, P2), 26);
        assert_eq!(health(&s, P1), 24);
    }

    #[test]
    fn r1180002_once_per_hit() {
        let mut s = game(
            "m+007-per-hit",
            json!({ "field": [ID], "hand": [TWINSPELL, TRUE_STRIKE], "health": 20, "mana": 9 }),
            json!({}),
        );
        s.play(TWINSPELL, json!({}));
        s.play(TRUE_STRIKE, json!({ "targets": hero(P2) }));
        s.answer(hero(P2));
        assert_eq!(heals_on(&s, "hero-p1"), vec![4, 4]);
        assert_eq!(health(&s, P1), 28);
    }

    #[test]
    fn r1180002_a_field_spells_hit_heals_nothing() {
        let mut s = game(
            "m+007-field-spell",
            json!({ "field": [ID], "backrow": [ECHOES], "exile": [FILLER, FILLER], "health": 20 }),
            json!({}),
        );
        s.start_turn();
        assert_eq!(health(&s, P2), 28);
        assert_eq!(health(&s, P1), 20);
    }

    #[test]
    fn r1180002_a_traps_hit_heals_nothing() {
        let mut s = game(
            "m+007-trap",
            json!({ "field": [ID], "backrow": [{ "def": TRAP, "faceUp": true }], "health": 20 }),
            json!({}),
        );
        trap_hits_enemy_hero(&mut s, 3);
        assert_eq!(health(&s, P2), 27);
        assert_eq!(health(&s, P1), 20, "a Trap has no Lifesteal from the aura");
    }

    #[test]
    fn r1180002_radiant_a_traps_hit_heals_nothing() {
        let mut s = game(
            "m+007-radiant-trap",
            json!({
                "field": [{ "def": ID, "radiant": true }],
                "backrow": [{ "def": TRAP, "faceUp": true }],
                "health": 20,
            }),
            json!({}),
        );
        trap_hits_enemy_hero(&mut s, 3);
        assert_eq!(health(&s, P2), 27);
        assert_eq!(health(&s, P1), 20, "no Lifesteal heal to double");
    }

    #[test]
    fn r1180002_the_opponents_spells_heal_nothing() {
        let mut s = game(
            "m+007-theirs",
            json!({ "field": [ID] }),
            json!({ "hand": [TRUE_STRIKE, FILLER], "health": 20 }),
        );
        s.end_turn();
        s.play(TRUE_STRIKE, json!({ "targets": hero(P1) }));
        assert_eq!(health(&s, P1), 26);
        assert_eq!(health(&s, P2), 20);
    }

    #[test]
    fn r1180002_leaving_the_field_ends_it() {
        let mut s = game(
            "m+007-gone",
            json!({ "field": [ID], "hand": [HIT_JOB, TRUE_STRIKE], "health": 20, "mana": 9 }),
            json!({}),
        );
        let mage = s.unit(P1, 1).expect("the mage");
        s.play(HIT_JOB, json!({ "targets": instance(&mage) }));
        s.expect_in_zone(mage.id.as_str(), "graveyard");
        s.play(TRUE_STRIKE, json!({ "targets": hero(P2) }));
        assert_eq!(health(&s, P1), 20);
    }

    #[test]
    fn r1180002_two_give_lifesteal_once() {
        let mut s = game(
            "m+007-two",
            json!({ "field": [ID, ID], "hand": [TRUE_STRIKE], "health": 20 }),
            json!({}),
        );
        s.play(TRUE_STRIKE, json!({ "targets": hero(P2) }));
        assert_eq!(heals_on(&s, "hero-p1"), vec![4]);
    }

    #[test]
    fn r1180003_base_heals_are_not_doubled() {
        let mut s = game(
            "m+007-base-heal",
            json!({ "field": [ID], "hand": [STOCKPILE], "health": 20 }),
            json!({}),
        );
        s.play(STOCKPILE, json!({}));
        assert_eq!(health(&s, P1), 22);
    }

    #[test]
    fn r1180003_radiant_doubles_a_heal_of_your_hero() {
        let mut s = game(
            "m+007-radiant-hero",
            json!({ "field": [{ "def": ID, "radiant": true }], "hand": [STOCKPILE], "health": 20 }),
            json!({}),
        );
        s.play(STOCKPILE, json!({}));
        assert_eq!(health(&s, P1), 24);
    }

    #[test]
    fn r1180003_radiant_doubles_a_heal_of_your_unit() {
        let mut s = game(
            "m+007-radiant-unit",
            json!({
                "field": [{ "def": ID, "radiant": true }, { "def": BIG, "radiant": true, "damage": 13 }],
                "hand": [BOOK_OF_HEAL],
            }),
            json!({}),
        );
        let big = s.unit(P1, 2).expect("the 14/14");
        s.play(BOOK_OF_HEAL, json!({ "targets": instance(&big) }));
        // 9 doubled is 18: every point of its 13 damage, where 9 would have left 4.
        assert_eq!(s.card(&big.id).damage, 0);
    }

    #[test]
    fn r1180003_radiant_doubles_the_lifesteal_heal() {
        let mut s = game(
            "m+007-radiant-lifesteal",
            json!({ "field": [{ "def": ID, "radiant": true }], "hand": [TRUE_STRIKE], "health": 20 }),
            json!({}),
        );
        s.play(TRUE_STRIKE, json!({ "targets": hero(P2) }));
        assert_eq!(heals_on(&s, "hero-p1"), vec![8]);
    }

    #[test]
    fn r1180003_two_radiant_life_mages_quadruple() {
        let mut s = game(
            "m+007-radiant-two",
            json!({
                "field": [{ "def": ID, "radiant": true }, { "def": ID, "radiant": true }],
                "hand": [STOCKPILE],
                "health": 20,
            }),
            json!({}),
        );
        s.play(STOCKPILE, json!({}));
        assert_eq!(health(&s, P1), 28);
    }

    #[test]
    fn r1180003_the_opponents_heals_are_untouched() {
        let mut s = game(
            "m+007-radiant-theirs",
            json!({ "field": [{ "def": ID, "radiant": true }] }),
            json!({ "hand": [STOCKPILE, FILLER], "health": 20 }),
        );
        s.end_turn();
        s.play(STOCKPILE, json!({}));
        assert_eq!(health(&s, P2), 22);
    }

    #[test]
    fn no_tuned_numbers() {
        assert!(crate::card_def(ID).params.is_none());
    }
}
