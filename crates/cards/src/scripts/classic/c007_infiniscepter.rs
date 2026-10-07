//! C #7 InfiniScepter (SPEC §8.6 row 7). Field Spell, cost 1, Legendary.
//!   Both faces: "Cry: Exile a ({costLimit}) Cost or less Spell from your hand.
//!                Activate: Cast a copy of that Spell." — costLimit 1 on the base face, 2 on the Radiant.
//!
//! The Cry (a Field Spell's Cry, as #73 Anti-oneshot Armor's) declares a hand pick (R81): a Spell — the
//! Spell type — in your hand whose cost now (R65, a hand card at its hand cost; an X Spell 0) is
//! ({costLimit}) or less, a number no declaration field can carry once Degrade or Upgrade moves it, so
//! it is a `targetChecks` rule (§10.6). It exiles that card and remembers its definition and Radiant
//! flag on the instance (`memory.scepter`). With no such Spell the play is still legal (R90) and the
//! Cry does nothing, so nothing is remembered and the card can never activate. The exiled card stays
//! in exile; the Radiant face keeps "from your hand", since the Cry has nowhere else to look.
//!
//! Activate (R384), once per turn: cast a fresh copy of the remembered Spell (§6.3 Cast, B5 E12,
//! `castNew`): free, counted as a play (R70) — the activation itself is not one — with its radiant flag
//! kept, its targets and modes chosen by you in prompts as the cast begins (R70, R81), the `activate`
//! action carrying none; the copy goes to your graveyard after it resolves (R87). With nothing
//! remembered the ability can't be activated (`canActivate`). Leaving the field clears the memory
//! (R78), so a replayed InfiniScepter needs a new Cry.
//!
//! R520: a remembered X-cost Spell is cast with the X its caster chooses as the cast begins, from 1 to
//! their current mana (R348); the cast pays nothing, so the X is not paid.

use jackioh_engine::effects::{cast_new, exile, instance_of, remember};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-007";

/// Where the instance keeps the Spell it exiled.
const SCEPTER: &str = "scepter";
/// The name of the Cry's hand-pick rule in `targetChecks`.
const WITHIN_LIMIT: &str = "spellWithinLimit";

/// TS `type Held = { defId: string; radiant: boolean }`.
#[derive(Clone, Debug, PartialEq)]
struct Held {
    def_id: String,
    radiant: bool,
}

/// TS `heldSpell(ctx: Pick<EffectContext, "self" | "data">)`: what the instance remembers under
/// `scepter`, read as `query::recalled` reads it (`self.memory[partMemoryKey(data, key)]`, the running
/// ingredient's own key on a fused card, R102). The memory is JSON, so it is narrowed, never cast.
fn held_spell(self_: Option<&CardInstance>, data: &IndexMap<String, Value>) -> Option<Held> {
    let held = self_?
        .memory
        .get(&jackioh_engine::work::part_memory_key(data, SCEPTER))?;
    // TS `held === null || typeof held !== "object"`, then `typeof defId === "string"`.
    let held = held.as_object()?;
    let def_id = held.get("defId")?.as_str()?;
    Some(Held {
        def_id: def_id.to_string(),
        radiant: held.get("radiant") == Some(&Value::Bool(true)),
    })
}

/// "A ({costLimit}) Cost or less Spell from your hand", at the cost it would be played for now (R65).
fn within_limit(args: TargetCheckArgs<'_>) -> bool {
    let Some(candidate) = args.candidate else {
        return false;
    };
    let limit = HookArgs {
        state: args.state,
        self_: args.self_,
        radiant: args.radiant,
    };
    matches!(candidate.zone, Zone::Hand { .. })
        && def_of(Some(args.state), &candidate.def_id).type_ == CardType::Spell
        && effective_cost(args.state, candidate, Default::default()) <= param(&limit, "costLimit")
}

fn cry(ctx: &EffectContext<'_>) -> Vec<Effect> {
    let Some(picked) = instance_of(ctx, &json_as(json!({ "of": "chosen" }))) else {
        return vec![];
    };
    vec![
        remember(json_as(json!({ "key": SCEPTER, "value": { "defId": picked.def_id, "radiant": picked.radiant } }))),
        exile(json_as(json!({ "target": { "of": "chosen" } }))),
    ]
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: vec![TargetDecl::hand(
            1,
            1,
            json!({ "of": ["hand"], "type": "Spell", "check": WITHIN_LIMIT }),
        )],
        target_checks: IndexMap::from([(WITHIN_LIMIT, target_check(within_limit))]),
        cry: Some(hook(|ctx| cry(ctx))),
        activations: vec![ActivationDecl {
            id: "cast-copy".to_string(),
            label: "Cast a copy of that Spell".to_string(),
            uses: ActivationUses::Count(1),
            cost: None,
            targets: vec![],
            modes: vec![],
            can_activate: Some(condition_hook(|c| held_spell(Some(c.self_), &IndexMap::new()).is_some())),
            has: None,
            run: hook(|_ctx| {
                vec![cast_new(CastNewArgs {
                    def: CastNewDef::Read(Arc::new(|ctx: &EffectContext<'_>| -> Option<CastDef> {
                        held_spell(ctx.live_self(), &ctx.data)
                            .map(|held| json_as(json!({ "defId": held.def_id, "radiant": held.radiant })))
                    })),
                    radiant: None,
                    how: CastHow::default(),
                })]
            }),
        }],
        ..Script::default()
    };

    // The same script: the Radiant face's (2) is its declared costLimit, which `param` reads off the face.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C #7 InfiniScepter — SPEC §8.6 row 7, BUILD M9 Classic row C 7: "Cry: exile a (1) Cost or less Spell
// (the Spell type) of your choice from your hand, a hand pick carried in the play action (R81),
// remembering its definition and Radiant flag; no such Spell → the Cry does nothing and the card can
// never activate; the exiled card stays in exile; Activate, once per turn (R384): cast a fresh copy
// (free, counted as played, R70), its targets and modes chosen by you in prompts as the cast begins
// (R70, R81), the `activate` action carrying none, the copy going to your graveyard afterwards (R87);
// usable the turn it is played, only in your main phase with no prompt open; a second activation that
// turn is refused and absent from `legalActions`, and it is back next turn; activating is not a play
// (Combo, Quickstriker and Ceaseless Void don't count it) while the cast is one; leaving the field
// clears the memory (R78), so a replayed one needs a new Cry; radiant: a (2) Cost or less Spell, still
// from your hand; its tuned number (cost limit) reads through `param()` (R386)".
//
// R520 (this workstream's ruling on OPEN-QUESTIONS' InfiniScepter item): an X-cost Spell it holds is
// cast with the X its caster picks as the cast begins, 1 to their current mana (R348), and not paid.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SCEPTER: &str = "classic-007";
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
    const CALL: &str = "core-069"; // (2) Spell: Recruit 3 (1) Cost or less Units.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy target Unit.
    const FLAME: &str = "classic-016"; // (1) Spell, Book: Deal 4 damage (8 Radiant).
    const DIVIDEND: &str = "core-024"; // (X) Spell: modes damage / heal / mana.
    const JAMMED: &str = "core-036"; // (1) Spell: Destroy target backrow card; Lock its zone.
    const TIMMY: &str = "core-011"; // (1) Unit.
    const VANILLA: &str = "core-008";
    const FILLER: &str = "core-010"; // (0) Spell Rapid Replenish.
    const TUTOR: &str = "core-051"; // (1) Spell: opens a type prompt as it resolves.
    const MUTATE: &str = "classic-078"; // (1) Field Spell: Radiant fuses an enemy permanent onto a card of yours of its type.
    const MANA_WELL: &str = "core-006"; // (3) Field Spell.

    /// An engine value as the JSON TS compares it by.
    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("an engine value serialises")
    }

    /// TS `toMatchObject`: every key the pattern names holds a matching value; arrays match item by item.
    fn matches_object(actual: &Value, pattern: &Value) -> bool {
        match (actual, pattern) {
            (Value::Object(actual), Value::Object(pattern)) => pattern
                .iter()
                .all(|(key, want)| actual.get(key).is_some_and(|got| matches_object(got, want))),
            (Value::Array(actual), Value::Array(pattern)) => {
                actual.len() == pattern.len() && actual.iter().zip(pattern).all(|(got, want)| matches_object(got, want))
            }
            _ => actual == pattern,
        }
    }

    /// TS `{ ...defaults, ...over }` on a side's setup.
    fn merged(mut defaults: Value, over: Value) -> Value {
        if let (Some(into), Value::Object(over)) = (defaults.as_object_mut(), over) {
            for (key, value) in over {
                into.insert(key, value);
            }
        }
        defaults
    }

    fn setup(p1: Value, radiant_face: bool, p2: Value) -> Scenario {
        scenario(json!({
            "p1": merged(
                json!({
                    "hand": [{ "def": SCEPTER, "radiant": radiant_face }, FILLER],
                    "library": [VANILLA, VANILLA, VANILLA, VANILLA],
                    "health": 20,
                }),
                p1,
            ),
            "p2": merged(json!({ "hand": [FILLER], "library": [VANILLA, VANILLA] }), p2),
        }))
    }

    fn pick(card: &CardInstance) -> Value {
        json!([{ "pick": "instance", "instanceId": card.id }])
    }

    /// The `activate` actions `legalActions` lists for the InfiniScepter, as JSON.
    fn activations(s: &Scenario, player: PlayerId) -> Vec<Value> {
        let card = s.card(SCEPTER).id.clone();
        legal_actions(s.state(), player)
            .iter()
            .map(js)
            .filter(|action| action["type"] == "activate" && action["instanceId"] == card.as_str())
            .collect()
    }

    fn count(events: &[GameEvent], event_type: &str) -> usize {
        events.iter().filter(|event| event.event_type().as_str() == event_type).count()
    }

    fn remembered(s: &Scenario, card: &str) -> Option<Value> {
        s.card(card).memory.get("scepter").cloned()
    }

    mod c_n7_infiniscepter {
        use super::*;

        #[test]
        fn declares_its_hand_pick_one_activate_once_per_turn_and_the_same_script_on_both_faces() {
            crate::register_all();
            assert_eq!(ID, SCEPTER);
            let scripts = script();
            // TS `expect(radiant).toBe(base)`: the radiant face is the very same script.
            let (Some(base_cry), Some(radiant_cry)) = (&scripts.base.cry, &scripts.radiant.cry) else {
                panic!("both faces have a Cry");
            };
            assert!(Arc::ptr_eq(base_cry, radiant_cry));
            assert!(matches_object(
                &js(&scripts.base.targets[0]),
                &json!({ "kind": "hand", "filter": { "type": "Spell", "check": "spellWithinLimit" } })
            ));
            assert_eq!(scripts.base.activations[0].uses, ActivationUses::Count(1));
        }

        mod base_the_cry {
            use super::*;

            #[test]
            fn r81_exiles_the_1_cost_or_less_spell_picked_in_the_play_action_and_remembers_it() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [SCEPTER, STOCKPILE, FILLER] }), false, json!({}));
                let spell = s.card(STOCKPILE).clone();

                s.play(SCEPTER, json!({ "zone": 1, "targets": pick(&spell) }));

                s.expect_in_zone(&spell, "exile");
                assert_eq!(remembered(&s, SCEPTER), Some(json!({ "defId": STOCKPILE, "radiant": false })));
            }

            #[test]
            fn r57_remembers_the_spell_s_radiant_flag() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [SCEPTER, { "def": STOCKPILE, "radiant": true }, FILLER] }), false, json!({}));

                let targets = pick(s.card(STOCKPILE));
                s.play(SCEPTER, json!({ "zone": 1, "targets": targets }));

                assert_eq!(remembered(&s, SCEPTER), Some(json!({ "defId": STOCKPILE, "radiant": true })));
            }

            #[test]
            fn s10_6_a_2_cost_spell_a_unit_or_a_spell_elsewhere_is_no_pick_for_it() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [SCEPTER, CALL, TIMMY, FILLER] }), false, json!({}));
                let card = s.card(SCEPTER).id.clone();
                let offered: Vec<String> = legal_actions(s.state(), P1)
                    .iter()
                    .map(js)
                    .filter(|action| action["type"] == "play" && action["instanceId"] == card.as_str())
                    .flat_map(|action| action["targets"].as_array().cloned().unwrap_or_default())
                    .filter(|selection| selection["pick"] == "instance")
                    .filter_map(|selection| selection["instanceId"].as_str().map(str::to_string))
                    .collect();

                assert!(!offered.contains(&s.card(CALL).id));
                assert!(!offered.contains(&s.card(TIMMY).id));
                assert!(offered.contains(&s.card(FILLER).id));
                s.expect_refused(|s| {
                    let targets = pick(s.card(CALL));
                    s.play(SCEPTER, json!({ "zone": 1, "targets": targets }))
                });
            }

            #[test]
            fn r90_with_no_such_spell_the_play_is_legal_the_cry_does_nothing_and_it_can_never_activate() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [SCEPTER, CALL, TIMMY] }), false, json!({}));

                s.play(SCEPTER, json!({ "zone": 1 }));

                s.expect_in_zone(SCEPTER, "field");
                assert!(remembered(&s, SCEPTER).is_none());
                assert_eq!(activations(&s, P1).len(), 0);
                s.expect_refused(|s| s.activate(SCEPTER, json!({})));
            }

            #[test]
            fn the_exiled_card_stays_in_exile_after_the_copies_are_cast() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [SCEPTER, STOCKPILE, FILLER] }), false, json!({}));
                let spell = s.card(STOCKPILE).clone();

                s.play(SCEPTER, json!({ "zone": 1, "targets": pick(&spell) })).activate(SCEPTER, json!({}));

                s.expect_in_zone(&spell, "exile");
            }

            #[test]
            fn r386_an_upgrade_of_its_cost_limit_admits_a_2_cost_spell() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [SCEPTER, CALL, FILLER] }), false, json!({}));
                step_param(s.card_mut(SCEPTER), "costLimit", 1);

                let targets = pick(s.card(CALL));
                s.play(SCEPTER, json!({ "zone": 1, "targets": targets }));

                s.expect_in_zone(CALL, "exile");
            }
        }

        mod base_the_activate {
            use super::*;

            #[test]
            fn r70_casts_a_fresh_copy_free_and_counted_as_a_play_the_copy_lands_in_your_graveyard_r87() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [SCEPTER, STOCKPILE, FILLER] }), false, json!({}));
                let spell = s.card(STOCKPILE).clone();
                s.play(SCEPTER, json!({ "zone": 1, "targets": pick(&spell) }));
                let played = s.state().players.p1.turn_log.cards_played;

                s.activate(SCEPTER, json!({}));

                assert_eq!(count(s.last_events(), "drawn"), 2);
                s.expect_health(P1, 22);
                s.expect_mana(P1, 3);
                assert_eq!(s.state().players.p1.turn_log.cards_played, played + 1);
                let copies: Vec<CardInstance> =
                    s.pile(P1, "graveyard").into_iter().filter(|card| card.def_id == STOCKPILE).collect();
                assert_eq!(copies.len(), 1);
                assert_ne!(copies.first().map(|card| card.id.clone()), Some(spell.id.clone()));
            }

            #[test]
            fn r81_the_copy_s_targets_are_chosen_in_a_prompt_as_the_cast_begins_the_action_carries_none() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [SCEPTER, FLAME, FILLER] }), false, json!({}));
                let targets = pick(s.card(FLAME));
                s.play(SCEPTER, json!({ "zone": 1, "targets": targets }));

                s.activate(SCEPTER, json!({}));
                assert_eq!(s.state().pending.as_ref().map(|pending| pending.player_id), Some(P1));
                s.answer(json!([{ "pick": "hero", "player": "p2" }]));

                s.expect_health(P2, 26);
            }

            #[test]
            fn r57_a_radiant_spell_s_copy_is_cast_on_its_radiant_face() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [SCEPTER, { "def": FLAME, "radiant": true }, FILLER] }), false, json!({}));
                let targets = pick(s.card(FLAME));
                s.play(SCEPTER, json!({ "zone": 1, "targets": targets }));

                s.activate(SCEPTER, json!({})).answer(json!([{ "pick": "hero", "player": "p2" }]));

                s.expect_health(P2, 22);
            }

            #[test]
            fn r384_usable_the_turn_it_is_played_once_per_turn_a_second_is_refused_and_not_listed_back_next_turn() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [SCEPTER, STOCKPILE, FILLER] }), false, json!({}));
                let targets = pick(s.card(STOCKPILE));
                s.play(SCEPTER, json!({ "zone": 1, "targets": targets }));

                s.activate(SCEPTER, json!({}));
                assert_eq!(activations(&s, P1).len(), 0);
                s.expect_refused(|s| s.activate(SCEPTER, json!({})));

                s.end_turn().end_turn();
                assert_eq!(s.state().active, P1);
                assert!(!activations(&s, P1).is_empty());
            }

            #[test]
            fn r384_only_in_its_controller_s_own_turn() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [SCEPTER, STOCKPILE, FILLER] }), false, json!({ "hand": [FILLER, VANILLA] }));
                let targets = pick(s.card(STOCKPILE));
                s.play(SCEPTER, json!({ "zone": 1, "targets": targets }));
                assert!(!activations(&s, P1).is_empty());

                s.end_turn();

                assert_eq!(s.state().active, P2);
                assert_eq!(activations(&s, P1).len(), 0);
                s.expect_refused(|s| s.activate(SCEPTER, json!({})));
            }

            #[test]
            fn r384_not_while_a_prompt_is_open() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [SCEPTER, STOCKPILE, TUTOR, FILLER] }), false, json!({}));
                let targets = pick(s.card(STOCKPILE));
                s.play(SCEPTER, json!({ "zone": 1, "targets": targets }));
                assert!(!activations(&s, P1).is_empty());

                s.play(TUTOR, json!({}));

                assert_eq!(s.state().pending.as_ref().map(|pending| pending.player_id), Some(P1));
                assert_eq!(activations(&s, P1).len(), 0);
                s.expect_refused(|s| s.activate(SCEPTER, json!({})));
            }

            #[test]
            fn r384_the_activation_is_no_play_it_adds_nothing_to_the_turn_s_plays_beyond_the_cast() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [SCEPTER, STOCKPILE, FILLER] }), false, json!({}));
                let targets = pick(s.card(STOCKPILE));
                s.play(SCEPTER, json!({ "zone": 1, "targets": targets }));
                let before = s.state().players.p1.turn_log.cards_played;

                s.activate(SCEPTER, json!({}));

                let plays: Vec<Value> =
                    s.last_events().iter().map(js).filter(|event| event["type"] == "cardPlayed").collect();
                assert_eq!(plays.len(), 1);
                assert!(matches_object(&plays[0], &json!({ "defId": STOCKPILE })));
                assert_eq!(s.state().players.p1.turn_log.cards_played, before + 1);
            }

            #[test]
            fn r78_leaving_the_field_clears_what_it_remembered() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [SCEPTER, STOCKPILE, FILLER] }), false, json!({ "hand": [JAMMED, FILLER] }));
                let targets = pick(s.card(STOCKPILE));
                s.play(SCEPTER, json!({ "zone": 1, "targets": targets }));
                let scepter = s.card(SCEPTER).clone();

                s.end_turn();
                s.play(JAMMED, json!({ "targets": pick(&scepter) }));

                s.expect_in_zone(&scepter, "graveyard");
                assert!(s.card(&scepter).memory.get("scepter").is_none());
            }

            #[test]
            fn r520_an_x_cost_spell_it_holds_is_cast_with_the_x_its_caster_picks_from_1_to_their_current_mana() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [SCEPTER, DIVIDEND, FILLER] }), false, json!({}));
                // In hand an X Spell costs 0 (R65), so it is within the limit.
                let targets = pick(s.card(DIVIDEND));
                s.play(SCEPTER, json!({ "zone": 1, "targets": targets }));
                s.expect_mana(P1, 3);

                s.activate(SCEPTER, json!({}));
                let asked = s.state().pending.clone();
                assert_eq!(asked.as_ref().map(|pending| pending.player_id), Some(P1));
                let selections: Option<Vec<Value>> =
                    asked.map(|pending| pending.options.iter().map(|option| js(&option.selection)).collect());
                let expected: Vec<Value> =
                    [1, 2, 3].iter().map(|n| json!({ "pick": "mode", "option": n.to_string() })).collect();
                assert_eq!(selections, Some(expected));
                s.answer(json!("3"));
                s.answer(json!("damage"));
                s.answer(json!([{ "pick": "hero", "player": "p2" }]));

                // Efficiency Dividend's damage mode deals X = 3, and the cast paid nothing for its X.
                s.expect_health(P2, 27);
                s.expect_mana(P1, 3);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r386_its_limit_is_2_a_2_cost_spell_may_be_exiled_and_cast() {
                crate::register_all();
                let mut s = setup(
                    json!({ "hand": [{ "def": SCEPTER, "radiant": true }, CALL, FILLER], "library": [TIMMY, TIMMY, VANILLA] }),
                    true,
                    json!({}),
                );

                let targets = pick(s.card(CALL));
                s.play(SCEPTER, json!({ "zone": 1, "targets": targets }));
                s.expect_in_zone(CALL, "exile");
                s.activate(SCEPTER, json!({}));

                // Call to Arms recruits the (1) Cost Units from the top of the deck.
                assert!(count(s.last_events(), "summoned") > 0);
            }

            #[test]
            fn a_3_cost_spell_is_still_no_pick_for_it() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [{ "def": SCEPTER, "radiant": true }, HIT_JOB, FILLER] }), true, json!({}));

                s.expect_refused(|s| {
                    let targets = pick(s.card(HIT_JOB));
                    s.play(SCEPTER, json!({ "zone": 1, "targets": targets }))
                });
            }

            #[test]
            fn r386_a_degrade_of_the_radiant_limit_to_1_refuses_a_2_cost_spell() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [{ "def": SCEPTER, "radiant": true }, CALL, FILLER] }), true, json!({}));
                step_param(s.card_mut(SCEPTER), "costLimit", -1);

                s.expect_refused(|s| {
                    let targets = pick(s.card(CALL));
                    s.play(SCEPTER, json!({ "zone": 1, "targets": targets }))
                });
                let targets = pick(s.card(FILLER));
                s.play(SCEPTER, json!({ "zone": 1, "targets": targets }));
                s.expect_in_zone(FILLER, "exile");
            }

            #[test]
            fn the_radiant_face_still_picks_from_your_hand_only() {
                crate::register_all();
                let mut s = setup(
                    json!({ "hand": [{ "def": SCEPTER, "radiant": true }, FILLER], "graveyard": [STOCKPILE] }),
                    true,
                    json!({}),
                );
                let Some(grave) = s.pile(P1, "graveyard").into_iter().next() else {
                    panic!("fixture");
                };

                s.expect_refused(|s| s.play(SCEPTER, json!({ "zone": 1, "targets": pick(&grave) })));
            }
        }

        mod r102_fused {
            use super::*;

            #[test]
            fn r102_a_fuse_that_keeps_it_still_lets_it_cast_the_spell_its_own_cry_remembered() {
                crate::register_all();
                let mut s = setup(
                    json!({ "hand": [SCEPTER, STOCKPILE, FILLER], "backrow": [{ "def": MUTATE, "radiant": true }] }),
                    false,
                    json!({ "backrow": [{ "def": MANA_WELL, "counters": { "plague": 1 } }] }),
                );
                let scepter = s.card(SCEPTER).id.clone();
                let targets = pick(s.card(STOCKPILE));
                s.play(SCEPTER, json!({ "zone": 2, "targets": targets }));
                // Radiant Mutate Spell fuses the enemy Mana Well onto a Field Spell of yours you pick: this one.
                let well = pick(s.card(MANA_WELL));
                s.activate(MUTATE, json!({ "targets": well }));
                s.answer(json!(scepter));
                let fused = s.card(&scepter).clone();
                assert_eq!(js(&fused_id_parts(Some(s.state()), &fused.def_id)), json!([MANA_WELL, SCEPTER]));
                // R77: what its Cry remembered moved with its text, to the place that text now runs at.
                assert!(fused.memory.get("scepter").is_none());
                assert_eq!(
                    fused.memory.get("scepter@1").cloned(),
                    Some(json!({ "defId": STOCKPILE, "radiant": false }))
                );
                let listed: Vec<Value> = legal_actions(s.state(), P1)
                    .iter()
                    .map(js)
                    .filter(|action| action["type"] == "activate" && action["instanceId"] == scepter.as_str())
                    .collect();
                assert_eq!(listed, vec![json!({ "type": "activate", "instanceId": scepter, "ability": "cast-copy" })]);

                s.activate(&scepter, json!({}));

                assert_eq!(count(s.last_events(), "drawn"), 2);
                s.expect_health(P1, 22);
                assert_eq!(
                    s.pile(P1, "graveyard").iter().filter(|card| card.def_id == STOCKPILE).count(),
                    1
                );
            }
        }
    }
}
