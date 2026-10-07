//! C+ #59 All Purpose Apple (SPEC §8.7 row 59). (1) Spell, Fruit, Rare.
//!   Base:    "Summon a Rush Token. Heal your hero {heal}. Deal {damage} damage." — heal 2, damage 1
//!   Radiant: "Summon a Radiant Rush Token. Heal your hero {heal}. Deal {damage} damage." — 4, 2
//!   Engine:  "Resolves in the order written. The damage's target, any unit or hero, is declared at play
//!            (R81); a full board summons nothing and the rest still happens. Tunes: heal 2 ↑; damage 1 ↑."
//!
//! The token takes the leftmost free unit zone (R64) and fizzles on a full row (§3.2); a hero heal has
//! no cap (§3, R19); the hit is one §4.4 instance from this Spell, which Spell Damage raises (E6).

use jackioh_engine::effects::{damage, heal, summon};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-059";

/// TS `cardDef("core-t-rush").id`.
const RUSH_TOKEN: &str = "core-t-rush";

/// §8 Conventions: "Deal N damage" with no target named is targeted — any unit or hero, either side.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "hero"] }))]
}

fn apple(radiant: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            vec![
                summon(json_as(json!({ "defId": RUSH_TOKEN, "radiant": radiant }))),
                heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": param(&*ctx, "heal") }))),
                damage(json_as(json!({ "to": { "of": "chosen" }, "amount": param(&*ctx, "damage") }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: apple(false),
        radiant: apple(true),
    }
}

// C+ #59 All Purpose Apple — SPEC §8.7 row 59, BUILD M9 Classic+ row C+ 59: "Summons a Rush Token (3/3
// Rush) into your leftmost open zone (none on a full board, the rest still resolving), heals your hero
// 2 (past 30 allowed) and deals 1 damage to a target chosen with the play, any Unit or hero, which
// Spell Damage raises; heal and damage read through `param()`; radiant a Radiant Rush Token, heal 4,
// 2 damage".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const APPLE: &str = "classicplus-059";
    const RUSH: &str = "core-t-rush";
    const VANILLA: &str = "core-008"; // Mr. Vanilla 4/4
    const SOLARIUS: &str = "classicplus-038"; // a Unit printing Spell Damage +2
    const FILLER: &str = "core-005";

    /// `_harness.ts` registers every card on import; the engine's testkit cannot, so this does.
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    /// TS `stepParam(s.card(ref), key, steps)`: TS stepped the live card; here the card under its id.
    fn step(s: &mut Scenario, card: &str, key: &str, steps: i32) {
        let id = s.card(card).id.clone();
        let live = find_instance_mut(s.state_mut(), &id).expect("the card is in the state");
        step_param(live, key, steps);
    }

    /// TS `s.unit(p, lane) ?? ""`: the unit's instance id, or "" (which no card answers to).
    fn unit_id(s: &Scenario, player: PlayerId, lane: i32) -> String {
        s.unit(player, lane).map(|card| card.id).unwrap_or_default()
    }

    /// TS `ENEMY_HERO: Selection[]`.
    fn enemy_hero() -> Value {
        json!([{ "pick": "hero", "player": "p2" }])
    }

    /// TS `apple({ radiant?, field?, health? })`.
    fn apple(radiant: bool, field: Value, health: Option<i32>) -> Scenario {
        let mut p1 = json!({ "hand": [{ "def": APPLE, "radiant": radiant }, FILLER], "field": field });
        if let Some(health) = health {
            p1["health"] = json!(health);
        }
        scenario(json!({ "p1": p1, "p2": { "hand": [FILLER], "field": [VANILLA] } }))
    }

    #[test]
    fn is_a_1_spell_fruit_that_declares_one_target_any_unit_or_hero() {
        crate::register_all();
        let def = crate::card_def(super::ID);
        assert_eq!(def.id, APPLE);
        assert_eq!(def.tags, vec![Tag::Fruit]);
        assert_eq!(
            serde_json::to_value(&super::script().base.targets).unwrap(),
            json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit", "hero"] } }])
        );
    }

    mod base {
        use super::*;

        #[test]
        fn summons_a_rush_token_heals_your_hero_2_then_deals_1_damage_in_that_order() {
            let mut s = apple(false, json!([]), Some(20));
            s.play(APPLE, json!({ "targets": enemy_hero() }));
            let token = s.unit(PlayerId::P1, 1);
            assert_eq!(token.as_ref().map(|card| card.def_id.as_str()), Some(RUSH));
            assert_eq!(token.as_ref().map(|card| card.radiant), Some(false));
            let id = token.map(|card| card.id).unwrap_or_default();
            s.expect_stats(&id, json!({ "attack": 3, "health": 3 }));
            s.expect_health(PlayerId::P1, 22).expect_health(PlayerId::P2, 29);
            s.expect_events(json!(["summoned", "healed", "damage"]));
        }

        #[test]
        fn r19_the_heal_goes_past_30() {
            let mut s = apple(false, json!([]), None);
            s.play(APPLE, json!({ "targets": enemy_hero() }));
            s.expect_health(PlayerId::P1, 32);
        }

        #[test]
        fn r81_the_damage_may_hit_any_unit_yours_included() {
            let mut s = apple(false, json!([VANILLA]), None);
            let own = unit_id(&s, PlayerId::P1, 1);
            s.play(APPLE, json!({ "targets": [{ "pick": "instance", "instanceId": own }] }));
            let unit = unit_id(&s, PlayerId::P1, 1);
            s.expect_stats(&unit, json!({ "health": 3 }));
            assert_eq!(s.unit(PlayerId::P1, 2).map(|card| card.def_id), Some(RUSH.to_string()));
        }

        #[test]
        fn r64_the_token_takes_your_leftmost_open_zone() {
            let mut s = apple(false, json!([VANILLA]), None);
            s.play(APPLE, json!({ "targets": enemy_hero() }));
            assert_eq!(s.unit(PlayerId::P1, 2).map(|card| card.def_id), Some(RUSH.to_string()));
        }

        #[test]
        fn s3_2_a_full_board_summons_nothing_and_the_rest_still_resolves() {
            let mut s = apple(false, json!([VANILLA, VANILLA, VANILLA, VANILLA, VANILLA]), Some(20));
            s.play(APPLE, json!({ "targets": enemy_hero() }));
            assert!(!s.last_events().iter().any(|event| matches!(event, GameEvent::Summoned { .. })));
            s.expect_health(PlayerId::P1, 22).expect_health(PlayerId::P2, 29);
        }

        #[test]
        fn e6_spell_damage_raises_the_hit_spell_damage_2_3() {
            let mut s = apple(false, json!([SOLARIUS]), None);
            s.play(APPLE, json!({ "targets": enemy_hero() }));
            s.expect_health(PlayerId::P2, 27);
        }

        #[test]
        fn r386_an_upgrade_heals_3_and_deals_2_a_degrade_never_takes_either_below_1() {
            let mut up = apple(false, json!([]), Some(20));
            step(&mut up, APPLE, "heal", 1);
            step(&mut up, APPLE, "damage", 1);
            up.play(APPLE, json!({ "targets": enemy_hero() }));
            up.expect_health(PlayerId::P1, 23).expect_health(PlayerId::P2, 28);

            let mut down = apple(false, json!([]), Some(20));
            step(&mut down, APPLE, "heal", -5);
            step(&mut down, APPLE, "damage", -5);
            down.play(APPLE, json!({ "targets": enemy_hero() }));
            down.expect_health(PlayerId::P1, 21).expect_health(PlayerId::P2, 29);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn s7_a_radiant_rush_token_6_6_rush_cleave_heal_4_and_2_damage() {
            let mut s = apple(true, json!([]), Some(20));
            let enemy = unit_id(&s, PlayerId::P2, 1);
            s.play(APPLE, json!({ "targets": [{ "pick": "instance", "instanceId": enemy }] }));
            let token = s.unit(PlayerId::P1, 1);
            assert_eq!(token.as_ref().map(|card| card.def_id.as_str()), Some(RUSH));
            assert_eq!(token.as_ref().map(|card| card.radiant), Some(true));
            let id = token.map(|card| card.id).unwrap_or_default();
            s.expect_stats(&id, json!({ "attack": 6, "health": 6 }));
            s.expect_health(PlayerId::P1, 24);
            let theirs = unit_id(&s, PlayerId::P2, 1);
            s.expect_stats(&theirs, json!({ "health": 2 }));
        }

        #[test]
        fn r386_the_radiant_numbers_step_from_4_and_2() {
            let mut s = apple(true, json!([]), Some(20));
            step(&mut s, APPLE, "heal", 1);
            step(&mut s, APPLE, "damage", 1);
            s.play(APPLE, json!({ "targets": enemy_hero() }));
            s.expect_health(PlayerId::P1, 25).expect_health(PlayerId::P2, 27);
        }
    }
}
