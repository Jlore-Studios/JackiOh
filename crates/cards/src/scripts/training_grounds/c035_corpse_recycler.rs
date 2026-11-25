//! TG #35 Corpse Recycler (SPEC §8.11 row 35, R2199006, R3837005, R4423000). (3) Field Spell, Epic. "After a
//! non-Token Unit dies, summon a Rush Token for you." (Radiant: a Radiant Rush Token.)
//! A trigger on `destroyed`, as C+ #19.5 Bot Loser's: a Field Spell's, so both players' Units answer it
//! (R2199006). Since issue #3837 the token goes to the Recycler's controller whoever's Unit died
//! (R3837005); through v0.6.2c it went to the Unit's controller as it died (R172), which `script_v0_6_2c`
//! keeps for those replays (R767000). It lands in R64's leftmost open zone (the event carries no lane,
//! R89), none on a full row (§3.2); a Token's death gives nothing (§7). R4423000: a Unit's death is read by
//! what the card was on the field as it died (`is_unit_death`), so a Unit set face-down by its Cloaked and
//! destroyed before it reveals gives nothing and an Animated Field Spell or Trap killed in a unit zone gives
//! a token; `script_v0_6_9i` keeps the definition's type for v0.6.9l and before (R767000).

use jackioh_engine::catalog::def_of;
use jackioh_engine::effects::summon;
use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-035";

/// §7: the Rush Token, 3/3 Rush (Radiant 6/6 Rush, Cleave).
const RUSH_TOKEN: &str = "core-t-rush";

/// A non-Token Unit's death (R2199006): the player who controlled it as it died (R172), or nothing. A Unit
/// by what the card was as it died (R4423000), or `by_print` by its definition, as v0.6.9l and before.
fn died_unit(ctx: &EffectContext<'_>, event: &GameEvent, by_print: bool) -> Option<PlayerId> {
    let GameEvent::Destroyed { def_id, controller, .. } = event else {
        return None;
    };
    let def = def_of(Some(&*ctx.state), def_id);
    let unit = if by_print {
        def.type_ == CardType::Unit
    } else {
        is_unit_death(&*ctx.state, event)
    };
    (unit && !def.is_token()).then_some(*controller)
}

/// The condition is written twice, as Core #18's: a queued trigger takes an empty list as "not met".
/// `for_you`: the token is the Recycler's controller's (R3837005), not the dead Unit's (R172).
/// `by_print`: as `died_unit`'s.
fn recycler(radiant: bool, for_you: bool, by_print: bool) -> Script {
    let trigger = TriggerDef::new("corpse-recycler", &[GameEventType::Destroyed], move |ctx, event| {
        let Some(controller) = died_unit(ctx, event, by_print) else {
            return vec![];
        };
        let side = if for_you || controller == ctx.controller { "self" } else { "enemy" };
        vec![summon(json_as(json!({ "defId": RUSH_TOKEN, "player": side, "radiant": radiant })))]
    })
    .with_when(move |ctx, event| died_unit(ctx, event, by_print).is_some());
    Script {
        triggers: vec![trigger],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: recycler(false, true, false),
        radiant: recycler(true, true, false),
    }
}

/// R767000: a Unit's death read by the dead card's definition (R4423000), through v0.6.9l; `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_6_9i() -> CardScripts {
    CardScripts {
        base: recycler(false, true, true),
        radiant: recycler(true, true, true),
    }
}

/// R767000: the token for the dead Unit's controller (R172), through v0.6.2c; `KEPT_SCRIPTS` folds those
/// replays under it.
pub fn script_v0_6_2c() -> CardScripts {
    CardScripts {
        base: recycler(false, false, true),
        radiant: recycler(true, false, true),
    }
}

// TG #35 Corpse Recycler — SPEC §8.11 row 35, BUILD M13 row TG 35.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const RECYCLER: &str = super::ID;
    const RUSH: &str = "core-t-rush";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const SPECIESIST: &str = "core-002"; // Speciesist
    const TIMMY: &str = "stand-in (1) 3/3 Rush, First Strike R6/6 Charge, First Strike";
    const BIG_FELINOR: &str = "core-043"; // Big Felinor: Cry: destroy all non-Felinor Units
    const COLLATERAL: &str = "core-034"; // Collateral Damage: exiles a permanent
    const HIT_JOB: &str = "core-016";

    fn board(radiant: bool, p1: Value, p2: Value) -> Scenario {
        crate::register_all();
        let mut p1 = p1;
        p1["backrow"] = json!([{ "def": RECYCLER, "radiant": radiant, "faceUp": true, "lane": 1 }]);
        if p1.get("mana").is_none() {
            p1["mana"] = json!(20);
        }
        scenario(json!({ "seed": "corpse-recycler", "p1": p1, "p2": p2 }))
    }

    fn units(s: &Scenario, player: PlayerId) -> Vec<Option<String>> {
        (1..=5).map(|lane| s.unit(player, lane).map(|card| card.def_id)).collect()
    }

    fn tokens(s: &Scenario, player: PlayerId) -> usize {
        units(s, player).iter().filter(|def| def.as_deref() == Some(RUSH)).count()
    }

    mod base {
        use super::*;

        #[test]
        fn r2199006_your_non_token_unit_dies_and_you_get_a_rush_token() {
            let mut s = board(false, json!({ "field": [{ "def": SPECIESIST, "lane": 3 }] }), json!({ "field": [VANILLA] }));
            let speciesist = s.unit(P1, 3).unwrap();
            let vanilla = s.unit(P2, 1).unwrap();
            s.attack(&speciesist, &vanilla);

            // R64: the leftmost open zone, not the zone it died in.
            assert_eq!(units(&s, P1)[0].as_deref(), Some(RUSH));
            assert!(s.unit(P1, 3).is_none());
            s.expect_stats(RUSH, json!({ "attack": 3, "health": 3 }));
        }

        #[test]
        fn r3837005_an_enemy_unit_dies_and_you_get_the_token() {
            // Timmy's First Strike kills the Speciesist before it strikes back.
            let mut s = board(false, json!({ "field": [TIMMY] }), json!({ "field": [SPECIESIST] }));
            let vanilla = s.unit(P1, 1).unwrap();
            let speciesist = s.unit(P2, 1).unwrap();
            s.attack(&vanilla, &speciesist);

            // "For you": the Recycler's controller's, in their leftmost open zone (R64).
            assert_eq!(tokens(&s, P1), 1);
            assert_eq!(units(&s, P1)[1].as_deref(), Some(RUSH));
            assert_eq!(tokens(&s, P2), 0);
        }

        #[test]
        fn r767000_the_kept_v0_6_2c_script_gives_the_enemy_its_token() {
            crate::under_kept(RECYCLER, super::super::script_v0_6_2c(), || {
                let mut s = board(false, json!({ "field": [TIMMY] }), json!({ "field": [SPECIESIST] }));
                let vanilla = s.unit(P1, 1).unwrap();
                let speciesist = s.unit(P2, 1).unwrap();
                s.attack(&vanilla, &speciesist);

                assert_eq!(tokens(&s, P2), 1);
                assert_eq!(tokens(&s, P1), 0);
            });
        }

        #[test]
        fn r2199006_a_token_dying_gives_nothing() {
            let mut s = board(false, json!({ "field": [VANILLA] }), json!({ "field": [{ "def": RUSH, "statsOverride": { "attack": 1, "health": 1 } }] }));
            let vanilla = s.unit(P1, 1).unwrap();
            let token = s.unit(P2, 1).unwrap();
            s.attack(&vanilla, &token);

            assert_eq!(tokens(&s, P2), 0);
            assert!(!s.events().iter().any(|e| matches!(e, GameEvent::Summoned { .. })));
        }

        #[test]
        fn r64_a_full_row_summons_nothing() {
            // The Speciesist tops a Stack pile (§3.2): the Vanilla beneath resumes, so the row stays full.
            let mut s = board(
                false,
                json!({ "field": [VANILLA, { "def": SPECIESIST, "stack": true }, VANILLA, VANILLA, VANILLA, VANILLA] }),
                json!({ "field": [VANILLA] }),
            );
            let speciesist = s.unit(P1, 1).unwrap();
            assert_eq!(speciesist.def_id, SPECIESIST);
            let target = s.unit(P2, 1).unwrap();
            s.attack(&speciesist, &target);

            s.expect_in_zone(SPECIESIST, "graveyard");
            assert_eq!(tokens(&s, P1), 0);
            assert!(units(&s, P1).iter().all(|def| def.as_deref() == Some(VANILLA)));
        }

        #[test]
        fn r68_several_deaths_in_one_check_give_one_token_each() {
            let mut s = board(
                false,
                json!({ "hand": [BIG_FELINOR, HIT_JOB], "field": [{ "def": VANILLA, "lane": 4 }] }),
                json!({ "field": [VANILLA, TIMMY] }),
            );
            s.play(BIG_FELINOR, json!({ "zone": 1 }));

            // Three non-Felinor Units died: three tokens, every one p1's (R3837005).
            assert_eq!(tokens(&s, P1), 3);
            assert_eq!(tokens(&s, P2), 0);

            // R68: death triggers run in side then lane order, the active player's first.
            let p1_vanilla = |e: &GameEvent| matches!(e, GameEvent::Destroyed { controller: P1, def_id, .. } if def_id == VANILLA);
            let deaths: Vec<(PlayerId, String)> = s
                .events()
                .iter()
                .filter_map(|e| match e {
                    GameEvent::Destroyed { controller, def_id, .. } if def_id != BIG_FELINOR => Some((*controller, def_id.clone())),
                    _ => None,
                })
                .collect();
            assert_eq!(deaths, vec![(P1, VANILLA.to_string()), (P2, VANILLA.to_string()), (P2, TIMMY.to_string())]);
            let summons: Vec<(PlayerId, i32)> = s
                .events()
                .iter()
                .skip_while(|e| !p1_vanilla(e))
                .filter_map(|e| match e {
                    GameEvent::Summoned { player, def_id, lane, .. } if def_id == RUSH => Some((*player, *lane)),
                    _ => None,
                })
                .collect();
            // Each in p1's leftmost open zone: lane 2 (the Felinor in 1), then 3, then 4 (its Vanilla's).
            assert_eq!(summons, vec![(P1, 2), (P1, 3), (P1, 4)]);
        }

        #[test]
        fn r153_once_it_is_gone_nothing_happens() {
            let mut s = board(false, json!({ "hand": [COLLATERAL, HIT_JOB], "field": [SPECIESIST] }), json!({ "field": [VANILLA] }));
            let recycler = s.card(RECYCLER).clone();
            s.play(COLLATERAL, json!({ "targets": [{ "pick": "instance", "instanceId": recycler.id }] }));
            let speciesist = s.unit(P1, 1).unwrap();
            let vanilla = s.unit(P2, 1).unwrap();
            s.attack(&speciesist, &vanilla);

            assert_eq!(tokens(&s, P1), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r74_a_radiant_rush_token_6_6_rush_cleave() {
            let mut s = board(true, json!({ "field": [SPECIESIST] }), json!({ "field": [VANILLA] }));
            let speciesist = s.unit(P1, 1).unwrap();
            let vanilla = s.unit(P2, 1).unwrap();
            s.attack(&speciesist, &vanilla);

            let token = s.unit(P1, 1).expect("a token");
            assert_eq!(token.def_id, RUSH);
            assert!(token.radiant);
            s.expect_stats(RUSH, json!({ "attack": 6, "health": 6 }));
        }
    }

    /// R4423000: a death gives a token by what the card was on the field as it died.
    mod r4423000_what_it_died_as {
        use super::*;

        const MAGIC_JAMMED: &str = "core-036";
        const FILLER: &str = "core-005";

        /// p1 sets a Cloaked Toe Cracker face-down and destroys it with Magic Jammed before it reveals,
        /// its Recycler face-up in backrow lane 1; the set card's id.
        fn jammed() -> (Scenario, String) {
            let mut s = board(
                false,
                json!({ "hand": [crate::CLOAKED_UNIT, MAGIC_JAMMED, FILLER], "mana": 10 }),
                json!({ "hand": [FILLER] }),
            );
            let set = crate::set_cloaked(&mut s, P1, crate::CLOAKED_UNIT, 2);
            s.play(MAGIC_JAMMED, json!({ "targets": pick::id(&set) }));
            (s, set)
        }

        #[test]
        fn r4423000_a_set_cloaked_unit_destroyed_before_it_reveals_is_no_unit_death_for_it() {
            let (s, set) = jammed();
            assert_eq!(crate::died_as_of(&s, &set), Some(CardType::FieldTrap));
            assert_eq!(tokens(&s, P1), 0);
            assert_eq!(tokens(&s, P2), 0);
        }

        /// p1's set Catnip animates after p2's Spell, and p2's Hit Job kills it in its unit zone.
        #[test]
        fn r4423000_an_animated_field_trap_killed_in_a_unit_zone_is_a_unit_death_for_it() {
            let mut s = scenario(json!({
                "seed": "corpse-recycler",
                "active": "p2",
                "p1": {
                    "hand": [FILLER],
                    "backrow": [
                        { "def": RECYCLER, "faceUp": true, "lane": 1 },
                        { "def": crate::ANIMATED_TRAP, "faceUp": false, "lane": 2 },
                    ],
                },
                "p2": { "hand": [FILLER, HIT_JOB, FILLER], "mana": 10 },
            }));
            s.play(FILLER, json!({}));
            let catnip = s.card(crate::ANIMATED_TRAP).id.clone();
            assert!(s.unit(P1, 2).is_some_and(|unit| unit.id == catnip), "Catnip animated");
            s.play(HIT_JOB, json!({ "targets": pick::id(&catnip) }));

            assert_eq!(crate::died_as_of(&s, &catnip), Some(CardType::Unit));
            assert_eq!(tokens(&s, P1), 1);
            assert_eq!(tokens(&s, P2), 0);
        }

        /// R767000: v0.6.9l and before gave a token for the set card's death, by the definition's type.
        #[test]
        fn r767000_the_script_kept_for_v0_6_9i_counts_a_set_cloaked_unit_s_death() {
            crate::under_kept(RECYCLER, super::super::script_v0_6_9i(), || {
                let (s, _) = jammed();
                assert_eq!(tokens(&s, P1), 1);
                assert_eq!(tokens(&s, P2), 0);
            });
        }
    }
}
