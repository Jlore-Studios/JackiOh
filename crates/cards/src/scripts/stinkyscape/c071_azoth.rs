//! S #71 A'Zoth (SPEC §8.10 row 71): (4) Unit, Legendary, 7/5 → 14/10, of the Stinkyscape set.
//!   Base:    "Cry: Summon every Unit with a Cry from your graveyard. Trigger each of their Cries with
//!            random targets."
//!   Radiant: "Cry: Summon every Unit with a Cry from your graveyard. Trigger each of their Cries
//!            {repeats|time|times}, with random targets."
//! Cry (R1, R1869001): summons GY units with Cry (C #44, R113, has_triggerable_cry, R78, §3, R64).
//! Triggers each summoned Cry in order with random choices (R136, R467, R1855001). Radiant repeats {repeats} times.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-071";

/// "Trigger each of their Cries": once on the base face, which declares no `repeats` (`tunedOn: radiant`).
const BASE_TRIGGERS: i32 = 1;

/// Your graveyard's Units with a Cry to trigger, oldest first (§3).
fn graveyard_cry_units(ctx: &EffectContext<'_>) -> Vec<String> {
    let state: &GameState = ctx.state;
    zone_cards(state, ctx.controller, OffFieldZone::Graveyard)
        .iter()
        .filter(|card| def_of(Some(state), &card.def_id).type_ == CardType::Unit && has_triggerable_cry(state, card))
        .map(|card| card.id.clone())
        .collect()
}

fn azoth(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(|ctx: &mut EffectContext<'_>| graveyard_cry_units(ctx)),
                    each: Arc::new(|instance_id: &str| {
                        summon(json_as(json!({ "instance": { "of": "instance", "instanceId": instance_id } })))
                    }),
                }),
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(move |ctx: &mut EffectContext<'_>| {
                        let times = if radiant { param(&*ctx, "repeats") } else { BASE_TRIGGERS };
                        let times = usize::try_from(times).unwrap_or(0);
                        summoned_so_far(ctx)
                            .into_iter()
                            .flat_map(|id| std::iter::repeat_n(id, times))
                            .collect()
                    }),
                    each: Arc::new(|instance_id: &str| {
                        trigger_cry(json_as(json!({ "instanceId": instance_id, "random": true })))
                    }),
                }),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: azoth(false),
        radiant: azoth(true),
    }
}

// S #71 A'Zoth — SPEC §8.10 row 71, BUILD M12 row S 71.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::def_ids;

    const FILLER: &str = "core-005"; // (1) Spell: a card in hand, so no turn ends by itself.
    const VANILLA: &str = "core-008"; // (1) 4/4, no Cry.
    const MR_TOKEN: &str = "core-015"; // (1) 1/1, "Cry: Summon a Rush Token."
    const FELINORS: &str = "core-012"; // (2) 3/4, "Cry: Summon a copy of this."
    const SORCERER: &str = "core-068"; // (2) 5/5, "Cry: Deal 4 damage" to any Unit or hero.
    const SORCERER_DAMAGE: i32 = 4;
    const RUSH_TOKEN: &str = "core-t-rush";

    fn holding(seed: &str, radiant: bool, graveyard: Value, field: Value) -> Scenario {
        crate::scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "graveyard": graveyard, "field": field },
            "p2": { "hand": [FILLER], "field": [VANILLA] },
        }))
    }

    fn summoned(s: &Scenario, def: &str) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == def))
            .count()
    }

    fn sorcerer_hits(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, amount, .. } if *amount == SORCERER_DAMAGE => Some(target_id.clone()),
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r1869001_summons_each_cry_unit_and_triggers_each_cry_once() {
            let mut s = holding("azoth", false, json!([MR_TOKEN, VANILLA, FELINORS]), json!([]));
            s.play(ID, json!({ "zone": 1 }));
            assert!(s.state().pending.is_none());
            let board: Vec<Option<String>> = (1..=5).map(|lane| s.unit(P1, lane).map(|card| card.def_id)).collect();
            let some = |id: &str| Some(id.to_string());
            assert_eq!(board, vec![some(ID), some(MR_TOKEN), some(FELINORS), some(RUSH_TOKEN), some(FELINORS)]);
            assert_eq!(summoned(&s, RUSH_TOKEN), 1);
            assert_eq!(def_ids(&s.pile(P1, "graveyard")), vec![VANILLA]);
            // A'Zoth alone was played: a triggered Cry is no play (R467).
            assert_eq!(
                s.events().iter().filter(|event| matches!(event, GameEvent::CardPlayed { .. })).count(),
                1
            );
        }

        #[test]
        fn a_random_target_is_drawn_and_nothing_is_asked() {
            let mut s = holding("azoth-aim", false, json!([SORCERER]), json!([]));
            s.play(ID, json!({ "zone": 1 }));
            assert!(s.state().pending.is_none(), "nothing is asked");
            assert_eq!(sorcerer_hits(&s).len(), 1, "one target, drawn at random");
        }

        #[test]
        fn the_same_seed_picks_the_same_target() {
            let run = |seed: &str| {
                let mut s = holding(seed, false, json!([SORCERER]), json!([]));
                s.play(ID, json!({ "zone": 1 }));
                sorcerer_hits(&s)
            };
            assert_eq!(run("azoth-seed"), run("azoth-seed"));
            let first = run("azoth-seed");
            assert!((0..40).any(|n| run(&format!("azoth-seed-{n}")) != first));
        }

        #[test]
        fn a_summoned_azoth_triggers_only_what_its_own_cry_summoned() {
            let mut s = holding("azoth-twice", false, json!([ID, MR_TOKEN]), json!([]));
            s.play(ID, json!({ "zone": 1 }));
            assert!(s.state().pending.is_none());
            assert_eq!(summoned(&s, ID), 2, "the one played and the second, once");
            assert_eq!(summoned(&s, RUSH_TOKEN), 1);
        }

        #[test]
        fn a_unit_left_in_the_graveyard_is_not_triggered() {
            let mut s = holding("azoth-full", false, json!([MR_TOKEN, SORCERER]), json!([VANILLA, VANILLA, VANILLA]));
            s.play(ID, json!({ "zone": 4 }));
            assert_eq!(s.unit_at(P1, 5).def_id, MR_TOKEN);
            assert_eq!(def_ids(&s.pile(P1, "graveyard")), vec![SORCERER]);
            assert!(sorcerer_hits(&s).is_empty(), "the Sorcerer's Cry never fired");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn each_cry_twice_and_repeats_reads_through_param() {
            let mut s = holding("azoth-radiant", true, json!([MR_TOKEN]), json!([]));
            s.expect_stats(ID, json!({ "attack": 14, "health": 10 }));
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(summoned(&s, RUSH_TOKEN), 2);

            let mut s = holding("azoth-radiant", true, json!([MR_TOKEN]), json!([]));
            step_param(s.card_mut(ID), "repeats", 1);
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(summoned(&s, RUSH_TOKEN), 3);
        }

        #[test]
        fn each_cry_fires_its_repeats_before_the_next_ones() {
            let mut s = holding("azoth-order", true, json!([MR_TOKEN, FELINORS]), json!([]));
            s.play(ID, json!({ "zone": 1 }));
            let order: Vec<String> = s
                .events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Summoned { def_id, .. } if def_id == RUSH_TOKEN || def_id == FELINORS => Some(def_id.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(order[0], FELINORS);
            assert_eq!(&order[1..3], &[RUSH_TOKEN.to_string(), RUSH_TOKEN.to_string()]);
            assert!(order[3..].iter().all(|def| def == FELINORS));
        }
    }
}
