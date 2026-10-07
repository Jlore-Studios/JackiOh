//! #93 Combo-Index (SPEC §8.4, R27, R60, R62, R85, R86, BUILD M4-T4 row 93).
//!
//! Base: a Field Spell carrying a grade counter that starts at E and rises at the end of its
//! controller's turn whenever they played at least `grade` cards that turn, running every step from
//! E up to the new grade. Radiant: "Start of turn: add a Combo-Fodder to your hand; same" — one
//! clause ADDED ("same" keeps the whole base text, §8 Conventions), so the radiant face is the base
//! script plus a `startOfTurn`.
//!
//! Almost none of that is in this file, and deliberately: `engine/src/subsystems/comboIndex.ts` owns
//! the grade counter and the cascade, and its header names this file's shape — "the card file (M4)
//! stays a list of effects: `endOfTurn: (ctx) => comboIndexEndOfTurn(ctx, ctx.self)`". Calling the
//! subsystem instead of re-deriving the cascade is what keeps the counter, `viewFor` and a replay
//! reading the same number (§10.1), and it is the only place `counters.grade` is written.
//!
//! The two hooks:
//!   `cry`        — "Grade counter, starts at E": `startGrade()` writes the counter as the card
//!                  arrives so the client has a grade to show before the first end of turn. `cry` is
//!                  a Field Spell's on-resolve hook as well as a unit's Cry (§10.5 step 5, and
//!                  `runHook`'s doc in `engine/src/resolve.ts`); #73 Anti-oneshot Armor is the other
//!                  Field Spell that uses it.
//!   `endOfTurn`  — the whole threshold-and-cascade check, at its R62 point (end-of-turn triggers,
//!                  before the trap window, while `turnLog.cardsPlayed` is still this turn's count:
//!                  `turn.ts` runs the hooks before `cleanup`).
//!
//! The rulings the subsystem implements, named here so a change has a test with its name on it:
//! R27 (E adds a fresh copy keeping the radiant flag, D picks 2 DIFFERENT hand cards, steps run
//! E→new grade in order, S is terminal), R60 (a random "becomes Radiant" pick only considers
//! non-Radiant cards), R85 (grade A's 8 damage has Lifesteal of its own without #93 gaining the
//! keyword) and R86 (grade E's pool skips cards that have ceased to exist).
//!
//! R195, the yellow glow: on the field, during its controller's turn, the card glows exactly when
//! the end of this turn would raise the grade — the cards played reach the grade and it is not at S.
//! `conditionMet` reads the subsystem's own `gradeRises`, the predicate `comboIndexEndOfTurn` checks,
//! so the glow and the rise cannot disagree; `yourTurn` stands for "the end of turn that fires
//! `endOfTurn` is this one" without the card reading `state.active`. In hand it never glows: the
//! grade is a counter on a card in play.
//!
//! R372, the grade in play: the printed text starts "Grade (starts at E)" and names the threshold as
//! "N = the grades from E to the current one", so a player reading the card in play needs the letter
//! it has reached and the N it asks for now. `preview` returns both, on the field only (in hand the
//! card has no grade yet, and the text already says it starts at E): "Grade {C}", the letter carried
//! as the value's `display` because the text names a grade by its letter, and "N … {3}", which is
//! the same number `gradeRises` compares the plays with. At S nothing rises (R27), so there is no N to
//! show. Both read the counter through the subsystem, so the view, the glow and the rise agree.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-093";

/// §7, §8: the token the radiant face hands you each turn. `build.rs` proves the id is a catalog
/// entry (SURFACE §7.4), as `cardDef` did.
const COMBO_FODDER: &str = "core-093-1";

/// "Grade counter, starts at E" (§8, §10.1).
fn cry() -> Hook {
    hook(|_ctx| {
        vec![subsystems::combo_index::start_grade(
            subsystems::combo_index::StartGradeArgs::default(),
        )]
    })
}

/// "End of turn: if cards played this turn ≥ grade, grade +1 and run every step from E up to the new
/// grade" (§8, R27). The subsystem returns the whole cascade as one effect list, so the resolution
/// loop sees one trigger and R59's state check falls between whole steps, not inside them.
fn end_of_turn() -> Hook {
    hook(|ctx| match ctx.self_.clone() {
        None => vec![],
        Some(card) => subsystems::combo_index::combo_index_end_of_turn(ctx, &card),
    })
}

/// R195: field only, on its controller's turn, when `endOfTurn` would raise the grade now.
fn condition_met() -> ConditionHook {
    condition_hook(|ctx| {
        matches!(ctx.zone, ConditionZone::Field)
            && ctx.your_turn
            && subsystems::combo_index::grade_rises(ctx.state, ctx.self_)
    })
}

/// R372: the words of the printed text the two values follow, on both faces.
const GRADE_LABEL: &str = "Grade";
const THRESHOLD_LABEL: &str = "N = the grades from E to the current one";

/// R372: "Grade {C}" and "N = … {3}" on the field; nothing in hand, where the card has no grade yet.
/// The counter is public on the Field Spell (§10.8), so the values say nothing the board does not.
fn preview() -> PreviewHook {
    condition_hook(|ctx| {
        if !matches!(ctx.zone, ConditionZone::Field) {
            return vec![];
        }
        let grade = subsystems::combo_index::grade_of(ctx.self_);
        let letter = PreviewValue {
            label: GRADE_LABEL.to_string(),
            value: grade,
            display: Some(subsystems::combo_index::grade_name(grade).to_string()),
            ids: None,
        };
        if subsystems::combo_index::is_terminal_grade(grade) {
            vec![letter]
        } else {
            vec![
                letter,
                PreviewValue {
                    label: THRESHOLD_LABEL.to_string(),
                    value: grade,
                    display: None,
                    ids: None,
                },
            ]
        }
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(cry()),
        end_of_turn: Some(end_of_turn()),
        condition_met: Some(condition_met()),
        preview: Some(preview()),
        ..Script::default()
    };

    let radiant = Script {
        // "Start of turn: add a Combo-Fodder to your hand". A full hand burns it (§2.4, R4), which
        // `addToHand` already does.
        start_of_turn: Some(hook(|_ctx| vec![add_to_hand(json_as(json!({ "defId": COMBO_FODDER })))])),
        ..base.clone()
    };

    CardScripts { base, radiant }
}

// #93 Combo-Index and #93.1 Combo-Fodder (SPEC §8 rows 93 / 93.1, §7, §2.2, §10.1, §10.4;
// R4, R11, R27, R50, R60, R62, R63, R81, R85, R86).
//
// BUILD M4-T4 row 93:   "Grade 1 needs 1 play, grade 2 needs 2; cascade E→new grade in order;
//                        E adds a copy (R27); S terminal (R27); radiant adds Combo-Fodder each
//                        start of turn".
// BUILD M4-T4 row 93.1: "2 damage with Lifesteal; no radiant change". R276 has since given it a
//                        Radiant face, 4 damage (R275). Since v0.1.1 the faces print
//                        "Lifesteal / Deal 2 damage." and "Lifesteal / Deal 4 damage." (R372).
//
// The two are one file because #93.1 is the radiant text's companion: radiant #93 is "Start of
// turn: add a Combo-Fodder to your hand; same", so the token only ever exists because of #93.
//
// THE MODEL (engine/src/subsystems/comboIndex.ts): the grade is STATE, in `instance.counters.grade`
// as 1..6 for E..S (§10.1), so a replay and `viewFor` read the same number. At the end of its
// controller's turn — an ordinary end-of-turn trigger at R62's point — the grade rises by one if
// the cards that player played this turn is at or above the CURRENT grade, and then every step from
// E up to the NEW grade runs, in order (R27). Grade S is terminal: at S nothing rises and no step
// runs, and the S step is "run E–A again".
//
//   E  add a copy of a random card played this turn to your hand   → `addedToHand`
//   D  2 different random hand cards cost 1 less                   → `costChanged` ×2
//   C  the opponent exiles a random hand card                      → `exiled`
//   B  a random hand card becomes Radiant                          → `radiantSet`
//   A  8 damage to the enemy hero with Lifesteal                   → `damage` + `healed`
//
// That one-step-one-event-type mapping is what makes R27's "in order" testable: the cascade's
// event log is the step list, and grade S shows it twice.
//
// HARNESS GAP (reported): `SideSetup` cannot seed `counters`, so `setGrade` below writes the
// counter the way the engine would have. Reaching grade A by playing 1 + 2 + 3 + 4 cards over four
// of the controller's own turns is the same arithmetic with four cascades of side effects in the
// way, so the boundary tests drive the counter naturally (E→D, D→C) and the deep-cascade tests
// seed it.
//
// R195's yellow glow (`conditionMet`): both answers of this card's hook, checked against the branch
// its resolution then takes, are in condition-active.test.ts with the other hooked cards (README §5).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const COMBO_INDEX: &str = "core-093";
    const COMBO_FODDER: &str = "core-093-1";

    /// E..S as 1..6 (§8, `comboIndex.GRADES`).
    const E: i32 = 1;
    const D: i32 = 2;
    const C: i32 = 3;
    const B: i32 = 4;
    const A: i32 = 5;
    const S: i32 = 6;

    /// §8 grade A.
    const GRADE_A_DAMAGE: i32 = 8;
    /// R4.
    const HAND_CAP: usize = 10;

    /// Keyword-only units: no Cry, no trigger, nothing but a body — so a play is only a play.
    const FODDER: [&str; 5] = ["core-003", "core-008", "core-011", "core-020", "core-045"];
    /// Cards that only ever sit in a hand, as material for steps B, C and D.
    const HELD: [&str; 3] = ["core-005", "core-010", "core-056"];

    /// HARNESS GAP (reported): no `SideSetup` key seeds `counters.grade`, and §10.1 makes the counter
    /// the whole model, so a test that starts above E has to write it.
    fn set_grade(s: &mut Scenario, grade: i32) -> &mut Scenario {
        let id = s.card(COMBO_INDEX).id.clone();
        find_instance_mut(s.state_mut(), &id)
            .expect("#93 is on the board")
            .counters
            .grade = Some(grade);
        s
    }

    fn grade_of(s: &Scenario) -> Option<i32> {
        s.card(COMBO_INDEX).counters.grade
    }

    /// `type` of an event, as TS names it.
    fn type_of(event: &GameEvent) -> String {
        event.event_type().to_string()
    }

    fn of_type(s: &Scenario, kind: &str) -> Vec<GameEvent> {
        s.events().iter().filter(|event| type_of(event) == kind).cloned().collect()
    }

    fn damage_to(s: &Scenario, target: &str) -> Vec<i32> {
        of_type(s, "damage")
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, amount, .. } if target_id == target => Some(*amount),
                _ => None,
            })
            .collect()
    }

    fn healed_on(s: &Scenario, target: &str) -> Vec<i32> {
        of_type(s, "healed")
            .iter()
            .filter_map(|event| match event {
                GameEvent::Healed { target_id, amount, .. } if target_id == target => Some(*amount),
                _ => None,
            })
            .collect()
    }

    fn def_ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
    }

    /// The defIds one player gained by an "add to hand" — never a draw, which emits `drawn` first and
    /// then `addedToHand`, so an assertion has to name the player AND ignore the turn's own draw.
    fn adds_for(s: &Scenario, player: PlayerId) -> Vec<String> {
        let drawn: IndexSet<String> = of_type(s, "drawn")
            .iter()
            .filter_map(|event| match event {
                GameEvent::Drawn { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        of_type(s, "addedToHand")
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand {
                    player: who,
                    instance_id,
                    def_id,
                    ..
                } if *who == player && !drawn.contains(instance_id) => Some(def_id.clone()),
                _ => None,
            })
            .collect()
    }

    use crate::js;

    /// An event's `instanceId`, read off its JSON (TS `event.instanceId`).
    fn instance_id_of(event: &GameEvent) -> Option<String> {
        js(event).get("instanceId").and_then(Value::as_str).map(str::to_string)
    }

    /// `[...a, ...b]` over card id lists.
    fn cards(parts: &[&[&'static str]]) -> Vec<&'static str> {
        parts.concat()
    }

    // =========================================================================================
    // #93 — the grade counter (§8, §10.1)
    // =========================================================================================

    mod n93_combo_index_the_grade_counter {
        use super::*;

        #[test]
        fn s8_the_cry_writes_the_grade_at_e_as_the_card_arrives() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-start-grade",
                "p1": { "hand": [COMBO_INDEX, "core-005"] },
                "p2": { "hand": ["core-005"] },
            }));

            s.play(COMBO_INDEX, json!({}));

            assert_eq!(s.backrow(P1, 1).map(|card| card.def_id), Some(COMBO_INDEX.to_string()));
            assert_eq!(grade_of(&s), Some(E));
            let id = s.card(COMBO_INDEX).id.clone();
            assert_eq!(
                js(&of_type(&s, "counterChanged")),
                json!([{ "type": "counterChanged", "instanceId": id, "counter": "grade", "value": E }])
            );
            s.expect_mana(P1, 2);
        }

        #[test]
        fn s10_1_an_instance_that_never_ran_the_cry_still_reads_e_so_the_threshold_is_1_play() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-default-grade",
                "p1": { "backrow": [COMBO_INDEX], "hand": cards(&[&FODDER[..1], &HELD[..]]), "library": ["core-016"] },
                "p2": { "hand": ["core-005"], "library": ["core-016"] },
            }));
            assert_eq!(grade_of(&s), None);

            s.play(FODDER[0], json!({})).end_turn();

            assert_eq!(grade_of(&s), Some(D));
        }

        #[test]
        fn s10_8_the_grade_is_public_on_the_field_spell_to_both_players() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-view",
                "p1": { "backrow": [COMBO_INDEX], "hand": ["core-005"] },
                "p2": { "hand": ["core-005"] },
            }));
            set_grade(&mut s, C);

            let mine = s.view(Some(P1)).you.backrow.first().cloned().flatten();
            let theirs = s.view(Some(P2)).opponent.backrow.first().cloned().flatten();
            for zone in [mine, theirs] {
                assert!(zone.is_some());
                // §10.8: a Field Spell is public, so both sides read the same grade.
                let grade = match &zone {
                    Some(BackrowView::Public(public)) if !public.face_down => public.counters.grade,
                    _ => None,
                };
                assert_eq!(grade, Some(C));
            }
        }

        #[test]
        fn r85_n93_prints_no_lifesteal_keyword_grade_a_s_heal_comes_from_the_effect_not_the_card() {
            crate::register_all();
            let s = scenario(json!({
                "seed": "core-093-no-lifesteal",
                "p1": { "backrow": [COMBO_INDEX], "hand": ["core-005"] },
                "p2": { "hand": ["core-005"] },
            }));

            assert!(!s
                .stats(COMBO_INDEX)
                .keywords
                .iter()
                .any(|keyword| keyword.kind() == KeywordKind::Lifesteal));
        }
    }

    // =========================================================================================
    // #93 — R372 the grade in play: its letter, and the N it asks for now
    // =========================================================================================

    mod n93_combo_index_r372_the_grade_in_play {
        use super::*;

        const LETTERS: [&str; 6] = ["E", "D", "C", "B", "A", "S"];
        const GRADE_LABEL: &str = "Grade";
        const THRESHOLD_LABEL: &str = "N = the grades from E to the current one";

        /// Lane 1 of p1's backrow as `viewer` sees it, failing unless it is the public Combo-Index.
        fn combo_in(s: &Scenario, viewer: PlayerId) -> PublicBackrowView {
            let view = s.view(viewer);
            let side = if viewer == P1 { view.you } else { view.opponent };
            match side.backrow.into_iter().next().flatten() {
                Some(BackrowView::Public(public)) if !public.face_down => {
                    assert_eq!(public.def_id, COMBO_INDEX);
                    public
                }
                _ => panic!("#93 must be public in lane 1 (§10.8)"),
            }
        }

        /// TS `onField(seed, radiant = false)`.
        fn on_field(seed: &str, radiant: bool) -> Scenario {
            scenario(json!({
                "seed": seed,
                "p1": {
                    "backrow": [{ "def": COMBO_INDEX, "radiant": radiant }],
                    "hand": cards(&[&FODDER[..3], &HELD[..]]),
                    "library": ["core-016"],
                },
                "p2": { "hand": ["core-005", "core-010"], "library": ["core-016"] },
            }))
        }

        #[test]
        fn r372_the_view_names_each_grade_s_letter_e_to_s_on_both_seats() {
            crate::register_all();
            for (at, letter) in LETTERS.iter().enumerate() {
                let mut s = on_field(&format!("core-093-r372-letter-{letter}"), false);
                set_grade(&mut s, at as i32 + 1);
                for viewer in [P1, P2] {
                    assert_eq!(
                        js(&combo_in(&s, viewer).counters),
                        json!({ "grade": at as i32 + 1, "gradeLetter": letter })
                    );
                }
            }
        }

        #[test]
        fn r372_on_the_field_the_preview_is_grade_letter_and_n_labelled_with_words_both_faces_print() {
            crate::register_all();
            for radiant in [false, true] {
                let mut s = on_field(&format!("core-093-r372-labels-{radiant}"), radiant);
                set_grade(&mut s, C);
                let def = crate::card_def(COMBO_INDEX);
                let text = if radiant { def.radiant.text.clone() } else { def.base.text.clone() };
                for viewer in [P1, P2] {
                    let preview = combo_in(&s, viewer).preview;
                    assert_eq!(
                        js(&preview),
                        json!([
                            { "label": GRADE_LABEL, "value": C, "display": "C" },
                            { "label": THRESHOLD_LABEL, "value": C },
                        ]),
                        "{viewer} reads the same public values"
                    );
                    for entry in preview.unwrap_or_default() {
                        assert!(text.contains(&entry.label), "{}", entry.label);
                    }
                }
            }
        }

        #[test]
        fn r372_n_is_the_threshold_the_end_of_turn_checks_n_plays_raise_the_grade_one_fewer_does_not() {
            crate::register_all();
            for plays in [2usize, 3] {
                let mut s = on_field(&format!("core-093-r372-n-{plays}"), false);
                set_grade(&mut s, C);
                let n = combo_in(&s, P1)
                    .preview
                    .and_then(|preview| preview.into_iter().find(|entry| entry.label == THRESHOLD_LABEL))
                    .map(|entry| entry.value);
                assert_eq!(n, Some(C));
                for id in &FODDER[..plays] {
                    s.play(*id, json!({}));
                }
                s.end_turn();
                let rises = n.is_some_and(|n| plays as i32 >= n);
                assert_eq!(
                    grade_of(&s),
                    Some(if rises { B } else { C }),
                    "{plays} plays against N = {n:?}"
                );
            }
        }

        #[test]
        fn r372_the_letter_follows_a_rise_after_e_to_d_the_view_reads_d_and_n_2() {
            crate::register_all();
            let mut s = on_field("core-093-r372-rise", false);
            assert_eq!(
                js(&combo_in(&s, P1).preview),
                json!([
                    { "label": GRADE_LABEL, "value": E, "display": "E" },
                    { "label": THRESHOLD_LABEL, "value": E },
                ])
            );
            s.play(FODDER[0], json!({})).end_turn();
            assert_eq!(combo_in(&s, P1).counters.grade_letter, Some("D".to_string()));
            assert_eq!(
                js(&combo_in(&s, P2).preview),
                json!([
                    { "label": GRADE_LABEL, "value": D, "display": "D" },
                    { "label": THRESHOLD_LABEL, "value": D },
                ])
            );
        }

        #[test]
        fn r372_at_s_only_the_letter_shows_nothing_rises_past_it_r27_so_there_is_no_n() {
            crate::register_all();
            let mut s = on_field("core-093-r372-s", false);
            set_grade(&mut s, S);
            assert_eq!(
                js(&combo_in(&s, P1).preview),
                json!([{ "label": GRADE_LABEL, "value": S, "display": "S" }])
            );
        }

        #[test]
        fn r372_in_hand_the_card_has_no_grade_yet_so_it_carries_no_preview() {
            crate::register_all();
            let s = scenario(json!({
                "seed": "core-093-r372-hand",
                "p1": { "hand": [COMBO_INDEX, "core-005"] },
                "p2": { "hand": ["core-005"] },
            }));
            let hand = match s.view(Some(P1)).you.hand {
                HandView::Cards(cards) => cards,
                HandView::Count { .. } => panic!("the viewer's own hand travels in full (§10.8)"),
            };
            let card = hand.iter().find(|entry| entry.def_id == COMBO_INDEX);
            assert!(card.is_some());
            assert!(!card.is_some_and(|card| card.preview.is_some()));
        }

        #[test]
        fn r372_the_printed_text_is_short_lines_the_grade_the_rule_then_one_line_per_step_e_to_s() {
            crate::register_all();
            let base_text = crate::card_def(COMBO_INDEX).base.text;
            let base: Vec<&str> = base_text.split('\n').collect();
            assert_eq!(base[0], "Grade (starts at E).");
            let heads: Vec<String> = base[2..].iter().map(|line| line.chars().take(2).collect()).collect();
            let letters: Vec<String> = LETTERS.iter().map(|letter| format!("{letter}:")).collect();
            assert_eq!(heads, letters);
            let radiant_text = crate::card_def(COMBO_INDEX).radiant.text;
            let radiant: Vec<&str> = radiant_text.split('\n').collect();
            let mut expected = vec![base[0], "Start of turn: Add a Combo-Fodder to your hand."];
            expected.extend_from_slice(&base[1..]);
            assert_eq!(radiant, expected);
            assert_eq!(crate::card_def(COMBO_FODDER).base.text, "Lifesteal\nDeal 2 damage.");
            assert_eq!(crate::card_def(COMBO_FODDER).radiant.text, "Lifesteal\nDeal 4 damage.");
        }
    }

    // =========================================================================================
    // #93 — R27's threshold
    // =========================================================================================

    mod n93_combo_index_r27_the_threshold {
        use super::*;

        #[test]
        fn r27_at_grade_e_with_no_plays_the_grade_stays_put_and_no_step_runs() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-below-e",
                "p1": { "backrow": [COMBO_INDEX], "hand": HELD, "library": ["core-016"] },
                "p2": { "hand": ["core-005"], "library": ["core-016"] },
            }));
            let before = def_ids(&s.hand(P1));

            s.end_turn();

            assert_eq!(grade_of(&s), None);
            assert!(of_type(&s, "counterChanged").is_empty());
            assert_eq!(def_ids(&s.hand(P1)), before);
            assert!(adds_for(&s, P1).is_empty());
        }

        #[test]
        fn r27_grade_e_needs_1_play_one_play_raises_it_to_d_and_runs_steps_e_then_d() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-e-to-d",
                "p1": { "backrow": [COMBO_INDEX], "hand": [FODDER[1], HELD[0]], "library": ["core-016"] },
                "p2": { "hand": ["core-005"], "library": ["core-016"] },
            }));
            let played = s.hand(P1).first().cloned();

            s.play(FODDER[1], json!({})).end_turn();

            assert_eq!(grade_of(&s), Some(D));
            // Step E: a copy of the one card played this turn.
            let hand = s.hand(P1);
            assert_eq!(def_ids(&hand).iter().filter(|id| *id == FODDER[1]).count(), 1);
            let copy = hand.iter().find(|card| card.def_id == FODDER[1]);
            assert_ne!(copy.map(|card| card.id.clone()), played.map(|card| card.id));
            // Step D: the two cards in hand now each cost 1 less.
            assert_eq!(hand.iter().map(|card| card.cost_mod).collect::<Vec<i32>>(), vec![-1, -1]);
            s.expect_events(json!(["cardPlayed", "counterChanged", "addedToHand", "costChanged", "costChanged"]));
        }

        #[test]
        fn r27_grade_d_needs_2_plays_one_play_is_below_the_threshold_and_nothing_happens() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-below-d",
                "p1": { "backrow": [COMBO_INDEX], "hand": [FODDER[1], HELD[0]], "library": ["core-016"] },
                "p2": { "hand": ["core-005"], "library": ["core-016"] },
            }));
            set_grade(&mut s, D);

            s.play(FODDER[1], json!({})).end_turn();

            assert_eq!(grade_of(&s), Some(D));
            assert!(of_type(&s, "counterChanged").is_empty());
            assert!(adds_for(&s, P1).is_empty());
            assert_eq!(s.hand(P1).len(), 1);
            assert_eq!(s.hand(P1).first().map(|card| card.cost_mod), Some(0));
        }

        #[test]
        fn r27_grade_d_at_2_plays_rises_to_c_and_runs_e_d_c_in_order() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-d-to-c",
                "p1": {
                    "backrow": [COMBO_INDEX],
                    "hand": cards(&[&[FODDER[1], FODDER[2]][..], &HELD[..]]),
                    "library": ["core-016"],
                },
                "p2": { "hand": ["core-005", "core-010"], "library": ["core-016"] },
            }));
            set_grade(&mut s, D);

            s.play(FODDER[1], json!({})).play(FODDER[2], json!({})).end_turn();

            assert_eq!(grade_of(&s), Some(C));
            // E added one card to the three that were held.
            let hand = s.hand(P1);
            assert_eq!(hand.len(), 4);
            // R27: D picks 2 DIFFERENT cards, so exactly two carry −1 and nobody carries −2.
            assert_eq!(hand.iter().filter(|card| card.cost_mod == -1).count(), 2);
            assert!(hand.iter().all(|card| card.cost_mod == 0 || card.cost_mod == -1));
            // C: the opponent exiled one card out of their own hand.
            assert_eq!(s.pile(P2, "exile").len(), 1);
            s.expect_events(json!(["counterChanged", "addedToHand", "costChanged", "costChanged", "exiled"]));
        }
    }

    // =========================================================================================
    // #93 — R27's cascade, in order, and grade S
    // =========================================================================================

    /// A board that is one play short of rising from `grade`, with material for every step.
    fn cascade(seed: &str, grade: i32, plays: usize) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "backrow": [COMBO_INDEX],
                "hand": cards(&[&FODDER[..], &HELD[..]]),
                "library": ["core-016"],
                "mana": 20,
                "health": 20,
            },
            "p2": { "hand": ["core-005", "core-010", "core-056"], "library": ["core-016"], "health": 30 },
        }));
        set_grade(&mut s, grade);
        for card in &FODDER[..plays] {
            s.play(*card, json!({}));
        }
        assert_eq!(s.state().players[P1].turn_log.cards_played, plays as i32);
        s
    }

    mod n93_combo_index_r27_the_cascade {
        use super::*;

        #[test]
        fn r27_rising_to_a_runs_e_d_c_b_a_in_that_order() {
            crate::register_all();
            let mut s = cascade("core-093-cascade-a", B, 4);

            s.end_turn();

            assert_eq!(grade_of(&s), Some(A));
            s.expect_events(json!([
                "counterChanged",
                "addedToHand", // E
                "costChanged", // D
                "costChanged", // D
                "exiled",      // C
                "radiantSet",  // B
                "damage",      // A
                "healed",      // A
            ]));
            assert_eq!(damage_to(&s, "hero-p2"), vec![GRADE_A_DAMAGE]);
            assert_eq!(healed_on(&s, "hero-p1"), vec![GRADE_A_DAMAGE]);
            s.expect_health(P2, 30 - GRADE_A_DAMAGE);
            s.expect_health(P1, 20 + GRADE_A_DAMAGE);
        }

        #[test]
        fn r27_the_s_step_is_e_a_again_so_rising_to_s_runs_all_ten_steps_in_order() {
            crate::register_all();
            let mut s = cascade("core-093-cascade-s", A, 5);

            s.end_turn();

            assert_eq!(grade_of(&s), Some(S));
            s.expect_events(json!([
                "counterChanged",
                "addedToHand",
                "costChanged",
                "costChanged",
                "exiled",
                "radiantSet",
                "damage",
                "healed",
                // and once more, in the same order
                "addedToHand",
                "costChanged",
                "costChanged",
                "exiled",
                "radiantSet",
                "damage",
                "healed",
            ]));
            assert_eq!(damage_to(&s, "hero-p2"), vec![GRADE_A_DAMAGE, GRADE_A_DAMAGE]);
            assert_eq!(healed_on(&s, "hero-p1"), vec![GRADE_A_DAMAGE, GRADE_A_DAMAGE]);
            s.expect_health(P2, 30 - 2 * GRADE_A_DAMAGE);
            s.expect_health(P1, 20 + 2 * GRADE_A_DAMAGE);
            // Two C steps, so the opponent lost two cards out of hand.
            assert_eq!(s.pile(P2, "exile").len(), 2);
        }

        #[test]
        fn r27_grade_s_is_terminal_nothing_rises_and_no_step_runs_however_many_cards_were_played() {
            crate::register_all();
            let mut s = cascade("core-093-terminal", S, 5);

            s.end_turn();

            assert_eq!(grade_of(&s), Some(S));
            assert!(of_type(&s, "counterChanged").is_empty());
            assert!(adds_for(&s, P1).is_empty());
            assert!(of_type(&s, "radiantSet").is_empty());
            assert!(damage_to(&s, "hero-p2").is_empty());
            s.expect_health(P2, 30);
            s.expect_health(P1, 20);
            assert_eq!(s.pile(P2, "exile").len(), 0);
        }

        #[test]
        fn the_grade_rises_at_most_one_step_per_turn_end_however_far_past_the_threshold_the_turn_went() {
            crate::register_all();
            // 5 plays at grade E is five times the threshold and still rises by exactly one.
            let mut s = cascade("core-093-one-step", E, 5);

            s.end_turn();

            assert_eq!(grade_of(&s), Some(D));
            // E and D only: no C, so nothing was exiled from the opponent's hand.
            assert_eq!(s.pile(P2, "exile").len(), 0);
            assert!(damage_to(&s, "hero-p2").is_empty());
        }

        #[test]
        fn s6_2_the_end_of_the_opponent_s_turn_does_not_advance_your_grade() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-enemy-turn",
                "active": "p2",
                "p1": { "backrow": [COMBO_INDEX], "hand": HELD, "library": ["core-016"] },
                "p2": { "hand": [FODDER[1], "core-005"], "library": ["core-016"] },
            }));

            s.play(FODDER[1], json!({})).end_turn();

            assert_eq!(grade_of(&s), None);
            assert!(of_type(&s, "counterChanged").is_empty());
            assert!(adds_for(&s, P1).is_empty());
            assert!(adds_for(&s, P2).is_empty());
        }
    }

    // =========================================================================================
    // #93 — the individual steps
    // =========================================================================================

    mod n93_combo_index_step_e_r27_r86 {
        use super::*;

        #[test]
        fn r27_the_copy_is_fresh_and_keeps_the_radiant_flag_of_the_card_it_copies() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-step-e-radiant",
                "p1": {
                    "backrow": [COMBO_INDEX],
                    "hand": [{ "def": FODDER[1], "radiant": true }, HELD[0]],
                    "library": ["core-016"],
                },
                "p2": { "hand": ["core-005"], "library": ["core-016"] },
            }));
            let played = s.hand(P1).first().cloned();

            s.play(FODDER[1], json!({})).end_turn();

            let copy = s.hand(P1).into_iter().find(|card| card.def_id == FODDER[1]);
            assert!(copy.is_some());
            assert_ne!(copy.as_ref().map(|card| card.id.clone()), played.as_ref().map(|card| card.id.clone()));
            assert_eq!(copy.map(|card| card.radiant), Some(true));
            // Fresh: the original is still on the field, untouched.
            assert_eq!(s.unit(P1, 1).map(|card| card.id), played.map(|card| card.id));
        }

        #[test]
        fn r86_an_instance_that_has_ceased_to_exist_drops_out_of_the_pool_instead_of_fizzling_the_step() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-r86",
                "p1": { "backrow": [COMBO_INDEX], "hand": ["core-t-rush", FODDER[1]] },
                // A 9/9 Taunt, so the 3/3 Rush Token has one legal target and dies on it.
                "p2": { "field": ["core-019"], "hand": ["core-005"], "library": ["core-016"] },
            }));

            s.play("core-t-rush", json!({}));
            let token = s.unit(P1, 1);
            assert_eq!(token.as_ref().map(|card| card.def_id.as_str()), Some("core-t-rush"));
            match &token {
                Some(token) => s.play(FODDER[1], json!({})).attack(token, "core-019"),
                None => s.play(FODDER[1], json!({})).attack("core-t-rush", "core-019"),
            };

            // R11: a unit token that left the field ceased to exist — it is `gone`, not exiled.
            if let Some(token) = &token {
                s.expect_in_zone(token, "gone");
            }
            assert_eq!(s.state().players[P1].turn_log.played_ids.len(), 2);

            s.end_turn();

            // R86: the pool is the one surviving card, so the step neither fizzles nor copies the token.
            assert_eq!(def_ids(&s.hand(P1)), vec![FODDER[1].to_string()]);
            assert_eq!(grade_of(&s), Some(D));
        }
    }

    mod n93_combo_index_step_c {
        use super::*;

        #[test]
        fn the_card_exiled_is_the_opponent_s_out_of_their_hand_and_yours_is_untouched() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-step-c",
                "p1": {
                    "backrow": [COMBO_INDEX],
                    "hand": cards(&[&[FODDER[1], FODDER[2]][..], &HELD[..]]),
                    "library": ["core-016"],
                    "mana": 20,
                },
                "p2": { "hand": ["core-005", "core-010"], "library": ["core-016"] },
            }));
            set_grade(&mut s, D);
            let enemy_hand: Vec<String> = s.hand(P2).iter().map(|card| card.id.clone()).collect();

            s.play(FODDER[1], json!({})).play(FODDER[2], json!({})).end_turn();

            let exiled = s.pile(P2, "exile");
            assert_eq!(exiled.len(), 1);
            assert!(exiled.first().is_some_and(|card| enemy_hand.contains(&card.id)));
            assert_eq!(s.pile(P1, "exile").len(), 0);
        }

        #[test]
        fn an_empty_enemy_hand_fizzles_the_step_and_the_rest_of_the_cascade_still_runs() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-step-c-empty",
                "p1": {
                    "backrow": [COMBO_INDEX],
                    "hand": cards(&[&[FODDER[1], FODDER[2]][..], &HELD[..]]),
                    "library": ["core-016"],
                    "mana": 20,
                },
                // A unit on the board keeps p2's own turn meaningful (§2.5) with no card in hand.
                "p2": { "field": ["core-019"], "hand": [], "library": [] },
            }));
            set_grade(&mut s, D);

            s.play(FODDER[1], json!({})).play(FODDER[2], json!({})).end_turn();

            assert!(of_type(&s, "exiled").is_empty());
            assert_eq!(grade_of(&s), Some(C));
            assert_eq!(adds_for(&s, P1).len(), 1);
        }
    }

    mod n93_combo_index_step_b_r60 {
        use super::*;

        #[test]
        fn r60_exactly_one_non_radiant_hand_card_becomes_radiant() {
            crate::register_all();
            let mut s = cascade("core-093-step-b", C, 3);

            s.end_turn();

            assert_eq!(grade_of(&s), Some(B));
            let set = of_type(&s, "radiantSet");
            assert_eq!(set.len(), 1);
            assert_eq!(s.hand(P1).iter().filter(|card| card.radiant).count(), 1);
            let picked = set.first().and_then(instance_id_of);
            assert!(s.hand(P1).iter().any(|card| Some(&card.id) == picked.as_ref()));
        }

        #[test]
        fn r60_with_every_hand_card_already_radiant_the_step_does_nothing() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-step-b-all-radiant",
                "p1": {
                    "backrow": [COMBO_INDEX],
                    "hand": [
                        { "def": FODDER[1], "radiant": true },
                        { "def": FODDER[2], "radiant": true },
                        { "def": FODDER[3], "radiant": true },
                        { "def": HELD[0], "radiant": true },
                    ],
                    "library": ["core-016"],
                    "mana": 20,
                },
                "p2": { "hand": ["core-005", "core-010"], "library": ["core-016"] },
            }));
            set_grade(&mut s, C);

            s.play(FODDER[1], json!({})).play(FODDER[2], json!({})).play(FODDER[3], json!({})).end_turn();

            assert_eq!(grade_of(&s), Some(B));
            // Step E copied a Radiant card (R27), so nothing non-Radiant is left for step B to pick, and
            // nothing changes. R177: the pick is still cued once, on a hand card that was Radiant already.
            assert!(s.hand(P1).iter().all(|card| card.radiant));
            let cues = of_type(&s, "radiantSet");
            assert_eq!(cues.len(), 1);
            let cued = cues.first().and_then(instance_id_of);
            assert!(s.hand(P1).iter().any(|card| Some(&card.id) == cued.as_ref()));
        }
    }

    mod n93_combo_index_step_a_r85_r63 {
        use super::*;

        #[test]
        fn r85_the_8_damage_heals_your_hero_by_what_landed_without_n93_gaining_lifesteal() {
            crate::register_all();
            let mut s = cascade("core-093-step-a", B, 4);

            s.end_turn();

            assert_eq!(damage_to(&s, "hero-p2"), vec![GRADE_A_DAMAGE]);
            assert_eq!(healed_on(&s, "hero-p1"), vec![GRADE_A_DAMAGE]);
            s.expect_health(P1, 20 + GRADE_A_DAMAGE);
            assert!(!s
                .stats(COMBO_INDEX)
                .keywords
                .iter()
                .any(|keyword| keyword.kind() == KeywordKind::Lifesteal));
        }

        #[test]
        fn r85_armor_reduces_the_hit_and_the_heal_follows_the_reduced_amount() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-step-a-armor",
                "p1": {
                    "backrow": [COMBO_INDEX],
                    "hand": cards(&[&FODDER[..], &HELD[..]]),
                    "library": ["core-016"],
                    "mana": 20,
                    "health": 20,
                },
                "p2": { "hand": ["core-005", "core-010"], "library": ["core-016"], "health": 30, "armor": 3 },
            }));
            set_grade(&mut s, B);
            for card in &FODDER[..4] {
                s.play(*card, json!({}));
            }

            s.end_turn();

            // 8 − 3 armor = 5 dealt, so 5 healed.
            assert_eq!(damage_to(&s, "hero-p2"), vec![5]);
            assert_eq!(healed_on(&s, "hero-p1"), vec![5]);
            s.expect_health(P2, 25);
            s.expect_health(P1, 25);
        }

        #[test]
        fn r63_a_hit_reduced_to_0_by_armor_heals_nothing_at_all_and_the_cascade_still_ran() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-step-a-zero",
                "p1": {
                    "backrow": [COMBO_INDEX],
                    "hand": cards(&[&FODDER[..], &HELD[..]]),
                    "library": ["core-016"],
                    "mana": 20,
                    "health": 20,
                },
                "p2": { "hand": ["core-005", "core-010"], "library": ["core-016"], "health": 30, "armor": 8 },
            }));
            set_grade(&mut s, B);
            for card in &FODDER[..4] {
                s.play(*card, json!({}));
            }

            s.end_turn();

            // R63: no damage event, so no §4.4 step 8 and no heal.
            assert!(damage_to(&s, "hero-p2").is_empty());
            assert!(healed_on(&s, "hero-p1").is_empty());
            s.expect_health(P2, 30);
            s.expect_health(P1, 20);
            // The four steps before A still happened.
            assert_eq!(grade_of(&s), Some(A));
            assert_eq!(of_type(&s, "radiantSet").len(), 1);
        }

        #[test]
        fn r85_the_anti_oneshot_cap_clamps_the_hit_and_the_heal_follows_the_clamp_s4_4_step_3() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-step-a-cap",
                "p1": {
                    "backrow": [COMBO_INDEX],
                    "hand": cards(&[&FODDER[..], &HELD[..]]),
                    "library": ["core-016"],
                    "mana": 20,
                    "health": 20,
                },
                // #73 Anti-oneshot Armor, placed rather than played, so its Cry never fires (R1).
                "p2": { "backrow": ["core-073"], "hand": ["core-005", "core-010"], "library": ["core-016"], "health": 30 },
            }));
            set_grade(&mut s, B);
            for card in &FODDER[..4] {
                s.play(*card, json!({}));
            }

            s.end_turn();

            assert_eq!(damage_to(&s, "hero-p2"), vec![5]);
            assert_eq!(healed_on(&s, "hero-p1"), vec![5]);
            s.expect_health(P2, 25);
            s.expect_health(P1, 25);
        }
    }

    // =========================================================================================
    // #93 — radiant
    // =========================================================================================

    mod n93_combo_index_radiant {
        use super::*;

        #[test]
        fn s8_start_of_turn_a_combo_fodder_is_added_to_your_hand() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-radiant-start",
                "p1": { "backrow": [{ "def": COMBO_INDEX, "radiant": true }], "hand": [], "library": ["core-016"] },
                "p2": { "hand": ["core-005"] },
            }));

            s.start_turn();

            assert!(def_ids(&s.hand(P1)).contains(&COMBO_FODDER.to_string()));
            assert_eq!(adds_for(&s, P1), vec![COMBO_FODDER.to_string()]);
            // §2.2/R62: the start-of-turn trigger fires BEFORE the draw, so the drawn card is also there.
            assert!(def_ids(&s.hand(P1)).contains(&"core-016".to_string()));
        }

        #[test]
        fn the_base_face_adds_nothing_at_the_start_of_turn() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-base-start",
                "p1": { "backrow": [COMBO_INDEX], "hand": [], "library": ["core-016"] },
                "p2": { "hand": ["core-005"] },
            }));

            s.start_turn();

            assert!(!def_ids(&s.hand(P1)).contains(&COMBO_FODDER.to_string()));
        }

        #[test]
        fn it_fires_every_turn_so_two_start_of_turns_give_two_combo_fodders() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-radiant-repeat",
                "p1": {
                    "backrow": [{ "def": COMBO_INDEX, "radiant": true }],
                    "hand": [],
                    "library": ["core-016", "core-010"],
                },
                "p2": { "hand": ["core-005"] },
            }));

            s.start_turn().start_turn();

            assert_eq!(def_ids(&s.hand(P1)).iter().filter(|id| *id == COMBO_FODDER).count(), 2);
        }

        #[test]
        fn r4_a_full_hand_burns_the_combo_fodder_to_the_graveyard() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-radiant-hand-cap",
                "p1": {
                    "backrow": [{ "def": COMBO_INDEX, "radiant": true }],
                    "hand": vec!["core-005"; HAND_CAP],
                    "library": ["core-016"],
                },
                "p2": { "hand": ["core-005"] },
            }));

            s.start_turn();

            assert_eq!(s.hand(P1).len(), HAND_CAP);
            let burned: Vec<String> = of_type(&s, "burned")
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Burned { def_id, .. } => Some(def_id.clone()),
                    _ => None,
                })
                .collect();
            assert!(burned.contains(&COMBO_FODDER.to_string()));
            assert!(def_ids(&s.pile(P1, "graveyard")).contains(&COMBO_FODDER.to_string()));
        }

        #[test]
        fn same_the_radiant_face_keeps_the_whole_base_cascade() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-radiant-same",
                "p1": {
                    "backrow": [{ "def": COMBO_INDEX, "radiant": true }],
                    "hand": [FODDER[1], HELD[0]],
                    "library": ["core-016"],
                },
                "p2": { "hand": ["core-005"], "library": ["core-016"] },
            }));

            s.play(FODDER[1], json!({})).end_turn();

            assert_eq!(grade_of(&s), Some(D));
            assert_eq!(adds_for(&s, P1), vec![FODDER[1].to_string()]);
            assert_eq!(of_type(&s, "costChanged").len(), 2);
        }

        #[test]
        fn same_the_radiant_face_is_still_terminal_at_s() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-radiant-terminal",
                "p1": {
                    "backrow": [{ "def": COMBO_INDEX, "radiant": true }],
                    "hand": [FODDER[1], HELD[0]],
                    "library": ["core-016"],
                },
                "p2": { "hand": ["core-005"], "library": ["core-016"], "health": 30 },
            }));
            set_grade(&mut s, S);

            s.play(FODDER[1], json!({})).end_turn();

            assert_eq!(grade_of(&s), Some(S));
            assert!(damage_to(&s, "hero-p2").is_empty());
        }
    }

    // =========================================================================================
    // #93.1 Combo-Fodder
    // =========================================================================================

    mod n93_1_combo_fodder_base {
        use super::*;

        #[test]
        fn s8_costs_0_and_deals_2_damage_to_a_chosen_unit_healing_you_2_lifesteal() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-1-unit",
                "p1": { "hand": [COMBO_FODDER, "core-005"], "health": 20 },
                "p2": { "field": ["core-019"], "hand": ["core-005"] },
            }));
            let menace_id = s.unit(P2, 1).map(|card| card.id).unwrap_or_default();

            s.play(COMBO_FODDER, json!({ "targets": [{ "pick": "instance", "instanceId": menace_id }] }));

            s.expect_mana(P1, 4);
            assert_eq!(damage_to(&s, &menace_id), vec![2]);
            s.expect_health(P1, 22);
        }

        #[test]
        fn r81_a_target_reaches_the_enemy_hero_picked_with_the_play_and_never_by_a_prompt() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-1-hero",
                "p1": { "hand": [COMBO_FODDER, "core-005"], "health": 20 },
                "p2": { "hand": ["core-005"] },
            }));

            s.play(COMBO_FODDER, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

            assert!(s.state().pending.is_none());
            s.expect_health(P2, 28);
            s.expect_health(P1, 22);
            assert_eq!(healed_on(&s, "hero-p1"), vec![2]);
        }

        #[test]
        fn r81_a_target_is_unnarrowed_so_your_own_unit_is_a_legal_pick_too() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-1-friendly",
                "p1": { "field": ["core-019"], "hand": [COMBO_FODDER, "core-005"], "health": 20 },
                "p2": { "hand": ["core-005"] },
            }));
            let mine_id = s.unit(P1, 1).map(|card| card.id).unwrap_or_default();

            s.play(COMBO_FODDER, json!({ "targets": [{ "pick": "instance", "instanceId": mine_id }] }));

            assert_eq!(damage_to(&s, &mine_id), vec![2]);
            // Lifesteal heals the controller whatever it hit.
            s.expect_health(P1, 22);
        }

        #[test]
        fn r90_it_declares_one_target_and_refuses_a_play_that_names_none() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-1-no-target",
                "p1": { "hand": [COMBO_FODDER, "core-005"] },
                "p2": { "field": ["core-019"], "hand": ["core-005"] },
            }));

            s.expect_refused_with(|s| s.play(COMBO_FODDER, json!({})), "target");
        }

        #[test]
        fn r63_armor_to_0_means_no_damage_event_and_no_heal() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-1-armor",
                "p1": { "hand": [COMBO_FODDER, "core-005"], "health": 20 },
                "p2": { "hand": ["core-005"], "armor": 2 },
            }));

            s.play(COMBO_FODDER, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

            assert!(damage_to(&s, "hero-p2").is_empty());
            assert!(healed_on(&s, "hero-p1").is_empty());
            s.expect_health(P2, 30);
            s.expect_health(P1, 20);
        }

        #[test]
        fn r11_the_spell_token_reaches_the_graveyard_so_r50_can_discover_it_back() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-1-graveyard",
                "p1": { "hand": [COMBO_FODDER, "core-005"] },
                "p2": { "hand": ["core-005"] },
            }));
            let fodder = s.hand(P1).first().cloned();

            s.play(COMBO_FODDER, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

            if let Some(fodder) = &fodder {
                s.expect_in_zone(fodder, "graveyard");
            }
            assert!(def_ids(&s.pile(P1, "graveyard")).contains(&COMBO_FODDER.to_string()));
        }

        #[test]
        fn r70_s8_conventions_it_counts_as_a_card_played_which_is_what_feeds_n93_s_own_threshold() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-1-counts",
                "p1": { "backrow": [COMBO_INDEX], "hand": [COMBO_FODDER, "core-005"], "library": ["core-016"] },
                "p2": { "hand": ["core-005"], "library": ["core-016"] },
            }));

            s.play(COMBO_FODDER, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

            assert_eq!(s.state().players[P1].turn_log.cards_played, 1);

            s.end_turn();

            assert_eq!(grade_of(&s), Some(D));
        }
    }

    mod n93_1_combo_fodder_radiant {
        use super::*;

        #[test]
        fn r275_the_radiant_face_deals_4_with_lifesteal_the_enemy_hero_loses_4_and_you_heal_4() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-1-radiant",
                "p1": { "hand": [{ "def": COMBO_FODDER, "radiant": true }, "core-005"], "health": 20 },
                "p2": { "hand": ["core-005"] },
            }));
            assert_eq!(s.hand(P1).first().map(|card| card.radiant), Some(true));

            s.play(COMBO_FODDER, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

            assert_eq!(damage_to(&s, "hero-p2"), vec![4]);
            assert_eq!(healed_on(&s, "hero-p1"), vec![4]);
            s.expect_health(P2, 26);
            s.expect_health(P1, 24);
            // Still a 0-cost token.
            s.expect_mana(P1, 4);
        }

        #[test]
        fn r275_the_4_reaches_a_unit_as_one_hit_a_4_health_unit_dies_where_the_base_2_leaves_it_standing() {
            crate::register_all();
            // #61 Postdoc is 2/4: the radiant 4 kills it outright, where the base 2 would not.
            let mut s = scenario(json!({
                "seed": "core-093-1-radiant-unit",
                "p1": { "hand": [{ "def": COMBO_FODDER, "radiant": true }, "core-005"], "health": 20 },
                "p2": { "field": ["core-061"], "hand": ["core-005"] },
            }));
            let postdoc = s.unit(P2, 1);
            let postdoc_id = postdoc.as_ref().map(|card| card.id.clone()).unwrap_or_default();

            s.play(COMBO_FODDER, json!({ "targets": [{ "pick": "instance", "instanceId": postdoc_id }] }));

            assert_eq!(damage_to(&s, &postdoc_id), vec![4]);
            if let Some(postdoc) = &postdoc {
                s.expect_in_zone(postdoc, "graveyard");
            }
            s.expect_health(P1, 24);
        }

        #[test]
        fn r85_armor_reduces_the_radiant_hit_and_the_heal_follows_what_landed() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-093-1-radiant-armor",
                "p1": { "hand": [{ "def": COMBO_FODDER, "radiant": true }, "core-005"], "health": 20 },
                "p2": { "hand": ["core-005"], "armor": 1 },
            }));

            s.play(COMBO_FODDER, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

            assert_eq!(damage_to(&s, "hero-p2"), vec![3]);
            assert_eq!(healed_on(&s, "hero-p1"), vec![3]);
            s.expect_health(P1, 23);
        }

        #[test]
        fn s5_2_the_flag_still_sets_so_a_counting_effect_sees_a_radiant_card() {
            crate::register_all();
            let s = scenario(json!({
                "seed": "core-093-1-radiant-flag",
                "p1": { "hand": [{ "def": COMBO_FODDER, "radiant": true }, "core-005"] },
                "p2": { "hand": ["core-005"] },
            }));

            assert_eq!(s.hand(P1).iter().filter(|card| card.radiant).count(), 1);
        }
    }

    /// TS `query(args).map((card) => card.id)`: the ids `catalog.query` answers (§5.1).
    fn query_ids(args: Value) -> Vec<String> {
        crate::query::query(&json_as::<CatalogQueryArgs>(args))
            .iter()
            .map(|card| card.id.clone())
            .collect()
    }

    mod n93_1_combo_fodder_s5_1_pools {
        use super::*;

        #[test]
        fn s5_1_no_random_pool_offers_the_token() {
            crate::register_all();
            assert!(!query_ids(json!({})).contains(&COMBO_FODDER.to_string()));
            assert!(!query_ids(json!({ "type": "Spell", "cost": 0 })).contains(&COMBO_FODDER.to_string()));
            assert!(query_ids(json!({ "tags": ["Token"] })).contains(&COMBO_FODDER.to_string()));
        }

        #[test]
        fn r35_n93_itself_is_in_the_legendary_pool_n83_transmogulate_draws_from() {
            crate::register_all();
            assert!(query_ids(json!({ "rarity": "Legendary" })).contains(&COMBO_INDEX.to_string()));
            assert!(!query_ids(json!({ "rarity": "Legendary" })).contains(&COMBO_FODDER.to_string()));
        }
    }
}
