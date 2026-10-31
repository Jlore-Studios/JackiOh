//! S #70 Z'Zoth (SPEC §8.10 row 70): (4) Unit, Legendary, 5/7 → 10/14, of the Stinkyscape set.
//!   Base:    "Cry: Summon every Unit with a Death from your graveyard."
//!   Radiant: "Cry: Summon every Unit with a Death from your graveyard. Give them Reborn."
//! Engine: Cry (R1), R1869001: C #44 Back from the GY's Radiant summon from the graveyard (`for_each_card`
//! read once as the clause begins, R113), filtered to the Unit cards whose running face prints a Death
//! (`Script.death`; a Death granted on the field is gone in the graveyard, R78), oldest first (§3), each
//! summoned into R64's zone with no Cry; with no room the rest stay in the graveyard. Radiant: each Unit
//! this Cry summoned (`summoned_so_far`, R136) is given Reborn (`grant_keyword`).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-070";

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

fn zzoth(radiant: bool) -> Script {
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
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: zzoth(false),
        radiant: zzoth(true),
    }
}

// S #70 Z'Zoth — SPEC §8.10 row 70, BUILD M12 row S 70.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::def_ids;

    const FILLER: &str = "core-005"; // (1) Spell: a card in hand, so no turn ends by itself; a non-Unit.
    const VANILLA: &str = "core-008"; // (1) 4/4, no Death.
    const SAINTESS: &str = "core-081"; // (1) 2/2, "Death: Make your other Units Radiant."
    const CUBE: &str = "core-022"; // (3) 4/6, a Cry and a Death.
    const MR_TOKEN: &str = "core-015"; // (1) 1/1, "Cry: Summon a Rush Token.": a Cry and no Death.

    fn holding(radiant: bool, graveyard: Value, field: Value) -> Scenario {
        crate::scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "graveyard": graveyard, "field": field },
            "p2": { "hand": [FILLER], "graveyard": [SAINTESS] },
        }))
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
