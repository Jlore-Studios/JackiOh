//! M #49.2 Yile's Virus (SPEC §8.8 row 49.2): (2) Unit, CN, Acclaimed, Token (printed Mythic),
//! 0/8 → 0/16, Can't attack.
//!
//! "Can't attack. Can't be in Defense Position. Activate: Destroy this. Death: Discard
//! {discards|random card|random cards}. Start of turn: Deal {damage} damage to each adjacent Unit
//! that isn't a Yile's Virus, and to your hero."
//! Engine: Can't attack is a keyword; "can't be in Defense Position" is #65.1 Spikey Pillow's
//! `neverDefense` flag, so the token never has Defense's Taunt. Activate (R384) destroys itself
//! once a turn in its controller's main phase. Death discards at random from its controller's hand
//! (R682), firing though the token vanishes (R11, §6.2). At its controller's start of turn, one hit
//! each on the Units beside it on its side and row (§3.1) — those of Yile's Virus's definition
//! skipped — then one on its controller's hero. "Your" is the Virus's controller, its victim, since
//! M #49 and M #49.1 make it for the opponent (R12); forced attacks still make it attack (R53).

use jackioh_engine::effects::{damage, destroy, discard_random, for_each_card};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-049-2";

fn destroy_self() -> ActivationDecl {
    ActivationDecl {
        id: "destroy".to_string(),
        label: "Destroy this".to_string(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(|_ctx| vec![destroy(json_as(json!({ "target": { "of": "self" } })))]),
    }
}

pub fn script() -> CardScripts {
    // Both faces share the one script: the declared `discards` and `damage` (1 and 2, Radiant 2
    // and 4) are what differ. The amount is read as the hook resolves and moved into each hit, so
    // a Nerf or Buff on the numbers moves every hit of the turn (R386).
    let virus = Script {
        static_flags: Some(StaticFlags {
            never_defense: Some(true),
            ..StaticFlags::default()
        }),
        activations: vec![destroy_self()],
        death: Some(hook(|ctx| {
            vec![discard_random(json_as(json!({
                "count": param(&*ctx, "discards"),
            })))]
        })),
        start_of_turn: Some(hook(|ctx| {
            let amount = param(&*ctx, "damage");
            vec![
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(|each: &mut EffectContext<'_>| {
                        adjacent_to(each, &json_as(json!({ "of": "self" })), &BoardScope::default())
                            .into_iter()
                            .filter(|card| card.def_id != ID)
                            .map(|card| card.id)
                            .collect()
                    }),
                    each: Arc::new(move |instance_id: &str| {
                        damage(json_as(json!({
                            "to": { "of": "instance", "instanceId": instance_id },
                            "amount": amount,
                        })))
                    }),
                }),
                damage(json_as(json!({
                    "to": { "of": "selfHero" },
                    "amount": amount,
                }))),
            ]
        })),
        ..Script::default()
    };
    CardScripts {
        base: virus.clone(),
        radiant: virus,
    }
}

// M #49.2 Yile's Virus — SPEC §8.8 row 49.2, BUILD M10 row M 49.2.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VIRUS: &str = "meditative-049-2";
    const TAMED: &str = "meditative-049"; // "Cry: Summon a Yile's Virus for your opponent across from this."
    const GIRLFRIEND: &str = "meditative-049-3"; // every enemy Unit attacks it at its start of turn.
    const NEIGHBOUR: &str = "core-012"; // 3/4, standing beside the Virus.
    const FILLER: &str = "core-005";

    /// p1's Virus in lane 2 with non-Virus neighbours in lanes 1 and 3, p1 to move.
    fn yard(seed: &str, radiant_face: bool) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "field": [
                    { "def": NEIGHBOUR, "lane": 1 },
                    { "def": VIRUS, "radiant": radiant_face, "lane": 2 },
                    { "def": NEIGHBOUR, "lane": 3 },
                ],
                "hand": [FILLER, FILLER],
                "library": [FILLER, FILLER],
            },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }))
    }

    mod m49_2_yiles_virus {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn cannot_attack_and_cannot_switch_to_defense() {
                let mut s = yard("virus-static", false);
                let virus = s.unit(P1, 2).expect("the virus").clone();
                s.expect_refused(|s| s.attack(&virus, "hero"));
                s.expect_refused(|s| s.switch_position(&virus));
            }

            #[test]
            fn it_never_taunts_an_attack_on_the_hero_goes_past_it() {
                let mut s = scenario(json!({
                    "seed": "virus-no-taunt",
                    "p1": {
                        "field": [{ "def": NEIGHBOUR, "lane": 1 }],
                        "hand": [FILLER],
                        "library": [FILLER, FILLER],
                    },
                    "p2": {
                        "field": [{ "def": VIRUS, "lane": 1 }],
                        "hand": [FILLER],
                        "library": [FILLER, FILLER],
                    },
                }));
                let virus = s.unit(P2, 1).expect("the virus");
                assert_eq!(s.stats(&virus).position, Position::Atk);
                let attacker = s.unit(P1, 1).expect("the attacker");

                s.attack(&attacker, "hero");

                s.expect_health(P2, 30 - 3);
            }

            #[test]
            fn r53_forced_attacks_still_make_it_attack() {
                let mut s = scenario(json!({
                    "seed": "virus-forced",
                    "active": "p2",
                    "p1": {
                        "field": [{ "def": VIRUS, "lane": 1 }],
                        "hand": [FILLER],
                        "library": [FILLER, FILLER],
                    },
                    "p2": {
                        "hand": [GIRLFRIEND, FILLER],
                        "library": [FILLER, FILLER],
                        "mana": 4,
                    },
                }));
                let virus = s.unit(P1, 1).expect("the virus");
                s.play(GIRLFRIEND, json!({ "zone": 1 }));
                let girlfriend = s.unit(P2, 1).expect("the animated girlfriend");

                s.start_turn();

                // Can't attack, yet the Girlfriend's forced attack makes it attack (R53).
                assert!(s.events().iter().any(|event| matches!(
                    event,
                    GameEvent::AttackDeclared { attacker_id, target_id, forced: true, .. }
                        if *attacker_id == virus.id && *target_id == girlfriend.id
                )));
            }

            #[test]
            fn r384_activate_only_in_its_controllers_main_phase() {
                let mut s = yard("virus-their-turn", false);
                let virus = s.unit(P1, 2).expect("the virus");

                s.end_turn();

                // p2's turn: p1 can't activate it.
                s.expect_refused(|s| s.activate(&virus, json!({})));
                s.expect_in_zone(&virus, "field");
            }

            #[test]
            fn its_death_with_an_empty_hand_discards_nothing() {
                let mut s = scenario(json!({
                    "seed": "virus-empty-hand",
                    "p1": {
                        "field": [{ "def": VIRUS, "lane": 1 }],
                        "library": [FILLER, FILLER],
                    },
                    "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                }));
                let virus = s.unit(P1, 1).expect("the virus");

                s.activate(&virus, json!({}));

                assert!(s.unit(P1, 1).is_none());
                assert!(
                    !s.events()
                        .iter()
                        .any(|event| matches!(event, GameEvent::Discarded { .. }))
                );
            }

            #[test]
            fn r12_summoned_for_the_opponent_they_are_its_controller_and_victim() {
                let mut s = scenario(json!({
                    "seed": "virus-for-them",
                    "p1": { "hand": [TAMED, FILLER], "library": [FILLER, FILLER] },
                    "p2": { "hand": [FILLER, FILLER], "library": [FILLER, FILLER] },
                }));
                s.play(TAMED, json!({ "zone": 1 }));
                let virus = s.unit(P2, 1).expect("the virus across from it");
                assert_eq!(virus.def_id, VIRUS);
                assert_eq!(virus.controller, P2);

                s.end_turn();

                // p2's start of turn: p2's hero is hit, p1's is not.
                s.expect_health(P2, 28);
                s.expect_health(P1, 30);
                // p2 activates it; its Death discards from p2's hand.
                let before = s.hand(P2).len();
                s.activate(&virus, json!({}));
                assert_eq!(s.hand(P2).len(), before - 1);
            }

            #[test]
            fn r386_its_numbers_read_through_param() {
                let mut s = yard("virus-param", false);
                let virus = s.unit(P1, 2).expect("the virus");
                step_param(s.card_mut(&virus), "damage", 1);
                step_param(s.card_mut(&virus), "discards", 1);
                let left = s.unit(P1, 1).expect("left");

                s.start_turn();

                // 3 damage now: the 3/4 neighbour sits at 1, the hero at 27.
                s.expect_stats(&left, json!({ "health": 1 }));
                s.expect_health(P1, 27);
                // And its Death discards 2.
                let before = s.hand(P1).len();
                s.activate(&virus, json!({}));
                assert_eq!(s.hand(P1).len(), before - 2);
            }

            #[test]
            fn r384_activate_destroys_this_and_its_death_discards_a_random_card() {
                let mut s = scenario(json!({
                    "seed": "virus-activate",
                    "p1": {
                        "field": [{ "def": VIRUS, "lane": 1 }],
                        "hand": [FILLER, FILLER],
                        "library": [FILLER, FILLER],
                    },
                    "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                }));
                let virus = s.unit(P1, 1).expect("the virus").clone();

                s.activate(&virus, json!({}));

                // It destroyed itself (a token: ceased to exist, R11) and its Death discarded one.
                assert!(s.unit(P1, 1).is_none());
                assert_eq!(s.hand(P1).len(), 1);
            }

            #[test]
            fn start_of_turn_hits_each_adjacent_non_virus_unit_and_its_hero() {
                let mut s = yard("virus-sot", false);
                let left = s.unit(P1, 1).expect("left").clone();
                let right = s.unit(P1, 3).expect("right").clone();

                s.start_turn();

                // One hit of 2 on each adjacent non-Virus Unit, then one on its hero.
                s.expect_stats(&left, json!({ "health": 2, "maxHealth": 4 }));
                s.expect_stats(&right, json!({ "health": 2, "maxHealth": 4 }));
                s.expect_health(P1, 28);
            }

            #[test]
            fn start_of_turn_skips_adjacent_viruses() {
                let mut s = scenario(json!({
                    "seed": "virus-skip",
                    "p1": {
                        "field": [
                            { "def": VIRUS, "lane": 1 },
                            { "def": VIRUS, "lane": 2 },
                        ],
                        "hand": [FILLER, FILLER],
                        "library": [FILLER, FILLER],
                    },
                    "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                }));
                let other = s.unit(P1, 2).expect("the other virus").clone();

                s.start_turn();

                // The neighbouring Virus is skipped; the hero takes one hit per Virus.
                s.expect_stats(&other, json!({ "health": 8, "maxHealth": 8 }));
                s.expect_health(P1, 26);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn radiant_is_0_16() {
                let s = yard("virus-radiant-stats", true);
                let virus = s.unit(P1, 2).expect("the virus");
                let stats = s.stats(&virus);
                assert_eq!((stats.attack, stats.health), (0, 16));
            }

            #[test]
            fn radiant_discards_2_and_deals_4() {
                let mut s = scenario(json!({
                    "seed": "virus-radiant-activate",
                    "p1": {
                        "field": [{ "def": VIRUS, "radiant": true, "lane": 1 }],
                        "hand": [FILLER, FILLER, FILLER],
                        "library": [FILLER, FILLER],
                    },
                    "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                }));
                let virus = s.unit(P1, 1).expect("the virus").clone();

                s.activate(&virus, json!({}));

                assert!(s.unit(P1, 1).is_none());
                assert_eq!(s.hand(P1).len(), 1);
            }

            #[test]
            fn radiant_start_of_turn_deals_4() {
                let mut s = yard("virus-radiant-sot", true);

                s.start_turn();

                // 4 damage: a 3/4 neighbour falls to 0 and dies; the hero takes 4.
                assert!(s.unit(P1, 1).is_none());
                assert!(s.unit(P1, 3).is_none());
                s.expect_health(P1, 26);
            }
        }
    }
}
