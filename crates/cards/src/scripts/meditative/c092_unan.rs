//! M #92 Unan (SPEC §8.8 row 92): (3) Unit, Rare, 3/13 with Armor 3 → 6/26 with Armor 7.
//!
//! Base:    "Armor 3\nLethal damage your hero or your other Units would take is redirected to this."
//! Radiant: "Armor 7\nLethal damage your hero or your other Units would take is redirected to this."
//! Engine: `ReplacementDef { id: "92-lethal-guard", on: LethalHit, instead: redirect SelfCard }`
//! (ME-LETHALGUARD, R1204): §4.4 step 4a opens for Units too, so a fatal hit on a friendly ally
//! goes to Unan through Unan's own Armor, each Unan catching a given hit once. Armor is catalog
//! data; there is nothing else to run.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-092";

pub fn script() -> CardScripts {
    // Both faces catch the same way; the Radiant face's Armor is catalog data beside it.
    let base = Script {
        replacements: vec![ReplacementDef {
            id: "92-lethal-guard".to_string(),
            on: ReplacementMoment::LethalHit,
            where_: None,
            when: None,
            instead: ReplacementInstead {
                redirect: Some(InsteadRedirect::SelfCard),
                ..Default::default()
            },
            then: None,
            by: None,
        }],
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// M #92 Unan — SPEC §8.8 row 92, BUILD M10 row M 92: "§4.4 step 4a opens for Units as well as heroes,
// so a fatal hit on a friendly ally goes to Unan through Unan's own Armor, each Unan catching a
// given hit once (Covers R1204); the Radiant face is 6/26 with Armor 7".
//
// The guard is proved in `crates/engine/tests/rules/mb22.rs` (R1204); these tests prove the card's
// own faces carry it.
#[cfg(test)]
mod tests {
    use super::*;

    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const UNAN: &str = "meditative-092";
    const ALLY: &str = "core-011"; // A plain 3/3 Unit to guard.
    const STRIKER: &str = "classic-049"; // A plain 9/9 Unit to strike with.
    const VITAL_KILL: &str = "classic-029"; // Sets a declared hero's health to 13.
    const HEAVY: &str = "classic-086"; // A plain 14/14 Unit.
    const FILLER: &str = "core-005";
    const BURN: &str = "classic-036"; // (0) Spell: deal 2 damage to any unit or hero.

    mod m92_unan {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn is_a_rare_3_cost_3_13_with_armor_3_and_the_guard() {
                crate::register_all();
                let def = crate::card_def(ID);
                assert_eq!(def.cost, CardCost::Fixed(3));
                assert_eq!(def.rarity, Rarity::Rare);
                assert_eq!(
                    [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
                    [Some(3), Some(13), Some(6), Some(26)]
                );
                assert_eq!(def.base.keywords, vec![Keyword::Armor { n: 3 }]);
                assert_eq!(
                    def.base.text,
                    "Armor 3\nLethal damage your hero or your other Units would take is redirected to this."
                );
                let guard = &registered_scripts()[ID].base.replacements;
                assert_eq!(guard.len(), 1);
                assert_eq!(guard[0].id, "92-lethal-guard");
                assert_eq!(guard[0].on, ReplacementMoment::LethalHit);
                assert_eq!(
                    guard[0].instead.redirect,
                    Some(InsteadRedirect::SelfCard)
                );
            }

            #[test]
            fn r1204_a_lethal_hit_on_an_ally_reaches_the_guard_through_its_armor() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [UNAN, ALLY] },
                    "p2": { "hand": [FILLER], "field": [STRIKER] },
                }));
                s.end_turn();
                // 9 damage on the 3-health ally is lethal …
                s.attack(STRIKER, ALLY);
                // … so Unan takes it instead, through its Armor 3: 9 − 3.
                assert_eq!(s.stats(ALLY).health, 3, "the ally is untouched");
                assert_eq!(s.stats(UNAN).health, 13 - 6);
                assert!(
                    s.events().iter().any(|event| matches!(
                        event,
                        GameEvent::Redirected { .. }
                    )),
                    "a redirect was emitted"
                );
            }

            #[test]
            fn r1204_a_survivable_hit_is_not_caught() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [UNAN, ALLY] },
                    "p2": { "hand": [BURN, FILLER] },
                }));
                s.end_turn();
                // 2 damage on the 3-health ally is survivable: no window opens.
                let ally = s.card(ALLY).id.clone();
                s.play(
                    BURN,
                    json!({ "targets": [{ "pick": "instance", "instanceId": ally }] }),
                );
                assert_eq!(s.stats(ALLY).health, 1);
                assert_eq!(s.stats(UNAN).health, 13);
            }

            #[test]
            fn r1204_enemy_units_are_not_guarded() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [UNAN, STRIKER] },
                    "p2": { "hand": [FILLER], "field": [ALLY] },
                }));
                // 9 damage on the enemy 3-health ally is lethal, but it is no friendly ally.
                s.attack(STRIKER, ALLY);
                s.expect_in_zone(ALLY, "graveyard");
                assert_eq!(s.stats(UNAN).health, 13);
            }

            #[test]
            fn r1204_the_hero_is_guarded() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [UNAN] },
                    "p2": { "hand": [VITAL_KILL, FILLER], "field": [HEAVY] },
                }));
                s.end_turn();
                s.play(
                    VITAL_KILL,
                    json!({ "targets": [{ "pick": "hero", "player": "p1" }] }),
                );
                // 14 damage on the 13-health hero is lethal, so Unan takes it through Armor 3.
                s.attack(HEAVY, "hero");
                s.expect_health(P1, 13);
                assert_eq!(s.stats(UNAN).health, 13 - 11);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r1204_radiant_is_6_26_with_armor_7_and_guards_through_it() {
                crate::register_all();
                let def = crate::card_def(ID);
                assert_eq!(def.radiant.keywords, vec![Keyword::Armor { n: 7 }]);
                assert_eq!(
                    def.radiant.text,
                    "Armor 7\nLethal damage your hero or your other Units would take is redirected to this."
                );
                assert_eq!(
                    registered_scripts()[ID].radiant.replacements.len(),
                    1,
                    "the Radiant face guards the same way"
                );
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": UNAN, "radiant": true }, ALLY] },
                    "p2": { "hand": [FILLER], "field": [STRIKER] },
                }));
                s.end_turn();
                s.attack(STRIKER, ALLY);
                assert_eq!(s.stats(ALLY).health, 3, "the ally is untouched");
                assert_eq!(s.stats(UNAN).health, 26 - (9 - 7));
            }
        }
    }
}
