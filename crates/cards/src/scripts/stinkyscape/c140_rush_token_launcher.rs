//! Stinkyscape #140 Rush Token Launcher (SPEC §8.10 row 140). Unit 0/4 → 0/8, Weapon, cost 2, Rare.
//!   Base:    "Starship Piece, Equip
//!            Activate: Summon a Rush Token that attacks an enemy Unit, Taunt Units first."
//!   Radiant: "Starship Piece, Equip
//!            Activate: Summon a Radiant Rush Token that attacks an enemy Unit, Taunt Units first."
//!
//! fuses into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one
//! (R1852001); played alone, its death fuses a copy into its owner's Dormant Starship (R1852000).
//! R1862003: the Activate's target is a declared enemy Unit, never a hero (R81), once a turn (R384).
//! While the enemy has a Taunt Unit, only those may be chosen (§4.2 step 3 read for the choice): its
//! Taunt read through the layers, so an Indestructible one has none (R347), and one no attack may name
//! (Can't be attacked, a carrier's Unit) binds nobody (B5 E35). Then §7's Rush Token is summoned into
//! the leftmost open unit zone (R64) and makes a forced attack on that Unit (R53), which strikes back,
//! as Core #60 Bear Honeypot's tokens attack; one the token may not attack (§4.2 step 2) takes none.

use jackioh_engine::effects::{forced_attacks, summon};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-140";

/// §7's shared Rush Token, 3/3 with Rush (Radiant 6/6 with Rush and Cleave).
const RUSH_TOKEN: &str = "core-t-rush";

/// The target check that keeps "Taunt Units first".
const TAUNT_FIRST: &str = "rushLauncherTauntFirst";

/// Whether `unit`'s Taunt binds the choice: it has Taunt (R347), and an attack may name it at all.
fn taunts(state: &GameState, unit: &CardInstance) -> bool {
    unit_has(state, unit, KeywordKind::Taunt) && !cannot_be_attacked(state, unit) && !is_carried(state, unit)
}

/// "Taunt Units first": `candidate` binds the choice, or nothing on its side does.
fn taunt_first(state: &GameState, candidate: &CardInstance) -> bool {
    taunts(state, candidate)
        || !active_units_of(state, candidate.controller)
            .into_iter()
            .any(|unit| taunts(state, unit))
}

fn launch(radiant: bool) -> ActivationDecl {
    ActivationDecl {
        id: "rushToken".into(),
        label: "Summon a Rush Token that attacks an enemy Unit".into(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "enemy", "of": ["unit"], "check": TAUNT_FIRST }),
        )],
        modes: vec![],
        can_activate: Some(condition_hook(|c| {
            // R430, R64: room for the token, as #60 Bear Honeypot waits for it; and "attacks an enemy
            // Unit": with none it is not activated, as C #78 Mutate Spell's ability needs a pick.
            !open_zones(c.state, c.controller, Row::Units).is_empty()
                && !active_units_of(c.state, opponent_of(c.controller)).is_empty()
        })),
        has: None,
        run: hook(move |ctx| {
            // A declaration with no legal pick is passed (R81): no Unit chosen, no token.
            if ctx.targets.is_empty() {
                return Vec::new();
            }
            vec![
                summon(json_as(json!({ "defId": RUSH_TOKEN, "radiant": radiant }))),
                // R53: only the token this list summoned, at the chosen Unit.
                forced_attacks(json_as(json!({
                    "attackers": { "side": "self", "defId": RUSH_TOKEN, "summonedThisScript": true },
                    "target": { "spec": { "of": "chosen" } },
                }))),
            ]
        }),
    }
}

fn launcher(radiant: bool) -> Script {
    Script {
        activations: vec![launch(radiant)],
        target_checks: IndexMap::from([(
            TAUNT_FIRST,
            target_check(|args| {
                args.candidate
                    .is_some_and(|candidate| taunt_first(args.state, candidate))
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: launcher(false),
        radiant: launcher(true),
    }
}

// Stinkyscape #140 Rush Token Launcher — SPEC §8.10 row 140, BUILD M12 row S 140 (R53).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hits, weapon_board};
    use jackioh_engine::subsystems::activate::activate_actions_for;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MENACE: &str = "core-019"; // (3) 9/9 Taunt.
    const VANILLA: &str = "core-008"; // (1) 4/4.

    const TOKEN_ATTACK: i32 = 3;
    const RADIANT_TOKEN_ATTACK: i32 = 6;
    const VANILLA_ATTACK: i32 = 4;

    fn at(target: &str) -> Value {
        json!({ "ability": "rushToken", "targets": pick::id(target) })
    }

    fn launcher_on(radiant: bool, p2: Value) -> Scenario {
        weapon_board(
            "s140",
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            p2,
        )
    }

    fn summoned(events: &[GameEvent]) -> Vec<(String, i32)> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned {
                    instance_id,
                    def_id,
                    lane,
                    ..
                } if def_id == RUSH_TOKEN => Some((instance_id.clone(), *lane)),
                _ => None,
            })
            .collect()
    }

    fn activations(s: &Scenario) -> usize {
        activate_actions_for(s.state(), P1, s.card(ID)).len()
    }

    mod base {
        use super::*;

        #[test]
        fn r1862003_the_token_attacks_the_chosen_unit_and_takes_its_strike_back() {
            let mut s = launcher_on(false, json!({ "field": [VANILLA, VANILLA] }));
            let target = s.unit_at(P2, 2).id;
            s.activate(ID, at(&target));
            let tokens = summoned(s.last_events());
            let [(token, lane)] = tokens.as_slice() else {
                panic!("one Rush Token: {tokens:?}");
            };
            // The leftmost open zone (R64): the launcher stands in lane 1.
            assert_eq!(*lane, 2);
            assert_eq!(
                hits(s.last_events()),
                vec![(target.clone(), TOKEN_ATTACK), (token.clone(), VANILLA_ATTACK)]
            );
            s.expect_stats(target.as_str(), json!({ "health": 4 - TOKEN_ATTACK }));
            s.expect_in_zone(token.as_str(), "gone");
            s.expect_health(P2, HERO_HEALTH);
        }

        #[test]
        fn r1862003_a_taunt_unit_is_the_only_choice_while_the_enemy_has_one() {
            let mut s = launcher_on(false, json!({ "field": [VANILLA, MENACE] }));
            let vanilla = s.unit_at(P2, 1).id;
            let menace = s.unit_at(P2, 2).id;
            assert_eq!(activations(&s), 1);
            s.expect_refused(|s| s.activate(ID, at(&vanilla)));
            s.activate(ID, at(&menace));
            assert_eq!(hits(s.last_events())[0], (menace, TOKEN_ATTACK));
        }

        #[test]
        fn r1862003_r347_an_indestructible_taunt_unit_binds_nobody() {
            let mut s = launcher_on(false, json!({ "field": [VANILLA, MENACE] }));
            let vanilla = s.unit_at(P2, 1).id;
            let menace = s.unit_at(P2, 2).id;
            s.card_mut(menace.as_str())
                .granted_keywords
                .push(Keyword::Indestructible);
            assert_eq!(activations(&s), 2);
            s.activate(ID, at(&vanilla));
            assert_eq!(hits(s.last_events())[0], (vanilla, TOKEN_ATTACK));
        }

        #[test]
        fn r1862003_no_hero_may_be_chosen() {
            let at_hero = json!({ "ability": "rushToken", "targets": [{ "pick": "hero", "player": "p2" }] });
            let mut s = launcher_on(false, json!({ "field": [VANILLA] }));
            assert_eq!(activations(&s), 1);
            s.expect_refused(|s| s.activate(ID, at_hero.clone()));
            let mut s = launcher_on(false, json!({}));
            assert_eq!(activations(&s), 0);
            s.expect_refused(|s| s.activate(ID, at_hero.clone()));
            s.expect_refused(|s| s.activate(ID, json!({ "ability": "rushToken", "targets": [] })));
        }

        #[test]
        fn r1862003_with_no_open_unit_zone_it_is_not_activated() {
            let mut s = weapon_board(
                "s140-full",
                json!({ "field": [ID, VANILLA, VANILLA, VANILLA, VANILLA] }),
                json!({ "field": [VANILLA] }),
            );
            let target = s.unit_at(P2, 1).id;
            assert_eq!(activations(&s), 0);
            s.expect_refused(|s| s.activate(ID, at(&target)));
            assert!(summoned(s.events()).is_empty());
        }

        #[test]
        fn r1086_an_equipped_host_takes_the_activate() {
            let mut s = weapon_board(
                "s140-equip",
                json!({ "hand": [ID], "field": [VANILLA] }),
                json!({ "field": [VANILLA] }),
            );
            let host = s.unit_at(P1, 1).id;
            let target = s.unit_at(P2, 1).id;
            s.play(ID, json!({ "zone": 1, "equip": true }));
            s.activate(host.as_str(), at(&target));
            assert_eq!(summoned(s.last_events()).len(), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1862003_the_token_is_a_radiant_rush_token_which_cleaves() {
            let mut s = launcher_on(true, json!({ "field": [VANILLA, VANILLA, VANILLA] }));
            let lanes = [1, 2, 3].map(|lane| s.unit_at(P2, lane).id);
            s.activate(ID, at(&lanes[1]));
            let tokens = summoned(s.last_events());
            let [(token, _)] = tokens.as_slice() else {
                panic!("one Rush Token: {tokens:?}");
            };
            assert!(s.card(token.as_str()).radiant);
            s.expect_stats(token.as_str(), json!({ "attack": RADIANT_TOKEN_ATTACK }));
            let struck: Vec<String> = hits(s.last_events()).into_iter().map(|(id, _)| id).collect();
            for lane in &lanes {
                assert!(struck.contains(lane), "{lane} is hit: {struck:?}");
            }
        }

        #[test]
        fn its_health_is_doubled() {
            let s = launcher_on(true, json!({}));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (0, 8));
        }
    }
}
