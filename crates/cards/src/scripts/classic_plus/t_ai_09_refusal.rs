//! T-AI-9 Refusal (SPEC §8.7 row T-AI-9, §7, B8). (1) Trap, AI, Token.
//!   Base:    "Activates when your opponent plays a Spell that targets one of your Units: Counter it."
//!   Radiant: "Activates when your opponent plays a Spell that targets you or one of your cards: Counter
//!            it. Draw 1."
//!   Engine:  "Counter (§6.3), in §10.5's announce window (`cardAnnounced`): a Spell whose declared
//!            targets (R81) include a Unit you control (Radiant: your hero or any card you control or
//!            hold). The countered Spell goes to its owner's graveyard, treated as never played: no
//!            `cardPlayed`, no counts, no Echo repeats; its mana stays spent. Picks made during
//!            resolution are not targets of the play. The Radiant's draw is its controller's. Tunes: none."
//!
//! E1, R448: the announce names the play's declared targets (a card by its id, a hero as `hero-<p>`), so
//! the condition is a read of those ids against your side as the window opens — your units on the field
//! (a carried Unit included, R446), or on the Radiant face your hero, your field and your hand. It lives
//! in `when` (R99), so any other Spell, a Field Spell, a Trap or a Unit leaves it armed and face-down; a
//! cast is announced like a play (R70), so a cast Spell that declared one of yours sets it off too.

use jackioh_engine::effects::{counter_play, draw};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-t-ai-09";

/// §8.7: the Radiant face's "Draw 1". An AI card declares no params (B8).
const RADIANT_DRAW: i32 = 1;

/// TS `typeof yourUnit`: which of the announced ids reach your side. A plain function, so both the
/// trigger's `when` and its `run` can carry it.
type Reaches = fn(&GameState, PlayerId, &str) -> bool;

/// "One of your Units": a Unit of yours on the field, as the announce names it.
fn your_unit(state: &GameState, you: PlayerId, id: &str) -> bool {
    active_units_of(state, you).iter().any(|unit| unit.id == id)
}

/// "You or one of your cards": your hero, or a card you control on the field or hold in your hand.
fn you_or_yours(state: &GameState, you: PlayerId, id: &str) -> bool {
    if id == format!("hero-{you}") {
        return true;
    }
    match find_instance(state, id) {
        Some(card) => matches!(card.zone.z(), ZoneName::Field | ZoneName::Hand) && card.controller == you,
        None => false,
    }
}

/// The opponent's announced Spell whose declared targets `reaches` one of yours, or `None`.
fn refused(event: &GameEvent, state: &GameState, you: PlayerId, reaches: Reaches) -> Option<String> {
    let GameEvent::CardAnnounced {
        player,
        card_type,
        targets,
        instance_id,
        ..
    } = event
    else {
        return None;
    };
    if *player == you || *card_type != CardType::Spell {
        return None;
    }
    if targets.iter().any(|id| reaches(state, you, id)) {
        Some(instance_id.clone())
    } else {
        None
    }
}

/// TS `TrapTrigger`: part 1's `TriggerDef`, which carries the `when` (R99).
fn refusal(reaches: Reaches, drawn: i32) -> TriggerDef {
    TriggerDef::new("refusal", &[GameEventType::CardAnnounced], move |ctx, event| {
        let countered = refused(event, &*ctx.state, ctx.controller, reaches).unwrap_or_default();
        let mut effects = vec![counter_play(json_as(json!({
            "target": { "of": "instance", "instanceId": countered },
        })))];
        if drawn > 0 {
            effects.push(draw(json_as(json!({ "count": drawn }))));
        }
        effects
    })
    .with_when(move |ctx, event| refused(event, &*ctx.state, ctx.controller, reaches).is_some())
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            triggers: vec![refusal(your_unit, 0)],
            ..Script::default()
        },
        radiant: Script {
            triggers: vec![refusal(you_or_yours, RADIANT_DRAW)],
            ..Script::default()
        },
    }
}

// T-AI-9 Refusal — SPEC §8.7 row T-AI-9, BUILD M9 Classic+ row T-AI-9: "Face-down Trap in the announce
// window of §10.5 (the price paid, the card not yet moved): when the opponent plays or casts a Spell
// whose declared targets include one of your Units, it Counters it: the Spell goes to its owner's
// graveyard unresolved and is treated as never played (no `cardPlayed`, no count for Combo,
// Quickstriker, Ceaseless Void or the turn log; its Echo repeats never happen), the mana and Tributes
// staying spent; `countered` is public; a Spell with no declared target, a Field Spell or a Trap never
// sets it off; with two Refusals the first cancels and the second stays set; hidden until it fires
// (R33); radiant also when the Spell targets you or any card of yours, and you draw 1".
//
// Every case sets the trap face-down in p1's backrow and makes p2 the active player.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const REFUSAL: &str = "classicplus-t-ai-09";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy target Unit.
    const TRUE_STRIKE: &str = "core-044"; // (1) Spell: Pierce. Deal 4 damage. Exile this.
    const MAGIC_JAMMED: &str = "core-036"; // (1) Spell: Destroy target backrow card. Lock its zone.
    const FLOOD: &str = "core-017"; // (4) Spell: Bounce all Units.
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
    const TWINSPELL: &str = "core-079"; // (2) Field Spell: your next Spell gains Echo +1.
    const BEAR: &str = "core-060"; // (1) Trap
    const COUNTERSPELL_TEST_FILLER: &str = "core-011"; // (1) Unit
    const SCEPTER: &str = "classic-007"; // (1) Field Spell: Cry: exile a (1) Cost or less Spell from your hand. Activate: cast a copy of it.
    const FLAME: &str = "classic-016"; // (1) Spell, Book: Deal 4 damage.

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    use crate::js;

    /// TS's `{ ...base, ...extra }` on a side setup.
    fn spread(base: Value, extra: &Value) -> Value {
        let mut out = base;
        if let (Some(into), Some(from)) = (out.as_object_mut(), extra.as_object()) {
            for (key, value) in from {
                into.insert(key.clone(), value.clone());
            }
        }
        out
    }

    fn refusal(radiant_face: bool, lane: i32) -> Value {
        json!({ "def": REFUSAL, "radiant": radiant_face, "faceUp": false, "lane": lane })
    }

    fn setup(p1: Value, p2: Value, radiant_face: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "active": "p2",
            "p1": spread(
                json!({
                    "hand": [VANILLA],
                    "field": [MENACE],
                    "backrow": [refusal(radiant_face, 2)],
                    "library": [VANILLA, STOCKPILE],
                }),
                &p1,
            ),
            "p2": spread(
                json!({
                    "hand": [HIT_JOB, TRUE_STRIKE, STOCKPILE, VANILLA],
                    "field": [VANILLA],
                    "library": [VANILLA, VANILLA, VANILLA],
                    "mana": 8,
                }),
                &p2,
            ),
        }))
    }

    fn at(s: &Scenario, player: PlayerId, lane: i32) -> Value {
        json!([{ "pick": "instance", "instanceId": s.unit(player, lane).map(|unit| unit.id).unwrap_or_default() }])
    }

    fn p1_hero() -> Value {
        json!([{ "pick": "hero", "player": "p1" }])
    }

    fn count(events: &[GameEvent], kind: &str) -> usize {
        events.iter().filter(|event| js(*event)["type"] == kind).count()
    }

    fn view_text(s: &Scenario, player: PlayerId) -> String {
        serde_json::to_string(&s.view(player)).expect("serialises")
    }

    mod t_ai_9_refusal {
        use super::*;

        #[test]
        fn is_a_1_ai_trap_token_whose_condition_lives_in_when_r99() {
            crate::register_all();
            let def = crate::card_def(super::super::ID);
            assert_eq!(def.type_, CardType::Trap);
            assert_eq!(js(&def.cost), json!(1));
            assert_eq!(js(&def.tags), json!(["AI", "Token"]));
            let scripts = super::super::script();
            assert_eq!(
                scripts.base.triggers.first().map(|trigger| trigger.on.clone()),
                Some(vec![GameEventType::CardAnnounced])
            );
            assert!(scripts.radiant.triggers.first().is_some_and(|trigger| trigger.when.is_some()));
        }

        mod base {
            use super::*;

            #[test]
            fn r448_counters_a_spell_that_targets_one_of_your_units_it_never_resolves_and_goes_to_its_owners_graveyard() {
                let mut s = setup(json!({}), json!({}), false);
                let menace = s.unit(P1, 1).map(|unit| unit.id).unwrap_or_default();
                let hit_job = s.card(HIT_JOB).clone();
                let targets = at(&s, P1, 1);
                s.play(HIT_JOB, json!({ "targets": targets }));

                s.expect_in_zone(&hit_job, "graveyard");
                s.expect_in_zone(menace.as_str(), "field");
                s.expect_in_zone(REFUSAL, "graveyard");
                s.expect_events(json!(["cardAnnounced", "trapFired", "countered"]));
                assert_eq!(count(s.last_events(), "cardPlayed"), 0);
                assert_eq!(count(s.last_events(), "cardResolved"), 0);
            }

            #[test]
            fn r448_treated_as_never_played_the_turns_and_the_games_counts_dont_move_and_its_mana_stays_spent() {
                let mut s = setup(json!({}), json!({}), false);
                let turn = s.state().players.p2.turn_log.cards_played;
                let game = s.state().counters.played;
                let targets = at(&s, P1, 1);
                s.play(HIT_JOB, json!({ "targets": targets }));
                assert_eq!(s.state().players.p2.turn_log.cards_played, turn);
                assert_eq!(s.state().counters.played, game);
                s.expect_mana(P2, 5);
            }

            #[test]
            fn r448_a_countered_spell_makes_no_echo_repeats() {
                let mut s = setup(json!({}), json!({ "backrow": [{ "def": TWINSPELL, "faceUp": true }] }), false);
                let targets = at(&s, P1, 1);
                s.play(HIT_JOB, json!({ "targets": targets }));
                assert_eq!(count(s.last_events(), "cardResolved"), 0);
                assert_eq!(count(s.last_events(), "destroyed"), 0);
            }

            #[test]
            fn r33_hidden_until_it_fires_countered_is_public_to_both_players() {
                let mut s = setup(json!({}), json!({}), false);
                assert!(!view_text(&s, P2).contains(REFUSAL));
                let targets = at(&s, P1, 1);
                s.play(HIT_JOB, json!({ "targets": targets }));
                for player in [P1, P2] {
                    let countered: Vec<Value> = js(&s.view(player))["events"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|event| event["type"] == "countered")
                        .collect();
                    assert_eq!(countered.len(), 1);
                    assert_eq!(countered[0]["defId"], json!(HIT_JOB));
                    assert_eq!(countered[0]["to"], json!("graveyard"));
                }
            }

            #[test]
            fn r81_a_spell_that_targets_your_hero_or_one_of_their_own_units_leaves_it_set() {
                let mut s = setup(json!({}), json!({}), false);
                s.play(TRUE_STRIKE, json!({ "targets": p1_hero() }));
                s.expect_health(P1, 26);
                let targets = at(&s, P2, 1);
                s.play(HIT_JOB, json!({ "targets": targets }));
                assert!(s.unit(P2, 1).is_none());
                assert_eq!(s.card(REFUSAL).face_up, Some(false));
                assert_eq!(count(s.events(), "countered"), 0);
            }

            #[test]
            fn r81_a_spell_with_no_declared_target_never_sets_it_off_even_one_that_reaches_your_units() {
                let mut s = setup(json!({}), json!({ "hand": [FLOOD, STOCKPILE, VANILLA] }), false);
                s.play(STOCKPILE, json!({}));
                s.play(FLOOD, json!({}));
                // Flood bounced every Unit, p1's included: it named none of them.
                assert!(s.unit(P1, 1).is_none());
                assert_eq!(s.card(REFUSAL).face_up, Some(false));
                assert_eq!(count(s.events(), "countered"), 0);
            }

            #[test]
            fn a_field_spell_a_trap_and_a_unit_never_set_it_off() {
                let mut s = setup(json!({}), json!({ "hand": [TWINSPELL, BEAR, COUNTERSPELL_TEST_FILLER] }), false);
                s.play(TWINSPELL, json!({ "zone": 1 }))
                    .play(BEAR, json!({ "zone": 2 }))
                    .play(COUNTERSPELL_TEST_FILLER, json!({ "zone": 2 }));
                assert_eq!(count(s.events(), "countered"), 0);
                assert_eq!(s.card(REFUSAL).face_up, Some(false));
            }

            #[test]
            fn your_own_spells_never_set_it_off() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [HIT_JOB, VANILLA], "field": [MENACE], "backrow": [refusal(false, 2)], "library": [VANILLA], "mana": 8 },
                    "p2": { "hand": [VANILLA], "library": [VANILLA] },
                }));
                let targets = at(&s, P1, 1);
                s.play(HIT_JOB, json!({ "targets": targets }));
                assert!(s.unit(P1, 1).is_none());
                assert_eq!(count(s.events(), "countered"), 0);
            }

            #[test]
            fn r70_r448_a_cast_spell_is_announced_like_a_play_an_infini_scepters_copy_aimed_at_your_unit_is_countered() {
                let mut s = setup(json!({}), json!({ "hand": [SCEPTER, FLAME, VANILLA] }), false);
                let flame = s.card(FLAME).id.clone();
                s.play(SCEPTER, json!({ "zone": 1, "targets": [{ "pick": "instance", "instanceId": flame }] }));
                s.activate(SCEPTER, json!({}));
                let targets = at(&s, P1, 1);
                s.answer(targets);
                assert_eq!(count(s.events(), "countered"), 1);
                s.expect_in_zone(REFUSAL, "graveyard");
                let unit = s.unit(P1, 1).map(|unit| unit.id).unwrap_or_default();
                s.expect_stats(unit.as_str(), json!({ "health": 9 }));
                assert!(s.pile(P2, "graveyard").iter().any(|card| card.def_id == FLAME));
            }

            #[test]
            fn r81_a_pick_made_while_it_resolves_is_no_target_of_the_play_an_echo_repeat_aimed_at_your_unit_leaves_it_set() {
                let mut s = setup(json!({}), json!({ "backrow": [{ "def": TWINSPELL, "faceUp": true }] }), false);
                s.play(TRUE_STRIKE, json!({ "targets": p1_hero() }));
                assert_eq!(s.state().pending.as_ref().map(|pending| pending.player_id), Some(P2));
                let targets = at(&s, P1, 1);
                s.answer(targets);
                assert_eq!(count(s.events(), "countered"), 0);
                assert_eq!(s.card(REFUSAL).face_up, Some(false));
                s.expect_health(P1, 26);
                assert_eq!(s.unit(P1, 1).map(|unit| unit.damage), Some(4));
            }

            #[test]
            fn r448_with_two_refusals_the_first_cancels_the_spell_and_the_second_stays_set() {
                let mut s = setup(json!({ "backrow": [refusal(false, 2), refusal(false, 3)] }), json!({}), false);
                let targets = at(&s, P1, 1);
                s.play(HIT_JOB, json!({ "targets": targets }));
                assert_eq!(count(s.events(), "countered"), 1);
                assert_eq!(count(s.events(), "trapFired"), 1);
                assert!(s.backrow(P1, 2).is_none());
                assert_eq!(s.backrow(P1, 3).and_then(|card| card.face_up), Some(false));
            }

            #[test]
            fn base_a_spell_aimed_at_one_of_your_backrow_cards_leaves_it_set() {
                let mut s = setup(
                    json!({ "backrow": [refusal(false, 2), { "def": TWINSPELL, "faceUp": true, "lane": 3 }] }),
                    json!({ "hand": [MAGIC_JAMMED, VANILLA] }),
                    false,
                );
                let jammed = s.backrow(P1, 3).map(|card| card.id).unwrap_or_default();
                s.play(MAGIC_JAMMED, json!({ "targets": [{ "pick": "instance", "instanceId": jammed }] }));
                assert_eq!(count(s.events(), "countered"), 0);
                s.expect_in_zone(jammed.as_str(), "graveyard");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r448_counters_a_spell_that_targets_your_hero_and_you_draw_1() {
                let mut s = setup(json!({}), json!({}), true);
                let hand = s.hand(P1).len();
                s.play(TRUE_STRIKE, json!({ "targets": p1_hero() }));
                s.expect_health(P1, 30);
                s.expect_in_zone(TRUE_STRIKE, "graveyard");
                assert_eq!(s.hand(P1).len(), hand + 1);
                assert_eq!(s.hand(P2).len(), 3);
                s.expect_events(json!(["countered", "drawn"]));
            }

            #[test]
            fn r448_counters_a_spell_that_targets_one_of_your_backrow_cards() {
                let mut s = setup(
                    json!({ "backrow": [refusal(true, 2), { "def": TWINSPELL, "faceUp": true, "lane": 3 }] }),
                    json!({ "hand": [MAGIC_JAMMED, VANILLA] }),
                    true,
                );
                let jammed = s.backrow(P1, 3).map(|card| card.id);
                let target = jammed.clone().unwrap_or_default();
                s.play(MAGIC_JAMMED, json!({ "targets": [{ "pick": "instance", "instanceId": target }] }));
                assert_eq!(count(s.events(), "countered"), 1);
                assert_eq!(s.backrow(P1, 3).map(|card| card.id), jammed);
            }

            #[test]
            fn r448_still_counters_a_spell_that_targets_one_of_your_units() {
                let mut s = setup(json!({}), json!({}), true);
                let targets = at(&s, P1, 1);
                s.play(HIT_JOB, json!({ "targets": targets }));
                assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(MENACE.to_string()));
                assert_eq!(count(s.events(), "countered"), 1);
            }

            #[test]
            fn a_spell_aimed_only_at_their_own_side_leaves_it_set_and_nobody_draws() {
                let mut s = setup(json!({}), json!({}), true);
                let hand = s.hand(P1).len();
                let targets = at(&s, P2, 1);
                s.play(HIT_JOB, json!({ "targets": targets }));
                assert_eq!(count(s.events(), "countered"), 0);
                assert_eq!(s.hand(P1).len(), hand);
                assert_eq!(s.card(REFUSAL).face_up, Some(false));
            }
        }
    }
}
