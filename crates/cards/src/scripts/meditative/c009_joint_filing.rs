//! Meditative #9 Joint Filing (SPEC §8.8 row 9; docs/meditative-set.md M6 #9; ME-TRIG (a); R820, R821,
//! R3134001, R4440055). (2) Field Spell, Epic.
//!   Base:    "Aura: Your Start of turn and End of turn effects trigger {extra|additional time|additional
//!            times}. / Activate: Discover a card with a Start of turn or End of turn effect." (extra 1)
//!   Radiant: the same, extra 2.
//!   Through v0.6.9n: "Cry: Add a random card with a Start of turn or an End of turn effect to your hand."
//!   (Radiant: "Cry: Add a random card with a Start of turn effect and a random card with an End of turn
//!   effect to your hand.") and the Aura (`script_v0_6_9n`, kept, R767000; issue #4440).
//!   Through v0.5.11r: the Aura alone (`script_v0_5_11o`, kept).
//!
//! R4440055: the Activate, once a turn (R384) and free, is a Discover (§6.3) of the catalog pool the
//! `turnHook` filter names (R3134001): the non-token cards of every set that ships (R380, R1420) whose
//! base-face script sets `start_of_turn`, `end_of_turn` or either, never this card (R387), three drawn
//! through the match rng (R60); the pick goes to the hand on its base face, a full hand burning it (§2.4),
//! as M #10 Double Counting's does (R4154211). The Cry it replaced drew the same pool, the Radiant face
//! adding two cards, a Start of turn one and then an End of turn one. Neither declares a number, so the
//! card declares no new param.
//!
//! The engine's turn-hook multiplier (`Script.turn_hook_extra`): while this acts on its controller's
//! field, each of their start-of-turn and end-of-turn hooks is queued 1 + extra times as the turn's
//! trigger step queues them (`triggers::queue_hooks_in_trigger_order`), each copy its own entry re-read
//! as it pops (R153, R821). Several multipliers do not add: the highest holds (R820).

use jackioh_engine::effects::{add_random_from_catalog, add_to_hand, discover_from_catalog};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-009";

/// The Activate's id, and the resume step the Discover's answer comes back to (§10.6).
const FILE: &str = "file";
const PICKED: &str = "picked";
/// "Activate" is once a turn (R384).
const USES: i32 = 1;

/// R3134001: "a random card with a ... turn effect", `which` being `turnHook`'s `start`, `end` or `either`.
fn add_with_turn_hook(which: &'static str) -> Effect {
    add_random_from_catalog(json_as(json!({ "query": { "turnHook": which } })))
}

/// The Aura both faces share (R821).
fn aura() -> Script {
    Script {
        turn_hook_extra: Some(read_hook(|args| param(&args, "extra"))),
        ..Script::default()
    }
}

/// R4440055: "Activate: Discover a card with a Start of turn or End of turn effect.", once a turn (R384).
fn file() -> ActivationDecl {
    ActivationDecl {
        id: FILE.to_string(),
        label: "Discover a card with a Start of turn or End of turn effect".to_string(),
        uses: ActivationUses::Count(USES),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(|_ctx| {
            vec![discover_from_catalog(json_as(json!({
                "step": PICKED,
                "query": { "turnHook": "either" },
                "prompt": "Discover a card with a Start of turn or End of turn effect",
            })))]
        }),
    }
}

/// Both faces since issue #4440: the Aura and the Activate, its pick to the hand (R4440055).
fn filing() -> Script {
    Script {
        activations: vec![file()],
        resume: IndexMap::from([(
            PICKED,
            // A Discover's pick arrives as a `mode` selection carrying a catalog id (§10.6).
            hook(|ctx| {
                let Some(picked) = chosen_options(ctx).into_iter().next() else {
                    return vec![];
                };
                vec![add_to_hand(json_as(json!({ "defId": picked })))]
            }),
        )]),
        ..aura()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: filing(),
        radiant: filing(),
    }
}

/// R767000: after v0.5.11r through v0.6.9n the faces had a Cry that added the random cards, and no Activate
/// (issue #4440); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_9n() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| vec![add_with_turn_hook("either")])),
            ..aura()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| vec![add_with_turn_hook("start"), add_with_turn_hook("end")])),
            ..aura()
        },
    }
}

/// R767000: the scripts v0.5.11r and the versions before it ran, kept for their replays: the Aura alone, no Cry.
pub fn script_v0_5_11o() -> CardScripts {
    CardScripts {
        radiant: aura(),
        base: aura(),
    }
}

// Meditative #9 Joint Filing — SPEC §8.8 row 9, BUILD M10 row M 9, R59, R62, R3134001, R4440055.
// Core #13 and #58 are the hooks it multiplies, Core #23 stands in for M 8; M 12's case is in Fear Mongerer's file.
// The Activate's offers and the kept Cry's cards come from the `turnHook` pool: every card is checked against
// the registered scripts, over a run of seeds.
#[cfg(test)]
mod tests {
    use super::{ID, script_v0_5_11o, script_v0_6_9n};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SHREDDER: &str = "core-013"; // Jlockeed Shredder-10: End of turn: 2 damage to each enemy.
    const FARM: &str = "core-058"; // Rush Token Farm: Field Spell. Start of turn: summon a Rush Token.
    const RUSH_TOKEN: &str = "core-t-rush";
    const DREAM: &str = "core-023"; // Reoccurring Dream: Spell. End of turn: return this to hand (from the graveyard).
    const RECYCLING: &str = "core-039"; // Recycling Initiative: Spell. Exile this. End of turn (delayed): copy each other card played.
    const BREAD_AND_BUTTER: &str = "core-018"; // Bread and Butter: fires in the end-of-turn trap window
    const BREAD_TOKEN: &str = "core-t-bread";
    const MAGIC_JAMMED: &str = "core-036"; // Magic Jammed: destroy a backrow card, Lock its zone.
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-005"; // Stockpile: a Spell

    /// A side with something to do and a deck to draw, so no turn auto-ends (§2.5) or fatigues.
    fn side(extra: Value) -> Value {
        crate::merged(json!({ "hand": [FILLER], "library": [FILLER, FILLER, FILLER] }), extra)
    }

    fn units(s: &Scenario, player: PlayerId, def_id: &str) -> usize {
        (1..=5)
            .filter_map(|lane| s.unit(player, lane))
            .filter(|unit| unit.def_id == def_id)
            .count()
    }

    mod base {
        use super::*;

        #[test]
        fn r821_your_end_of_turn_hooks_run_twice() {
            let mut s = scenario(json!({ "p1": side(json!({ "backrow": [ID], "field": [SHREDDER] })), "p2": side(json!({})) }));
            s.end_turn();
            s.expect_health(P2, 26);
        }

        #[test]
        fn r821_your_start_of_turn_hooks_run_twice() {
            let mut s = scenario(json!({ "p1": side(json!({ "backrow": [ID, FARM] })), "p2": side(json!({})) }));
            s.start_turn();
            assert_eq!(units(&s, P1, RUSH_TOKEN), 2);
        }

        #[test]
        fn r821_r153_a_return_spell_comes_back_once_its_copy_finding_it_gone() {
            let mut s = scenario(json!({ "p1": side(json!({ "backrow": [ID], "hand": [DREAM] })), "p2": side(json!({})) }));
            let dream = s.card(DREAM).clone();
            s.play(DREAM, json!({}));
            s.end_turn();
            s.expect_in_zone(&dream, "hand");
            let returns = s
                .events()
                .iter()
                .filter(|event| matches!(event, GameEvent::AddedToHand { instance_id, .. } if *instance_id == dream.id))
                .count();
            assert_eq!(returns, 1);
        }

        #[test]
        fn r821_a_delayed_end_of_turn_effect_runs_once() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID], "hand": [RECYCLING, VANILLA] })),
                "p2": side(json!({})),
            }));
            s.play(RECYCLING, json!({}));
            s.play(VANILLA, json!({}));
            s.end_turn();
            let copies = s.hand(P1).iter().filter(|card| card.def_id == VANILLA).count();
            assert_eq!(copies, 1, "Recycling Initiative's delayed copy is not multiplied (R62)");
        }

        #[test]
        fn r821_the_end_of_turn_trap_window_is_not_multiplied() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID, BREAD_AND_BUTTER] })),
                "p2": side(json!({})),
            }));
            s.end_turn();
            assert_eq!(units(&s, P1, BREAD_TOKEN), 1);
        }

        #[test]
        fn r821_the_opponents_end_of_turn_hooks_run_once() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID] })),
                "p2": side(json!({ "field": [SHREDDER] })),
                "active": "p2",
            }));
            s.end_turn();
            s.expect_health(P1, 28);
        }

        #[test]
        fn r820_two_joint_filings_give_one_extra_not_two() {
            let mut s = scenario(json!({ "p1": side(json!({ "backrow": [ID, ID], "field": [SHREDDER] })), "p2": side(json!({})) }));
            s.end_turn();
            s.expect_health(P2, 26);
        }

        #[test]
        fn r820_one_in_your_hand_multiplies_nothing() {
            let mut s = scenario(json!({ "p1": side(json!({ "hand": [ID], "field": [SHREDDER] })), "p2": side(json!({})) }));
            s.end_turn();
            s.expect_health(P2, 28);
        }

        #[test]
        fn r820_leaving_the_field_ends_it() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID], "field": [SHREDDER], "hand": [MAGIC_JAMMED] })),
                "p2": side(json!({})),
            }));
            let filing = s.card(ID).clone();
            s.play(MAGIC_JAMMED, json!({ "targets": [{ "pick": "instance", "instanceId": filing.id }] }));
            s.expect_in_zone(&filing, "graveyard");
            s.end_turn();
            s.expect_health(P2, 28);
        }

        #[test]
        fn r386_extra_reads_through_param_a_buff_gives_three_runs() {
            let mut s = scenario(json!({ "p1": side(json!({ "backrow": [ID], "field": [SHREDDER] })), "p2": side(json!({})) }));
            assert_eq!(crate::upgrade_number(&mut s, ID, "extra"), 2);
            s.end_turn();
            s.expect_health(P2, 24);
        }
    }

    /// The ids of the cards the Cry added to p1's hand: those after the one card `side` leaves there.
    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().skip(1).collect()
    }

    /// Whether the registered script of `def_id`'s base face sets `start_of_turn` and `end_of_turn`.
    fn hooks_of(def_id: &str) -> (bool, bool) {
        let scripts = crate::scripts_of();
        let base = &scripts.get(def_id).unwrap_or_else(|| panic!("no script for {def_id}")).base;
        (base.start_of_turn.is_some(), base.end_of_turn.is_some())
    }

    /// p1 plays Joint Filing from a hand of it and a Stockpile, under a seed (the Cry is v0.6.9n's: the
    /// callers run under `script_v0_6_9n`).
    fn played(seed: &str, hand: Value) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": side(json!({ "hand": hand, "mana": 5 })),
            "p2": side(json!({})),
        }));
        s.play(ID, json!({}));
        s
    }

    fn filing_hand(radiant_face: bool) -> Value {
        json!([FILLER, { "def": ID, "radiant": radiant_face }])
    }

    /// p1's Joint Filing on its backrow (`radiant` its face), mana to spare.
    fn filed(seed: &str, radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": side(json!({ "backrow": [{ "def": ID, "radiant": radiant }], "mana": 3 })),
            "p2": side(json!({})),
        }));
        s.state_mut().players.p1.auto_end_turn = Some(false);
        s
    }

    mod activate {
        use super::*;

        /// R4440055: both faces Discover one of three cards with a Start of turn or an End of turn effect,
        /// free, once a turn, the pick to the hand on its base face.
        #[test]
        fn r4440055_it_discovers_a_card_with_a_start_or_end_of_turn_effect_free_once_a_turn() {
            for radiant in [false, true] {
                for n in 0..10 {
                    let mut s = filed(&format!("filing-activate-{radiant}-{n}"), radiant);
                    s.activate(ID, json!({}));
                    assert_eq!(s.state().players.p1.mana.current, 3, "radiant {radiant}: it pays nothing");
                    let pending = s.state().pending.clone().expect("the Discover");
                    assert_eq!((pending.kind, pending.player_id), (PromptKind::Discover, P1));
                    let options = s.prompt_modes();
                    assert_eq!(options.len(), DISCOVER_OPTIONS as usize);
                    for option in &options {
                        let (start, end) = hooks_of(option);
                        assert!(start || end, "seed {n}: {option} has neither hook");
                        assert_ne!(option, ID);
                    }
                    let held = s.hand(P1).len();
                    s.answer(json!(options[0]));
                    let hand = s.hand(P1);
                    assert_eq!(hand.len(), held + 1);
                    let added = hand.last().expect("the pick");
                    assert_eq!(added.def_id, options[0]);
                    assert!(!added.radiant, "radiant {radiant}: the pick is on its base face");
                    s.expect_refused(|s| s.activate(ID, json!({})));
                }
            }
        }

        /// Issue #4440: the Cry is gone; played, it adds nothing.
        #[test]
        fn r4440055_played_it_adds_nothing() {
            for radiant in [false, true] {
                let s = played("filing-no-cry", filing_hand(radiant));
                assert!(added(&s).is_empty(), "radiant {radiant}");
            }
        }

        #[test]
        fn r4440055_the_aura_does_not_multiply_the_activate() {
            // R821 multiplies Start of turn and End of turn effects, and an Activate is neither.
            let mut s = filed("filing-activate-once", false);
            s.activate(ID, json!({}));
            let options = s.prompt_modes();
            s.answer(json!(options[0]));
            assert!(s.state().pending.is_none(), "one Discover");
        }
    }

    /// R767000: the Cry kept for v0.6.9n (issue #4440), the R3134001 pool it drew.
    mod cry {
        use super::*;

        #[test]
        fn r3134001_the_base_cry_adds_one_card_with_a_start_of_turn_or_an_end_of_turn_effect() {
            let mut kinds = (false, false);
            for n in 0..30 {
                let s = crate::under_kept(ID, script_v0_6_9n(), || played(&format!("filing-either-{n}"), filing_hand(false)));
                let cards = added(&s);
                assert_eq!(cards.len(), 1, "seed {n}");
                let (start, end) = hooks_of(&cards[0].def_id);
                assert!(start || end, "seed {n}: {} has neither hook", cards[0].def_id);
                kinds = (kinds.0 || (start && !end), kinds.1 || (end && !start));
            }
            // Both sorts come out of the one pool: a Start-only card and an End-only one.
            assert_eq!(kinds, (true, true));
        }

        #[test]
        fn r3134001_the_radiant_cry_adds_one_of_each_a_start_of_turn_card_then_an_end_of_turn_card() {
            for n in 0..30 {
                let s = crate::under_kept(ID, script_v0_6_9n(), || played(&format!("filing-both-{n}"), filing_hand(true)));
                let cards = added(&s);
                assert_eq!(cards.len(), 2, "seed {n}");
                assert!(hooks_of(&cards[0].def_id).0, "seed {n}: {} has no Start of turn effect", cards[0].def_id);
                assert!(hooks_of(&cards[1].def_id).1, "seed {n}: {} has no End of turn effect", cards[1].def_id);
            }
        }

        #[test]
        fn r387_r380_the_pool_is_every_shipped_non_token_card_with_the_hook_never_this_card() {
            crate::register_all();
            for (which, wanted) in [
                ("start", (true, false)),
                ("end", (false, true)),
                ("either", (true, true)),
            ] {
                let pool = jackioh_engine::catalog::query(&json_as(json!({ "turnHook": which })));
                assert!(pool.len() >= 20, "{which}: only {} cards", pool.len());
                for def in &pool {
                    assert!(!def.is_token(), "{which}: {} is a token", def.id);
                    assert_ne!(def.id, ID);
                    let (start, end) = hooks_of(&def.id);
                    assert!(
                        match which {
                            "start" => start,
                            "end" => end,
                            _ => start || end,
                        },
                        "{which}: {}",
                        def.id,
                    );
                }
                // And every shipped card with the hook is in it (R380).
                let ids: Vec<&str> = pool.iter().map(|def| def.id.as_str()).collect();
                let scripts = crate::scripts_of();
                let all = jackioh_engine::catalog::query(&jackioh_engine::CatalogQueryArgs::default());
                for def in all {
                    let base = &scripts[&def.id].base;
                    let has = match wanted {
                        (true, false) => base.start_of_turn.is_some(),
                        (false, true) => base.end_of_turn.is_some(),
                        _ => base.start_of_turn.is_some() || base.end_of_turn.is_some(),
                    };
                    assert_eq!(ids.contains(&def.id.as_str()), has, "{which}: {}", def.id);
                }
            }
        }

        #[test]
        fn s2_4_a_full_hand_burns_the_card_added_past_the_cap() {
            // The Radiant Cry adds two: with 9 cards left in hand after the play, the second burns.
            let mut hand = vec![json!({ "def": ID, "radiant": true })];
            hand.extend((0..9).map(|_| json!(FILLER)));
            let s = crate::under_kept(ID, script_v0_6_9n(), || played("filing-burn", json!(hand)));
            assert_eq!(s.hand(P1).len(), 10);
            let burned = s.last_events().iter().filter(|event| matches!(event, GameEvent::Burned { .. })).count();
            assert_eq!(burned, 1);
            assert_eq!(s.pile(P1, "graveyard").len(), 1);
        }

        #[test]
        fn neither_the_cry_nor_the_activate_declares_a_number_and_the_aura_keeps_extra_1_and_2() {
            crate::register_all();
            let def = crate::card_def(ID);
            let keys: Vec<&str> = def.params.iter().flatten().map(|param| param.key.as_str()).collect();
            assert_eq!(keys, ["extra"]);
            let param = &def.params.as_ref().expect("params")[0];
            assert_eq!((param.base, param.radiant), (1, 2));
        }

        #[test]
        fn the_cry_is_not_multiplied_by_the_aura() {
            // R821 multiplies Start of turn and End of turn effects, and a Cry is neither.
            let s = crate::under_kept(ID, script_v0_6_9n(), || played("filing-once", filing_hand(false)));
            assert_eq!(added(&s).len(), 1);
        }
    }

    mod through_v0_5_11o {
        use super::*;

        #[test]
        fn r767000_the_kept_script_has_the_aura_and_no_cry_on_both_faces() {
            let kept = script_v0_5_11o();
            assert!(kept.base.cry.is_none() && kept.radiant.cry.is_none());
            assert!(kept.base.turn_hook_extra.is_some() && kept.radiant.turn_hook_extra.is_some());
            for radiant_face in [false, true] {
                crate::under_kept(ID, kept.clone(), || {
                    let s = played("filing-kept", filing_hand(radiant_face));
                    assert!(added(&s).is_empty());
                });
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r821_three_runs_in_all() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [{ "def": ID, "radiant": true }], "field": [SHREDDER] })),
                "p2": side(json!({})),
            }));
            s.end_turn();
            s.expect_health(P2, 24);
        }

        #[test]
        fn r821_three_rush_tokens_at_the_start_of_your_turn() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [{ "def": ID, "radiant": true }, FARM] })),
                "p2": side(json!({})),
            }));
            s.start_turn();
            assert_eq!(units(&s, P1, RUSH_TOKEN), 3);
        }
    }
}
