//! #81 Radiant Saintess (SPEC §8.4 row 81; R22, R78, R177, R275, R276): Unit, Human, cost 1, Epic,
//! 2/2 → 4/4.
//!   Base:    "Reborn; Death: all your other units become Radiant"
//!   Radiant: "Reborn; Death: all your other units and every card in your hand become Radiant"
//!
//! THE RADIANT FACE WIDENS THE SCOPE (R275: "a broader scope"): her Death radiates your hand as well
//! as your board, the way #29 GIGA Glowy Jelly Bean radiates a hand — one `setRadiant` per card,
//! named by id, never a random pick. A hand is its owner's alone (§9.1), so whether each of those
//! cards was base-face is hidden from the opponent, and R177 is what keeps the cue from telling:
//! `setRadiant` reports a `radiantSet` for a named hidden card whether or not the flag changed, so
//! the opponent's stream holds one redacted cue per hand card either way.
//!
//! Reborn is PRINTED on both faces (`catalog.json` core-081 `keywords`), which `layers.faceOf`
//! reads off the instance's radiant flag, so no line of script grants it. Nothing here restates a
//! stat, a cost or a keyword (the `def` above is the only source of those).
//!
//! R22 "Radiant on the field": "Base layer swaps, damage and buffs stay, Cry does not re-fire;
//! Saintess includes itself". `setRadiant` is exactly that — it sets the instance's `radiant` flag
//! and nothing else (`effects/radiant.ts`), so §10.4's printed-stat layer swaps on the next read
//! while layer 4's buffs and the instance's damage are untouched, and no card re-enters the field,
//! so no Cry fires again. R22's "Saintess includes itself" was about her old Cry, which is gone; her
//! Death never includes her (below). "Her Reborn body fires Death again" (§8's Engine cell) is R8
//! ("Death fires on both deaths") plus §4.5 step 4, both the state check's, not this file's.
//!
//! R78 is why Death does NOT name `self`. Leaving the field resets an instance and "effects that
//! react to a card leaving read its last-known state from just before it left": `stateCheck` moves
//! the dying unit to the graveyard at step 1 and runs the Death hook at step 3 off a SNAPSHOT of the
//! instance, so `ctx.self` is a detached copy of a card that is no longer a unit on the field and
//! `ctx.controller` is the controller it had as it died. "All your units" is therefore the units
//! still standing, which is what `activeUnitsOf(state, ctx.controller)` returns. The radiant flag
//! persists in every zone (R78), so a Reborn body comes back already Radiant and needs no help.
//!
//! R13 "Stack dormancy": cards under a Stack are not on the field, so they are not "your units".
//! `activeUnitsOf` reads the top of each pile only, which is the same rule (§3.2), and R23 leaves
//! Make Radiant legal on an Immutable unit, so no keyword filter belongs here either.
//!
//! R64 (a Reborn unit reserves its zone) and R83 (the Reborn body takes the current turn as its
//! `summonedTurn`, so it is summoning sick) are both the engine's; this card only sets flags.

use indexmap::IndexSet;
use jackioh_engine::effects::set_radiant;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-081";

/// "All your units become Radiant", as one `setRadiant` per unit named by instance id — nobody chose
/// these cards, so there is no `TargetSpec` to resolve (R81). `includeSelf` is R22's ruling for the
/// Cry; the ids are de-duplicated so the effect list has one entry per card even when `self` is
/// already standing in a unit zone.
fn radiate_your_units(ctx: &EffectContext<'_>, include_self: bool) -> Vec<Effect> {
    let mut ids: Vec<String> = Vec::new();
    if include_self {
        if let Some(self_) = &ctx.self_ {
            ids.push(self_.id.clone());
        }
    }
    ids.extend(active_units_of(ctx.state, ctx.controller).iter().map(|unit| unit.id.clone()));
    ids.into_iter()
        .collect::<IndexSet<String>>()
        .into_iter()
        .map(|instance_id| set_radiant(json_as(json!({ "instanceId": instance_id }))))
        .collect()
}

/// R78: by the time Death runs she is in the graveyard, so "your OTHER units" — which is what
/// the card now says — is simply everyone left standing. The `false` below is that word.
///
/// THERE IS NO CRY. She had "Cry and Death" and the Cry was cut for burst: playing her turned the
/// board Radiant the instant she landed, including herself (she arrived 4/4 with Reborn for one
/// mana). On Death alone the same effect has to be paid for with her body, which is the cost the
/// card was missing. `includeSelf` stays a parameter because Death is the only caller and passing
/// `false` at the one call site is what R78 is about.
fn death(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    radiate_your_units(ctx, false)
}

/// "…and every card in your hand": the controller's hand, read through `zoneCards` (a copy) and named
/// card by card, as #29 does. Every card, so no rng is drawn (§9.3), and a card already Radiant is
/// still named — `setRadiant` cues it without changing it (R177), which is what hides the hand's
/// faces from the opponent.
fn radiate_your_hand(ctx: &EffectContext<'_>) -> Vec<Effect> {
    zone_cards(ctx.state, ctx.controller, OffFieldZone::Hand)
        .iter()
        .map(|card| set_radiant(json_as(json!({ "instanceId": card.id }))))
        .collect()
}

/// Radiant Death: the board first, in the order the text names it, then the hand.
fn radiant_death(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let mut effects = radiate_your_units(ctx, false);
    effects.extend(radiate_your_hand(ctx));
    effects
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            death: Some(hook(death)),
            ..Script::default()
        },
        radiant: Script {
            death: Some(hook(radiant_death)),
            ..Script::default()
        },
    }
}

// #81 Radiant Saintess (SPEC §8.4 row 81; R13, R22, R78, R177, R275).
//
// BUILD M4-T4's must-pass row: "Death makes every other unit you control Radiant; no Reborn, so she
// dies once; radiant also every card in your hand, hidden from the opponent (R177)".
//   Base:    "Death: Make your other Units Radiant."
//   Radiant: "Death: Make your other Units and every card in your hand Radiant."
// (R275's broader scope). Her old Cry, which radiated the board as she landed, is cut (§8's row),
// and patch v0.1.1 took Reborn off both faces.
//
// Two fixtures do all the work:
//   - #11 Tempo Timmy (3/3 → 6/6, Rush + First Strike, cost 1) has an EMPTY script, so a stat
//     change on it can only be the radiant face swapping in — it is the observable for "became
//     Radiant" throughout.
//   - #44 True Strike ("deal 4 damage to a target, ignoring Armor") is how a Saintess is killed
//     mid-test without waiting for combat: 4 damage kills a 4/4 body and the 1-health Reborn body.
//
// A Saintess placed by the setup with damage equal to her health dies inside the setup's own state
// check, which is the cheapest way to fire a Death hook: the harness records no events for setup,
// so those cases assert the resulting state rather than the log.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// #11 Tempo Timmy: no script, 3/3 base and 6/6 radiant, so its stats report its face.
    const TIMMY: &str = "core-011";
    const SAINTESS: &str = "core-081";
    /// #44 True Strike: 4 damage to any target, which is exactly a 4/4 Saintess.
    const TRUE_STRIKE: &str = "core-044";
    /// #5 Stockpile and #2 Bigot: two more hand cards to watch the radiant Death reach.
    const STOCKPILE: &str = "core-005";
    const BIGOT: &str = "core-002";
    /// #15 Me and Mr Token: a Unit whose Cry makes fresh base-face Rush Tokens (3 on its radiant face).
    const ME_AND_MR_TOKEN: &str = "core-015";
    const RUSH_TOKEN: &str = "core-t-rush";
    /// #92 Felinor Fiender, the Stack unit: on top of a pile, the card under it lies dormant (§3.2).
    const FIENDER: &str = "core-092";

    /// TS `s.expectStats(card ?? fallback, stats)`: the card when there is one, else the reference by
    /// catalog id.
    fn expect_stats_or(s: &mut Scenario, card: Option<CardInstance>, fallback: &str, stats: Value) {
        match card {
            Some(card) => {
                s.expect_stats(&card, stats);
            }
            None => {
                s.expect_stats(fallback, stats);
            }
        }
    }

    /// True Strike on the Saintess in lane 1: her Death runs in the state check that follows.
    fn strike_saintess(s: &mut Scenario) -> &mut Scenario {
        let saintess_id = match s.unit(P1, 1) {
            Some(saintess) if saintess.def_id == SAINTESS => saintess.id.clone(),
            _ => panic!("the Saintess should stand in p1's lane 1"),
        };
        s.play(TRUE_STRIKE, json!({ "targets": [{ "pick": "instance", "instanceId": saintess_id }] }))
    }

    /// R13 on either face: the Saintess in lane 1 and, in lane 2, a §3.2 Stack pile — a Felinor Fiender
    /// on top of a dormant Tempo Timmy. True Strike kills her, and her Death reaches the top of the pile
    /// and not the card under it: a card dormant under a Stack is not on the field (R13), so it is not
    /// one of "your other units".
    fn expect_stack_spared(radiant_face: bool) {
        let mut s = scenario(json!({
            "seed": "saintess-stack",
            "p1": {
                "field": [{ "def": SAINTESS, "radiant": radiant_face }, TIMMY, { "def": FIENDER, "stack": true }],
                "hand": [TRUE_STRIKE, STOCKPILE]
            },
            "p2": { "hand": [STOCKPILE] }
        }));
        let buried = s.card(TIMMY).clone();
        let fiender = s.card(FIENDER).clone();
        assert_eq!(
            s.unit(P1, 2).map(|unit| unit.id.clone()),
            Some(fiender.id.clone()),
            "the Fiender is on top of lane 2"
        );

        strike_saintess(&mut s);

        assert!(s.card(&fiender).radiant, "the Fiender on top of the pile");
        // Still buried, still on its base face, and nothing was said about it.
        assert_eq!(s.unit(P1, 2).map(|unit| unit.id.clone()), Some(fiender.id.clone()));
        s.expect_in_zone(&buried, "field");
        assert!(!s.card(&buried).radiant, "the Timmy dormant under it");
        assert_eq!(
            s.events()
                .iter()
                .filter(|event| matches!(event, GameEvent::RadiantSet { instance_id, .. } if *instance_id == buried.id))
                .count(),
            0
        );
    }

    mod n81_radiant_saintess_base {
        use super::*;

        #[test]
        fn s8_the_cry_is_gone_playing_her_radiates_nothing_herself_included() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "saintess",
                "p1": { "hand": [SAINTESS], "field": [TIMMY] },
                "p2": { "field": [TIMMY] }
            }));
            s.play(SAINTESS, json!({ "zone": 2 }));

            // She used to arrive 4/4 with Reborn and turn the board up on the spot for one mana. She now
            // lands as the 2/2 she is printed as, and nothing else moves until she dies.
            assert!(!s.card(SAINTESS).radiant);
            s.expect_stats(SAINTESS, json!({ "attack": 2, "health": 2, "maxHealth": 2 }));
            assert_eq!(s.unit(P1, 1).map(|unit| unit.radiant), Some(false));
            s.expect_events(json!(["cardPlayed", "summoned"]));
        }

        #[test]
        fn patch_v0_1_1_the_base_face_prints_no_keywords_reborn_included() {
            crate::register_all();
            let mut s = scenario(json!({ "seed": "saintess", "p1": { "hand": [SAINTESS] } }));
            s.play(SAINTESS, json!({ "zone": 1 }));

            assert!(!s.card(SAINTESS).radiant);
            assert_eq!(keywords_of(s.state(), s.card(SAINTESS)), Vec::<Keyword>::new());
        }

        #[test]
        fn s8_your_units_is_yours_the_opponent_s_units_are_untouched() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "saintess",
                "p1": { "hand": [SAINTESS] },
                "p2": { "field": [TIMMY] }
            }));
            s.play(SAINTESS, json!({ "zone": 1 }));

            let enemy = s.unit(P2, 1);
            assert_eq!(enemy.as_ref().map(|unit| unit.radiant), Some(false));
            expect_stats_or(&mut s, enemy, SAINTESS, json!({ "attack": 3, "health": 3, "maxHealth": 3 }));
        }

        #[test]
        fn s8_units_a_backrow_card_of_yours_is_not_a_unit_and_stays_as_it_is() {
            crate::register_all();
            // #73 Anti-oneshot Armor is a Field Spell, so it sits in the backrow and is never radiated.
            let mut s = scenario(json!({
                "seed": "saintess",
                "p1": { "hand": [SAINTESS], "backrow": [{ "def": "core-073", "lane": 1 }] }
            }));
            s.play(SAINTESS, json!({ "zone": 1 }));

            assert_eq!(s.backrow(P1, 1).map(|card| card.radiant), Some(false));
        }

        #[test]
        fn r22_the_base_layer_swaps_while_damage_and_buffs_stay_a_damaged_unit_keeps_its_damage() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "saintess",
                // She dies in the setup's state check (§4.5 step 3), which runs her Death.
                "p1": { "field": [{ "def": SAINTESS, "damage": 2 }, { "def": TIMMY, "damage": 1 }] }
            }));

            // 3/3 with 1 damage becomes 6/6 with 1 damage — max health up, the damage untouched.
            let timmy = s.unit(P1, 2);
            expect_stats_or(&mut s, timmy, TIMMY, json!({ "attack": 6, "health": 5, "maxHealth": 6 }));
        }

        #[test]
        fn s6_3_a_unit_that_is_already_radiant_is_untouched_and_emits_nothing_for_it() {
            crate::register_all();
            let s = scenario(json!({
                "seed": "saintess",
                "p1": { "field": [{ "def": SAINTESS, "damage": 2 }, { "def": TIMMY, "radiant": true }] }
            }));

            // The only other unit was already Radiant, so her Death has nothing left to set.
            assert_eq!(
                s.last_events()
                    .iter()
                    .filter(|event| event.event_type() == GameEventType::RadiantSet)
                    .count(),
                0
            );
            assert_eq!(s.unit(P1, 2).map(|unit| unit.radiant), Some(true));
        }

        #[test]
        fn death_radiates_the_units_still_standing_and_with_no_reborn_she_stays_in_the_graveyard() {
            crate::register_all();
            // 2/2 with 2 damage dies in the setup's state check, which runs her Death hook (§4.5 step 3).
            let mut s = scenario(json!({
                "seed": "saintess",
                "p1": { "field": [{ "def": SAINTESS, "damage": 2 }, TIMMY] }
            }));

            assert_eq!(s.unit(P1, 2).map(|unit| unit.radiant), Some(true));
            let timmy = s.unit(P1, 2);
            expect_stats_or(&mut s, timmy, TIMMY, json!({ "attack": 6, "health": 6, "maxHealth": 6 }));
            s.expect_in_zone(SAINTESS, "graveyard");
            assert!(s.unit(P1, 1).is_none());
        }

        #[test]
        fn r78_death_does_not_include_herself_she_reaches_the_graveyard_non_radiant() {
            crate::register_all();
            // R78 has her leave the field before the Death hook runs, so "your other Units" never includes
            // her — the text says what the rule always did.
            let s = scenario(json!({
                "seed": "saintess",
                "p1": { "field": [{ "def": SAINTESS, "damage": 2 }, TIMMY] }
            }));

            assert!(!s.card(SAINTESS).radiant);
            assert_eq!(
                s.pile(P1, "graveyard").iter().map(|card| card.def_id.clone()).collect::<Vec<_>>(),
                vec![SAINTESS.to_string()]
            );
        }

        #[test]
        fn patch_v0_1_1_played_base_she_dies_once_the_first_true_strike_puts_her_in_the_graveyard_for_good() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "saintess",
                "p1": { "hand": [SAINTESS, TRUE_STRIKE, TIMMY, STOCKPILE] }
            }));
            s.play(SAINTESS, json!({ "zone": 1 }));

            let saintess = s.card(SAINTESS).clone();
            s.expect_stats(&saintess, json!({ "attack": 2, "health": 2, "maxHealth": 2 }));
            s.play(TIMMY, json!({ "zone": 2 }));

            s.play(TRUE_STRIKE, json!({ "targets": [{ "pick": "instance", "instanceId": saintess.id }] }));

            s.expect_in_zone(&saintess, "graveyard");
            assert!(s.unit(P1, 1).is_none());
            assert_eq!(s.unit(P1, 2).map(|unit| unit.radiant), Some(true), "her Death radiates the board");
        }

        #[test]
        fn s8_the_base_death_reaches_the_board_only_the_hand_keeps_its_faces() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "saintess-base-hand",
                "p1": { "field": [SAINTESS, TIMMY], "hand": [TRUE_STRIKE, TIMMY, STOCKPILE] },
                "p2": { "hand": [STOCKPILE] }
            }));
            strike_saintess(&mut s);

            assert_eq!(s.unit(P1, 2).map(|unit| unit.radiant), Some(true), "the other unit");
            assert_eq!(s.hand(P1).iter().map(|card| card.radiant).collect::<Vec<_>>(), vec![false, false]);
        }

        #[test]
        fn r13_a_card_dormant_under_a_stack_is_not_one_of_your_units_death_turns_up_the_top_of_the_pile_only() {
            crate::register_all();
            expect_stack_spared(false);
        }
    }

    mod n81_radiant_saintess_radiant {
        use super::*;

        #[test]
        fn the_radiant_face_is_a_4_4_with_no_reborn_and_it_radiates_nothing_on_arrival_either() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "saintess",
                "p1": { "hand": [{ "def": SAINTESS, "radiant": true }, TIMMY], "field": [TIMMY] }
            }));
            s.play(SAINTESS, json!({ "zone": 2 }));

            s.expect_stats(SAINTESS, json!({ "attack": 4, "health": 4, "maxHealth": 4 }));
            assert_eq!(keywords_of(s.state(), s.card(SAINTESS)), Vec::<Keyword>::new());
            // Her script is Death alone, so neither the ally nor the hand moves until she dies.
            assert_eq!(s.unit(P1, 1).map(|unit| unit.radiant), Some(false));
            assert_eq!(s.hand(P1).iter().map(|card| card.radiant).collect::<Vec<_>>(), vec![false]);
        }

        #[test]
        fn r275_death_every_other_unit_and_every_card_in_your_hand_become_radiant() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "saintess-radiant-hand",
                "p1": {
                    "field": [{ "def": SAINTESS, "radiant": true }, TIMMY],
                    "hand": [TRUE_STRIKE, TIMMY, STOCKPILE, BIGOT]
                },
                "p2": { "field": [TIMMY], "hand": [STOCKPILE, BIGOT] }
            }));
            strike_saintess(&mut s);

            // The board, as on the base face.
            assert_eq!(s.unit(P1, 2).map(|unit| unit.radiant), Some(true));
            let timmy = s.unit(P1, 2);
            expect_stats_or(&mut s, timmy, TIMMY, json!({ "attack": 6, "health": 6, "maxHealth": 6 }));
            // Every card left in the hand (True Strike was resolving, so it is not one of them).
            assert_eq!(
                s.hand(P1).iter().map(|card| card.def_id.clone()).collect::<Vec<_>>(),
                vec![TIMMY, STOCKPILE, BIGOT]
            );
            assert!(s.hand(P1).iter().all(|card| card.radiant));
            // A Radiant hand card reads its radiant face: Timmy is a 6/6 in hand now.
            let hand_timmy = s.hand(P1).into_iter().find(|card| card.def_id == TIMMY);
            expect_stats_or(&mut s, hand_timmy, TIMMY, json!({ "attack": 6, "maxHealth": 6 }));
            // "your": the opponent's board and hand are untouched.
            assert_eq!(s.unit(P2, 1).map(|unit| unit.radiant), Some(false));
            assert!(!s.hand(P2).iter().any(|card| card.radiant));
            // R78: she is not one of "your other Units"; with no Reborn she stays in the graveyard, still
            // Radiant (the flag persists in every zone).
            s.expect_in_zone(SAINTESS, "graveyard");
            assert!(s.card(SAINTESS).radiant);
        }

        #[test]
        fn r13_on_the_radiant_face_too_a_card_dormant_under_a_stack_is_not_one_of_your_units() {
            crate::register_all();
            expect_stack_spared(true);
        }

        #[test]
        fn r275_an_empty_hand_leaves_the_death_to_the_board_alone() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "saintess-radiant-empty-hand",
                "p1": { "field": [{ "def": SAINTESS, "radiant": true }, TIMMY], "hand": [TRUE_STRIKE] },
                "p2": { "hand": [STOCKPILE] }
            }));
            strike_saintess(&mut s);

            assert_eq!(s.hand(P1).len(), 0);
            assert_eq!(s.unit(P1, 2).map(|unit| unit.radiant), Some(true));
        }

        #[test]
        fn r177_the_opponent_s_view_does_not_learn_which_of_her_owner_s_hand_cards_were_base_face() {
            crate::register_all();
            // Two games that differ only in the face p1's hand Timmy had while p2 could not read it. After
            // her Death both hands are wholly Radiant; a cue only for the cards that changed would count,
            // for p2, how many were Radiant before.
            let game = |timmy_radiant: bool| -> Scenario {
                let mut s = scenario(json!({
                    "seed": "saintess-r177",
                    "p1": {
                        "field": [{ "def": SAINTESS, "radiant": true }],
                        "hand": [TRUE_STRIKE, { "def": TIMMY, "radiant": timmy_radiant }, STOCKPILE],
                        "library": [BIGOT]
                    },
                    "p2": { "hand": [STOCKPILE], "library": [BIGOT] }
                }));
                strike_saintess(&mut s);
                s
            };
            let was_base = game(false);
            let was_radiant = game(true);

            for s in [&was_base, &was_radiant] {
                assert!(s.hand(P1).iter().all(|card| card.radiant));
                // One cue per hand card in both games, named or not changed (R177).
                let cues = s
                    .view(P2)
                    .events
                    .iter()
                    .filter(|event| event.event_type() == GameEventType::RadiantSet)
                    .count();
                assert_eq!(cues, 2);
            }
            assert_eq!(
                was_radiant.view(P2).events.iter().map(GameEvent::event_type).collect::<Vec<_>>(),
                was_base.view(P2).events.iter().map(GameEvent::event_type).collect::<Vec<_>>()
            );
            assert_eq!(was_radiant.view(P2), was_base.view(P2));
        }

        #[test]
        fn patch_v0_1_1_radiant_she_dies_once_too_and_a_later_arrival_is_never_turned_up() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "saintess",
                "p1": { "hand": [{ "def": SAINTESS, "radiant": true }, TRUE_STRIKE, ME_AND_MR_TOKEN, STOCKPILE] }
            }));
            s.play(SAINTESS, json!({ "zone": 1 }));

            let saintess = s.card(SAINTESS).clone();
            s.play(TRUE_STRIKE, json!({ "targets": [{ "pick": "instance", "instanceId": saintess.id }] }));
            s.expect_in_zone(&saintess, "graveyard");
            // Her one Death reached the hand.
            assert!(s.card(ME_AND_MR_TOKEN).radiant);

            // The tokens its radiant Cry makes arrive after that Death, and no second one comes.
            s.play(ME_AND_MR_TOKEN, json!({ "zone": 2 }));
            // Her lane is free again, so the three tokens take lanes 1, 3 and 4 (R64).
            let tokens: Vec<Option<CardInstance>> = [1, 3, 4].into_iter().map(|lane| s.unit(P1, lane)).collect();
            assert_eq!(
                tokens.iter().map(|token| token.as_ref().map(|t| t.def_id.clone())).collect::<Vec<_>>(),
                vec![Some(RUSH_TOKEN.to_string()); 3]
            );
            assert_eq!(
                tokens.iter().map(|token| token.as_ref().map(|t| t.radiant)).collect::<Vec<_>>(),
                vec![Some(false); 3]
            );
        }
    }
}
