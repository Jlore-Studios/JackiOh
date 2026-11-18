//! S #70 Z'Zoth (SPEC §8.10 row 70): (4) Unit, Legendary, 5/7 → 10/14, of the Stinkyscape set.
//!   Base:    "Cry: Summon every Unit with a Death from your graveyard. Trigger their Deaths."
//!   Radiant: "Cry: Summon every Unit with a Death from your graveyard. Give them Reborn. Trigger their
//!            Deaths twice."
//! Engine: Cry (R1), R1869001: C #44 Back from the GY's Radiant summon from the graveyard (`for_each_card`
//! read once as the clause begins, R113), filtered to the Unit cards whose running face prints a Death
//! (`Script.death`; a Death granted on the field is gone in the graveyard, R78), oldest first (§3), each
//! summoned into R64's zone with no Cry; with no room the rest stay in the graveyard. Radiant: each Unit
//! this Cry summoned (`summoned_so_far`, R136) is given Reborn (`grant_keyword`). Issue #4154 then has
//! each Unit it summoned run its Death where it stands, in the order summoned, without dying
//! (`trigger_death`, R4154205); the Radiant face runs that round twice, after the Reborn. "Them" are read
//! once, before any Death runs, so a Unit a Death summons runs none, nor does one an earlier Death took
//! off the field. Through v0.6.6e no Death ran, which `script_v0_6_6e` keeps
//! (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-070";

/// R4154205: how many rounds of the summoned Units' Deaths each face runs (since issue #4154).
const DEATH_ROUNDS: usize = 1;
const RADIANT_DEATH_ROUNDS: usize = 2;

/// Your graveyard's Units whose running face prints a Death, oldest first (§3).
fn graveyard_death_units(ctx: &EffectContext<'_>) -> Vec<String> {
    let state: &GameState = ctx.state;
    zone_cards(state, ctx.controller, OffFieldZone::Graveyard)
        .iter()
        .filter(|card| {
            def_of(Some(state), &card.def_id).type_ == CardType::Unit
                && jackioh_engine::scripts::script_of(state, *card).death.is_some()
        })
        .map(|card| card.id.clone())
        .collect()
}

/// `radiant`: whether the summoned are given Reborn; `rounds`: how many times their Deaths then run.
fn zzoth(radiant: bool, rounds: usize) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            let mut effects = vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(|ctx: &mut EffectContext<'_>| graveyard_death_units(ctx)),
                each: Arc::new(|instance_id: &str| {
                    summon(json_as(json!({ "instance": { "of": "instance", "instanceId": instance_id } })))
                }),
            })];
            if radiant {
                effects.push(for_each_card(ForEachCardArgs {
                    cards: Arc::new(|ctx: &mut EffectContext<'_>| summoned_so_far(ctx)),
                    each: Arc::new(|instance_id: &str| {
                        grant_keyword(json_as(json!({
                            "target": { "of": "instance", "instanceId": instance_id },
                            "keyword": { "kind": "Reborn" },
                        })))
                    }),
                }));
            }
            if rounds > 0 {
                // Read once, before any Death runs: a Unit a Death summons is none of "them".
                effects.push(for_each_card(ForEachCardArgs {
                    cards: Arc::new(move |ctx: &mut EffectContext<'_>| {
                        let summoned = summoned_so_far(ctx);
                        (0..rounds).flat_map(|_| summoned.clone()).collect()
                    }),
                    each: Arc::new(|instance_id: &str| {
                        trigger_death(TriggerDeathArgs {
                            instance_id: instance_id.to_string(),
                        })
                    }),
                }));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: zzoth(false, DEATH_ROUNDS),
        radiant: zzoth(true, RADIANT_DEATH_ROUNDS),
    }
}

/// R767000: the scripts through v0.6.6e, which triggered no Death (issue #4154). `KEPT_SCRIPTS` folds
/// those replays under it.
pub fn script_v0_6_6e() -> CardScripts {
    CardScripts {
        base: zzoth(false, 0),
        radiant: zzoth(true, 0),
    }
}

// S #70 Z'Zoth — SPEC §8.10 row 70, BUILD M12 row S 70.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::def_ids;

    const FILLER: &str = "core-005"; // Stockpile: a card in hand, so no turn ends by itself; a non-Unit
    const VANILLA: &str = "stand-in (1) 4/4 R12/12"; // no Death.
    const SAINTESS: &str = "core-081"; // Radiant Saintess: "Death: Make your other Units Radiant."
    const CUBE: &str = "core-022"; // Carnivorous Cube: a Cry and a Death.
    const MR_TOKEN: &str = "core-015"; // Me and Mr Token: "Cry: Summon a Rush Token.": a Cry and no Death.

    fn holding(radiant: bool, graveyard: Value, field: Value) -> Scenario {
        crate::scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "graveyard": graveyard, "field": field },
            "p2": { "hand": [FILLER], "graveyard": [SAINTESS] },
        }))
    }

    const LAR: &str = "classicplus-076"; // Brother Lar: "Death: Summon a Brother Ping."
    const PING: &str = "classicplus-076-1"; // Brother Ping

    fn pings(s: &Scenario) -> usize {
        (1..=5).filter_map(|lane| s.unit(P1, lane)).filter(|unit| unit.def_id == PING).count()
    }

    /// R4154205: Z'Zoth with Brother Lar in the graveyard: Lar's lane, then how many Pings its Deaths made.
    fn lar_back(radiant: bool) -> (Scenario, usize) {
        let mut s = holding(radiant, json!([LAR]), json!([]));
        s.play(ID, json!({ "zone": 1 }));
        assert_eq!(s.unit(P1, 2).map(|card| card.def_id), Some(LAR.to_string()), "Lar is summoned and stays");
        assert_eq!(s.count("destroyed"), 0, "no Unit died");
        let made = pings(&s);
        (s, made)
    }

    #[test]
    fn r4154205_the_base_face_triggers_each_summoned_units_death_once() {
        let (_, made) = lar_back(false);
        assert_eq!(made, 1);
    }

    #[test]
    fn r4154205_the_radiant_face_triggers_them_twice_and_a_unit_a_death_summoned_runs_none() {
        let (s, made) = lar_back(true);
        assert_eq!(made, 2);
        let lar = s.unit_at(P1, 2);
        assert!(has_keyword(&s.stats(&lar).keywords, KeywordKind::Reborn), "Lar has Reborn first");
    }

    /// R767000: through v0.6.6e no Death ran (issue #4154).
    #[test]
    fn r767000_the_script_kept_for_v0_6_6e_triggers_no_death() {
        crate::under_kept(ID, super::script_v0_6_6e(), || {
            for radiant in [false, true] {
                let mut s = holding(radiant, json!([LAR]), json!([]));
                s.play(ID, json!({ "zone": 1 }));
                assert_eq!(pings(&s), 0, "radiant {radiant}");
            }
        });
    }

    fn units_on_board(s: &Scenario) -> Vec<Option<String>> {
        (1..=5).map(|lane| s.unit(P1, lane).map(|card| card.def_id)).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r1869001_summons_each_death_unit_without_a_cry_and_leaves_the_rest() {
            let mut s = holding(false, json!([SAINTESS, VANILLA, FILLER, CUBE, MR_TOKEN]), json!([]));
            s.play(ID, json!({ "zone": 1 }));
            // No Cry (R1): the Cube's would ask for one of your other Units to tribute.
            assert!(s.state().pending.is_none(), "nothing is asked");
            let some = |id: &str| Some(id.to_string());
            assert_eq!(units_on_board(&s), vec![some(ID), some(SAINTESS), some(CUBE), None, None]);
            assert_eq!(def_ids(&s.pile(P1, "graveyard")), vec![VANILLA, FILLER, MR_TOKEN]);
            assert_eq!(def_ids(&s.pile(P2, "graveyard")), vec![SAINTESS]);
        }

        #[test]
        fn a_full_row_leaves_the_rest_in_the_graveyard() {
            let mut s = holding(false, json!([SAINTESS, CUBE]), json!([VANILLA, VANILLA, VANILLA]));
            s.play(ID, json!({ "zone": 4 }));
            let some = |id: &str| Some(id.to_string());
            assert_eq!(
                units_on_board(&s),
                vec![some(VANILLA), some(VANILLA), some(VANILLA), some(ID), some(SAINTESS)]
            );
            assert_eq!(def_ids(&s.pile(P1, "graveyard")), vec![CUBE]);
        }

        #[test]
        fn the_summoned_have_no_reborn_on_the_base_face() {
            let mut s = holding(false, json!([SAINTESS]), json!([]));
            s.play(ID, json!({ "zone": 1 }));
            let saintess = s.unit_at(P1, 2);
            assert!(!has_keyword(&s.stats(&saintess).keywords, KeywordKind::Reborn));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_summoned_have_reborn() {
            let mut s = holding(true, json!([SAINTESS, VANILLA, CUBE]), json!([]));
            s.play(ID, json!({ "zone": 1 }));
            let zzoth = s.unit_at(P1, 1);
            s.expect_stats(&zzoth, json!({ "attack": 10, "health": 14 }));
            assert!(!has_keyword(&s.stats(&zzoth).keywords, KeywordKind::Reborn), "not itself");
            for lane in [2, 3] {
                let unit = s.unit_at(P1, lane);
                assert!(has_keyword(&s.stats(&unit).keywords, KeywordKind::Reborn), "lane {lane}");
            }
            assert_eq!(def_ids(&s.pile(P1, "graveyard")), vec![VANILLA]);
        }
    }
}
