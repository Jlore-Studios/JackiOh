//! Declaration kinds and filters a play's choices go through (SPEC §10.5 step 1, §10.6, R81, R90):
//! several declarations reading the flat `targets` list in order, the `type`, `tags` and `notTags`
//! filters, the backrow, hero and zone kinds, `excludeSelf`, an ally-only side, two mode declarations.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::harness::{in_hand, new_game, put, slot};

/// A shallow merge of `extra`'s keys over `object`'s.
fn spread(object: &mut Value, extra: Value) {
    if let (Some(object), Value::Object(extra)) = (object.as_object_mut(), extra) {
        for (key, value) in extra {
            object.insert(key, value);
        }
    }
}

fn def(name: &str, type_: &str, index: i32, extra: Value) -> CardDef {
    let mut card = json!({
        "id": format!("pf-{name}"),
        "index": index.to_string(),
        "name": name,
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

/// #24's shape: two declarations, an enemy unit then one of yours, read in that order (R90).
fn two_step() -> CardDef {
    def("two-step", "Spell", 1101, json!({}))
}
/// A fixed pick plus an "up to 3": the last declaration takes the remainder (R90).
fn remainder() -> CardDef {
    def("remainder", "Spell", 1102, json!({}))
}
/// #26's hand pick, narrowed to Spells (the `type` filter as a single value).
fn hand_spells() -> CardDef {
    def("hand-spells", "Spell", 1103, json!({}))
}
/// A backrow pick narrowed to traps (the `type` filter as a list, on the `backrow` kind).
fn backrow_traps() -> CardDef {
    def("backrow-traps", "Spell", 1104, json!({}))
}
/// A hand pick that wants every tag it names and none it excludes.
fn tagged() -> CardDef {
    def("tagged", "Spell", 1105, json!({}))
}
/// A declared `hero` pick on the enemy side only.
fn hero_hitter() -> CardDef {
    def("hero-hitter", "Spell", 1106, json!({}))
}
/// A declared `zone` pick with no `of`, on your own side (§10.6's `zone` kind).
fn zone_namer() -> CardDef {
    def("zone-namer", "Spell", 1107, json!({}))
}
/// A unit that buffs one of your *other* units: `excludeSelf` on a unit pick.
fn selfless() -> CardDef {
    unit("selfless", 1108, json!({}))
}
/// The same flag on a backrow pick.
fn backrow_selfless() -> CardDef {
    def("backrow-selfless", "Field Spell", 1109, json!({}))
}
/// Two mode declarations, as #59 and Silly Silas's direction would combine (R81).
fn two_modes() -> CardDef {
    def("two-modes", "Spell", 1110, json!({}))
}
/// A hand pick whose filter says `side: "any"`: §9.1 still allows only your own hand.
fn any_hand() -> CardDef {
    def("any-hand", "Spell", 1111, json!({}))
}
/// A bare `target` declaration: no filter at all, which §10.6 reads as a unit on either side.
fn bare_target() -> CardDef {
    def("bare-target", "Spell", 1112, json!({}))
}
/// A unit pick narrowed by tag, so the tag filter runs on the unit kind too.
fn tribal_hitter() -> CardDef {
    def("tribal-hitter", "Spell", 1113, json!({}))
}
/// "Choose 2 or 3": a wide board makes this the enumeration bound of §10.2 (R90).
fn up_to_three() -> CardDef {
    def("up-to-three", "Spell", 1114, json!({}))
}

// Hand candidates for the filters.
fn spell_candidate() -> CardDef {
    def("spell-candidate", "Spell", 1115, json!({}))
}
fn felinor_ky() -> CardDef {
    unit("felinor-ky", 1116, json!({ "tags": ["Felinor", "KY"] }))
}
fn felinor_only() -> CardDef {
    unit("felinor-only", 1117, json!({ "tags": ["Felinor"] }))
}
fn felinor_cn() -> CardDef {
    unit("felinor-cn", 1118, json!({ "tags": ["Felinor", "KY", "CN"] }))
}
// Backrow candidates.
fn trap_card() -> CardDef {
    def("trap", "Trap", 1119, json!({}))
}
fn field_spell_card() -> CardDef {
    def("field-spell", "Field Spell", 1120, json!({}))
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

/// A script's `targets`, written as JSON.
fn decls(list: Value) -> Vec<TargetDecl> {
    json_as(list)
}

fn nothing() -> Option<Hook> {
    Some(hook(|_ctx| vec![]))
}

fn damage(literal: Value) -> Effect {
    effects::damage(json_as(literal))
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut out: IndexMap<String, CardScripts> = IndexMap::new();
    out.insert(
        two_step().id,
        both(Script {
            targets: decls(json!([
                { "kind": "target", "min": 1, "max": 1, "filter": { "side": "enemy", "of": ["unit"] } },
                { "kind": "target", "min": 1, "max": 1, "filter": { "side": "ally", "of": ["unit"] } },
            ])),
            // Which declaration a selection belongs to is observable: 2 on the enemy, 1 on your own.
            cry: Some(hook(|_ctx| {
                vec![
                    damage(json!({ "to": { "of": "chosen", "index": 0 }, "amount": 2 })),
                    damage(json!({ "to": { "of": "chosen", "index": 1 }, "amount": 1 })),
                ]
            })),
            ..Script::default()
        }),
    );
    out.insert(
        remainder().id,
        both(Script {
            targets: decls(json!([
                { "kind": "target", "min": 1, "max": 1, "filter": { "side": "enemy", "of": ["unit"] } },
                { "kind": "target", "min": 1, "max": 3, "filter": { "side": "any", "of": ["unit"] } },
            ])),
            cry: nothing(),
            ..Script::default()
        }),
    );
    out.insert(
        hand_spells().id,
        both(Script {
            targets: decls(json!([{ "kind": "hand", "min": 1, "max": 1, "filter": { "of": ["hand"], "type": "Spell" } }])),
            cry: nothing(),
            ..Script::default()
        }),
    );
    out.insert(
        backrow_traps().id,
        both(Script {
            targets: decls(json!([
                { "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["backrow"], "type": ["Trap", "Field Trap"] } },
            ])),
            cry: nothing(),
            ..Script::default()
        }),
    );
    out.insert(
        tagged().id,
        both(Script {
            targets: decls(json!([
                { "kind": "hand", "min": 1, "max": 1, "filter": { "of": ["hand"], "tags": ["Felinor", "KY"], "notTags": ["CN"] } },
            ])),
            cry: nothing(),
            ..Script::default()
        }),
    );
    out.insert(
        hero_hitter().id,
        both(Script {
            targets: decls(json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "enemy", "of": ["hero"] } }])),
            cry: Some(hook(|_ctx| vec![damage(json!({ "to": { "of": "chosen" }, "amount": 3 }))])),
            ..Script::default()
        }),
    );
    out.insert(
        zone_namer().id,
        both(Script {
            targets: decls(json!([{ "kind": "zone", "min": 1, "max": 1, "filter": { "side": "ally" } }])),
            cry: nothing(),
            ..Script::default()
        }),
    );
    out.insert(
        selfless().id,
        both(Script {
            targets: decls(json!([
                { "kind": "target", "min": 1, "max": 1, "filter": { "side": "ally", "of": ["unit"], "excludeSelf": true } },
            ])),
            cry: nothing(),
            ..Script::default()
        }),
    );
    out.insert(
        backrow_selfless().id,
        both(Script {
            targets: decls(json!([
                { "kind": "target", "min": 1, "max": 1, "filter": { "side": "ally", "of": ["backrow"], "excludeSelf": true } },
            ])),
            cry: nothing(),
            ..Script::default()
        }),
    );
    out.insert(
        two_modes().id,
        both(Script {
            modes: json_as(json!([
                { "kind": "mode", "options": ["burn", "freeze"] },
                { "kind": "direction", "options": ["left", "right"] },
            ])),
            cry: nothing(),
            ..Script::default()
        }),
    );
    out.insert(
        any_hand().id,
        both(Script {
            targets: decls(
                json!([{ "kind": "hand", "min": 1, "max": 1, "filter": { "of": ["hand"], "side": "any" } }]),
            ),
            cry: nothing(),
            ..Script::default()
        }),
    );
    out.insert(
        bare_target().id,
        both(Script {
            targets: decls(json!([{ "kind": "target", "min": 1, "max": 1 }])),
            cry: Some(hook(|_ctx| {
                vec![damage(json!({ "to": { "of": "chosen" }, "amount": 1 }))]
            })),
            ..Script::default()
        }),
    );
    out.insert(
        tribal_hitter().id,
        both(Script {
            targets: decls(json!([
                { "kind": "target", "min": 1, "max": 1, "filter": { "side": "ally", "of": ["unit"], "tags": ["Felinor"] } },
            ])),
            cry: nothing(),
            ..Script::default()
        }),
    );
    out.insert(
        up_to_three().id,
        both(Script {
            targets: decls(json!([{ "kind": "target", "min": 2, "max": 3, "filter": { "side": "any", "of": ["unit"] } }])),
            cry: nothing(),
            ..Script::default()
        }),
    );
    out
}

fn defs() -> Vec<CardDef> {
    vec![
        two_step(),
        remainder(),
        hand_spells(),
        backrow_traps(),
        tagged(),
        hero_hitter(),
        zone_namer(),
        selfless(),
        backrow_selfless(),
        two_modes(),
        any_hand(),
        bare_target(),
        tribal_hitter(),
        up_to_three(),
        spell_candidate(),
        felinor_ky(),
        felinor_only(),
        felinor_cn(),
        trap_card(),
        field_spell_card(),
    ]
}

/// Unique across the tests, which run on parallel threads.
static SEQ: AtomicU32 = AtomicU32::new(0);

/// `body` is the action literal (its `playerId` included); a fresh nonce is added.
fn act(state: &GameState, body: Value) -> ReduceResult {
    let seq = SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    let mut body = body;
    body["nonce"] = json!(format!("pf{seq}"));
    reduce(state, &json_as::<Action>(body))
}

fn hand_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player]
        .hand
        .iter()
        .map(|card| card.id.clone())
        .collect()
}

/// Past the mulligans, in p1's main phase, with the fixtures registered and 4 mana.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(seed, None)).state;
    let keep = hand_ids(&state, PlayerId::P1);
    state = act(
        &state,
        json!({ "type": "mulligan", "keep": keep, "playerId": "p1" }),
    )
    .state;
    let keep = hand_ids(&state, PlayerId::P2);
    state = act(
        &state,
        json!({ "type": "mulligan", "keep": keep, "playerId": "p2" }),
    )
    .state;
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

fn on_instance(id: &str) -> Selection {
    Selection::Instance {
        instance_id: id.to_string(),
    }
}

fn on_zone(player: PlayerId, row: Row, lane: i32) -> Selection {
    Selection::Zone { player, row, lane }
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

fn strs(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

/// The first item.
fn first<T>(items: Vec<T>) -> T {
    items.into_iter().next().expect("expected at least one item")
}

/// `result.error` as text, empty when the action went through.
fn error_of(result: &ReduceResult) -> String {
    result.error.clone().unwrap_or_default()
}

/// A refusal as `Some(message)`.
fn refusal(answer: Result<(), EngineError>) -> Option<String> {
    answer.err().map(|error| error.message)
}

/// `{ type: "play", instanceId, ...choices }` as the `PlayAction` `whyChoicesRefused` takes.
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

/// `{ type: "play", instanceId, playerId: "p1", ...combo }`, an action body for `act`.
fn combo_body(instance_id: &str, combo: &PlayChoices) -> Value {
    let mut body = json!({ "type": "play", "instanceId": instance_id, "playerId": "p1" });
    if let Some(targets) = &combo.targets {
        body["targets"] = json!(targets);
    }
    if let Some(modes) = &combo.modes {
        body["modes"] = json!(modes);
    }
    body
}

fn targeting(instance_id: &str, targets: Vec<Selection>) -> Value {
    json!({ "type": "play", "instanceId": instance_id, "playerId": "p1", "targets": targets })
}

/// `state.players[player].units[lane]?.[0]?.damage`.
fn top_damage(state: &GameState, player: PlayerId, lane: usize) -> Option<i32> {
    state.players[player]
        .units
        .get(lane)
        .and_then(|pile| pile.as_ref())
        .and_then(|pile| pile.first())
        .map(|card| card.damage)
}

fn legal(state: &GameState, card: &CardInstance, decl: &TargetDecl) -> Vec<Selection> {
    play_choices::legal_selections_for(state, PlayerId::P1, card, decl)
}

fn first_decl(state: &GameState, card: &CardInstance) -> TargetDecl {
    first(play_choices::declared_targets(state, card))
}

mod r81_r90_play_choice_declarations_and_filters {
    use super::*;

    /// "R90 reads two declarations off the flat targets list in order, and refuses the swapped order"
    #[test]
    fn r90_reads_two_declarations_off_the_flat_targets_list_in_order_and_refuses_the_swapped_order() {
        let mut state = playing("two-declarations");
        let theirs = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let mine = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let card = first(in_hand(&mut state, &two_step().id, PlayerId::P1, 1));

        let played = act(
            &state,
            targeting(&card.id, vec![on_instance(&theirs.id), on_instance(&mine.id)]),
        );
        assert_eq!(played.error, None);
        assert_eq!(top_damage(&played.state, PlayerId::P2, 0), Some(2));
        assert_eq!(top_damage(&played.state, PlayerId::P1, 0), Some(1));

        // The same two selections in the other order are refused: each declaration reads its own slot.
        let swapped = act(
            &state,
            targeting(&card.id, vec![on_instance(&mine.id), on_instance(&theirs.id)]),
        );
        assert!(
            error_of(&swapped).contains("not a legal target"),
            "{:?}",
            swapped.error
        );
        assert_eq!(swapped.state, state);

        // One selection does not feed two declarations: the second one is still owed its minimum.
        let short = act(&state, targeting(&card.id, vec![on_instance(&theirs.id)]));
        assert!(error_of(&short).contains("needs 1 target"), "{:?}", short.error);
    }

    /// "R90 enumerates a declaration pair as a cross product, and both may name the same card"
    #[test]
    fn r90_enumerates_a_declaration_pair_as_a_cross_product_and_both_may_name_the_same_card() {
        let mut state = playing("declaration-pairs");
        let enemy_a = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let enemy_b = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P2, Row::Units, 2),
            json!({}),
        );
        let ally_a = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let ally_b = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 2),
            json!({}),
        );
        let card = first(in_hand(&mut state, &two_step().id, PlayerId::P1, 1));

        let combos = play_choices::play_choice_combinations(&state, PlayerId::P1, &card, None);
        let picked: Vec<Vec<String>> = combos
            .iter()
            .map(|combo| ids(combo.targets.as_deref().unwrap_or(&[])))
            .collect();
        assert_eq!(
            picked,
            vec![
                vec![enemy_a.id.clone(), ally_a.id.clone()],
                vec![enemy_a.id.clone(), ally_b.id.clone()],
                vec![enemy_b.id.clone(), ally_a.id.clone()],
                vec![enemy_b.id.clone(), ally_b.id.clone()],
            ]
        );
        assert!(combos.len() <= MAX_CHOICE_COMBINATIONS);
        for combo in &combos {
            assert_eq!(
                refusal(play_choices::why_choices_refused(
                    &state,
                    PlayerId::P1,
                    &card,
                    &combo_action(&card.id, combo)
                )),
                None
            );
        }

        // R90: "each declaration is checked on its own, so two different declarations may both name the
        // same card" — here the same-side declaration cannot reach the enemy unit, so the pair that
        // proves it is the "up to 3" card, whose second declaration sees both sides.
        let wide = first(in_hand(&mut state, &remainder().id, PlayerId::P1, 1));
        assert_eq!(
            refusal(play_choices::why_choices_refused(
                &state,
                PlayerId::P1,
                &wide,
                &play_action(
                    &wide.id,
                    Some(&[on_instance(&enemy_a.id), on_instance(&enemy_a.id)]),
                    None
                ),
            )),
            None
        );
        // Inside one declaration the same card twice is still refused.
        let twice = refusal(play_choices::why_choices_refused(
            &state,
            PlayerId::P1,
            &wide,
            &play_action(
                &wide.id,
                Some(&[
                    on_instance(&enemy_a.id),
                    on_instance(&ally_a.id),
                    on_instance(&ally_a.id),
                ]),
                None,
            ),
        ));
        assert!(
            twice.as_deref().unwrap_or("").contains("same target twice"),
            "{twice:?}"
        );
    }

    /// "R90 gives the last declaration the remainder, and refuses more than its own max"
    #[test]
    fn r90_gives_the_last_declaration_the_remainder_and_refuses_more_than_its_own_max() {
        let mut state = playing("remainder-split");
        let enemy_a = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let enemy_b = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P2, Row::Units, 2),
            json!({}),
        );
        let ally_a = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let ally_b = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 2),
            json!({}),
        );
        let ally_c = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 3),
            json!({}),
        );
        let card = first(in_hand(&mut state, &remainder().id, PlayerId::P1, 1));

        // 1 for the fixed declaration, then 3 for the "up to 3": four selections in one flat list.
        let played = act(
            &state,
            targeting(
                &card.id,
                vec![
                    on_instance(&enemy_a.id),
                    on_instance(&ally_a.id),
                    on_instance(&ally_b.id),
                    on_instance(&ally_c.id),
                ],
            ),
        );
        assert_eq!(played.error, None);

        // A fifth selection lands on the last declaration too, which takes at most three.
        let too_many = act(
            &state,
            targeting(
                &card.id,
                vec![
                    on_instance(&enemy_a.id),
                    on_instance(&ally_a.id),
                    on_instance(&ally_b.id),
                    on_instance(&ally_c.id),
                    on_instance(&enemy_b.id),
                ],
            ),
        );
        assert!(error_of(&too_many).contains("at most 3"), "{:?}", too_many.error);

        // The first declaration still reads the first selection only: an ally there is not enemy-side.
        let wrong_first = act(
            &state,
            targeting(&card.id, vec![on_instance(&ally_a.id), on_instance(&enemy_a.id)]),
        );
        assert!(
            error_of(&wrong_first).contains("not a legal target"),
            "{:?}",
            wrong_first.error
        );
    }

    /// "R81 filters a hand declaration by the card type it names (§10.6)"
    #[test]
    fn r81_filters_a_hand_declaration_by_the_card_type_it_names() {
        let mut state = playing("hand-type-filter");
        let a_spell = first(in_hand(&mut state, &spell_candidate().id, PlayerId::P1, 1));
        let a_unit = first(in_hand(&mut state, &felinor_ky().id, PlayerId::P1, 1));
        let card = first(in_hand(&mut state, &hand_spells().id, PlayerId::P1, 1));

        let decl = first_decl(&state, &card);
        let offered = ids(&legal(&state, &card, &decl));
        assert!(offered.contains(&a_spell.id));
        assert!(!offered.contains(&a_unit.id));
        assert!(!offered.contains(&card.id)); // never the card that is leaving the hand
        // The opening hand is fixture units, so the type filter removes every one of them.
        for held in &state.players.p1.hand {
            if held.id == a_spell.id || held.id == card.id {
                continue;
            }
            if held.def_id == hand_spells().id {
                continue;
            }
            assert!(!offered.contains(&held.id));
        }

        assert_eq!(
            act(&state, targeting(&card.id, vec![on_instance(&a_spell.id)])).error,
            None
        );
        assert!(
            error_of(&act(&state, targeting(&card.id, vec![on_instance(&a_unit.id)])))
                .contains("not a legal target")
        );
    }

    /// "R81 offers a backrow declaration both backrows, filtered by the list of types it names"
    #[test]
    fn r81_offers_a_backrow_declaration_both_backrows_filtered_by_the_list_of_types_it_names() {
        let mut state = playing("backrow-type-filter");
        let their_trap = put(
            &mut state,
            &trap_card().id,
            slot(PlayerId::P2, Row::Backrow, 1),
            json!({}),
        );
        let my_trap = put(
            &mut state,
            &trap_card().id,
            slot(PlayerId::P1, Row::Backrow, 2),
            json!({}),
        );
        let not_a_trap = put(
            &mut state,
            &field_spell_card().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        let card = first(in_hand(&mut state, &backrow_traps().id, PlayerId::P1, 1));

        let decl = first_decl(&state, &card);
        // Both sides, lane order within each: the ally side comes first (the chooser's own side).
        assert_eq!(
            ids(&legal(&state, &card, &decl)),
            vec![my_trap.id.clone(), their_trap.id.clone()]
        );

        assert_eq!(
            act(&state, targeting(&card.id, vec![on_instance(&their_trap.id)])).error,
            None
        );
        assert!(
            error_of(&act(
                &state,
                targeting(&card.id, vec![on_instance(&not_a_trap.id)])
            ))
            .contains("not a legal target")
        );
    }

    /// "R81 requires every tag a filter names and none of the tags it excludes (§10.6)"
    #[test]
    fn r81_requires_every_tag_a_filter_names_and_none_of_the_tags_it_excludes() {
        let mut state = playing("tag-filter");
        let wanted = first(in_hand(&mut state, &felinor_ky().id, PlayerId::P1, 1));
        let half_match = first(in_hand(&mut state, &felinor_only().id, PlayerId::P1, 1));
        let excluded = first(in_hand(&mut state, &felinor_cn().id, PlayerId::P1, 1));
        let card = first(in_hand(&mut state, &tagged().id, PlayerId::P1, 1));

        let decl = first_decl(&state, &card);
        let offered = ids(&legal(&state, &card, &decl));
        assert_eq!(offered, vec![wanted.id.clone()]); // every tag matches, and the excluded tag is absent

        assert_eq!(
            act(&state, targeting(&card.id, vec![on_instance(&wanted.id)])).error,
            None
        );
        // "Felinor" alone does not satisfy ["Felinor", "KY"]: every tag must match.
        assert!(
            error_of(&act(
                &state,
                targeting(&card.id, vec![on_instance(&half_match.id)])
            ))
            .contains("not a legal target")
        );
        // And one excluded tag is enough to drop a card that matches both wanted tags.
        assert!(
            error_of(&act(&state, targeting(&card.id, vec![on_instance(&excluded.id)])))
                .contains("not a legal target")
        );
    }

    /// "R81 offers a hero declaration only on the side its filter names (§10.6)"
    #[test]
    fn r81_offers_a_hero_declaration_only_on_the_side_its_filter_names() {
        let mut state = playing("hero-kind");
        let theirs = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let card = first(in_hand(&mut state, &hero_hitter().id, PlayerId::P1, 1));

        let decl = first_decl(&state, &card);
        assert_eq!(
            legal(&state, &card, &decl),
            vec![Selection::Hero { player: PlayerId::P2 }]
        );

        let played = act(
            &state,
            targeting(&card.id, vec![Selection::Hero { player: PlayerId::P2 }]),
        );
        assert_eq!(played.error, None);
        assert_eq!(played.state.players.p2.hero.health, 27);

        // Your own hero is a hero, but not the hero this declaration allows.
        let own_hero = act(
            &state,
            targeting(&card.id, vec![Selection::Hero { player: PlayerId::P1 }]),
        );
        assert!(
            error_of(&own_hero).contains("not a legal target"),
            "{:?}",
            own_hero.error
        );
        assert_eq!(own_hero.state, state);
        // A unit is not a hero either, even an enemy one.
        assert!(
            error_of(&act(&state, targeting(&card.id, vec![on_instance(&theirs.id)])))
                .contains("not a legal target")
        );
    }

    /// "R81 offers a zone declaration only your own open zones, never a Locked one (§3.2)"
    #[test]
    fn r81_offers_a_zone_declaration_only_your_own_open_zones_never_a_locked_one() {
        let mut state = playing("zone-kind");
        put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        ); // occupied: not open
        zones::lock_zone(&mut state, slot(PlayerId::P1, Row::Units, 2)); // Locked: never offered
        put(
            &mut state,
            &plain.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        ); // the enemy side is not offered at all
        let card = first(in_hand(&mut state, &zone_namer().id, PlayerId::P1, 1));

        let decl = first_decl(&state, &card);
        let offered = legal(&state, &card, &decl);
        assert_eq!(offered.len(), 8);
        assert!(offered.iter().all(|selection| matches!(
            selection,
            Selection::Zone {
                player: PlayerId::P1,
                ..
            }
        )));
        assert!(offered.contains(&on_zone(PlayerId::P1, Row::Units, 3)));
        assert!(!offered.contains(&on_zone(PlayerId::P1, Row::Units, 1)));
        assert!(!offered.contains(&on_zone(PlayerId::P1, Row::Units, 2)));
        assert!(!offered.contains(&on_zone(PlayerId::P2, Row::Units, 2)));

        assert_eq!(
            act(
                &state,
                targeting(&card.id, vec![on_zone(PlayerId::P1, Row::Backrow, 2)])
            )
            .error,
            None
        );
        for zone in [
            on_zone(PlayerId::P1, Row::Units, 2),
            on_zone(PlayerId::P1, Row::Units, 1),
            on_zone(PlayerId::P2, Row::Backrow, 2),
            on_zone(PlayerId::P1, Row::Units, 9),
        ] {
            let refused = act(&state, targeting(&card.id, vec![zone.clone()]));
            assert!(
                error_of(&refused).contains("not a legal target"),
                "{zone:?}: {:?}",
                refused.error
            );
        }
    }

    /// "R81 excludeSelf drops the declaring card from its own unit and backrow picks"
    #[test]
    fn r81_exclude_self_drops_the_declaring_card_from_its_own_unit_and_backrow_picks() {
        let mut state = playing("exclude-self");
        let self_card = put(
            &mut state,
            &selfless().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let other = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 2),
            json!({}),
        );

        let decl = first_decl(&state, &self_card);
        assert_eq!(legal(&state, &self_card, &decl), vec![on_instance(&other.id)]);
        // The same board without the flag offers both, which is what makes the exclusion visible.
        let unflagged: TargetDecl = json_as(json!({
            "kind": "target",
            "min": 1,
            "max": 1,
            "filter": { "side": "ally", "of": ["unit"] },
        }));
        assert_eq!(
            ids(&legal(&state, &self_card, &unflagged)),
            vec![self_card.id.clone(), other.id.clone()]
        );

        let back = put(
            &mut state,
            &backrow_selfless().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        let other_back = put(
            &mut state,
            &field_spell_card().id,
            slot(PlayerId::P1, Row::Backrow, 2),
            json!({}),
        );
        let back_decl = first_decl(&state, &back);
        assert_eq!(
            legal(&state, &back, &back_decl),
            vec![on_instance(&other_back.id)]
        );
    }

    /// "R81 enumerates every combination of two mode declarations and needs an answer to each (§10.6)"
    #[test]
    fn r81_enumerates_every_combination_of_two_mode_declarations_and_needs_an_answer_to_each() {
        let mut state = playing("two-modes");
        let card = first(in_hand(&mut state, &two_modes().id, PlayerId::P1, 1));

        assert_eq!(play_choices::declared_modes(&state, &card).len(), 2);
        assert_eq!(
            play_choices::declared_targets(&state, &card),
            Vec::<TargetDecl>::new()
        );

        let combos = play_choices::play_choice_combinations(&state, PlayerId::P1, &card, None);
        let modes: Vec<Option<Vec<String>>> = combos.iter().map(|combo| combo.modes.clone()).collect();
        assert_eq!(
            modes,
            vec![
                Some(strs(&["burn", "left"])),
                Some(strs(&["burn", "right"])),
                Some(strs(&["freeze", "left"])),
                Some(strs(&["freeze", "right"])),
            ]
        );
        assert!(combos.iter().all(|combo| combo.targets.is_none()));
        assert!(combos.len() <= MAX_CHOICE_COMBINATIONS);

        // Every combination really is playable, and `legalActions` offers exactly those four plays.
        for combo in &combos {
            assert_eq!(act(&state, combo_body(&card.id, combo)).error, None);
        }
        let plays: Vec<Option<Vec<String>>> = legal_actions(&state, PlayerId::P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Play {
                    instance_id, modes, ..
                } if instance_id == card.id => Some(modes),
                _ => None,
            })
            .collect();
        assert_eq!(plays, modes);

        // One answer for two declarations, three answers for two, and an option the second one
        // does not have are all refused.
        let with_modes = |modes: &[&str]| -> String {
            error_of(&act(
                &state,
                json!({ "type": "play", "instanceId": card.id, "playerId": "p1", "modes": modes }),
            ))
        };
        assert!(with_modes(&["burn"]).contains("needs a mode choice"));
        assert!(with_modes(&["burn", "left", "left"]).contains("takes 2 mode choices"));
        assert!(with_modes(&["burn", "burn"]).contains("is not a mode of"));
        // The first declaration's option in the second slot is wrong too, so the order is checked.
        assert!(with_modes(&["left", "left"]).contains("is not a mode of"));
    }

    /// "§9.1 offers a hand declaration only the chooser's own hand, even with side 'any'"
    #[test]
    fn s9_1_offers_a_hand_declaration_only_the_choosers_own_hand_even_with_side_any() {
        let mut state = playing("hand-side-any");
        let mine = first(in_hand(&mut state, &spell_candidate().id, PlayerId::P1, 1));
        let theirs = first(in_hand(&mut state, &spell_candidate().id, PlayerId::P2, 1));
        let card = first(in_hand(&mut state, &any_hand().id, PlayerId::P1, 1));

        let decl = first_decl(&state, &card);
        let offered = ids(&legal(&state, &card, &decl));
        assert!(offered.contains(&mine.id));
        for held in &state.players.p2.hand {
            assert!(!offered.contains(&held.id));
        }

        assert_eq!(
            act(&state, targeting(&card.id, vec![on_instance(&mine.id)])).error,
            None
        );
        assert!(
            error_of(&act(&state, targeting(&card.id, vec![on_instance(&theirs.id)])))
                .contains("not a legal target")
        );
    }

    /// "R90 refuses a mode pick or an empty pick where a card or a zone is declared (§10.6)"
    #[test]
    fn r90_refuses_a_mode_pick_or_an_empty_pick_where_a_card_or_a_zone_is_declared() {
        let mut state = playing("pick-kinds");
        put(
            &mut state,
            &plain.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let hitter = first(in_hand(&mut state, &hero_hitter().id, PlayerId::P1, 1));
        let zoner = first(in_hand(&mut state, &zone_namer().id, PlayerId::P1, 1));

        // A selection of the wrong kind never matches an offered one, whatever it carries.
        for pick in [
            Selection::Mode {
                option: "burn".to_string(),
            },
            Selection::None,
        ] {
            assert!(
                error_of(&act(&state, targeting(&hitter.id, vec![pick.clone()])))
                    .contains("not a legal target")
            );
            assert!(
                error_of(&act(&state, targeting(&zoner.id, vec![pick.clone()])))
                    .contains("not a legal target")
            );
        }
        // A zone where a hero is declared, and a hero where a zone is declared, are refused too.
        assert!(
            error_of(&act(
                &state,
                targeting(&hitter.id, vec![on_zone(PlayerId::P1, Row::Units, 1)])
            ))
            .contains("not a legal target")
        );
        assert!(
            error_of(&act(
                &state,
                targeting(&zoner.id, vec![Selection::Hero { player: PlayerId::P1 }])
            ))
            .contains("not a legal target")
        );
    }

    /// "R81 reads a bare target declaration as a unit on either side, and filters units by tag (§10.6)"
    #[test]
    fn r81_reads_a_bare_target_declaration_as_a_unit_on_either_side_and_filters_units_by_tag() {
        let mut state = playing("bare-and-tribal");
        let mine = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let felinor = put(
            &mut state,
            &felinor_ky().id,
            slot(PlayerId::P1, Row::Units, 2),
            json!({}),
        );
        let theirs = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let bare = first(in_hand(&mut state, &bare_target().id, PlayerId::P1, 1));
        let tribal = first(in_hand(&mut state, &tribal_hitter().id, PlayerId::P1, 1));

        let bare_decl = first_decl(&state, &bare);
        let tribal_decl = first_decl(&state, &tribal);
        assert_eq!(
            ids(&legal(&state, &bare, &bare_decl)),
            vec![mine.id.clone(), felinor.id.clone(), theirs.id.clone()]
        );
        assert_eq!(
            ids(&legal(&state, &tribal, &tribal_decl)),
            vec![felinor.id.clone()]
        );

        let played = act(&state, targeting(&bare.id, vec![on_instance(&theirs.id)]));
        assert_eq!(played.error, None);
        assert_eq!(top_damage(&played.state, PlayerId::P2, 0), Some(1));
        assert!(
            error_of(&act(
                &state,
                targeting(&bare.id, vec![Selection::Hero { player: PlayerId::P2 }])
            ))
            .contains("not a legal target")
        );
        assert!(
            error_of(&act(&state, targeting(&tribal.id, vec![on_instance(&mine.id)])))
                .contains("not a legal target")
        );
    }

    /// "R90 bounds a 'choose 2 or 3' enumeration at MAX_CHOICE_COMBINATIONS on a wide board (§10.2)"
    #[test]
    fn r90_bounds_a_choose_2_or_3_enumeration_at_max_choice_combinations_on_a_wide_board() {
        let mut state = playing("wide-board");
        for lane in [1, 2, 3, 4, 5] {
            put(
                &mut state,
                &plain.id,
                slot(PlayerId::P1, Row::Units, lane),
                json!({}),
            );
        }
        for lane in [1, 2, 3, 4, 5] {
            put(
                &mut state,
                &plain.id,
                slot(PlayerId::P2, Row::Units, lane),
                json!({}),
            );
        }
        let card = first(in_hand(&mut state, &up_to_three().id, PlayerId::P1, 1));

        // Ten units, so 45 pairs and 120 triples: the enumeration stops at the cap instead.
        let combos = play_choices::play_choice_combinations(&state, PlayerId::P1, &card, None);
        assert_eq!(combos.len(), MAX_CHOICE_COMBINATIONS);
        for combo in &combos {
            let picked: &[Selection] = combo.targets.as_deref().unwrap_or(&[]);
            assert!(picked.len() >= 2);
            assert!(picked.len() <= 3);
            let distinct: IndexSet<String> = ids(picked).into_iter().collect();
            assert_eq!(distinct.len(), picked.len());
            assert_eq!(
                refusal(play_choices::why_choices_refused(
                    &state,
                    PlayerId::P1,
                    &card,
                    &combo_action(&card.id, combo)
                )),
                None
            );
        }

        // The refusal names the plural minimum the declaration asks for.
        let units: Vec<CardInstance> = state
            .players
            .p1
            .units
            .iter()
            .flatten()
            .flatten()
            .cloned()
            .collect();
        let first_unit = units.first().cloned().expect("a unit on p1's side");
        assert!(
            error_of(&act(
                &state,
                targeting(&card.id, vec![on_instance(&first_unit.id)])
            ))
            .contains("needs 2 targets")
        );
    }
}
