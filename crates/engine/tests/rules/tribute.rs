//! Tribute as an additional cost of playing a card (SPEC §6.3's Tribute row, §3.2, §10.5 steps 1-2,
//! R81, R41).
//!
//! §6.3: "Tribute X | As an additional cost of playing a card, sacrifice X of your units; a card
//! whose own text tributes (Carnivorous Cube) sacrifices what that text names instead, which may be
//! any of your other permanents, backrow included (R41) | The play-time cost is the play validator's,
//! and the choice travels in the play action (R81); the Sheep Token counts as 2 toward that X while
//! it is on the field (§3.2), and Lava Golem may pick enemy units. A tribute written into a card's
//! script is an ordinary Sacrifice of the permanent that script names, where the Sheep Token's 2
//! never applies".
//!
//! §3.2: "Sheep Tokens are worth 2 Tributes while on the field." §10.5: step 1 validates "Tribute
//! available", step 2 pays "mana, Tributes (sacrifice)". The declared cost is `staticFlags.tribute`
//! in `src/script.ts`; the channel is `tributes?: string[]` on the `play` action in
//! `packages/shared/src/actions.ts`.
//!
//! Fixtures are prefixed `tb-` and indexed above 1550 so they cannot collide (BUILD §0). The Sheep
//! carries index `T-sheep` and the `tributeWorth` flag its script declares, which is what
//! `playChoices.tributeValueOf` reads.
//!
//! Port of `packages/engine/test/tribute.test.ts`.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::effects::{damage, sacrifice};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, sink_for, slot};

// ---------------------------------------------------------------------------
// Fixtures.
// ---------------------------------------------------------------------------

/// `{ ...defaults, ...extra }`: the keys of `extra` replace the defaults' (TS's object spread).
fn spread(mut base: Value, extra: Value) -> Value {
    if let (Some(fields), Value::Object(over)) = (base.as_object_mut(), extra) {
        fields.extend(over);
    }
    base
}

/// TS `def(name, type, extra)`; `index` is the one TS's `nextIndex` counter gave it (1551 on, in
/// declaration order: `unit()` calls `def()`, and the Sheep's own index then overrides its number).
fn def(name: &str, type_: &str, index: i32, extra: Value) -> CardDef {
    json_as(spread(
        json!({
            "id": format!("tb-{name}"),
            "index": index.to_string(),
            "name": format!("{name} (tribute)"),
            "set": "Core",
            "type": type_,
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 0,
            "base": { "keywords": [], "text": name },
            "radiant": { "keywords": [], "text": name },
        }),
        extra,
    ))
}

fn unit(name: &str, attack: i32, health: i32, index: i32, extra: Value) -> CardDef {
    def(
        name,
        "Unit",
        index,
        spread(
            json!({
                "base": { "attack": attack, "health": health, "keywords": [], "text": name },
                "radiant": { "attack": attack * 2, "health": health * 2, "keywords": [], "text": name },
            }),
            extra,
        ),
    )
}

/// §7's Sheep Token, the one unit "worth 2 Tributes while on the field" (§3.2): its script's
/// `tributeWorth` flag says so (see SCRIPTS); being a unit token it also ceases to exist when it
/// leaves the field rather than reaching a graveyard (R11).
fn sheep() -> CardDef {
    unit(
        "sheep",
        1,
        1,
        1551,
        json!({ "index": SHEEP_TOKEN_INDEX, "tags": ["Token"], "rarity": "Token", "token": true, "cost": 1 }),
    )
}

/// C #82 Sheeople's shape: worth 2 Tributes (Radiant 3) as a declared number, `worth`, which Degrade and
/// Upgrade move (B3.4 rule 5, R386).
fn worthy() -> CardDef {
    unit(
        "worthy",
        1,
        1,
        1552,
        json!({ "params": [{ "key": "worth", "base": 2, "radiant": 3, "better": "up", "step": 1, "min": 1 }] }),
    )
}

/// #66 The Rock's shape: Tribute 1 on a big body.
fn tribute_one() -> CardDef {
    unit("tribute-one", 10, 10, 1553, json!({ "cost": 4 }))
}
/// Tribute 2, the cost one Sheep alone can pay (§3.2).
fn tribute_two() -> CardDef {
    unit("tribute-two", 8, 8, 1554, json!({ "cost": 3 }))
}
/// #55 Lava Golem's shape: Tribute 3 that may pick enemy units.
fn lava_golem() -> CardDef {
    unit("lava-golem", 10, 5, 1555, json!({ "cost": 3 }))
}
/// A body with a Death hook, so "the tribute counts as a death" is observable (§6.3 Sacrifice).
fn death_pinger() -> CardDef {
    unit("death-pinger", 2, 2, 1556, json!({}))
}
/// #22 Carnivorous Cube's shape: its own text tributes a permanent, so no Tribute cost (R41).
fn cube() -> CardDef {
    unit("cube", 4, 6, 1557, json!({ "cost": 3 }))
}
/// A backrow permanent for the Cube to eat, which a Tribute X may never take (§6.3, R41).
fn field_card() -> CardDef {
    def("field-card", "Field Spell", 1558, json!({}))
}

/// #55's patch v0.1.1 base face: paid for with an opposing unit, it is summoned for the opponent (R360).
fn hand_over_golem() -> CardDef {
    unit("hand-over-golem", 10, 5, 1559, json!({ "cost": 3 }))
}
/// A body with Reborn, whose tributed body comes back into the zone it reserved (§4.5 step 4, R64).
fn reborn_body() -> CardDef {
    unit(
        "reborn-body",
        1,
        1,
        1560,
        json!({
            "base": { "attack": 1, "health": 1, "keywords": [{ "kind": "Reborn" }], "text": "Reborn" },
            "radiant": { "attack": 2, "health": 2, "keywords": [{ "kind": "Reborn" }], "text": "Reborn" },
        }),
    )
}

fn defs() -> Vec<CardDef> {
    vec![
        sheep(),
        worthy(),
        tribute_one(),
        tribute_two(),
        lava_golem(),
        death_pinger(),
        cube(),
        field_card(),
        hand_over_golem(),
        reborn_body(),
    ]
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn flags(value: Value) -> Option<StaticFlags> {
    Some(json_as(value))
}

/// #55 "may tribute enemy units". `src/script.ts`'s `StaticFlags` does not declare the flag yet —
/// `src/playChoices.ts` reads it structurally and says so in a comment — so the fixture asserts the
/// shape the engine reads.
fn lava_golem_flags() -> Option<StaticFlags> {
    flags(json!({ "tribute": 3, "tributeEnemies": true }))
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();
    // §3.2, §7: the Sheep's worth is its face's text, the static flag its script declares.
    scripts.insert(
        sheep().id,
        CardScripts {
            base: Script {
                static_flags: flags(json!({ "tributeWorth": SHEEP_TRIBUTE_VALUE })),
                ..Script::default()
            },
            radiant: Script {
                static_flags: flags(json!({ "tributeWorth": RADIANT_SHEEP_TRIBUTE_VALUE })),
                ..Script::default()
            },
        },
    );
    scripts.insert(
        worthy().id,
        CardScripts {
            base: Script {
                static_flags: flags(json!({ "tributeWorth": 2 })),
                ..Script::default()
            },
            radiant: Script {
                static_flags: flags(json!({ "tributeWorth": 3 })),
                ..Script::default()
            },
        },
    );
    scripts.insert(
        tribute_one().id,
        both(Script {
            static_flags: flags(json!({ "tribute": 1 })),
            ..Script::default()
        }),
    );
    scripts.insert(
        tribute_two().id,
        both(Script {
            static_flags: flags(json!({ "tribute": 2 })),
            ..Script::default()
        }),
    );
    scripts.insert(
        lava_golem().id,
        both(Script {
            static_flags: lava_golem_flags(),
            ..Script::default()
        }),
    );
    // R360: only the base face hands itself over; the Radiant face keeps the permission alone.
    scripts.insert(
        hand_over_golem().id,
        CardScripts {
            base: Script {
                static_flags: flags(json!({ "tribute": 3, "tributeEnemies": true, "enemyTributeHandsOver": true })),
                ..Script::default()
            },
            radiant: Script {
                static_flags: flags(json!({ "tribute": 3, "tributeEnemies": true })),
                ..Script::default()
            },
        },
    );
    scripts.insert(
        death_pinger().id,
        both(Script {
            death: Some(hook(|_ctx| vec![damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 3 })))])),
            ..Script::default()
        }),
    );
    // #22: the Cube's own text names what it sacrifices, so the choice is a target, not a Tribute.
    scripts.insert(
        cube().id,
        both(Script {
            targets: vec![TargetDecl::target(
                1,
                1,
                json!({ "side": "ally", "of": ["unit", "backrow"], "excludeSelf": true }),
            )],
            cry: Some(hook(|_ctx| vec![sacrifice(json_as(json!({ "target": { "of": "chosen" } })))])),
            ..Script::default()
        }),
    );
    scripts
}

// ---------------------------------------------------------------------------
// Harness.
// ---------------------------------------------------------------------------

static NONCE: AtomicU32 = AtomicU32::new(0);

fn act_result(state: &GameState, body: Value) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    let input: ActionInput = json_as(body);
    reduce(state, &input.with_nonce(format!("tb{nonce}")))
}

fn act(state: &GameState, body: Value) -> GameState {
    let result = act_result(state, body);
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

/// Past the mulligans, in p1's main phase, with this file's fixtures registered and 4 mana.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(seed, None)).state;
    let keep = ids(&state.players.p1.hand);
    state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p1" }));
    let keep = ids(&state.players.p2.hand);
    state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p2" }));
    let mut catalog = registered_catalog().clone();
    for card in defs() {
        catalog.insert(card.id.clone(), card);
    }
    register_catalog(catalog);
    let mut all = registered_scripts().clone();
    all.extend(scripts());
    register_scripts(all);
    state.players.p1.mana = ManaState {
        current: 4,
        max: 4,
        next_turn_mod: 0,
        perm_mod: 0,
    };
    state
}

fn only<T: Clone>(items: &[T]) -> T {
    items.first().cloned().expect("expected at least one item")
}

fn hand_card(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    only(&in_hand(state, def_id, player, 1))
}

fn unit_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    active_units_of(state, player).iter().map(|card| card.id.clone()).collect()
}

/// The card as it stands in the state now (TS held the live object).
fn live(state: &GameState, card: &CardInstance) -> CardInstance {
    find_instance(state, &card.id).cloned().expect("the card is in the state")
}

/// `whyChoicesRefused(state, "p1", card, { type: "play", instanceId, zone: { row: "units", lane }, tributes, targets? })`
/// as TS's `string | null`.
fn refused(state: &GameState, card: &CardInstance, lane: i32, tributes: &[&str], targets: Option<Value>) -> Option<String> {
    let mut play = json!({
        "type": "play",
        "instanceId": card.id,
        "zone": { "row": "units", "lane": lane },
        "tributes": tributes,
    });
    if let Some(targets) = targets {
        play["targets"] = targets;
    }
    let play: PlayAction = json_as(play);
    why_choices_refused(state, PlayerId::P1, card, &play).err().map(|error| error.message)
}

/// `toMatch(/text/)` on a refusal that must be there.
fn says(refusal: &Option<String>, text: &str) -> bool {
    refusal.as_deref().is_some_and(|message| message.contains(text))
}

fn field_of(events: &[GameEvent], kind: GameEventType, field: &str) -> Vec<Value> {
    events_of_type(events, kind)
        .into_iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises")[field].clone())
        .collect()
}

fn of_type(events: &[GameEvent], kind: GameEventType) -> Value {
    Value::Array(
        events_of_type(events, kind)
            .into_iter()
            .map(|event| serde_json::to_value(event).expect("an event serialises"))
            .collect(),
    )
}

/// `toMatchObject`: every key of `expected` is in `actual` with a matching value.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, value)| actual.get(key).is_some_and(|found| matches_object(found, value))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len()
                && actual.iter().zip(expected).all(|(found, value)| matches_object(found, value))
        }
        _ => actual == expected,
    }
}

fn top(pile: &Option<Pile>) -> Option<&CardInstance> {
    pile.as_ref().and_then(|cards| cards.first())
}

// ---------------------------------------------------------------------------

mod r81_tribute_as_an_additional_cost_of_a_play_6_3_3_2 {
    use super::*;

    #[test]
    fn section_6_3_refuses_a_play_whose_tributes_fall_short_of_its_tribute_x_and_refuses_a_board_that_cannot_pay() {
        let mut state = playing("tribute-short");
        let card = hand_card(&mut state, &tribute_two().id, PlayerId::P1);
        assert_eq!(tribute_cost_of(&card), 2);

        // An empty board cannot pay Tribute 2 at all, so the play is refused outright (§10.5 step 1).
        assert!(says(&refused(&state, &card, 3, &[], None), "needs Tribute 2"));

        // One body pays 1 of the 2, so the board still cannot pay: that is the refusal, whatever the
        // play named.
        let one = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        assert!(says(&refused(&state, &card, 3, &[], None), "needs Tribute 2"));
        assert!(says(&refused(&state, &card, 3, &[&one.id], None), "needs Tribute 2"));

        let two = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 2), json!({}));
        assert_eq!(refused(&state, &card, 3, &[&one.id, &two.id], None), None);
        assert!(says(&refused(&state, &card, 3, &[&one.id], None), "needs Tribute 2"));
        // Naming one unit twice is one unit, not two: a Tribute sacrifices X separate units (§6.3).
        assert!(says(&refused(&state, &card, 3, &[&one.id, &one.id], None), "same unit twice"));
        // The cost is exact: a third unit is not "sacrifice X of your units".
        let three = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 4), json!({}));
        assert!(says(
            &refused(&state, &card, 3, &[&one.id, &two.id, &three.id], None),
            "tributes 2, no more"
        ));
        // And a unit the chooser does not control is not theirs to tribute.
        let theirs = put(&mut state, &plain.id, slot(PlayerId::P2, Row::Units, 1), json!({}));
        assert!(says(&refused(&state, &card, 3, &[&one.id, &theirs.id], None), "cannot be tributed"));

        // A card with no Tribute cost takes no tributes at all (R90's "declared nothing" reading).
        let free = hand_card(&mut state, &death_pinger().id, PlayerId::P1);
        assert_eq!(tribute_cost_of(&free), 0);
        assert!(says(&refused(&state, &free, 5, &[&one.id], None), "needs no Tribute"));
    }

    #[test]
    fn r386_a_unit_that_declares_its_worth_is_worth_what_a_degrade_or_an_upgrade_left_it() {
        let mut state = playing("worth-tuned");
        let base = put(&mut state, &worthy().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let shining = put(&mut state, &worthy().id, slot(PlayerId::P1, Row::Units, 2), json!({}));
        find_instance_mut(&mut state, &shining.id).expect("shining").radiant = true;
        assert_eq!(tribute_value_of(&state, &live(&state, &base)), 2);
        assert_eq!(tribute_value_of(&state, &live(&state, &shining)), 3);

        step_param(find_instance_mut(&mut state, &base.id).expect("base"), "worth", 1);
        step_param(find_instance_mut(&mut state, &shining.id).expect("shining"), "worth", -1);
        assert_eq!(tribute_value_of(&state, &live(&state, &base)), 3);
        assert_eq!(tribute_value_of(&state, &live(&state, &shining)), 2);
    }

    #[test]
    fn section_3_2_a_sheep_token_counts_2_toward_a_tribute_cost_so_one_sheep_alone_pays_tribute_2() {
        let mut state = playing("sheep-counts-two");
        let woolly = put(&mut state, &sheep().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let ordinary = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 2), json!({}));

        assert_eq!(tribute_value_of(&state, &woolly), 2);
        assert_eq!(tribute_value_of(&state, &ordinary), 1);

        let card = hand_card(&mut state, &tribute_two().id, PlayerId::P1);

        // One Sheep is enough; the Sheep plus a body overpays, and an ordinary body alone underpays.
        assert_eq!(refused(&state, &card, 3, &[&woolly.id], None), None);
        assert!(says(&refused(&state, &card, 3, &[&ordinary.id], None), "needs Tribute 2"));
        assert!(says(
            &refused(&state, &card, 3, &[&woolly.id, &ordinary.id], None),
            "tributes 2, no more"
        ));

        // The enumeration says the same thing: the Sheep pays on its own, or two bodies together.
        let sets = legal_tribute_sets(&state, PlayerId::P1, &card);
        assert!(sets.contains(&vec![woolly.id.clone()]));
        assert!(!sets.contains(&vec![ordinary.id.clone()]));
        let third = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 4), json!({}));
        assert!(legal_tribute_sets(&state, PlayerId::P1, &card).contains(&vec![ordinary.id.clone(), third.id.clone()]));

        // Tribute 1 takes the Sheep too — 2 is worth "at least 1", and nothing smaller exists (§3.2).
        let cheap = hand_card(&mut state, &tribute_one().id, PlayerId::P1);
        assert_eq!(refused(&state, &cheap, 5, &[&woolly.id], None), None);
    }

    #[test]
    fn section_6_3_sacrifices_the_tributed_units_which_counts_as_a_death_rather_than_destroying_them() {
        // The primitive first: §6.3's Tribute row pays with a Sacrifice, and Sacrifice "counts as a
        // death" — the destroyed counter, the `destroyed` event and the Death trigger (R78).
        let mut direct = playing("tribute-sacrifice-primitive");
        let doomed = put(&mut direct, &death_pinger().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let before = direct.counters.destroyed;
        let events = {
            let mut sink = sink_for(&mut direct);
            let mut ctx = make_context(
                sink.reborrow(),
                None,
                HookOptions {
                    controller: Some(PlayerId::P1),
                    targets: Some(vec![Selection::Instance {
                        instance_id: doomed.id.clone(),
                    }]),
                    ..Default::default()
                },
            );
            apply_effects(&[sacrifice(json_as(json!({ "target": { "of": "chosen" } })))], &mut ctx);
            drop(ctx);
            sink.events.clone()
        };
        assert_eq!(direct.counters.destroyed, before + 1);
        assert_eq!(field_of(&events, GameEventType::Destroyed, "instanceId"), vec![json!(doomed.id)]);
        assert!(direct.players.p1.graveyard.iter().any(|card| card.id == doomed.id));
        assert_eq!(direct.players.p2.hero.health, HERO_HEALTH - 3); // its Death hook fired

        // And the play pays the same way: the tributed unit is sacrificed at §10.5 step 2, not marked
        // destroyed for the next state check (§6.3 Destroy vs Sacrifice).
        let mut state = playing("tribute-sacrifice-play");
        let food = put(&mut state, &death_pinger().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let card = hand_card(&mut state, &tribute_one().id, PlayerId::P1);
        let played = act_result(
            &state,
            json!({ "type": "play", "instanceId": card.id, "playerId": "p1", "zone": { "row": "units", "lane": 2 }, "tributes": [food.id] }),
        );
        assert_eq!(played.error, None);
        assert_eq!(unit_ids(&played.state, PlayerId::P1), vec![card.id.clone()]);
        assert!(played.state.players.p1.graveyard.iter().any(|held| held.id == food.id));
        assert_eq!(played.state.counters.destroyed, state.counters.destroyed + 1);
        assert_eq!(field_of(&played.events, GameEventType::Destroyed, "instanceId"), vec![json!(food.id)]);
        // A Sacrifice is immediate, so nothing is left marked for the state check to collect.
        assert!(
            !played
                .state
                .players
                .p1
                .units
                .iter()
                .flatten()
                .flatten()
                .any(|unit| unit.marked_destroyed == Some(true))
        );
        // "Counts as a death", so the Death hook ran.
        assert_eq!(played.state.players.p2.hero.health, HERO_HEALTH - 3);
    }

    #[test]
    fn r81_carries_the_tribute_choice_in_the_play_actions_tributes_and_never_opens_a_prompt_for_it() {
        let mut state = playing("tribute-travels");
        let a = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let b = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 2), json!({}));
        let card = hand_card(&mut state, &tribute_two().id, PlayerId::P1);

        // Every way to pay is enumerable ahead of the play, which is what "travels in the action" needs.
        assert_eq!(legal_tribute_sets(&state, PlayerId::P1, &card), vec![vec![a.id.clone(), b.id.clone()]]);

        let plays: Vec<ActionBody> = legal_actions(&state, PlayerId::P1)
            .into_iter()
            .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == card.id))
            .collect();
        assert!(!plays.is_empty());
        for play in &plays {
            let ActionBody::Play { tributes, .. } = play else {
                continue;
            };
            assert_eq!(tributes.as_ref(), Some(&vec![a.id.clone(), b.id.clone()]));
            let as_play: PlayAction = json_as(serde_json::to_value(play).expect("a play serialises"));
            assert!(why_choices_refused(&state, PlayerId::P1, &card, &as_play).is_ok());
        }

        // §10.6: "No Core card opens an `x`, `embiggen`, `zone`, `tribute` or `direction` prompt".
        let played = act_result(
            &state,
            json!({ "type": "play", "instanceId": card.id, "playerId": "p1", "zone": { "row": "units", "lane": 3 }, "tributes": [a.id, b.id] }),
        );
        assert_eq!(played.error, None);
        assert_eq!(played.state.pending, None);
        assert!(
            !played
                .events
                .iter()
                .any(|event| event.event_type() == GameEventType::PromptOpened)
        );
        assert_eq!(unit_ids(&played.state, PlayerId::P1), vec![card.id.clone()]);
    }

    #[test]
    fn r41_section_6_3_treats_a_tribute_a_cards_script_writes_as_an_ordinary_sacrifice_where_the_sheeps_2_never_applies() {
        let mut state = playing("script-tribute");
        let woolly = put(&mut state, &sheep().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let backrow = put(&mut state, &field_card().id, slot(PlayerId::P1, Row::Backrow, 1), json!({}));
        let card = hand_card(&mut state, &cube().id, PlayerId::P1);

        // #22 declares no Tribute cost: what it sacrifices is named by its own text, so the play carries
        // a target, not a tribute, and a `tributes` list is refused.
        assert_eq!(tribute_cost_of(&card), 0);
        assert!(says(
            &refused(
                &state,
                &card,
                2,
                &[&woolly.id],
                Some(json!([{ "pick": "instance", "instanceId": woolly.id }]))
            ),
            "needs no Tribute"
        ));

        // R41: the script's reach is "any of your other permanents, backrow included", which a Tribute X
        // never offers — that cost is "X of your units".
        let rock = hand_card(&mut state, &tribute_one().id, PlayerId::P1);
        assert_eq!(
            legal_tribute_units(&state, PlayerId::P1, &rock)
                .iter()
                .map(|unit| unit.id.clone())
                .collect::<Vec<_>>(),
            vec![woolly.id.clone()]
        );
        let played = act_result(
            &state,
            json!({
                "type": "play",
                "instanceId": card.id,
                "playerId": "p1",
                "zone": { "row": "units", "lane": 2 },
                "targets": [{ "pick": "instance", "instanceId": backrow.id }],
            }),
        );
        assert_eq!(played.error, None);
        assert!(played.state.players.p1.backrow[0].is_none());
        assert_eq!(played.state.counters.destroyed, state.counters.destroyed + 1);

        // And aimed at the Sheep it takes exactly one permanent: the Sheep's 2 is a Tribute value only,
        // so a script Sacrifice can never get two permanents' worth out of one Sheep (§6.3).
        let mut on_sheep = playing("script-tribute-sheep");
        let woolly2 = put(&mut on_sheep, &sheep().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let other = put(&mut on_sheep, &plain.id, slot(PlayerId::P1, Row::Units, 2), json!({}));
        let cube2 = hand_card(&mut on_sheep, &cube().id, PlayerId::P1);
        let eaten = act_result(
            &on_sheep,
            json!({
                "type": "play",
                "instanceId": cube2.id,
                "playerId": "p1",
                "zone": { "row": "units", "lane": 3 },
                "targets": [{ "pick": "instance", "instanceId": woolly2.id }],
            }),
        );
        assert_eq!(eaten.error, None);
        assert_eq!(eaten.state.counters.destroyed, on_sheep.counters.destroyed + 1);
        // R11: a unit token ceases to exist instead of reaching a graveyard, and the other unit stays.
        assert!(!eaten.state.players.p1.graveyard.iter().any(|held| held.id == woolly2.id));
        assert_eq!(unit_ids(&eaten.state, PlayerId::P1), vec![other.id.clone(), cube2.id.clone()]);
    }

    #[test]
    fn section_8_55_counts_both_sides_for_a_tribute_that_may_pick_enemy_units_where_a_sheep_is_still_2() {
        let mut state = playing("lava-golem");
        let mine = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let their_sheep = put(&mut state, &sheep().id, slot(PlayerId::P2, Row::Units, 1), json!({}));
        let their_body = put(&mut state, &plain.id, slot(PlayerId::P2, Row::Units, 2), json!({}));
        let card = hand_card(&mut state, &lava_golem().id, PlayerId::P1);
        assert_eq!(tribute_cost_of(&card), 3);

        // The validator counts both sides' units for #55, and the enemy Sheep is worth 2 there too.
        let offered: Vec<String> =
            legal_tribute_units(&state, PlayerId::P1, &card).iter().map(|unit| unit.id.clone()).collect();
        assert_eq!(offered, vec![mine.id.clone(), their_sheep.id.clone(), their_body.id.clone()]);

        assert_eq!(refused(&state, &card, 2, &[&mine.id, &their_sheep.id], None), None); // 1 + 2 = 3
        assert!(says(&refused(&state, &card, 2, &[&mine.id, &their_body.id], None), "needs Tribute 3")); // 1 + 1 = 2
        assert_eq!(refused(&state, &card, 2, &[&their_sheep.id, &their_body.id], None), None); // 2 + 1 = 3, all enemy
        assert!(legal_tribute_sets(&state, PlayerId::P1, &card).contains(&vec![mine.id.clone(), their_sheep.id.clone()]));

        // Every other Tribute card stays on its own side (§6.3: "X of *your* units").
        let ordinary = hand_card(&mut state, &tribute_two().id, PlayerId::P1);
        assert_eq!(
            legal_tribute_units(&state, PlayerId::P1, &ordinary)
                .iter()
                .map(|unit| unit.id.clone())
                .collect::<Vec<_>>(),
            vec![mine.id.clone()]
        );
    }
}

mod r360_a_tribute_that_takes_an_opposing_unit_summons_55s_base_face_for_the_opponent {
    use super::*;

    #[test]
    fn r360_lands_it_in_the_opponents_zone_in_the_lane_the_play_named_under_their_control_still_the_players_play_and_card() {
        let mut state = playing("hand-over-same-lane");
        let mine = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let mine2 = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 2), json!({}));
        let theirs = put(&mut state, &plain.id, slot(PlayerId::P2, Row::Units, 1), json!({}));
        let card = hand_card(&mut state, &hand_over_golem().id, PlayerId::P1);

        let result = act_result(
            &state,
            json!({
                "type": "play",
                "instanceId": card.id,
                "zone": { "row": "units", "lane": 3 },
                "tributes": [mine.id, mine2.id, theirs.id],
                "playerId": "p1",
            }),
        );
        assert_eq!(result.error, None);
        let after = &result.state;
        let golem = top(&after.players.p2.units[2]);
        assert_eq!(golem.map(|unit| unit.id.clone()), Some(card.id.clone()));
        assert_eq!(golem.map(|unit| unit.controller), Some(PlayerId::P2));
        assert_eq!(golem.map(|unit| unit.owner), Some(PlayerId::P1));
        assert!(after.players.p1.units[2].is_none());
        // It is p1's play (a Sheepish of p2's answers it), and it lands on p2's side.
        assert!(matches_object(
            &of_type(&result.events, GameEventType::CardPlayed),
            &json!([{ "player": "p1", "instanceId": card.id }])
        ));
        assert!(matches_object(
            &of_type(&result.events, GameEventType::Summoned),
            &json!([{ "player": "p2", "instanceId": card.id, "lane": 3 }])
        ));
    }

    #[test]
    fn r360_r15_takes_the_opponents_leftmost_open_zone_when_that_lane_is_taken_there() {
        let mut state = playing("hand-over-leftmost");
        let mine = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let theirs = put(&mut state, &plain.id, slot(PlayerId::P2, Row::Units, 1), json!({}));
        let theirs2 = put(&mut state, &plain.id, slot(PlayerId::P2, Row::Units, 2), json!({}));
        put(&mut state, &plain.id, slot(PlayerId::P2, Row::Units, 4), json!({}));
        let card = hand_card(&mut state, &hand_over_golem().id, PlayerId::P1);

        let after = act(
            &state,
            json!({
                "type": "play",
                "instanceId": card.id,
                "zone": { "row": "units", "lane": 4 },
                "tributes": [mine.id, theirs.id, theirs2.id],
                "playerId": "p1",
            }),
        );
        // p2's lane 4 is taken, so it takes p2's leftmost open zone, lane 1 (freed by the Tribute).
        assert_eq!(top(&after.players.p2.units[0]).map(|unit| unit.id.clone()), Some(card.id.clone()));
        assert_eq!(top(&after.players.p2.units[0]).map(|unit| unit.controller), Some(PlayerId::P2));
    }

    #[test]
    fn r360_stays_with_the_player_when_every_tributed_unit_was_their_own() {
        let mut state = playing("hand-over-own");
        let own: Vec<CardInstance> = [1, 2, 4]
            .into_iter()
            .map(|lane| put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, lane), json!({})))
            .collect();
        put(&mut state, &plain.id, slot(PlayerId::P2, Row::Units, 3), json!({}));
        let card = hand_card(&mut state, &hand_over_golem().id, PlayerId::P1);

        let after = act(
            &state,
            json!({
                "type": "play",
                "instanceId": card.id,
                "zone": { "row": "units", "lane": 3 },
                "tributes": ids(&own),
                "playerId": "p1",
            }),
        );
        assert_eq!(top(&after.players.p1.units[2]).map(|unit| unit.id.clone()), Some(card.id.clone()));
        assert_eq!(top(&after.players.p1.units[2]).map(|unit| unit.controller), Some(PlayerId::P1));
    }

    #[test]
    fn r360_the_radiant_face_keeps_the_permission_and_stays_with_the_player() {
        let mut state = playing("hand-over-radiant");
        let theirs: Vec<CardInstance> = [1, 2, 3]
            .into_iter()
            .map(|lane| put(&mut state, &plain.id, slot(PlayerId::P2, Row::Units, lane), json!({})))
            .collect();
        let card = hand_card(&mut state, &hand_over_golem().id, PlayerId::P1);
        find_instance_mut(&mut state, &card.id).expect("the golem").radiant = true;

        let after = act(
            &state,
            json!({
                "type": "play",
                "instanceId": card.id,
                "zone": { "row": "units", "lane": 3 },
                "tributes": ids(&theirs),
                "playerId": "p1",
            }),
        );
        assert_eq!(top(&after.players.p1.units[2]).map(|unit| unit.id.clone()), Some(card.id.clone()));
        assert!(after.players.p2.units.iter().all(Option::is_none));
    }

    #[test]
    fn r360_stays_with_the_player_when_the_opponents_row_has_no_open_zone_once_the_tribute_is_paid() {
        let mut state = playing("hand-over-full");
        let mine = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let mine2 = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 2), json!({}));
        // p2's row is full, and the one unit of theirs tributed comes back through Reborn into its zone.
        let phoenix = put(&mut state, &reborn_body().id, slot(PlayerId::P2, Row::Units, 1), json!({}));
        for lane in [2, 3, 4, 5] {
            put(&mut state, &plain.id, slot(PlayerId::P2, Row::Units, lane), json!({}));
        }
        let card = hand_card(&mut state, &hand_over_golem().id, PlayerId::P1);

        let after = act(
            &state,
            json!({
                "type": "play",
                "instanceId": card.id,
                "zone": { "row": "units", "lane": 3 },
                "tributes": [mine.id, mine2.id, phoenix.id],
                "playerId": "p1",
            }),
        );
        assert_eq!(
            top(&after.players.p2.units[0]).map(|unit| unit.def_id.clone()),
            Some(reborn_body().id)
        );
        assert_eq!(top(&after.players.p1.units[2]).map(|unit| unit.id.clone()), Some(card.id.clone()));
        assert_eq!(top(&after.players.p1.units[2]).map(|unit| unit.controller), Some(PlayerId::P1));
    }
}
