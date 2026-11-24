//! SPEC §8.13 IA #184 Pattern Recognition ((1) Spell, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Look at the top three cards of your opponent's deck. Add a copy of one to your hand and exile another."
//!   Radiant: "Look at the top three cards of your opponent's deck. Add copies of two to your hand and exile another."
//!
//! Built by C8 (#4592). R4592000, R4592004.

use jackioh_engine::effects::{
    ChoosePickArgs, ExileArgs, PileSpec, PileZone, add_to_hand, choose_pick, exile,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-184";

const STEP_COPY: &str = "pattern-copy";
const STEP_EXILE: &str = "pattern-exile";
const STEP_RADIANT: &str = "pattern-radiant";

fn pattern_recognition(radiant: bool) -> Script {
    if !radiant {
        let mut resume = IndexMap::new();
        resume.insert(
            STEP_COPY,
            hook(|ctx| {
                let Some(Selection::Instance { instance_id }) = ctx.targets.first() else {
                    return vec![];
                };
                vec![
                    add_to_hand(json_as(json!({ "copyOf": instance_id }))),
                    choose_pick(ChoosePickArgs {
                        step: STEP_EXILE.to_string(),
                        from: vec![PileSpec {
                            zone: PileZone::Library,
                            player: Some(PlayerSpec::Enemy),
                            top: Some(3),
                        }],
                        except: Some(vec![instance_id.clone()]),
                        min: Some(1),
                        max: 1,
                        prompt: Some("Pattern Recognition: exile another".to_string()),
                        ..Default::default()
                    }),
                ]
            }),
        );
        resume.insert(
            STEP_EXILE,
            hook(|ctx| {
                let Some(Selection::Instance { instance_id }) = ctx.targets.first() else {
                    return vec![];
                };
                vec![exile(ExileArgs {
                    target: TargetSpec::Instance {
                        instance_id: instance_id.clone(),
                    },
                })]
            }),
        );
        Script {
            cry: Some(hook(|_ctx| {
                vec![choose_pick(ChoosePickArgs {
                    step: STEP_COPY.to_string(),
                    from: vec![PileSpec {
                        zone: PileZone::Library,
                        player: Some(PlayerSpec::Enemy),
                        top: Some(3),
                    }],
                    min: Some(1),
                    max: 1,
                    prompt: Some("Pattern Recognition: add a copy to your hand".to_string()),
                    ..Default::default()
                })]
            })),
            resume,
            ..Script::default()
        }
    } else {
        let mut resume = IndexMap::new();
        resume.insert(
            STEP_RADIANT,
            hook(|ctx| {
                let opp = opponent_of(ctx.controller);
                let picked_ids: Vec<String> = ctx
                    .targets
                    .iter()
                    .filter_map(|t| match t {
                        Selection::Instance { instance_id } => Some(instance_id.clone()),
                        _ => None,
                    })
                    .collect();

                let mut effects = Vec::new();
                for id in &picked_ids {
                    effects.push(add_to_hand(json_as(json!({ "copyOf": id }))));
                }

                // If 3 were available, exile the remaining one without prompting
                let top3: Vec<String> = ctx.state.players[opp]
                    .library
                    .iter()
                    .take(3)
                    .map(|c| c.id.clone())
                    .collect();
                if let Some(leftover) = top3.into_iter().find(|id| !picked_ids.contains(id)) {
                    effects.push(exile(ExileArgs {
                        target: TargetSpec::Instance {
                            instance_id: leftover,
                        },
                    }));
                }

                effects
            }),
        );
        Script {
            cry: Some(hook(|_ctx| {
                vec![choose_pick(ChoosePickArgs {
                    step: STEP_RADIANT.to_string(),
                    from: vec![PileSpec {
                        zone: PileZone::Library,
                        player: Some(PlayerSpec::Enemy),
                        top: Some(3),
                    }],
                    min: Some(2),
                    max: 2,
                    prompt: Some("Pattern Recognition: add copies of two to your hand".to_string()),
                    ..Default::default()
                })]
            })),
            resume,
            ..Script::default()
        }
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: pattern_recognition(false),
        radiant: pattern_recognition(true),
    }
}

// IA #184 Pattern Recognition — SPEC §8.13 row 184.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005";
    const CARD_A: &str = "core-001";
    const CARD_B: &str = "core-008";
    const CARD_C: &str = "core-010";

    fn board(radiant: bool, opp_library: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "mana": 10 },
            "p2": { "hand": [FILLER], "library": opp_library },
        }))
    }

    fn first_option(s: &Scenario) -> String {
        let pending = must(s.state().pending.clone(), "pending prompt");
        match &pending.options[0].selection {
            Selection::Instance { instance_id } => instance_id.clone(),
            _ => panic!("expected instance selection"),
        }
    }

    mod base {
        use super::*;

        #[test]
        fn copies_one_and_exiles_another() {
            let mut s = board(false, json!([CARD_A, CARD_B, CARD_C]));
            s.play(ID, json!({}));
            assert!(s.state().pending.is_some());
            let pick1 = first_option(&s); // pick CARD_A
            s.answer(json!(pick1));

            // Second prompt to exile another
            assert!(s.state().pending.is_some());
            let options: Vec<String> = s.state().pending.as_ref().unwrap().options.iter().map(|o| match &o.selection {
                Selection::Instance { instance_id } => instance_id.clone(),
                _ => panic!(),
            }).collect();
            assert!(!options.contains(&pick1), "the first pick cannot be exiled (R4592004)");
            s.answer(json!(options[0]));

            assert_eq!(s.hand(P1).len(), 2); // filler + copy
            assert_eq!(s.state().players[P2].exile.len(), 1);
            assert_eq!(s.state().players[P2].library.len(), 2);
        }

        #[test]
        fn with_only_one_card_in_deck_copies_and_exile_fizzles() {
            let mut s = board(false, json!([CARD_A]));
            s.play(ID, json!({}));
            assert!(s.state().pending.is_some());
            let pick1 = first_option(&s);
            s.answer(json!(pick1));
            // Second prompt should not open
            assert!(s.state().pending.is_none());
            assert_eq!(s.hand(P1).len(), 2);
            assert_eq!(s.state().players[P2].exile.len(), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn copies_two_and_exiles_the_third() {
            let mut s = board(true, json!([CARD_A, CARD_B, CARD_C]));
            s.play(ID, json!({}));
            assert!(s.state().pending.is_some());
            let options: Vec<String> = s.state().pending.as_ref().unwrap().options.iter().map(|o| match &o.selection {
                Selection::Instance { instance_id } => instance_id.clone(),
                _ => panic!(),
            }).collect();
            s.answer(json!([options[0].clone(), options[1].clone()]));
            assert!(s.state().pending.is_none());
            assert_eq!(s.hand(P1).len(), 3); // filler + 2 copies
            assert_eq!(s.state().players[P2].exile.len(), 1);
            assert_eq!(s.state().players[P2].library.len(), 2);
        }
    }
}
