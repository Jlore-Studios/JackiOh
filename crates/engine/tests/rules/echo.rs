//! Port of `packages/engine/test/echo.test.ts`.
//!
//! Echo and Twinspell (SPEC §6.3's Echo row, §10.5 step 6, §2.2's cleanup line, R30, R70).
//!
//! §6.3 Echo: "Echo X | Recast this card X more times | Play resolves, then the same instance
//! re-resolves X times with fresh mode/target prompts; Twinspell grants Echo +1 to the next spell.
//! The repeats outstanding live in `state.echoQueue` and resolve one at a time in the resolution
//! loop, so a prompt inside one repeat pauses the rest until it is answered (§10.5 step 6)".
//!
//! R30 Twinspell lifetime: "Stays until a spell is played, then goes to the GY" (#79). §2.2:
//! "Twinspell's pending Echo is not turn-scoped and survives cleanup". §8 #79: "The next Spell you
//! play gains Echo +1", radiant "Echo +2", "Consumed to the GY when it applies (ruling)". R70: "a
//! cast Spell does use Twinspell's Echo".
//!
//! DISCREPANCY (the whole file): SPEC §10.1 declares `echoQueue: EchoRepeat[]` on `GameState` and
//! §6.3 puts the outstanding repeats there, but `grep -rn "echoQueue" packages/engine/src` returns
//! nothing — the field does not exist, and no engine module repeats a played card. The only echo
//! machinery in the source is `state.ts`'s player modifier `{ kind: "echoNextSpell"; amount: number;
//! sourceId?: string }`, which nothing reads: `grep -rn "echoNextSpell" packages/engine/src` finds
//! only that type declaration. Per CLAUDE.md the tests below assert what SPEC says and each
//! affected one carries its own DISCREPANCY note; `turn.test.ts`'s R30 test and
//! `replay-scripted.test.ts`'s `state.echoQueue.length` are the surviving witnesses to the same gap.
//!
//! Fixtures are prefixed `ec-` and indexed above 1500 so they cannot collide (BUILD §0).

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::effects::add_to_hand as add_to_hand_effect;
use jackioh_engine::effects::{chosen_options, damage, discover_from_catalog};
use jackioh_engine::modifiers::add_modifier;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, slot};

// ---------------------------------------------------------------------------
// Fixtures.
// ---------------------------------------------------------------------------

/// TS `def(name, type, extra)`, its module `nextIndex` (from 1500) written out per definition, and
/// `extra` merged over the literal as TS's spread does.
fn def(name: &str, type_: &str, index: i32, extra: Value) -> CardDef {
    let mut literal = json!({
        "id": format!("ec-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (echo)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    });
    if let (Some(fields), Value::Object(extra)) = (literal.as_object_mut(), extra) {
        for (key, value) in extra {
            fields.insert(key, value);
        }
    }
    json_as(literal)
}

/// #79 Twinspell: a Field Spell that gives the next Spell you play Echo +1, or +2 radiant.
fn twinspell() -> CardDef {
    def("twinspell", "Field Spell", 1501, json!({ "cost": 2 }))
}

/// A Spell whose whole effect is countable, so N resolutions read as 2N damage.
fn pinger() -> CardDef {
    def("pinger", "Spell", 1502, json!({}))
}

/// A Spell whose resolution prompts, so a prompt inside one repeat can be seen to pause the rest.
fn ask_spell() -> CardDef {
    def("ask-spell", "Spell", 1503, json!({}))
}

/// A permanent with a Cry, so a cast can be read on something that goes to the field, not the GY.
fn cast_unit() -> CardDef {
    def(
        "cast-unit",
        "Unit",
        1504,
        json!({
            "base": { "attack": 1, "health": 3, "keywords": [], "text": "cast-unit" },
            "radiant": { "attack": 2, "health": 6, "keywords": [], "text": "cast-unit" },
        }),
    )
}

/// The same, with a printed Echo 1 (§6.1), which is the only Echo a permanent can have (R30).
fn echo_unit() -> CardDef {
    def(
        "echo-unit",
        "Unit",
        1505,
        json!({
            "base": { "attack": 1, "health": 3, "keywords": [], "text": "echo-unit" },
            "radiant": { "attack": 2, "health": 6, "keywords": [], "text": "echo-unit" },
        }),
    )
}

/// R802: a Spell whose Echo X is its caster's max mana (Meditative #5's shape, with a multiple of 1).
fn echo_x() -> CardDef {
    def("echo-x", "Spell", 1530, json!({}))
}

/// The same, whose resolution also raises its caster's max mana by 2, so a later read would differ.
fn echo_x_grow() -> CardDef {
    def("echo-x-grow", "Spell", 1531, json!({}))
}

/// The same computed Echo X beside a printed Echo 2: the larger holds.
fn echo_x_printed() -> CardDef {
    def("echo-x-printed", "Spell", 1532, json!({}))
}

fn defs() -> Vec<CardDef> {
    vec![
        twinspell(),
        pinger(),
        ask_spell(),
        cast_unit(),
        echo_unit(),
        echo_x(),
        echo_x_grow(),
        echo_x_printed(),
    ]
}

/// R802's fixture hook: X is the caster's max mana.
fn max_mana_echo() -> Option<EchoXHook> {
    Some(read_hook(|args| max_mana_of(args.state, args.self_.controller)))
}

/// Raises the running card's controller's max mana by 2 (a test-only write).
fn grow_max_mana() -> Effect {
    Effect::new("ec:growMaxMana", |ctx| {
        let controller = ctx.controller;
        ctx.state.players[controller].mana.max += 2;
    })
}

/// #79's engine half: the pending Echo is a player modifier naming the Twinspell that granted it.
fn grant_echo(amount: i32) -> Effect {
    Effect::new("ec:grantEcho", move |ctx| {
        let source_id = ctx.self_.as_ref().map(|card| card.id.clone());
        let controller = ctx.controller;
        add_modifier(
            ctx,
            controller,
            ModifierExpiry::Used,
            ModifierKind::EchoNextSpell { amount, source_id },
        );
    })
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn hit_two() -> Hook {
    hook(|_ctx| {
        vec![damage(json_as(
            json!({ "to": { "of": "enemyHero" }, "amount": 2 }),
        ))]
    })
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();
    scripts.insert(
        twinspell().id,
        CardScripts {
            base: Script {
                cry: Some(hook(|_ctx| vec![grant_echo(1)])),
                ..Script::default()
            },
            radiant: Script {
                cry: Some(hook(|_ctx| vec![grant_echo(2)])),
                ..Script::default()
            },
        },
    );
    scripts.insert(
        pinger().id,
        both(Script {
            cry: Some(hit_two()),
            ..Script::default()
        }),
    );
    scripts.insert(
        cast_unit().id,
        both(Script {
            cry: Some(hit_two()),
            ..Script::default()
        }),
    );
    scripts.insert(
        echo_unit().id,
        both(Script {
            static_flags: Some(json_as(json!({ "echo": 1 }))),
            cry: Some(hit_two()),
            ..Script::default()
        }),
    );
    scripts.insert(
        echo_x().id,
        both(Script {
            echo_x: max_mana_echo(),
            cry: Some(hit_two()),
            ..Script::default()
        }),
    );
    scripts.insert(
        echo_x_grow().id,
        both(Script {
            echo_x: max_mana_echo(),
            cry: Some(hook(|_ctx| {
                vec![
                    damage(json_as(
                        json!({ "to": { "of": "enemyHero" }, "amount": 2 }),
                    )),
                    grow_max_mana(),
                ]
            })),
            ..Script::default()
        }),
    );
    scripts.insert(
        echo_x_printed().id,
        both(Script {
            static_flags: Some(json_as(json!({ "echo": 2 }))),
            echo_x: max_mana_echo(),
            cry: Some(hit_two()),
            ..Script::default()
        }),
    );
    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    resume.insert(
        "picked",
        hook(|ctx| match chosen_options(ctx).first() {
            None => vec![],
            Some(def_id) => vec![add_to_hand_effect(json_as(json!({ "defId": def_id })))],
        }),
    );
    scripts.insert(
        ask_spell().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![discover_from_catalog(json_as(
                    json!({ "step": "picked", "query": { "type": "Unit" } }),
                ))]
            })),
            resume,
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
    let n = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    let mut action = body;
    action["nonce"] = json!(format!("ec{n}"));
    reduce(state, &json_as::<Action>(action))
}

fn act(state: &GameState, body: Value) -> GameState {
    let result = act_result(state, body);
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

fn four_mana() -> ManaState {
    ManaState {
        current: 4,
        max: 4,
        next_turn_mod: 0,
        perm_mod: 0,
    }
}

/// Past the mulligans, in p1's main phase, with this file's fixtures registered and 4 mana.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(seed, None)).state;
    let keep: Vec<String> = state.players.p1.hand.iter().map(|c| c.id.clone()).collect();
    state = act(
        &state,
        json!({ "type": "mulligan", "keep": keep, "playerId": "p1" }),
    );
    let keep: Vec<String> = state.players.p2.hand.iter().map(|c| c.id.clone()).collect();
    state = act(
        &state,
        json!({ "type": "mulligan", "keep": keep, "playerId": "p2" }),
    );
    let mut catalog = registered_catalog().clone();
    for card in defs() {
        catalog.insert(card.id.clone(), card);
    }
    register_catalog(catalog);
    let mut all = registered_scripts().clone();
    all.extend(scripts());
    register_scripts(all);
    state.players.p1.mana = four_mana();
    state
}

fn only<T: Clone>(items: &[T]) -> T {
    items.first().cloned().expect("expected at least one item")
}

fn hand_card(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    only(&in_hand(state, def_id, player, 1))
}

fn echo_mods(state: &GameState, player: PlayerId) -> Vec<PlayerModifier> {
    state.players[player]
        .mods
        .iter()
        .filter(|modifier| matches!(modifier.kind, ModifierKind::EchoNextSpell { .. }))
        .cloned()
        .collect()
}

/// §10.1's `echoQueue`.
///
/// DISCREPANCY: `state.ts`'s `GameState` declares no `echoQueue` and `createGame` initialises none,
/// although SPEC §10.1 lists it beside `triggerQueue` and §6.3 puts the outstanding repeats there;
/// `src/playSteps.ts` works around the same gap with its own lazily-created `echoQueueOf`. The test
/// below asserts the field is on a fresh state, which is where the gap shows.
fn echo_queue(state: &GameState) -> Vec<EchoItem> {
    state.echo_queue.clone()
}

/// TS `sinkFor(state)`: a sink's events and rng (from the state's cursor), lent with the state to one
/// engine call at a time.
struct Bench {
    events: Vec<GameEvent>,
    rng: Rng,
}

impl Bench {
    fn new(state: &GameState) -> Bench {
        Bench {
            events: Vec::new(),
            rng: Rng::new(&state.seed, state.rng_cursor),
        }
    }

    fn sink<'a>(&'a mut self, state: &'a mut GameState) -> EngineSink<'a> {
        EngineSink::new(state, &mut self.events, &mut self.rng)
    }
}

/// An Echo the engine owes, granted the way #79 grants one, so each test starts from one line.
fn pending_echo(state: &mut GameState, player: PlayerId, amount: i32) {
    let mut bench = Bench::new(state);
    add_modifier(
        &mut bench.sink(state),
        player,
        ModifierExpiry::Used,
        ModifierKind::EchoNextSpell {
            amount,
            source_id: None,
        },
    );
}

/// `eventsOfType(events, type).map((event) => event[key])`, over the events' JSON.
fn pluck(events: &[GameEvent], kind: GameEventType, key: &str) -> Vec<Value> {
    events_of_type(events, kind)
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises")[key].clone())
        .collect()
}

fn in_pile(pile: &[CardInstance], id: &str) -> bool {
    pile.iter().any(|held| held.id == id)
}

// ---------------------------------------------------------------------------

mod r30_r70_echo_and_twinspell_s6_3_s10_5_step_6 {
    use super::*;

    #[test]
    fn r30_keeps_a_pending_echo_until_a_spell_is_played_across_cleanup_then_sends_the_twinspell_to_the_gy() {
        let mut state = playing("r30-lifetime");
        let card = hand_card(&mut state, &twinspell().id, PlayerId::P1);
        state = act(
            &state,
            json!({ "type": "play", "instanceId": card.id, "playerId": "p1", "zone": { "row": "backrow", "lane": 1 } }),
        );

        // #79's Echo +1, tied to the Twinspell that granted it, and not spent by playing the Twinspell.
        let granted = only(&echo_mods(&state, PlayerId::P1));
        let ModifierKind::EchoNextSpell { amount, source_id } = &granted.kind else {
            panic!("an echoNextSpell modifier");
        };
        assert_eq!(*amount, 1);
        assert_eq!(granted.expiry, ModifierExpiry::Used);
        assert_eq!(source_id.as_deref(), Some(card.id.as_str()));
        assert_eq!(
            state.players.p1.backrow[0].as_ref().map(|held| held.id.clone()),
            Some(card.id.clone())
        );

        // §2.2: "Twinspell's pending Echo is not turn-scoped and survives cleanup" — two turns of it.
        state = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        assert_eq!(echo_mods(&state, PlayerId::P1).len(), 1);
        state = act(&state, json!({ "type": "endTurn", "playerId": "p2" }));
        assert_eq!(echo_mods(&state, PlayerId::P1).len(), 1);
        state.players.p1.mana = four_mana();

        // R30: playing a Spell is what ends it, and the Twinspell goes to the graveyard as it applies.
        // DISCREPANCY: nothing in packages/engine/src reads `echoNextSpell`, so the modifier is never
        // consumed and #79 never leaves the backrow. SPEC R30 and §8 #79 both say it must.
        let spell = hand_card(&mut state, &pinger().id, PlayerId::P1);
        state = act(
            &state,
            json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" }),
        );
        assert_eq!(echo_mods(&state, PlayerId::P1), Vec::<PlayerModifier>::new());
        assert!(in_pile(&state.players.p1.graveyard, &card.id));
        assert!(state.players.p1.backrow[0].is_none());
    }

    #[test]
    fn s6_3_re_resolves_a_played_spell_once_for_echo_1_and_twice_for_twinspell_radiants_echo_2() {
        // DISCREPANCY: no engine module repeats a played card. SPEC §6.3 ("Recast this card X more
        // times … the same instance re-resolves X times") and §10.5 step 6 ("Echo: repeat step 5 with
        // fresh prompts N times") both require it, so each play below lands only its first resolution.
        let mut once = playing("echo-1");
        pending_echo(&mut once, PlayerId::P1, 1);
        let card = hand_card(&mut once, &pinger().id, PlayerId::P1);
        let first = act_result(
            &once,
            json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }),
        );
        assert_eq!(first.error, None);
        // Play resolves, then the same instance re-resolves 1 more time: 2 damage twice.
        assert_eq!(first.state.players.p2.hero.health, HERO_HEALTH - 4);
        // One play, two resolutions: the repeat is not a second play, so `played` rises once (§6.3).
        assert_eq!(first.state.counters.played, once.counters.played + 1);

        let mut twice = playing("echo-2");
        pending_echo(&mut twice, PlayerId::P1, 2);
        let card = hand_card(&mut twice, &pinger().id, PlayerId::P1);
        let second = act_result(
            &twice,
            json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }),
        );
        assert_eq!(second.error, None);
        assert_eq!(second.state.players.p2.hero.health, HERO_HEALTH - 6);
    }

    #[test]
    fn s10_1_declares_echo_queue_on_every_game_state_beside_trigger_queue_and_delayed() {
        // DISCREPANCY: SPEC §10.1's state model lists `echoQueue: EchoRepeat[]` on `GameState`, so
        // every state carries one from `createGame` onward, the way `triggerQueue` and `delayed` do.
        // `packages/engine/src/state.ts` declares no such field and `createGame` initialises none;
        // `src/playSteps.ts` works around it with a lazily-created `echoQueueOf`, and
        // `replay-scripted.test.ts` reads `state.echoQueue.length` expecting the field to be there.
        let state = playing("echo-queue-field");
        let json = serde_json::to_value(&state).expect("a state serialises");
        assert!(json.get("echoQueue").is_some());
        assert_eq!(echo_queue(&state), Vec::<EchoItem>::new());
        assert!(json.get("triggerQueue").is_some());
        assert!(json.get("delayed").is_some());
    }

    #[test]
    fn s6_3_holds_the_outstanding_repeats_in_state_echo_queue_and_resolves_them_one_at_a_time() {
        let mut state = playing("echo-queue");
        assert_eq!(echo_queue(&state), Vec::<EchoItem>::new());

        pending_echo(&mut state, PlayerId::P1, 2);
        let card = hand_card(&mut state, &pinger().id, PlayerId::P1);
        let played = act_result(
            &state,
            json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }),
        );
        assert_eq!(played.error, None);

        // The repeats are state, not a loop variable, so they survive a JSON round trip (§9.3, §10.1).
        let round: GameState =
            serde_json::from_value(serde_json::to_value(&played.state).expect("a state serialises"))
                .expect("and parses");
        assert_eq!(echo_queue(&round), echo_queue(&played.state));
        // And by the time the action returns the loop has drained them (§10.3).
        assert_eq!(echo_queue(&played.state), Vec::<EchoItem>::new());
        assert_eq!(played.state.players.p2.hero.health, HERO_HEALTH - 6);
    }

    #[test]
    fn r81_s10_5_step_6_asks_fresh_prompts_inside_each_echo_repeat_and_one_pauses_the_repeats_behind_it() {
        // §6.3: "the same instance re-resolves X times with fresh mode/target prompts … a prompt inside
        // one repeat pauses the rest until it is answered". With Echo 2 there are three resolutions and
        // three Discovers, each its own prompt, and the repeat still owed waits in the queue meanwhile.
        let mut state = playing("echo-prompt");
        pending_echo(&mut state, PlayerId::P1, 2);
        let card = hand_card(&mut state, &ask_spell().id, PlayerId::P1);
        let played = act_result(
            &state,
            json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }),
        );
        assert_eq!(played.error, None);

        // Answer the open Discover with its first option, and say what opened next.
        let answer = |from: &GameState| -> GameState {
            let pending = from.pending.as_ref();
            assert_eq!(pending.map(|open| open.kind), Some(PromptKind::Discover));
            assert_eq!(pending.map(|open| open.player_id), Some(PlayerId::P1));
            let options = pending.map(|open| open.options.clone()).unwrap_or_default();
            act(
                from,
                json!({
                    "type": "answer",
                    "choiceId": pending.map(|open| open.id.clone()).unwrap_or_default(),
                    "selection": [only(&options).selection],
                    "playerId": "p1",
                }),
            )
        };

        // Resolution 1's prompt is open, and both repeats are still owed behind it.
        let first_id = played.state.pending.as_ref().map(|open| open.id.clone());
        assert_eq!(
            played.state.pending.as_ref().map(|open| open.kind),
            Some(PromptKind::Discover)
        );

        // Repeat 1 asks its own fresh prompt, with the last repeat still waiting in the queue (§6.3).
        let after_first = answer(&played.state);
        assert_eq!(
            after_first.pending.as_ref().map(|open| open.kind),
            Some(PromptKind::Discover)
        );
        assert_ne!(after_first.pending.as_ref().map(|open| open.id.clone()), first_id);
        assert_eq!(echo_queue(&after_first).len(), 1);

        // Repeat 2 asks the last one, and once it is answered nothing is owed and nothing is pending.
        let after_second = answer(&after_first);
        assert_eq!(
            after_second.pending.as_ref().map(|open| open.kind),
            Some(PromptKind::Discover)
        );
        assert_eq!(echo_queue(&after_second), Vec::<EchoItem>::new());
        let done = answer(&after_second);
        assert_eq!(done.pending, None);
        assert_eq!(echo_queue(&done), Vec::<EchoItem>::new());
    }

    #[test]
    fn r70_gives_a_cast_spell_twinspells_echo_even_though_the_cast_itself_pays_nothing() {
        // DISCREPANCY: `castCard` in packages/engine/src/resolve.ts runs the card's hook exactly once
        // and never looks at `echoNextSpell`. R70: "A cast never uses a cost discount, since it pays
        // nothing, but a cast Spell does use Twinspell's Echo".
        let mut state = playing("r70-cast-echo");
        pending_echo(&mut state, PlayerId::P1, 1);
        let card = hand_card(&mut state, &pinger().id, PlayerId::P1);

        let mut bench = Bench::new(&state);
        cast_card(&mut bench.sink(&mut state), &card, Default::default());
        settle(&mut bench.sink(&mut state), Default::default());

        // Free, counted as a play, and repeated once: 2 damage twice.
        assert_eq!(
            pluck(&bench.events, GameEventType::CardPlayed, "costPaid"),
            vec![json!(0)]
        );
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 4);
        assert_eq!(echo_mods(&state, PlayerId::P1), Vec::<PlayerModifier>::new());
        assert!(in_pile(&state.players.p1.graveyard, &card.id));
    }

    #[test]
    fn r70_sends_a_cast_permanent_to_the_field_not_the_graveyard_and_fires_its_cry_there_once() {
        // §6.3's Cast row: the card "goes where its type sends it", and §10.5 step 4 sends a permanent
        // to the field — the leftmost free zone, as a play that names none takes (R64). This is the case
        // step 7 could have swallowed: a card left `resolving` is graveyarded there.
        let mut state = playing("r70-cast-permanent");
        let before = state.counters.played;
        let card = hand_card(&mut state, &cast_unit().id, PlayerId::P1);

        let mut bench = Bench::new(&state);
        cast_card(&mut bench.sink(&mut state), &card, Default::default());
        settle(&mut bench.sink(&mut state), Default::default());

        assert_eq!(
            state.players.p1.units[0]
                .as_ref()
                .and_then(|pile| pile.first())
                .map(|held| held.id.clone()),
            Some(card.id.clone())
        );
        assert_eq!(
            find_instance(&state, &card.id).map(|held| held.zone.clone()),
            Some(Zone::Field {
                player: PlayerId::P1,
                row: Row::Units,
                lane: 1
            })
        );
        assert!(!in_pile(&state.players.p1.graveyard, &card.id));
        assert!(!in_pile(&state.players.p1.hand, &card.id));
        assert_eq!(state.players.p1.resolving, Vec::<CardInstance>::new());

        // A cast is a play: free, counted, `cardPlayed` and the `summoned` of a card entering the field.
        assert_eq!(
            pluck(&bench.events, GameEventType::CardPlayed, "costPaid"),
            vec![json!(0)]
        );
        assert_eq!(
            pluck(&bench.events, GameEventType::Summoned, "instanceId"),
            vec![json!(card.id)]
        );
        assert_eq!(state.counters.played, before + 1);
        // Once, not twice: the Cry has one owner (R1, R117).
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 2);
    }

    #[test]
    fn r70_s6_3_repeats_a_cast_permanents_printed_echo_and_leaves_it_on_the_field_not_in_the_gy() {
        let mut state = playing("r70-cast-permanent-echo");
        let card = hand_card(&mut state, &echo_unit().id, PlayerId::P1);

        let mut bench = Bench::new(&state);
        cast_card(&mut bench.sink(&mut state), &card, Default::default());
        // A cast is §10.5's pipeline (R70), so its repeat resolves inside the cast: nothing asked, so
        // nothing was owed to `state.work` (R117), and the cast is whole when the call returns.
        assert_eq!(echo_queue(&state), Vec::<EchoItem>::new());
        assert_eq!(state.work, Vec::<WorkItem>::new());
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 4);
        settle(&mut bench.sink(&mut state), Default::default());

        // Two resolutions, one play, and the card is on the field with nothing left owed.
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 4);
        assert_eq!(events_of_type(&bench.events, GameEventType::CardPlayed).len(), 1);
        assert_eq!(
            state.players.p1.units[0]
                .as_ref()
                .and_then(|pile| pile.first())
                .map(|held| held.id.clone()),
            Some(card.id.clone())
        );
        assert!(!in_pile(&state.players.p1.graveyard, &card.id));
        assert_eq!(echo_queue(&state), Vec::<EchoItem>::new());
        assert_eq!(state.work, Vec::<WorkItem>::new());
    }

    #[test]
    fn r30_leaves_the_pending_echo_armed_for_a_cast_permanent_since_only_a_spell_takes_it() {
        // R30 is "the next Spell you play"; R70 gives a cast Spell that grant, and a permanent — cast or
        // played — neither takes it nor spends the Twinspell that is holding it.
        let mut state = playing("r30-cast-permanent");
        pending_echo(&mut state, PlayerId::P1, 1);
        let card = hand_card(&mut state, &cast_unit().id, PlayerId::P1);

        let mut bench = Bench::new(&state);
        cast_card(&mut bench.sink(&mut state), &card, Default::default());
        settle(&mut bench.sink(&mut state), Default::default());

        assert_eq!(echo_mods(&state, PlayerId::P1).len(), 1);
        assert_eq!(echo_queue(&state), Vec::<EchoItem>::new());
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 2);
    }

    #[test]
    fn s10_5_step_7_lands_a_cast_permanent_with_no_free_zone_in_the_graveyard_after_it_resolves() {
        // A played permanent with no room is refused in step 1 ("no free zone"); a cast cannot be
        // refused, so the card resolves and is then still in `resolving`, which step 7 empties. NOTE:
        // §11 does not rule on this corner — it is reported with this milestone, not decided here.
        let mut state = playing("r70-cast-permanent-full");
        for lane in 1..=UNIT_ZONES {
            put(
                &mut state,
                &cast_unit().id,
                slot(PlayerId::P1, Row::Units, lane),
                Default::default(),
            );
        }
        let card = hand_card(&mut state, &cast_unit().id, PlayerId::P1);

        let mut bench = Bench::new(&state);
        cast_card(&mut bench.sink(&mut state), &card, Default::default());
        settle(&mut bench.sink(&mut state), Default::default());

        assert!(in_pile(&state.players.p1.graveyard, &card.id));
        assert_eq!(state.players.p1.resolving, Vec::<CardInstance>::new());
        assert_eq!(events_of_type(&bench.events, GameEventType::Summoned).len(), 0);
        // It still resolved: a cast fires the script whatever happens to the card afterwards (R70).
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 2);
    }
}

mod r802_computed_echo_x {
    use super::*;

    /// p1 at `max` max mana and 4 mana to spend, playing `def_id` from hand.
    fn play_at(seed: &str, def_id: &str, max: i32) -> (GameState, GameState) {
        let mut state = playing(seed);
        state.players.p1.mana.max = max;
        let card = hand_card(&mut state, def_id, PlayerId::P1);
        let after = act(
            &state,
            json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }),
        );
        (state, after)
    }

    #[test]
    fn r802_repeats_x_times_from_the_casters_max_mana_as_it_is_played() {
        // X = 1: one repeat, two resolutions of 2 damage.
        let (_, one) = play_at("r802-one", &echo_x().id, 1);
        assert_eq!(one.players.p2.hero.health, HERO_HEALTH - 4);
        // X = 3: three repeats, four resolutions.
        let (before, three) = play_at("r802-three", &echo_x().id, 3);
        assert_eq!(three.players.p2.hero.health, HERO_HEALTH - 8);
        // One play however many resolutions.
        assert_eq!(three.counters.played, before.counters.played + 1);
        assert_eq!(echo_queue(&three), Vec::<EchoItem>::new());
    }

    #[test]
    fn r802_max_mana_gained_while_it_resolves_changes_nothing() {
        // X is read once, at §10.5 step 4: each resolution raises max mana by 2, and still only the
        // two repeats X = 2 queued follow the first resolution.
        let (_, after) = play_at("r802-grow", &echo_x_grow().id, 2);
        assert_eq!(after.players.p2.hero.health, HERO_HEALTH - 6);
        assert_eq!(after.players.p1.mana.max, 2 + 3 * 2);
    }

    #[test]
    fn r802_the_larger_of_printed_and_computed_echo_holds() {
        // Printed Echo 2 against X = 1: Echo 2, three resolutions.
        let (_, low) = play_at("r802-printed-low", &echo_x_printed().id, 1);
        assert_eq!(low.players.p2.hero.health, HERO_HEALTH - 6);
        // Printed Echo 2 against X = 4: Echo 4, five resolutions; they do not add up.
        let (_, high) = play_at("r802-printed-high", &echo_x_printed().id, 4);
        assert_eq!(high.players.p2.hero.health, HERO_HEALTH - 10);
    }

    #[test]
    fn r802_twinspell_adds_one_to_a_computed_echo() {
        let mut state = playing("r802-twinspell");
        state.players.p1.mana.max = 2;
        pending_echo(&mut state, PlayerId::P1, 1);
        let card = hand_card(&mut state, &echo_x().id, PlayerId::P1);
        let after = act(
            &state,
            json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }),
        );
        // X = 2, plus the grant's 1: three repeats, four resolutions.
        assert_eq!(after.players.p2.hero.health, HERO_HEALTH - 8);
        assert_eq!(echo_mods(&after, PlayerId::P1), Vec::<PlayerModifier>::new());
    }

    #[test]
    fn r802_a_cast_reads_its_casters_max_mana() {
        let mut state = playing("r802-cast");
        state.players.p1.mana.max = 3;
        state.players.p2.mana.max = 1;
        let card = hand_card(&mut state, &echo_x().id, PlayerId::P1);
        let mut bench = Bench::new(&state);
        cast_card(&mut bench.sink(&mut state), &card, Default::default());
        settle(&mut bench.sink(&mut state), Default::default());
        // p1's max mana, 3: four resolutions, though p2's is 1.
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 8);
    }
}
