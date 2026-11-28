//! SPEC §8.14 FvH #71 Felinor Reanimator ((3) Unit, Felinor, Epic, 2/4 → 4/8), a card of the held Felinors vs
//! Humans set (R5235000).
//!   Base:    "End of turn: Summon a random Felinor Unit from your graveyard."
//!   Radiant: "End of turn: Summon a random Felinor Unit from your graveyard and make it Radiant. If there is
//!            none, summon a random Radiant Felinor Unit instead."
//!
//! At its controller's end of turn (R62): one uniform pick by the match rng (R60), drawn as the effect
//! resolves, of the Unit cards in its controller's graveyard whose tags hold Felinor (`tags_of`, R923; a
//! Felinor Trap is no pick), summoned by instance into the leftmost open unit zone (R64), running no Cry (R1);
//! a full row summons nothing and the card stays (R129). Radiant: the summon makes it Radiant, and only when
//! the graveyard holds no Felinor Unit does it `summon_random` a Radiant non-token Felinor Unit of the sets
//! that ship instead, a full row being no reason to (R5257002). Declares no numbers. Built by S07 (#5257).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-071";

/// The Unit cards in `player`'s graveyard whose tags hold Felinor, oldest first (§3).
fn felinor_units_in_graveyard(state: &GameState, player: PlayerId) -> Vec<String> {
    zone_cards(state, player, OffFieldZone::Graveyard)
        .iter()
        .filter(|card| {
            def_of(Some(state), &card.def_id).type_ == CardType::Unit
                && tags_of(state, card).contains(&Tag::Felinor)
        })
        .map(|card| card.id.clone())
        .collect()
}

fn reanimator(radiant: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| {
            // R5257002: the Radiant face's fallback answers an empty graveyard, never a full row.
            if radiant && felinor_units_in_graveyard(&*ctx.state, ctx.controller).is_empty() {
                return vec![summon_random(json_as(json!({
                    "query": { "type": "Unit", "tags": ["Felinor"] },
                    "radiant": true,
                })))];
            }
            vec![for_each_card(ForEachCardArgs {
                // The pick is drawn as the effect resolves (R60), never as the hook builds it.
                cards: Arc::new(|ctx: &mut EffectContext<'_>| -> Vec<String> {
                    let pool = felinor_units_in_graveyard(&*ctx.state, ctx.controller);
                    ctx.sink.rng.pick(&pool).cloned().into_iter().collect()
                }),
                each: Arc::new(move |instance_id: &str| {
                    let mut args = json!({ "instance": { "of": "instance", "instanceId": instance_id } });
                    if radiant {
                        args["radiant"] = json!(true);
                    }
                    summon(json_as(args))
                }),
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: reanimator(false),
        radiant: reanimator(true),
    }
}

// FvH #71 Felinor Reanimator — SPEC §8.14 row 71.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const MROW: &str = "core-086"; // "Miss" Mrow: a Felinor Unit
    const DUPLICATING: &str = "core-012"; // Duplicating Felinors: a Felinor Unit whose Cry would copy it
    const HUMAN: &str = "core-008"; // Mr. Vanilla: a Unit, no Felinor
    const GROOM_SHROOM: &str = "classicplus-002"; // Groom Shroom: a Felinor Trap
    const FILLER: &str = "core-005"; // Stockpile: a card to hold
    const VANILLA: &str = "stand-in (1) 4/4";

    fn board(radiant: bool, field: &[&str], graveyard: Value) -> Scenario {
        let mut units = vec![json!({ "def": ID, "radiant": radiant })];
        units.extend(field.iter().map(|id| json!(id)));
        scenario(json!({
            "p1": { "hand": [FILLER], "field": units, "graveyard": graveyard, "library": crate::filler(4) },
            "p2": { "hand": [FILLER], "field": [VANILLA], "library": crate::filler(4) },
        }))
    }

    /// The Units p1 summoned in the last step, by `summoned`.
    fn summoned(s: &Scenario) -> Vec<CardInstance> {
        s.last_events_of(GameEventType::Summoned)
            .into_iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { player: P1, instance_id, .. } => Some(s.card(instance_id.as_str()).clone()),
                _ => None,
            })
            .collect()
    }

    fn graveyard_ids(s: &Scenario) -> Vec<String> {
        s.pile(P1, "graveyard").into_iter().map(|card| card.id).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r62_at_its_end_of_turn_it_summons_one_felinor_unit_from_your_graveyard() {
            let mut s = board(false, &[], json!([HUMAN, MROW, GROOM_SHROOM, DUPLICATING]));
            let before = graveyard_ids(&s);
            s.end_turn();
            let units = nonempty(summoned(&s), "the Felinor it summoned");
            assert_eq!(units.len(), 1);
            for unit in &units {
                assert!(before.contains(&unit.id), "it came from the graveyard");
                assert!([MROW, DUPLICATING].contains(&unit.def_id.as_str()), "{} is a Felinor Unit", unit.def_id);
                assert!(!unit.radiant);
            }
            assert_eq!(active_units_of(s.state(), P1).len(), 2, "its Cry did not run: no copy");
            // The opponent's end of turn summons nothing for it.
            s.end_turn();
            assert!(summoned(&s).is_empty());
        }

        #[test]
        fn r60_the_pick_is_random() {
            let mut seen: Vec<String> = Vec::new();
            for seed in 0..16 {
                let mut s = scenario(json!({
                    "seed": format!("reanimator-{seed}"),
                    "p1": { "hand": [FILLER], "field": [ID], "graveyard": [MROW, DUPLICATING], "library": crate::filler(4) },
                    "p2": { "hand": [FILLER], "field": [VANILLA], "library": crate::filler(4) },
                }));
                s.end_turn();
                for unit in summoned(&s) {
                    if !seen.contains(&unit.def_id) {
                        seen.push(unit.def_id);
                    }
                }
            }
            assert_eq!(seen.len(), 2, "both Felinors come up: {seen:?}");
        }

        #[test]
        fn no_felinor_unit_in_the_graveyard_summons_nothing() {
            let mut s = board(false, &[], json!([HUMAN, GROOM_SHROOM]));
            s.end_turn();
            assert!(summoned(&s).is_empty());
        }

        #[test]
        fn a_full_row_summons_nothing_and_the_felinor_stays() {
            let mut s = board(false, &[VANILLA, VANILLA, VANILLA, VANILLA], json!([MROW]));
            s.end_turn();
            assert!(summoned(&s).is_empty());
            assert!(s.pile(P1, "graveyard").iter().any(|card| card.def_id == MROW));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn it_summons_a_felinor_unit_from_your_graveyard_and_makes_it_radiant() {
            let mut s = board(true, &[], json!([HUMAN, MROW]));
            let before = graveyard_ids(&s);
            s.end_turn();
            let units = nonempty(summoned(&s), "the Felinor it summoned");
            assert_eq!(units.len(), 1);
            for unit in &units {
                assert!(before.contains(&unit.id), "it came from the graveyard");
                assert_eq!(unit.def_id, MROW);
                assert!(unit.radiant, "made Radiant");
            }
        }

        #[test]
        fn r5257002_with_no_felinor_unit_there_it_summons_a_random_radiant_felinor_unit() {
            let mut s = board(true, &[], json!([HUMAN, GROOM_SHROOM]));
            s.end_turn();
            let units = nonempty(summoned(&s), "the Felinor it summoned");
            assert_eq!(units.len(), 1);
            for unit in &units {
                let def = def_of(Some(s.state()), &unit.def_id);
                assert_eq!(def.type_, CardType::Unit);
                assert!(def.tags.contains(&Tag::Felinor), "{} is a Felinor", def.id);
                assert!(!def.token, "{} is no token", def.id);
                assert!(unit.radiant);
            }
            assert_eq!(s.pile(P1, "graveyard").len(), 2, "the graveyard is untouched");
        }

        #[test]
        fn r5257002_a_full_row_is_no_reason_for_the_fallback() {
            let mut s = board(true, &[VANILLA, VANILLA, VANILLA, VANILLA], json!([MROW]));
            s.end_turn();
            assert!(summoned(&s).is_empty(), "the Felinor finds no zone and nothing replaces it");
            assert!(s.pile(P1, "graveyard").iter().any(|card| card.def_id == MROW));
        }
    }
}
