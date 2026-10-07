//! R153: a card registers only the triggers its zone allows (SPEC §10.3's registry, §6.2's
//! start-of-turn and end-of-turn hooks, §3.2 and R13 on a Stack, R68 on the graveyard return).
//!
//! The registry is keyed by hook AND by zone, and the zone half is what this file pins:
//!
//!   * on the field or in the backrow a card answers its `triggers` plus its `startOfTurn`,
//!     `endOfTurn`, `aura`, `setStat` and `onPlayHook` hooks;
//!   * in a hand it answers only `handTriggers` (#89 Corpse Eater) — no hook at all;
//!   * in a graveyard only the end-of-turn return of a spell that flagged itself when it was played
//!     (#23 Reoccurring Dream, #24 Efficiency Dividend, #31 KY's Math Equation, R68);
//!   * in a library, in exile, in the resolving zone or dormant under a Stack, nothing (R13).
//!
//! `triggers.triggerHoldersWithHook` read the hook off the script and ignored the zone, so a card in
//! a HAND or a GRAVEYARD answered every hook it carried. Neither half raised an error; both produced
//! a different game, which is why every test below asserts the board and not the registry alone:
//!
//!   * #58 Rush Token Farm in a hand summoned a Rush Token at the start of every turn, moving a unit
//!     into a lane from a card that was never on the board;
//!   * #64 Gifted Program in a hand made a 1-cost play Radiant, and a radiant #8 Mr. Vanilla is a
//!     7/7 rather than a 3/3 — a combat spec's arithmetic, silently rewritten.
//!
//! Each test has a second half on purpose: the same card on the field or in the backrow must fire.
//! Without it every assertion here would pass just as well on a card that never fires at all.
//!
//! Every fixture is its own: defs are prefixed `tz-` and indexed above 2500, so they cannot collide
//! with another test file's catalog (BUILD §0).
//!
//! Port of `packages/engine/test/trigger-zones.test.ts`.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{in_hand, new_game, put, slot};

// ---------------------------------------------------------------------------
// Fixtures. TS numbered them from a module counter starting at 2500; each def's index is written
// out here in the order TS created them.
// ---------------------------------------------------------------------------

fn def(name: &str, index: u32, type_: &str, extra: Value) -> CardDef {
    let mut literal = json!({
        "id": format!("tz-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (trigger zones)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    });
    if let (Some(into), Some(from)) = (literal.as_object_mut(), extra.as_object()) {
        for (key, value) in from {
            into.insert(key.clone(), value.clone());
        }
    }
    json_as(literal)
}

/// TS `unit(name, attack = 2, health = 4)`.
fn unit(name: &str, index: u32) -> CardDef {
    let (attack, health) = (2, 4);
    def(
        name,
        index,
        "Unit",
        json!({
            "base": { "attack": attack, "health": health, "keywords": [], "text": name },
            "radiant": { "attack": attack * 2, "health": health * 2, "keywords": [], "text": name },
        }),
    )
}

/// The note sink: a script-less Field Spell parked in p1's backrow lane 5.
fn log_card() -> CardDef {
    def("log", 2501, "Field Spell", json!({}))
}
/// #58 Rush Token Farm's shape: a Field Spell that summons a token at every start of turn.
fn farm() -> CardDef {
    def("farm", 2502, "Field Spell", json!({}))
}
/// #64 Gifted Program's shape: a Field Spell with a pre-resolution hook on every play.
fn gifter() -> CardDef {
    def("gifter", 2503, "Field Spell", json!({}))
}
/// #23/#24/#31's shape: a Spell that comes back from the graveyard at the end of the turn.
fn wanderer() -> CardDef {
    def("wanderer", 2504, "Spell", json!({}))
}
/// #13 Jlockeed Shredder-10's shape: a UNIT with an end-of-turn hook, which the graveyard denies.
fn shredder() -> CardDef {
    unit("shredder", 2505)
}
/// A unit with a start-of-turn hook, to be buried under a Stack (§3.2, R13).
fn sleeper() -> CardDef {
    unit("sleeper", 2506)
}
/// Classic #65 Ace in the Hole's shape: a Trap whose end-of-turn hook answers while it is face-down.
fn hidden_hook() -> CardDef {
    def("hidden-hook", 2507, "Trap", json!({}))
}
/// The same hook on a public card, a Field Spell, whose entry is numbered.
fn shown_hook() -> CardDef {
    def("shown-hook", 2508, "Field Spell", json!({}))
}

fn defs() -> Vec<CardDef> {
    vec![
        log_card(),
        farm(),
        gifter(),
        wanderer(),
        shredder(),
        sleeper(),
        hidden_hook(),
        shown_hook(),
    ]
}

/// The fixture catalog's Rush token, the thing #58's shape puts on the board.
const TOKEN_ID: &str = "fx-token-rush";
/// A plain 1-cost vanilla unit from the fixture catalog: the card the `onPlayHook` test plays.
const BAIT_ID: &str = "fx-1";

// ---------------------------------------------------------------------------
// The note log: what fired, in the order it fired.
// ---------------------------------------------------------------------------

const NOTE_LANE: usize = 5;

fn log_of(state: &GameState) -> Option<&CardInstance> {
    let log_id = log_card().id;
    state
        .players
        .p1
        .backrow
        .get(NOTE_LANE - 1)
        .and_then(Option::as_ref)
        .filter(|card| card.def_id == log_id)
}

fn log_of_mut(state: &mut GameState) -> Option<&mut CardInstance> {
    let log_id = log_card().id;
    state
        .players
        .p1
        .backrow
        .get_mut(NOTE_LANE - 1)
        .and_then(Option::as_mut)
        .filter(|card| card.def_id == log_id)
}

fn note(name: impl Into<String>) -> Effect {
    let name: String = name.into();
    Effect::new("tz:note", move |ctx| {
        let Some(log) = log_of_mut(ctx.state) else {
            return;
        };
        let mut steps: Vec<Value> = log
            .memory
            .get("steps")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        steps.push(json!(name));
        log.memory.insert("steps".into(), Value::Array(steps));
    })
}

fn notes(state: &GameState) -> Vec<String> {
    log_of(state)
        .and_then(|log| log.memory.get("steps"))
        .and_then(Value::as_array)
        .map(|steps| steps.iter().filter_map(|step| step.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn scripts() -> Vec<(String, CardScripts)> {
    vec![
        (
            farm().id,
            both(Script {
                start_of_turn: Some(hook(|_ctx| {
                    vec![
                        note("farm:start"),
                        effects::summon(json_as(json!({ "defId": TOKEN_ID }))),
                    ]
                })),
                ..Script::default()
            }),
        ),
        (
            gifter().id,
            both(Script {
                on_play_hook: Some(hook(|_ctx| vec![note("gifter:onPlay")])),
                ..Script::default()
            }),
        ),
        // Both hooks, so the graveyard's "end-of-turn return and nothing else" is a real restriction and
        // not just the only hook the card happens to carry.
        (
            wanderer().id,
            both(Script {
                start_of_turn: Some(hook(|_ctx| vec![note("wanderer:start")])),
                end_of_turn: Some(hook(|_ctx| vec![note("wanderer:end")])),
                ..Script::default()
            }),
        ),
        (
            shredder().id,
            both(Script {
                end_of_turn: Some(hook(|_ctx| vec![note("shredder:end")])),
                ..Script::default()
            }),
        ),
        (
            sleeper().id,
            both(Script {
                start_of_turn: Some(hook(|ctx| {
                    let id = ctx.self_.as_ref().map_or_else(|| "none".to_string(), |card| card.id.clone());
                    vec![note(format!("sleeper:{id}"))]
                })),
                ..Script::default()
            }),
        ),
        (
            hidden_hook().id,
            both(Script {
                end_of_turn: Some(hook(|_ctx| vec![note("hidden:end")])),
                ..Script::default()
            }),
        ),
        (
            shown_hook().id,
            both(Script {
                end_of_turn: Some(hook(|_ctx| vec![note("shown:end")])),
                ..Script::default()
            }),
        ),
    ]
}

// ---------------------------------------------------------------------------
// Harness.
// ---------------------------------------------------------------------------

/// A fresh game whose catalog and script registry also carry this file's fixtures.
fn game(seed: &str) -> GameState {
    let state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut registry = registered_scripts().clone();
    for (id, script) in scripts() {
        registry.insert(id, script);
    }
    register_scripts(registry);
    state
}

/// TS's module-level `let nonce`.
static NONCE: AtomicU32 = AtomicU32::new(0);

/// `body` is the TS `ActionInput` literal; the nonce is added here.
fn act(state: &GameState, body: Value) -> GameState {
    let nonce = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    let mut action = body;
    action["nonce"] = json!(format!("tz{nonce}"));
    let result = reduce(state, &json_as::<Action>(action));
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

fn hand_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player].hand.iter().map(|card| card.id.clone()).collect()
}

/// Past the mulligans, in p1's main phase, with the note log parked in p1's backrow lane 5.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&game(seed)).state;
    state = act(
        &state,
        json!({ "type": "mulligan", "keep": hand_ids(&state, PlayerId::P1), "playerId": "p1" }),
    );
    state = act(
        &state,
        json!({ "type": "mulligan", "keep": hand_ids(&state, PlayerId::P2), "playerId": "p2" }),
    );
    put(&mut state, &log_card().id, slot(PlayerId::P1, Row::Backrow, NOTE_LANE as i32));
    state
}

/// Round the table once, so p1's turn starts and p1's `startOfTurn` hooks are offered (§6.2).
fn round_to_p1_start(state: &GameState) -> GameState {
    act(
        &act(state, json!({ "type": "endTurn", "playerId": "p1" })),
        json!({ "type": "endTurn", "playerId": "p2" }),
    )
}

/// A card put straight into a graveyard, the way a mill or a discard leaves one there.
fn bury(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    let card = new_instance(state, def_id, player, Zone::Graveyard { player });
    state.players[player].graveyard.push(card.clone());
    card
}

/// A Stack card pushed onto an occupied unit zone (§3.2); the harness's `put` fills empty ones.
fn stack_onto(state: &mut GameState, def_id: &str, player: PlayerId, lane: i32) -> CardInstance {
    let mut card = new_instance(state, def_id, player, Zone::Hand { player });
    assert!(place_on_field(
        state,
        &mut card,
        slot(player, Row::Units, lane),
        json_as(json!({ "stack": true })),
    ));
    card
}

fn tokens_of(state: &GameState, player: PlayerId) -> usize {
    active_units_of(state, player)
        .into_iter()
        .filter(|card| card.def_id == TOKEN_ID)
        .count()
}

fn only<T>(items: Vec<T>) -> T {
    items.into_iter().next().expect("expected at least one item")
}

/// The live card TS's test kept a handle on, to write through as TS wrote through it.
fn card_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no instance {id}"))
}

fn holder_def_ids(holders: &[TriggerHolder]) -> Vec<String> {
    holders.iter().map(|holder| holder.card.def_id.clone()).collect()
}

fn holder_ids(holders: &[TriggerHolder]) -> Vec<String> {
    holders.iter().map(|holder| holder.card.id.clone()).collect()
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

// ---------------------------------------------------------------------------

mod r153_a_card_registers_only_the_triggers_its_zone_allows_s10_3_s6_2_r13_r68 {
    use super::*;

    #[test]
    fn r153_a_start_of_turn_hook_in_a_hand_does_nothing_and_the_same_card_in_the_backrow_fires() {
        // The hand half: #58's shape sits in p1's hand over a whole turn cycle and never acts.
        let mut held = playing("tz-start-hand");
        let card = only(in_hand(&mut held, &farm().id, PlayerId::P1, 1));
        assert!(trigger_holders_with_hook(&held, HookName::StartOfTurn, Some(PlayerId::P1)).is_empty());

        let after_hand = round_to_p1_start(&held);
        assert!(notes(&after_hand).is_empty());
        // The observable damage the bug did: a unit in a lane, from a card that was never on the board.
        assert_eq!(tokens_of(&after_hand, PlayerId::P1), 0);
        assert!(after_hand.players.p1.hand.iter().any(|in_hand_now| in_hand_now.id == card.id));

        // The backrow half, which is what stops the assertions above passing on a card that never
        // fires: the same fixture, the same turn cycle, on the board.
        let mut placed = playing("tz-start-field");
        put(&mut placed, &farm().id, slot(PlayerId::P1, Row::Backrow, 1));
        assert_eq!(
            holder_def_ids(&trigger_holders_with_hook(
                &placed,
                HookName::StartOfTurn,
                Some(PlayerId::P1)
            )),
            vec![farm().id]
        );

        let after_field = round_to_p1_start(&placed);
        assert_eq!(notes(&after_field), strings(&["farm:start"]));
        assert_eq!(tokens_of(&after_field, PlayerId::P1), 1);
    }

    #[test]
    fn r153_an_on_play_hook_in_a_hand_does_not_see_the_play_and_the_same_card_in_the_backrow_does() {
        // #64's shape: the hook runs at §10.5 step 3, before the played card resolves.
        let mut held = playing("tz-onplay-hand");
        in_hand(&mut held, &gifter().id, PlayerId::P1, 1);
        assert!(trigger_holders_with_hook(&held, HookName::OnPlayHook, None).is_empty());

        let bait = only(in_hand(&mut held, BAIT_ID, PlayerId::P1, 1));
        let after_hand = act(
            &held,
            json!({
                "type": "play",
                "instanceId": bait.id,
                "zone": { "row": "units", "lane": 2 },
                "playerId": "p1",
            }),
        );
        assert!(notes(&after_hand).is_empty());

        let mut placed = playing("tz-onplay-field");
        put(&mut placed, &gifter().id, slot(PlayerId::P1, Row::Backrow, 1));
        assert_eq!(
            holder_def_ids(&trigger_holders_with_hook(&placed, HookName::OnPlayHook, None)),
            vec![gifter().id]
        );

        let other = only(in_hand(&mut placed, BAIT_ID, PlayerId::P1, 1));
        let after_field = act(
            &placed,
            json!({
                "type": "play",
                "instanceId": other.id,
                "zone": { "row": "units", "lane": 2 },
                "playerId": "p1",
            }),
        );
        assert_eq!(notes(&after_field), strings(&["gifter:onPlay"]));
    }

    #[test]
    fn r153_r68_a_graveyard_spell_flagged_when_it_was_played_still_answers_its_end_of_turn_return() {
        let mut state = playing("tz-gy-flagged");
        let spell = bury(&mut state, &wanderer().id, PlayerId::P1);
        // §5.1: "Spells with 'End of turn: add this back to your hand' are flagged
        // `returnToHandAtEndOfTurn` when they are played". This is that flag, and nothing else.
        card_mut(&mut state, &spell.id).return_to_hand_at_end_of_turn = Some(true);
        assert_eq!(
            holder_ids(&trigger_holders_with_hook(&state, HookName::EndOfTurn, Some(PlayerId::P1))),
            vec![spell.id.clone()]
        );

        let ended = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        assert_eq!(notes(&ended), strings(&["wanderer:end"]));
    }

    #[test]
    fn r153_the_graveyard_answers_the_end_of_turn_return_only_not_a_start_of_turn_hook_on_the_same_spell() {
        let mut state = playing("tz-gy-start");
        let spell = bury(&mut state, &wanderer().id, PlayerId::P1);
        card_mut(&mut state, &spell.id).return_to_hand_at_end_of_turn = Some(true);
        // The same instance that answers `endOfTurn` above answers no `startOfTurn` at all.
        assert!(trigger_holders_with_hook(&state, HookName::StartOfTurn, Some(PlayerId::P1)).is_empty());

        let after_start = round_to_p1_start(&state);
        let starts: Vec<String> = notes(&after_start)
            .into_iter()
            .filter(|step| step == "wanderer:start")
            .collect();
        assert_eq!(starts, Vec::<String>::new());
    }

    #[test]
    fn r155_the_flag_not_the_turn_log_is_what_lets_a_graveyard_spell_answer_its_return() {
        // This test used to assert the turn log, because nothing set `returnToHandAtEndOfTurn` and the
        // registry took the log as a stand-in. R155 gives §10.5 step 7 the setter, so the flag is now
        // the gate and the log is no longer consulted — which is strictly more correct, since the log
        // cannot tell a spell that asked to return from a card that merely happened to be played.
        let mut played = playing("tz-gy-log");
        let spell = bury(&mut played, &wanderer().id, PlayerId::P1);
        card_mut(&mut played, &spell.id).return_to_hand_at_end_of_turn = Some(true);

        assert_eq!(
            notes(&act(&played, json!({ "type": "endTurn", "playerId": "p1" }))),
            strings(&["wanderer:end"])
        );

        // Being in this turn's play log is NOT enough on its own any more: that is the R155 change,
        // and asserting it here is what stops the flag check silently reverting to the old behaviour.
        let mut logged = playing("tz-gy-logonly");
        let via_log = bury(&mut logged, &wanderer().id, PlayerId::P1);
        logged.players.p1.turn_log.played_ids.push(via_log.id.clone());
        assert!(trigger_holders_with_hook(&logged, HookName::EndOfTurn, Some(PlayerId::P1)).is_empty());
        assert!(notes(&act(&logged, json!({ "type": "endTurn", "playerId": "p1" }))).is_empty());

        // Unflagged and unplayed: a copy that was milled or discarded, or one left from an earlier
        // turn, stays in the graveyard and answers nothing.
        let mut stale = playing("tz-gy-stale");
        bury(&mut stale, &wanderer().id, PlayerId::P1);
        assert!(trigger_holders_with_hook(&stale, HookName::EndOfTurn, Some(PlayerId::P1)).is_empty());
        assert!(notes(&act(&stale, json!({ "type": "endTurn", "playerId": "p1" }))).is_empty());
    }

    #[test]
    fn r153_a_unit_that_died_on_the_turn_it_was_played_does_not_fire_its_end_of_turn_hook_from_the_graveyard() {
        // #13 Jlockeed Shredder-10's shape. It is in the graveyard and in this turn's play log, so the
        // log alone would let it shred from there; the §5.1 return belongs to spells.
        let mut dead = playing("tz-gy-unit");
        let corpse = bury(&mut dead, &shredder().id, PlayerId::P1);
        dead.players.p1.turn_log.played_ids.push(corpse.id.clone());
        assert!(trigger_holders_with_hook(&dead, HookName::EndOfTurn, Some(PlayerId::P1)).is_empty());
        assert!(notes(&act(&dead, json!({ "type": "endTurn", "playerId": "p1" }))).is_empty());

        // On the field the same unit fires, so the assertion above is about the zone and not the card.
        let mut alive = playing("tz-field-unit");
        put(&mut alive, &shredder().id, slot(PlayerId::P1, Row::Units, 1));
        assert_eq!(
            notes(&act(&alive, json!({ "type": "endTurn", "playerId": "p1" }))),
            strings(&["shredder:end"])
        );
    }

    #[test]
    fn r153_r13_a_card_dormant_under_a_stack_answers_nothing_while_the_top_of_the_pile_fires() {
        let mut state = playing("tz-stack");
        let dormant = put(&mut state, &sleeper().id, slot(PlayerId::P1, Row::Units, 1));
        let top = stack_onto(&mut state, &sleeper().id, PlayerId::P1, 1);
        assert_eq!(
            state.players.p1.units[0]
                .as_ref()
                .map(|pile| pile.iter().map(|card| card.id.clone()).collect::<Vec<_>>()),
            Some(vec![top.id.clone(), dormant.id.clone()])
        );

        // §3.2: cards under a Stack are not on the field, so only the top of the pile is a holder.
        assert_eq!(
            holder_ids(&trigger_holders_with_hook(&state, HookName::StartOfTurn, Some(PlayerId::P1))),
            vec![top.id.clone()]
        );

        let started = round_to_p1_start(&state);
        assert_eq!(notes(&started), vec![format!("sleeper:{}", top.id)]);
    }

    #[test]
    fn r153_r177_a_face_down_trap_answers_its_end_of_turn_hook_and_its_queue_entry_takes_no_number_from_the_counter_both_seats_read()
     {
        let bare = act(
            &playing("tz-hidden-hook"),
            json!({ "type": "endTurn", "playerId": "p1" }),
        );

        let mut hidden = playing("tz-hidden-hook");
        put(&mut hidden, &hidden_hook().id, slot(PlayerId::P1, Row::Backrow, 1));
        let after_hidden = act(&hidden, json!({ "type": "endTurn", "playerId": "p1" }));
        assert_eq!(notes(&after_hidden), strings(&["hidden:end"]));
        assert_eq!(after_hidden.next_seq, bare.next_seq);

        // The public half, so the count above measures something: the same hook on a Field Spell is numbered.
        let mut shown = playing("tz-hidden-hook");
        put(&mut shown, &shown_hook().id, slot(PlayerId::P1, Row::Backrow, 1));
        let after_shown = act(&shown, json!({ "type": "endTurn", "playerId": "p1" }));
        assert_eq!(notes(&after_shown), strings(&["shown:end"]));
        assert_eq!(after_shown.next_seq, bare.next_seq + 1);
    }
}
