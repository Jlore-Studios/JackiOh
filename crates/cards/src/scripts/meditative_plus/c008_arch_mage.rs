//! M+ #8 Arch Mage (SPEC §8.9 row 8, BUILD M11 row M+ 8): (2) Unit, Human, Common,
//! 4/7 → 8/14.
//!   Base:    "Spell Damage +3"
//!   Radiant: "Spell Damage +6"
//!
//! The Spell Damage keyword (`Keyword::SpellDamage`, §6.1; C+ #38's shape): its controller's Spells
//! deal 3 more per hit (§4.4 step 0). Keywords are catalog data, so both faces run nothing.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-008";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face adds Spell Damage and doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// M+ #8 Arch Mage — SPEC §8.9 row 8, BUILD M11 row M+ 8: "while it acts each hit of your Spells
// deals 3 more (a Field Spell's and a Trap's do not, §6.1); the opponent's Spells are untouched;
// leaving the field ends it; a Nerf moves the keyword's number; radiant 8/14 and +6".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MAGE: &str = "meditativeplus-008";
    const LUNAR_ECLIPSE: &str = "core-035"; // (1) Spell: Deal 3 damage to a target.
    const TRUE_STRIKE: &str = "core-044"; // (1) Spell, Pierce: 4 damage to a declared Unit or hero.
    const ECHOES: &str = "core-040"; // Field Spell: start of turn, damage to the enemy hero = your exile count.
    const FILLER: &str = "core-005";
    const TRAP: &str = "classic-052"; // (2) Trap: a lethal hit is redirected to the enemy hero, then heal and draw.

    use crate::scenario;

    fn at_hero() -> Value {
        json!({ "targets": [{ "pick": "hero", "player": "p2" }] })
    }

    #[test]
    fn s4_4_step_0_your_spell_deals_3_more_per_hit() {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "hand": [LUNAR_ECLIPSE, FILLER], "field": [MAGE] }, "p2": { "hand": [FILLER] } }));
        s.play(LUNAR_ECLIPSE, at_hero());
        s.expect_health(P2, HERO_HEALTH - 6);
    }

    #[test]
    fn never_a_field_spells_hit() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "field": [MAGE], "backrow": [ECHOES], "exile": [FILLER, FILLER], "library": [FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }));
        s.end_turn();
        s.end_turn(); // p1's start of turn: Echoes deals 2.
        s.expect_health(P2, HERO_HEALTH - 2);
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

    #[test]
    fn r1180000_never_a_traps_hit() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "field": [MAGE], "backrow": [{ "def": TRAP, "faceUp": true }] },
            "p2": { "hand": [FILLER] },
        }));
        trap_hits_enemy_hero(&mut s, 2);
        s.expect_health(P2, HERO_HEALTH - 2);
    }

    #[test]
    fn r1180000_radiant_never_a_traps_hit() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": {
                "hand": [FILLER],
                "field": [{ "def": MAGE, "radiant": true }],
                "backrow": [{ "def": TRAP, "faceUp": true }],
            },
            "p2": { "hand": [FILLER] },
        }));
        trap_hits_enemy_hero(&mut s, 2);
        s.expect_health(P2, HERO_HEALTH - 2);
    }

    #[test]
    fn never_the_opponents_spells() {
        crate::register_all();
        let mut s = scenario(json!({
            "active": "p2",
            "p1": { "hand": [FILLER], "field": [MAGE] },
            "p2": { "hand": [LUNAR_ECLIPSE, FILLER] },
        }));
        s.play(LUNAR_ECLIPSE, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
        s.expect_health(P1, HERO_HEALTH - 3);
    }

    #[test]
    fn stops_once_it_has_left_the_field() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [TRUE_STRIKE, LUNAR_ECLIPSE, FILLER], "field": [MAGE] },
            "p2": { "hand": [FILLER] },
        }));
        let mage = s.card(MAGE).id.clone();
        // One True Strike at 4 + 3 kills the 4/7; the Lunar Eclipse after deals its own 3.
        s.play(TRUE_STRIKE, json!({ "targets": [{ "pick": "instance", "instanceId": mage }] }));
        s.expect_in_zone(MAGE, "graveyard");
        s.play(LUNAR_ECLIPSE, at_hero());
        s.expect_health(P2, HERO_HEALTH - 3);
    }

    #[test]
    fn r386_a_nerf_moves_its_spell_damage() {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "hand": [LUNAR_ECLIPSE, FILLER], "field": [MAGE] }, "p2": { "hand": [FILLER] } }));
        let id = s.card(MAGE).id.clone();
        find_instance_mut(s.state_mut(), &id).expect("the mage on the field").tuning =
            Some(json_as(json!({ "x": { "Spell Damage": -1 } })));
        assert!(s.stats(MAGE).keywords.contains(&Keyword::SpellDamage { n: 2 }));
        s.play(LUNAR_ECLIPSE, at_hero());
        s.expect_health(P2, HERO_HEALTH - 5);
    }

    #[test]
    fn radiant_spell_damage_6() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [LUNAR_ECLIPSE, FILLER], "field": [{ "def": MAGE, "radiant": true }] },
            "p2": { "hand": [FILLER] },
        }));
        s.play(LUNAR_ECLIPSE, at_hero());
        s.expect_health(P2, HERO_HEALTH - 9);
    }
}
