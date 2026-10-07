//! Port of `packages/engine/test/playChoices.test.ts`.
//!
//! The refusals a play's choices go through, and the channel they travel in (SPEC §10.5 step 1,
//! §10.6, R81, R90).
//!
//! This file owns the refusals the M3 BLOCKER was about: a card dormant under a Stack, the
//! opponent's hand, and the counts a declaration asks for. Its sibling
//! `playChoices-filters.test.ts` drives the rest of the module — several declarations reading the
//! flat `targets` list, the `type`, `tags` and `notTags` filters, the backrow, hero and zone kinds,
//! `excludeSelf`, an ally-only side and two mode declarations at once — so nothing here repeats
//! those.
//!
//! It also holds R81's core, which is a statement about *where* a choice lives: "Zone, X, embiggen,
//! Tribute and the targets and modes a card's script declares travel in the `play` action, which
//! `legalActions` enumerates … and never pause resolution. Every choice made during resolution
//! (Discover, chained steps, Echo repeats, casts, triggers, mulligan) opens a `PendingChoice`."
//! That sentence is the line between this module and `prompts.ts`, and it has its own test below.
//!
//! Tribute is the one play choice this file leaves alone: `tribute.test.ts` owns it.
//!
//! Fixtures are prefixed `pc-` and indexed above 1450 so they cannot collide (BUILD §0).

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{plain, stacker};
use crate::rules::fixtures::harness::{in_hand, new_game, put, slot};

// ---------------------------------------------------------------------------
// Fixtures.
// ---------------------------------------------------------------------------

// TS's `nextIndex` started at 1450 and every `def` took the next number, in declaration order; each
// def below is written with the index it took.

/// TS `{ ...object, ...extra }`: a shallow merge of `extra`'s keys over `object`'s.
fn spread(object: &mut Value, extra: Value) {
    if let (Some(object), Value::Object(extra)) = (object.as_object_mut(), extra) {
        for (key, value) in extra {
            object.insert(key, value);
        }
    }
}

fn def(name: &str, type_: &str, index: i32, extra: Value) -> CardDef {
    let mut card = json!({
        "id": format!("pc-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (playChoices)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    });
    spread(&mut card, extra);
    json_as(card)
}

fn unit(name: &str, index: i32, extra: Value) -> CardDef {
    let mut faces = json!({
        "base": { "attack": 2, "health": 2, "keywords": [], "text": name },
        "radiant": { "attack": 4, "health": 4, "keywords": [], "text": name },
    });
    spread(&mut faces, extra);
    def(name, "Unit", index, faces)
}

/// One unit pick on your own side: what a Stack pile offers is the whole question (R13, R90).
fn unit_hitter() -> CardDef {
    def("unit-hitter", "Spell", 1451, json!({}))
}
/// #26's shape: one card out of a hand. §9.1 decides whose hand that may be.
fn hand_picker() -> CardDef {
    def("hand-picker", "Spell", 1452, json!({}))
}
/// "Choose 1 or 2 units": the plain min-and-max counts of one declaration (R90).
fn one_to_two() -> CardDef {
    def("one-to-two", "Spell", 1453, json!({}))
}
/// A card that declares nothing at all, so it may be handed nothing (R90).
fn declares_nothing() -> CardDef {
    def("declares-nothing", "Spell", 1454, json!({}))
}
/// #2 Glowy Jelly Bean plus Silly Silas: a hand pick and a direction, both play choices (R81).
fn traveller() -> CardDef {
    unit("traveller", 1455, json!({}))
}
/// A Cry that Discovers, so the choice is made during resolution and becomes a prompt (R81).
fn discoverer() -> CardDef {
    def("discoverer", "Spell", 1456, json!({}))
}
/// A spare card to sit in a hand as a candidate.
fn candidate() -> CardDef {
    def("candidate", "Spell", 1457, json!({}))
}
/// #63 Plastic Surgery's shape: one unit on either side, which the play needs (R703).
fn needed_pick() -> CardDef {
    def("needed-pick", "Spell", 1458, json!({}))
}

fn defs() -> Vec<CardDef> {
    vec![
        unit_hitter(),
        hand_picker(),
        one_to_two(),
        declares_nothing(),
        traveller(),
        discoverer(),
        candidate(),
        needed_pick(),
    ]
}

/// Records the selections and modes a play delivered, so "it travelled" is observable.
fn record_choices() -> Effect {
    Effect::new("pc:record", |ctx| {
        let Some(self_id) = ctx.self_.as_ref().map(|card| card.id.clone()) else {
            return;
        };
        let got_targets: Vec<Value> = ctx
            .targets
            .iter()
            .map(|selection| match selection {
                Selection::Instance { instance_id } => json!(instance_id),
                other => json!(other)["pick"].clone(),
            })
            .collect();
        let got_modes = json!(ctx.modes.clone());
        // TS wrote through the live `ctx.self`; here through the card under that id.
        if let Some(card) = find_instance_mut(ctx.state, &self_id) {
            card.memory.insert("gotTargets".to_string(), Value::Array(got_targets));
            card.memory.insert("gotModes".to_string(), got_modes);
        }
    })
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

/// A script's `targets`, written as TS's literal.
fn decls(list: Value) -> Vec<TargetDecl> {
    json_as(list)
}

fn nothing() -> Option<Hook> {
    Some(hook(|_ctx| vec![]))
}

fn damage_chosen(amount: i32) -> Option<Hook> {
    Some(hook(move |_ctx| {
        vec![effects::damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount })))]
    }))
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut out: IndexMap<String, CardScripts> = IndexMap::new();
    out.insert(
        unit_hitter().id,
        both(Script {
            targets: decls(json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "ally", "of": ["unit"] } }])),
            cry: damage_chosen(1),
            ..Script::default()
        }),
    );
    out.insert(
        hand_picker().id,
        both(Script {
            targets: decls(json!([{ "kind": "hand", "min": 1, "max": 1 }])),
            cry: nothing(),
            ..Script::default()
        }),
    );
    out.insert(
        one_to_two().id,
        both(Script {
            targets: decls(json!([{ "kind": "target", "min": 1, "max": 2, "filter": { "side": "any", "of": ["unit"] } }])),
            cry: nothing(),
            ..Script::default()
        }),
    );
    out.insert(
        declares_nothing().id,
        both(Script {
            cry: nothing(),
            ..Script::default()
        }),
    );
    out.insert(
        needed_pick().id,
        both(Script {
            targets: decls(json!([
                { "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit"] }, "required": true },
            ])),
            cry: damage_chosen(1),
            ..Script::default()
        }),
    );
    out.insert(
        traveller().id,
        both(Script {
            targets: decls(json!([{ "kind": "hand", "min": 1, "max": 1 }])),
            modes: json_as(json!([{ "kind": "direction", "options": ["left", "right"] }])),
            cry: Some(hook(|_ctx| vec![record_choices()])),
            ..Script::default()
        }),
    );
    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    resume.insert(
        "picked",
        hook(|ctx| match effects::chosen_options(ctx).first() {
            None => vec![],
            Some(def_id) => vec![effects::add_to_hand(json_as(json!({ "defId": def_id })))],
        }),
    );
    out.insert(
        discoverer().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![effects::discover_from_catalog(json_as(
                    json!({ "step": "picked", "query": { "type": "Unit" } }),
                ))]
            })),
            resume,
            ..Script::default()
        }),
    );
    out
}

// ---------------------------------------------------------------------------
// Harness.
// ---------------------------------------------------------------------------

/// TS's module `let nonce`: unique across the tests, which run on parallel threads.
static NONCE: AtomicU32 = AtomicU32::new(0);

/// TS `actResult`: `body` is the TS `ActionInput` literal (its `playerId` included); a fresh nonce is
/// added.
fn act_result(state: &GameState, body: Value) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::Relaxed) + 1;
    let mut body = body;
    body["nonce"] = json!(format!("pc{nonce}"));
    reduce(state, &json_as::<Action>(body))
}

fn hand_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player].hand.iter().map(|card| card.id.clone()).collect()
}

/// Past the mulligans, in p1's main phase, with this file's fixtures registered and 4 mana.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(seed, None)).state;
    let keep = hand_ids(&state, PlayerId::P1);
    state = act_result(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p1" })).state;
    let keep = hand_ids(&state, PlayerId::P2);
    state = act_result(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p2" })).state;
    let mut catalog = registered_catalog().clone();
    for card in defs() {
        catalog.insert(card.id.clone(), card);
    }
    register_catalog(catalog);
    let mut registry = registered_scripts().clone();
    for (id, script) in scripts() {
        registry.insert(id, script);
    }
    register_scripts(registry);
    state.players.p1.mana = ManaState {
        current: 4,
        max: 4,
        next_turn_mod: 0,
        perm_mod: 0,
    };
    state
}

fn only<T>(items: Vec<T>) -> T {
    items.into_iter().next().expect("expected at least one item")
}

fn hand_card(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    only(in_hand(state, def_id, player, 1))
}

fn on_instance(id: &str) -> Selection {
    Selection::Instance {
        instance_id: id.to_string(),
    }
}

fn ids(selections: &[Selection]) -> Vec<String> {
    selections
        .iter()
        .filter_map(|selection| match selection {
            Selection::Instance { instance_id } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

fn first_decl(state: &GameState, card: &CardInstance) -> TargetDecl {
    only(play_choices::declared_targets(state, card))
}

/// `result.error` as text, empty when the action went through (TS `toMatch` on `undefined` fails).
fn error_of(result: &ReduceResult) -> String {
    result.error.clone().unwrap_or_default()
}

/// TS `whyChoicesRefused`'s `string | null`.
fn refusal(answer: Result<(), EngineError>) -> Option<String> {
    answer.err().map(|error| error.message)
}

fn refused_with(answer: Result<(), EngineError>, text: &str) -> bool {
    refusal(answer).is_some_and(|message| message.contains(text))
}

/// `{ type: "play", instanceId, targets?, modes? }` as the `PlayAction` `whyChoicesRefused` takes.
fn play_action(instance_id: &str, targets: Option<&[Selection]>, modes: Option<&[String]>) -> PlayAction {
    let mut body = json!({ "type": "play", "instanceId": instance_id });
    if let Some(targets) = targets {
        body["targets"] = json!(targets);
    }
    if let Some(modes) = modes {
        body["modes"] = json!(modes);
    }
    json_as(body)
}

/// `{ type: "play", instanceId, ...combo }`.
fn combo_action(instance_id: &str, combo: &PlayChoices) -> PlayAction {
    play_action(instance_id, combo.targets.as_deref(), combo.modes.as_deref())
}

/// `playChoiceCombinations(...)` answered `[{}]`: one answer, and it is the empty one.
fn is_one_empty_answer(combos: &[PlayChoices]) -> bool {
    combos.len() == 1 && combos[0].targets.is_none() && combos[0].modes.is_none()
}

fn targeting(instance_id: &str, targets: Vec<Selection>) -> Value {
    json!({ "type": "play", "instanceId": instance_id, "playerId": "p1", "targets": targets })
}

/// `state.players[player].units[lane]?.[0]`.
fn top_of(state: &GameState, player: PlayerId, lane: usize) -> Option<CardInstance> {
    state.players[player]
        .units
        .get(lane)
        .and_then(|pile| pile.as_ref())
        .and_then(|pile| pile.first())
        .cloned()
}

// ---------------------------------------------------------------------------

mod r81_r90_the_refusals_a_plays_choices_go_through_s10_5_step_1 {
    use super::*;

    /// "R90 offers only the top of a Stack pile and refuses a card dormant under it (R13)"
    #[test]
    fn r90_r13_offers_only_the_top_of_a_stack_pile_and_refuses_a_card_dormant_under_it() {
        let mut state = playing("stack-dormant");
        let buried = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let mut top = put(&mut state, &stacker.id, slot(PlayerId::P1, Row::Units, 2), json!({}));
        // Move the Stack card onto the occupied lane, which turns it into a pile (§3.2).
        assert!(zones::place_on_field(
            &mut state,
            &mut top,
            slot(PlayerId::P1, Row::Units, 1),
            PlaceOnFieldOptions {
                stack: Some(true),
                ..Default::default()
            },
        ));
        let pile: Vec<String> = state.players.p1.units[0]
            .as_ref()
            .map(|pile| pile.iter().map(|card| card.id.clone()).collect())
            .unwrap_or_default();
        assert_eq!(pile, vec![top.id.clone(), buried.id.clone()]);

        let card = hand_card(&mut state, &unit_hitter().id, PlayerId::P1);
        let decl = first_decl(&state, &card);
        // R13: a dormant card is not "on the field" for effects, so it is never offered.
        assert_eq!(
            ids(&play_choices::legal_selections_for(&state, PlayerId::P1, &card, &decl)),
            vec![top.id.clone()]
        );
        let combos: Vec<Vec<String>> = play_choices::play_choice_combinations(&state, PlayerId::P1, &card, None)
            .iter()
            .map(|combo| ids(combo.targets.as_deref().unwrap_or(&[])))
            .collect();
        assert_eq!(combos, vec![vec![top.id.clone()]]);

        // Naming it anyway is refused rather than silently retargeted: a client may not reach it.
        assert!(refused_with(
            play_choices::why_choices_refused(
                &state,
                PlayerId::P1,
                &card,
                &play_action(&card.id, Some(&[on_instance(&buried.id)]), None),
            ),
            "not a legal target",
        ));
        assert_eq!(
            refusal(play_choices::why_choices_refused(
                &state,
                PlayerId::P1,
                &card,
                &play_action(&card.id, Some(&[on_instance(&top.id)]), None),
            )),
            None
        );

        let refused = act_result(&state, targeting(&card.id, vec![on_instance(&buried.id)]));
        assert!(error_of(&refused).contains("not a legal target"), "{:?}", refused.error);
        assert_eq!(refused.state, state);
        assert_eq!(find_instance(&state, &buried.id).map(|card| card.damage), Some(0));
    }

    /// "R90 refuses a hand pick that names a card in the opponent's hand, which the chooser cannot see
    /// (§9.1)"
    #[test]
    fn r90_refuses_a_hand_pick_that_names_a_card_in_the_opponents_hand_which_the_chooser_cannot_see() {
        let mut state = playing("opponent-hand");
        let mine = hand_card(&mut state, &candidate().id, PlayerId::P1);
        let theirs = hand_card(&mut state, &candidate().id, PlayerId::P2);
        let card = hand_card(&mut state, &hand_picker().id, PlayerId::P1);
        let decl = first_decl(&state, &card);

        // §9.1: a hand pick only ever offers the chooser's own hand, and never the card leaving it.
        let offered = ids(&play_choices::legal_selections_for(&state, PlayerId::P1, &card, &decl));
        assert!(offered.contains(&mine.id));
        assert!(!offered.contains(&card.id));
        for held in &state.players.p2.hand {
            assert!(!offered.contains(&held.id));
        }

        assert!(refused_with(
            play_choices::why_choices_refused(
                &state,
                PlayerId::P1,
                &card,
                &play_action(&card.id, Some(&[on_instance(&theirs.id)]), None),
            ),
            "not a legal target",
        ));
        assert!(
            error_of(&act_result(&state, targeting(&card.id, vec![on_instance(&theirs.id)])))
                .contains("not a legal target")
        );
        // The hand pick the chooser does own goes through.
        assert_eq!(act_result(&state, targeting(&card.id, vec![on_instance(&mine.id)])).error, None);
    }

    /// "R90 refuses fewer picks than a declaration's minimum and more than its maximum"
    #[test]
    fn r90_refuses_fewer_picks_than_a_declarations_minimum_and_more_than_its_maximum() {
        let mut state = playing("counts");
        let a = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let b = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 2), json!({}));
        let c = put(&mut state, &plain.id, slot(PlayerId::P2, Row::Units, 1), json!({}));
        let card = hand_card(&mut state, &one_to_two().id, PlayerId::P1);
        let decl = first_decl(&state, &card);
        assert_eq!(decl.min, 1);
        assert_eq!(decl.max, 2);
        assert_eq!(
            ids(&play_choices::legal_selections_for(&state, PlayerId::P1, &card, &decl)),
            vec![a.id.clone(), b.id.clone(), c.id.clone()]
        );

        let play = |targets: &[Selection]| -> Option<String> {
            refusal(play_choices::why_choices_refused(
                &state,
                PlayerId::P1,
                &card,
                &play_action(&card.id, Some(targets), None),
            ))
        };

        assert!(play(&[]).unwrap_or_default().contains("needs 1 target"));
        assert_eq!(play(&[on_instance(&a.id)]), None);
        assert_eq!(play(&[on_instance(&a.id), on_instance(&c.id)]), None);
        assert!(play(&[on_instance(&a.id), on_instance(&b.id), on_instance(&c.id)])
            .unwrap_or_default()
            .contains("at most 2 targets"));
        // Within one declaration the same card twice is not two picks (R90).
        assert!(play(&[on_instance(&a.id), on_instance(&a.id)])
            .unwrap_or_default()
            .contains("same target twice"));

        // The enumeration offers exactly the answers the validator accepts: 3 singles and 3 pairs.
        let combos = play_choices::play_choice_combinations(&state, PlayerId::P1, &card, None);
        let picked: Vec<Vec<String>> = combos
            .iter()
            .map(|combo| ids(combo.targets.as_deref().unwrap_or(&[])))
            .collect();
        assert_eq!(
            picked,
            vec![
                vec![a.id.clone()],
                vec![b.id.clone()],
                vec![c.id.clone()],
                vec![a.id.clone(), b.id.clone()],
                vec![a.id.clone(), c.id.clone()],
                vec![b.id.clone(), c.id.clone()],
            ]
        );
        assert!(combos.len() <= MAX_CHOICE_COMBINATIONS);
        for combo in &combos {
            assert_eq!(
                refusal(play_choices::why_choices_refused(&state, PlayerId::P1, &card, &combo_action(&card.id, combo))),
                None
            );
        }
        assert!(error_of(&act_result(&state, targeting(&card.id, vec![]))).contains("needs 1 target"));
    }

    /// "R90 gives a card that declared nothing nothing: a target or a mode it never asked for is
    /// refused"
    #[test]
    fn r90_gives_a_card_that_declared_nothing_nothing_a_target_or_a_mode_it_never_asked_for_is_refused() {
        let mut state = playing("declared-nothing");
        let victim = put(&mut state, &plain.id, slot(PlayerId::P2, Row::Units, 1), json!({}));
        let card = hand_card(&mut state, &declares_nothing().id, PlayerId::P1);

        assert_eq!(play_choices::declared_targets(&state, &card), Vec::<TargetDecl>::new());
        assert_eq!(play_choices::declared_modes(&state, &card), Vec::<ModeDecl>::new());
        // One answer, and it is the empty one, so the card is still offered exactly once.
        assert!(is_one_empty_answer(&play_choices::play_choice_combinations(
            &state,
            PlayerId::P1,
            &card,
            None
        )));

        assert_eq!(
            refusal(play_choices::why_choices_refused(&state, PlayerId::P1, &card, &play_action(&card.id, None, None))),
            None
        );
        assert!(refused_with(
            play_choices::why_choices_refused(
                &state,
                PlayerId::P1,
                &card,
                &play_action(&card.id, Some(&[on_instance(&victim.id)]), None),
            ),
            "takes no targets",
        ));
        assert!(refused_with(
            play_choices::why_choices_refused(
                &state,
                PlayerId::P1,
                &card,
                &play_action(&card.id, None, Some(&["left".to_string()])),
            ),
            "takes no mode choices",
        ));

        assert!(
            error_of(&act_result(&state, targeting(&card.id, vec![on_instance(&victim.id)]))).contains("takes no targets")
        );
    }

    /// "R81 carries a declared hand pick in targets and a declared direction in modes, in the play
    /// action itself"
    #[test]
    fn r81_carries_a_declared_hand_pick_in_targets_and_a_declared_direction_in_modes_in_the_play_action_itself() {
        let mut state = playing("choices-travel");
        let spare = hand_card(&mut state, &candidate().id, PlayerId::P1);
        let card = hand_card(&mut state, &traveller().id, PlayerId::P1);

        // Both declarations are the card's own, so `legalActions` has to enumerate them (R81).
        let target_kinds: Vec<PromptKind> = play_choices::declared_targets(&state, &card)
            .iter()
            .map(|decl| decl.kind)
            .collect();
        assert_eq!(target_kinds, vec![PromptKind::Hand]);
        let mode_kinds: Vec<PromptKind> = play_choices::declared_modes(&state, &card)
            .iter()
            .map(|decl| decl.kind)
            .collect();
        assert_eq!(mode_kinds, vec![PromptKind::Direction]);
        let combos = play_choices::play_choice_combinations(&state, PlayerId::P1, &card, None);
        assert!(combos
            .iter()
            .all(|combo| combo.targets.as_ref().map_or(0, |targets| targets.len()) == 1));
        let directions: BTreeSet<String> = combos
            .iter()
            .map(|combo| only(combo.modes.clone().unwrap_or_default()))
            .collect();
        assert_eq!(directions, BTreeSet::from(["left".to_string(), "right".to_string()]));

        let plays: Vec<ActionBody> = legal_actions(&state, PlayerId::P1)
            .into_iter()
            .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == card.id))
            .collect();
        assert!(!plays.is_empty());
        for play in &plays {
            let ActionBody::Play { zone, targets, modes, .. } = play else {
                continue;
            };
            // A unit takes a zone, so zone, targets and modes ride in the one action (R81).
            assert!(zone.is_some());
            assert!(targets.is_some());
            assert!(modes.is_some());
            let action: PlayAction = json_as(json!(play));
            assert_eq!(refusal(play_choices::why_choices_refused(&state, PlayerId::P1, &card, &action)), None);
        }

        let played = act_result(
            &state,
            json!({
                "type": "play",
                "instanceId": card.id,
                "playerId": "p1",
                "zone": { "row": "units", "lane": 1 },
                "targets": [on_instance(&spare.id)],
                "modes": ["right"],
            }),
        );
        assert_eq!(played.error, None);
        // The script saw both answers, and resolution never paused for either (R81).
        let resolved = top_of(&played.state, PlayerId::P1, 0);
        assert_eq!(
            resolved.as_ref().and_then(|card| card.memory.get("gotTargets").cloned()),
            Some(json!([spare.id]))
        );
        assert_eq!(
            resolved.as_ref().and_then(|card| card.memory.get("gotModes").cloned()),
            Some(json!(["right"]))
        );
        assert!(played.state.pending.is_none());
    }

    /// "R81 opens a PendingChoice for a choice made during resolution, not a play choice"
    #[test]
    fn r81_opens_a_pending_choice_for_a_choice_made_during_resolution_not_a_play_choice() {
        let mut state = playing("resolution-choice");
        let card = hand_card(&mut state, &discoverer().id, PlayerId::P1);

        // The card declares nothing, so nothing travels in the play …
        assert_eq!(play_choices::declared_targets(&state, &card), Vec::<TargetDecl>::new());
        assert_eq!(play_choices::declared_modes(&state, &card), Vec::<ModeDecl>::new());
        assert!(is_one_empty_answer(&play_choices::play_choice_combinations(
            &state,
            PlayerId::P1,
            &card,
            None
        )));

        // … and its Discover, made while the card resolves, becomes the one open prompt of §10.1.
        let played = act_result(&state, json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }));
        assert_eq!(played.error, None);
        let pending = played.state.pending.clone().expect("a prompt is open");
        assert_eq!(pending.kind, PromptKind::Discover);
        assert_eq!(pending.player_id, PlayerId::P1);
        assert_eq!(pending.options.len(), 3);
        assert!(played.events.iter().any(|event| {
            serde_json::to_value(event).expect("an event serialises")["type"] == "promptOpened"
        }));
        // And a prompt is never something a `play` action could have answered up front.
        let key = only(pending.options.clone()).key;
        assert!(refused_with(
            play_choices::why_choices_refused(&state, PlayerId::P1, &card, &play_action(&card.id, None, Some(&[key]))),
            "takes no mode choices",
        ));
    }
}

mod r703_a_pick_the_play_needs {
    use super::*;

    fn offers(state: &GameState, card: &CardInstance) -> bool {
        legal_actions(state, PlayerId::P1)
            .iter()
            .any(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == card.id))
    }

    /// "R703 a needed pick the board cannot satisfy is neither offered nor accepted, where R90's plain
    /// pick plays and fizzles"
    #[test]
    fn r703_r90_a_needed_pick_the_board_cannot_satisfy_is_neither_offered_nor_accepted_where_the_plain_pick_plays_and_fizzles()
     {
        let mut state = playing("r657-empty");
        assert!(zones::active_units_of(&state, PlayerId::P1).is_empty());
        assert!(zones::active_units_of(&state, PlayerId::P2).is_empty());
        let needed = hand_card(&mut state, &needed_pick().id, PlayerId::P1);
        let plain_pick = hand_card(&mut state, &one_to_two().id, PlayerId::P1);

        // R90: the plain declaration is still offered on an empty board, and fizzles.
        assert!(offers(&state, &plain_pick));
        // R703: the needed one is not offered, and naming it anyway is refused with the state untouched.
        assert!(play_choices::play_choice_combinations(&state, PlayerId::P1, &needed, None).is_empty());
        assert!(!offers(&state, &needed));
        assert!(refused_with(
            play_choices::why_choices_refused(&state, PlayerId::P1, &needed, &play_action(&needed.id, None, None)),
            "cannot be played without 1 legal target",
        ));
        let refused = act_result(&state, json!({ "type": "play", "instanceId": needed.id, "playerId": "p1" }));
        assert!(
            error_of(&refused).contains("cannot be played without 1 legal target"),
            "{:?}",
            refused.error
        );
        assert_eq!(refused.state, state);

        // A unit on either side satisfies it: offered once per unit, and the play goes through.
        let foe = put(&mut state, &plain.id, slot(PlayerId::P2, Row::Units, 1), json!({}));
        let combos: Vec<Vec<String>> = play_choices::play_choice_combinations(&state, PlayerId::P1, &needed, None)
            .iter()
            .map(|combo| ids(combo.targets.as_deref().unwrap_or(&[])))
            .collect();
        assert_eq!(combos, vec![vec![foe.id.clone()]]);
        assert!(offers(&state, &needed));
        let played = act_result(&state, targeting(&needed.id, vec![on_instance(&foe.id)]));
        assert_eq!(played.error, None);
        assert_eq!(top_of(&played.state, PlayerId::P2, 0).map(|card| card.damage), Some(1));
    }

    /// "R703 a cast is never refused (R70): cast with no unit on the board, the needed pick fizzles"
    #[test]
    fn r703_r70_a_cast_is_never_refused_cast_with_no_unit_on_the_board_the_needed_pick_fizzles() {
        let mut state = playing("r657-cast");
        let card = hand_card(&mut state, &needed_pick().id, PlayerId::P1);
        // TS `sinkFor(state)`: the rng from the state's cursor; nothing writes the cursor back.
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let _ = resolve::cast_card(&mut sink, &card, CastOptions::default());
        }

        assert!(state.pending.is_none());
        assert!(!state.players.p1.hand.iter().any(|held| held.id == card.id));
        assert!(state.players.p1.graveyard.iter().any(|gone| gone.id == card.id));
    }
}
