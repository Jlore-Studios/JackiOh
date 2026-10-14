//! #35 Lunar Eclipse (SPEC §8.2 row 35): "Deal 3 damage to a target; the next Spell you play this
//! turn costs 1 less", radiant "6 damage; 2 less".
//!
//! The radiant cell changes only the two numbers, so both clauses stay (§8 Conventions).
//! R81: the target travels in the `play` action and resolution never pauses.
//!
//! The discount is a player-level `PlayerModifier`. `onlyType: "Spell"` keeps a unit play from using
//! it; `{ until: "used" }` would leak past cleanup (R30 needs #79 Twinspell's rider to survive it),
//! so the expiry stays `thisTurn` (§2.2 names this card) and `oncePerTurn` consumes it on the first
//! Spell (§8.2 Engine cell), which `consume_used_discounts` reads as a second "consumed on use":
//!
//! ```text
//! if (mod.kind !== "costDiscount") continue;
//! if (mod.expiry.until !== "used" && mod.oncePerTurn !== true) continue;
//! ```

use jackioh_engine::effects::{add_player_modifier, damage};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-035";

/// R81: "target" is any unit or hero (§8 Conventions), chosen with the play.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "hero"] }))]
}

/// 3 damage and −1, or 6 and −2: the declared numbers `damage` and `discount` (R386), read off the
/// face that is running, so one hook serves both faces.
fn eclipse() -> Hook {
    hook(|ctx| {
        vec![
            damage(json_as(json!({ "to": { "of": "chosen" }, "amount": param(&*ctx, "damage") }))),
            add_player_modifier(json_as(json!({
                "player": "self",
                "mod": {
                    "kind": "costDiscount",
                    "amount": param(&*ctx, "discount"),
                    // "the next Spell you play": Spells only, and only the next one.
                    "onlyType": "Spell",
                    "oncePerTurn": true,
                    // §2.2: cleanup takes it if no Spell used it.
                    "expiry": { "until": "thisTurn", "turn": ctx.state.turn },
                },
            }))),
        ]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(eclipse()),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// #35 Lunar Eclipse — SPEC §8.2 row 35, BUILD M4-T4 must-pass row 35:
// "3 damage; next spell this turn −1; a unit play does not consume it; expires at cleanup;
//  radiant 6 / −2".
//
// The second spell is #16 Hit Job, a (3) Cost Spell: at full price p1 would be left with 0 mana,
// with the discount 1 (and 2 with the radiant −2).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    /// Registers the catalog first: the engine's testkit cannot name the cards crate.
    fn scn(opts: Value) -> Scenario {
        crate::register_all();
        scenario(opts)
    }

    fn must(card: Option<CardInstance>, what: &str) -> CardInstance {
        card.unwrap_or_else(|| panic!("the scenario has no {what}"))
    }

    fn at_enemy_hero() -> Value {
        json!([{ "pick": "hero", "player": "p2" }])
    }

    /// `{ targets: [{ pick: "instance", instanceId }] }` for the unit in that lane.
    fn at_unit(s: &Scenario, seat: &str, lane: i32, what: &str) -> Value {
        let unit = must(s.unit(seat, lane), what);
        json!({ "targets": [{ "pick": "instance", "instanceId": unit.id }] })
    }

    /// p1 holds the eclipse, a (3) Cost spell and a (1) Cost unit; both sides keep a unit on the board.
    fn board() -> Scenario {
        scn(json!({
            "seed": "lunar",
            "p1": { "hand": ["35", "16", "15"], "field": ["43"], "library": ["15", "15", "15"] },
            "p2": { "field": ["15"], "library": ["15", "15", "15"] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn deals_3_damage_to_the_target_through_the_sec4_4_pipeline() {
            let mut s = board();
            s.play("35", json!({ "targets": at_enemy_hero() }));
            s.expect_health("p2", 27);
            s.expect_events(json!(["cardPlayed", "damage"]));
        }

        #[test]
        fn deals_its_3_damage_to_a_unit_target_as_well() {
            let mut s = board();
            let prey = must(s.unit("p2", 1), "p2 lane 1");
            s.play("35", json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));
            // A 1/1 dies to 3.
            s.expect_in_zone(&prey, "graveyard");
        }

        #[test]
        fn the_next_spell_this_turn_costs_1_less() {
            let mut s = board();
            s.expect_mana("p1", 4);

            s.play("35", json!({ "targets": at_enemy_hero() }));
            s.expect_mana("p1", 3);

            let hit_job = at_unit(&s, "p2", 1, "p2 lane 1");
            s.play("16", hit_job);
            // Hit Job's printed 3, less 1.
            s.expect_mana("p1", 1);
        }

        #[test]
        fn a_unit_play_does_not_consume_the_discount() {
            let mut s = board();
            s.play("35", json!({ "targets": at_enemy_hero() }));
            s.expect_mana("p1", 3);

            // #15 is a Unit, so `onlyType: "Spell"` leaves it at its printed 1 and the discount stands.
            s.play("15", json!({}));
            s.expect_mana("p1", 2);

            // Hit Job's printed 3, less the 1 the unit play left standing.
            let hit_job = at_unit(&s, "p2", 1, "p2 lane 1");
            s.play("16", hit_job);
            s.expect_mana("p1", 0);
        }

        #[test]
        fn only_the_next_spell_is_cheaper_not_every_spell_this_turn() {
            let mut s = scn(json!({
                "seed": "lunar-one-spell",
                "p1": { "hand": ["35", "16", "16"], "field": ["43"], "library": ["15", "15", "15"], "mana": 6 },
                "p2": { "field": ["15", "15"], "library": ["15", "15", "15"] },
            }));

            s.play("35", json!({ "targets": at_enemy_hero() }));
            s.expect_mana("p1", 5);
            let first = at_unit(&s, "p2", 1, "p2 lane 1");
            s.play("16", first);
            s.expect_mana("p1", 3);
            // The discount was consumed by the first Spell, so this one pays its printed 3.
            let second = at_unit(&s, "p2", 2, "p2 lane 2");
            s.play("16", second);
            s.expect_mana("p1", 0);
        }

        #[test]
        fn the_discount_expires_at_cleanup() {
            let mut s = board();
            s.play("35", json!({ "targets": at_enemy_hero() }));
            assert_eq!(s.state().players.p1.mods.len(), 1);

            s.end_turn();

            // §2.2: cleanup expires every "this turn" effect, the Lunar Eclipse discount by name.
            assert_eq!(s.state().players.p1.mods.len(), 0);
        }

        #[test]
        fn a_spell_on_a_later_turn_pays_full_price() {
            let mut s = board();
            s.play("35", json!({ "targets": at_enemy_hero() }));

            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().active, PlayerId::P1);

            let mana = s.state().players.p1.mana.current;
            let hit_job = at_unit(&s, "p2", 1, "p2 lane 1");
            s.play("16", hit_job);
            s.expect_mana("p1", mana - 3);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn radiant_deals_6_damage() {
            let mut s = board();
            s.card_mut("35").radiant = true;
            s.play("35", json!({ "targets": at_enemy_hero() }));
            s.expect_health("p2", 24);
        }

        #[test]
        fn radiant_makes_the_next_spell_this_turn_cost_2_less() {
            let mut s = board();
            s.card_mut("35").radiant = true;

            s.play("35", json!({ "targets": at_enemy_hero() }));
            s.expect_mana("p1", 3);

            let hit_job = at_unit(&s, "p2", 1, "p2 lane 1");
            s.play("16", hit_job);
            // Hit Job's printed 3, less 2 (R65).
            s.expect_mana("p1", 2);
        }

        #[test]
        fn the_radiant_discount_expires_at_cleanup_too() {
            let mut s = board();
            s.card_mut("35").radiant = true;
            s.play("35", json!({ "targets": at_enemy_hero() }));
            assert_eq!(s.state().players.p1.mods.len(), 1);

            s.end_turn();

            assert_eq!(s.state().players.p1.mods.len(), 0);
        }
    }

    #[test]
    fn r386_an_upgrade_deals_4_and_a_degrade_2() {
        for (upgrade, amount) in [(true, 4), (false, 2)] {
            let mut s = board();
            let moved = if upgrade {
                crate::upgrade_number(&mut s, "35", "damage")
            } else {
                crate::degrade_number(&mut s, "35", "damage")
            };
            assert_eq!(moved, amount);
            s.play("35", json!({ "targets": at_enemy_hero() }));
            s.expect_health("p2", 30 - amount);
        }
    }

    #[test]
    fn r386_an_upgrade_makes_the_discount_2_and_a_radiant_degrade_1() {
        for (radiant, upgrade, discount) in [(false, true, 2), (true, false, 1)] {
            let mut s = board();
            s.card_mut("35").radiant = radiant;
            let moved = if upgrade {
                crate::upgrade_number(&mut s, "35", "discount")
            } else {
                crate::degrade_number(&mut s, "35", "discount")
            };
            assert_eq!(moved, discount);
            s.play("35", json!({ "targets": at_enemy_hero() }));
            s.expect_mana("p1", 3);
            let hit_job = at_unit(&s, "p2", 1, "p2 lane 1");
            s.play("16", hit_job);
            // Hit Job's printed 3, less the discount.
            s.expect_mana("p1", discount);
        }
    }
}
