//! Setup and the mulligan when a question is asked during it (SPEC §2.1, §2.4, §9.3, §10.1, §10.6, R9,
//! R151, R158, R224, R635). Found by the polish-4 edge-case hunt, round 7 (docs/polish/4-edge-cases.md,
//! lenses L7, "legality-agreement" and "engine invariants"); every case here failed before its fix.
//!
//!  - R224: setup used to open the next mulligan over a question a draw's cast had asked, which
//!    replaced it and left the cast half-played in its caster's resolving zone for good. A cast-on-draw
//!    card is no longer dealt by setup (R635), so what asks during it now is a start-of-game clause run
//!    as its card arrives in a hand (R151), the one thing a card setup draws can still do. The rule is
//!    the same: setup owes the rest of itself behind the question and the answer finishes it (R113,
//!    R122).
//!  - §10.6: the mulligan's `promptAnswered` named the word "mulligan", not the prompt.
//!  - Round 8 (lens L10). R225: a Quickdraw card is the last of the opening draws it replaces,
//!    reported and counted as a draw, so #100's price, the deal's events and the counts while setup
//!    waits (R224) do not tell the other seat whether the opening hand holds one.
//!  - Round 10 (lens L8, and L7 for the clause that asks). Setup is turn 0, no player's turn (§2.1):
//!    a Spell a mulligan's replacement draw cast kept its return flag into its caster's first turn
//!    end (R155), and p1's cast armed an end-of-turn clause for turn 1 that p2's did not (R241). A
//!    start-of-game clause that asks at §2.1 step 4 now holds the rest of setup, and turn 1, until
//!    it is answered (R151, R113). Those two cases keep their casts: R635 sets a cast-on-draw card
//!    aside, so the Spell is cast by a start-of-game clause as it arrives in the hand a replacement
//!    draw fills, which is the same cast on turn 0.
//!
//! No Core card has a start-of-game clause that asks, so the asking card is a fixture (a transient def
//! and a registered script, the way paused-sequences.test.ts builds its asking cards).
//!
//! Port of `packages/cards/test/setup-and-mulligan.test.ts` (SURFACE §4.1, §8). Importing the TS
//! harness registered the real catalog and every card script (`registerAll()`); here each case calls
//! `jackioh_cards::register_all()` first. These cases build their games through `create_game`, since
//! a `scenario()` starts past the mulligan.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_cards::{CATALOG, register_all};
use jackioh_engine::effects;
use jackioh_engine::testkit::*;

fn must<T>(value: Option<T>, what: &str) -> T {
    match value {
        Some(value) => value,
        None => panic!("expected {what}"),
    }
}

fn fixture_def(id: &str, type_: CardType) -> CardDef {
    let face = if type_ == CardType::Unit {
        json!({ "attack": 2, "health": 2, "keywords": [], "text": id })
    } else {
        json!({ "keywords": [], "text": id })
    };
    json_as(json!({
        "id": id,
        "index": id,
        "name": id,
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face.clone(),
        "radiant": face,
    }))
}

/// `registerScripts({ ...registeredScripts(), [id]: { base: script, radiant: script } })`.
fn register_fixture_script(id: &str, script: Script) {
    let mut scripts = registered_scripts().clone();
    scripts.insert(
        id.to_string(),
        CardScripts {
            base: script.clone(),
            radiant: script,
        },
    );
    register_scripts(scripts);
}

/// A fixture card: a transient def in the match state and its script in the registry.
fn fixture(state: &mut GameState, id: &str, type_: CardType, script: Script) {
    state
        .transient_defs
        .insert(id.to_string(), fixture_def(id, type_));
    register_fixture_script(id, script);
}

/// R151: a start-of-game clause runs as its card arrives in a hand, so a card setup draws can ask a
/// question. It asks once: §2.1 step 4 runs the clause of every card still in a hand or library again
/// (R153).
fn asking_on_arrival(prompt: &str) -> Script {
    let prompt = prompt.to_string();
    Script {
        start_of_game: Some(hook(move |ctx| {
            let asked = ctx.live_self().and_then(|card| card.memory.get("asked")) == Some(&json!(true));
            if asked {
                vec![]
            } else {
                vec![
                    effects::remember(json_as(json!({ "key": "asked", "value": true }))),
                    effects::choose_mode(json_as(
                        json!({ "options": ["ok"], "step": "ok", "prompt": prompt }),
                    )),
                ]
            }
        })),
        resume: IndexMap::from([("ok", hook(|_ctx| vec![]))]),
        ..Script::default()
    }
}

/// A start-of-game clause that casts its card as it arrives in a hand, which only setup does (turn 0).
/// (TS `CAST_ON_SETUP_ARRIVAL: Pick<Script, "startOfGame">`.)
fn cast_on_setup_arrival() -> Script {
    Script {
        start_of_game: Some(hook(|ctx| {
            if ctx.state.turn == SETUP_TURN {
                vec![effects::cast(json_as(json!({ "target": { "of": "self" } })))]
            } else {
                vec![]
            }
        })),
        ..Script::default()
    }
}

/// Two legal decks straight from the catalog: the first 40 non-token cards in id order.
fn catalog_decks() -> (Vec<String>, Vec<String>) {
    let mut pool: Vec<String> = CATALOG
        .iter()
        .filter(|(_, def)| !def.token && !def.tags.contains(&Tag::Token))
        .map(|(id, _)| id.clone())
        .collect();
    pool.sort();
    let size = DECK_SIZE as usize;
    (pool[..size].to_vec(), pool[size..size * 2].to_vec())
}

static NONCE: AtomicU32 = AtomicU32::new(0);

/// An action through `reduce` with a fresh nonce; a refusal fails the test with the engine's message.
fn act(state: &GameState, body: Value) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    let action = json_as::<ActionInput>(body).with_nonce(format!("setup-mulligan-{nonce}"));
    let result = reduce(state, &action);
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result
}

/// `Array.from({ length: 20 }, (_, at) => \`core-${String(at + 1).padStart(3, "0")}\`)`.
fn p1_deck() -> Vec<String> {
    (0..20).map(|at| format!("core-{:03}", at + 1)).collect()
}

/// `Array.from({ length: 20 }, (_, at) => \`core-${String(at + 30).padStart(3, "0")}\`)`.
fn p2_deck() -> Vec<String> {
    (0..20).map(|at| format!("core-{:03}", at + 30)).collect()
}

fn game_args(seed: &str, decks: (Vec<String>, Vec<String>)) -> CreateGameArgs {
    CreateGameArgs {
        seed: seed.to_string(),
        decks,
        ..Default::default()
    }
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn mode_answer(choice_id: &str, option: &str, player: PlayerId) -> Value {
    json!({
        "type": "answer",
        "choiceId": choice_id,
        "selection": [{ "pick": "mode", "option": option }],
        "playerId": player,
    })
}

mod r224_setup_waits_for_a_question {
    use super::*;

    #[test]
    fn r224_r9_r151_r158_r265_the_question_a_replacement_draw_s_arrival_clause_asks_p1_is_not_overwritten_by_p2_s_mulligan_10_1()
     {
        register_all();
        let mut state = begin_game(&create_game(&game_args(
            "edge-r7-l7-mulligan",
            (p1_deck(), p2_deck()),
        )))
        .state;
        assert_eq!(
            mulligan_prompt_for(&state, PlayerId::P1).map(|prompt| prompt.kind),
            Some(PromptKind::Mulligan)
        );
        assert_eq!(mulligan_owed(&state), vec![PlayerId::P1, PlayerId::P2]);

        // A card whose start-of-game clause asks, on top of p1's library: the replacement draw reaches it.
        fixture(
            &mut state,
            "edge-r7-l7-asks",
            CardType::Spell,
            asking_on_arrival("the clause's question"),
        );
        let asks = new_instance(
            &mut state,
            "edge-r7-l7-asks",
            PlayerId::P1,
            Zone::Library { player: PlayerId::P1 },
        );
        state.players.p1.library.insert(0, asks);

        // p1 returns one card: R9 draws the replacement first, and it is the asking card (§2.4).
        // The answer is sealed until p2 answers too (R266); p2 returns one card as well.
        let hand = ids(&state.players.p1.hand);
        let returned = must(hand.first().cloned(), "a card to return");
        state = act(
            &state,
            json!({ "type": "mulligan", "keep": &hand[1..], "playerId": "p1" }),
        )
        .state;
        assert_eq!(ids(&state.players.p1.hand), hand);
        let p2_hand = ids(&state.players.p2.hand);
        let p2_returned = must(p2_hand.first().cloned(), "a card p2 returns");
        let result = act(
            &state,
            json!({ "type": "mulligan", "keep": &p2_hand[1..], "playerId": "p2" }),
        );
        state = result.state;
        // Both are in, so both resolve in seat order (R265): p1's replacement arrived, and its clause
        // asked p1 (R151).
        assert!(result.events.iter().any(|event| matches!(
            event,
            GameEvent::PromptOpened {
                kind: PromptKind::Mode,
                ..
            }
        )));

        // §9.3, §10.1: one prompt at a time, and the question is state until p1 answers it. p2's
        // resolution waits behind it: p2's hand is as it answered, and p1's returned card waits to go back.
        let pending = must(state.pending.clone(), "an open prompt");
        assert_eq!(
            pending.player_id,
            PlayerId::P1,
            "open prompt: {} \"{}\"",
            pending.kind,
            pending.prompt
        );
        assert_eq!(pending.kind, PromptKind::Mode);
        assert!(!state.players.p1.library.iter().any(|card| card.id == returned));
        assert_eq!(ids(&state.players.p2.hand), p2_hand);
        assert_eq!(state.mulliganed, Vec::<PlayerId>::new());

        // The answer finishes the rest of p1's mulligan (R122): the returned card is shuffled back, then
        // p2's sealed answer resolves, and then the game begins.
        state = act(&state, mode_answer(&pending.id, "ok", PlayerId::P1)).state;
        assert!(state.players.p1.library.iter().any(|card| card.id == returned));
        assert!(!ids(&state.players.p2.hand).contains(&p2_returned));
        assert!(state.players.p2.library.iter().any(|card| card.id == p2_returned));
        assert_eq!(state.mulliganed, vec![PlayerId::P1, PlayerId::P2]);
        assert_eq!(state.turn, 1);
    }

    #[test]
    fn r224_r151_r158_a_question_from_the_opening_draw_s_arrival_clauses_is_not_written_over_by_the_mulligan_2_1()
     {
        register_all();
        let deck: Vec<String> = catalog::query(&json_as(json!({})))
            .iter()
            .map(|def| def.id.clone())
            .take(20)
            .collect();
        let mut game = create_game(&game_args("edge-r7-setup", (deck.clone(), deck)));
        // A card whose start-of-game clause asks, as it arrives in a hand (R151).
        let id = "edge-r7-asking";
        fixture(
            &mut game,
            id,
            CardType::Spell,
            asking_on_arrival("the clause's question"),
        );
        let count = game.players.p1.library.len();
        let fresh: Vec<CardInstance> = (0..count)
            .map(|_| {
                new_instance(
                    &mut game,
                    id,
                    PlayerId::P1,
                    Zone::Library { player: PlayerId::P1 },
                )
            })
            .collect();
        game.players.p1.library = fresh;

        let started = begin_game(&game);
        let asked: Option<String> = started.events.iter().find_map(|event| match event {
            GameEvent::PromptOpened {
                kind: PromptKind::Mode,
                choice_id,
                ..
            } => Some(choice_id.clone()),
            _ => None,
        });
        assert!(
            asked.is_some(),
            "p1's opening draw reaches the card, whose clause asks"
        );

        // R158: a draw a prompt interrupts stops there and owes the rest, and §2.1's mulligan follows
        // the opening draw. A second prompt never overwrites an unanswered one (R156), so the question
        // stays open until it is answered.
        let answered = started.events.iter().any(|event| match event {
            GameEvent::PromptAnswered { choice_id, .. } => asked.as_deref() == Some(choice_id.as_str()),
            _ => false,
        });
        let still_open = asked.is_some()
            && started.state.pending.as_ref().map(|pending| pending.id.as_str()) == asked.as_deref();
        assert!(
            answered || still_open,
            "the question was replaced by a {} prompt",
            started
                .state
                .pending
                .as_ref()
                .map(|pending| pending.kind.to_string())
                .unwrap_or_else(|| "no".to_string())
        );

        // Every card in p1's library is one of these, so each of the three opening draws asks in turn.
        // Answering each goes on with the opening deal, and the first mulligan opens only once nothing is
        // asking.
        let mut state = started.state;
        let mut questions = 0;
        let mut guard = 0;
        while guard < 40 && state.pending.as_ref().map(|pending| pending.kind) == Some(PromptKind::Mode) {
            let open = must(state.pending.clone(), "a clause's question");
            state = act(&state, mode_answer(&open.id, "ok", PlayerId::P1)).state;
            questions += 1;
            guard += 1;
        }
        assert_eq!(questions, OPENING_DRAW[0]);
        assert_eq!(state.pending, None);
        assert_eq!(mulligan_owed(&state), vec![PlayerId::P1, PlayerId::P2]);
        assert_eq!(state.players.p1.hand.len(), OPENING_DRAW[0] as usize);
        assert_eq!(state.players.p2.hand.len(), OPENING_DRAW[1] as usize);
    }

    #[test]
    fn r224_the_mulligan_s_prompt_answered_names_the_prompt_that_prompt_opened_named_10_3_10_6() {
        register_all();
        let begun = begin_game(&create_game(&game_args("r7-mulligan-ids", catalog_decks())));
        let opened_id: Option<String> = begun.events.iter().find_map(|event| match event {
            GameEvent::PromptOpened { choice_id, .. } => Some(choice_id.clone()),
            _ => None,
        });
        assert!(opened_id.is_some());
        assert_eq!(
            mulligan_prompt_for(&begun.state, PlayerId::P1).map(|prompt| prompt.id.clone()),
            opened_id
        );

        let answered = reduce(
            &begun.state,
            &json_as::<Action>(json!({ "type": "mulligan", "keep": [], "playerId": "p1", "nonce": "m1" })),
        );
        assert_eq!(answered.error, None);
        let closed: Option<String> = answered.events.iter().find_map(|event| match event {
            GameEvent::PromptAnswered { choice_id, .. } => Some(choice_id.clone()),
            _ => None,
        });
        // Every other prompt's answer names its PendingChoice id (`prompts.ts`); the mulligan's named
        // the word "mulligan", which no prompt ever had.
        assert_eq!(closed, opened_id);
    }
}

const HEROIC_POWER: &str = "core-098";
const CRAFT: &str = "core-099";
const VOID: &str = "core-100";

fn same_view(a: &PlayerView, b: &PlayerView) {
    assert_eq!(b.events, a.events);
    assert_eq!(b, a);
}

// ---------------------------------------------------------------------------
// Quickdraw and the game's draw counter (#100 Ceaseless Void, R55)
// ---------------------------------------------------------------------------

const QD_P1_DECK: [&str; 20] = [
    "core-003", "core-004", "core-005", "core-007", "core-008", "core-010", "core-011", "core-015",
    "core-018", "core-023", "core-031", "core-035", "core-036", "core-039", "core-041", "core-044",
    "core-048", "core-050", "core-060", VOID,
];
/// p2's deck less one card: the twentieth is #98 Heroic Power (Quickdraw) or #99 Craft a Card.
const QD_P2_SHARED: [&str; 19] = [
    "core-003", "core-004", "core-005", "core-007", "core-008", "core-011", "core-015", "core-023",
    "core-031", "core-035", "core-036", "core-044", "core-050", "core-062", "core-063", "core-081",
    "core-082", "core-086", "core-012",
];

fn owned(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| id.to_string()).collect()
}

/// `[...QD_P2_SHARED, twentieth]`.
fn p2_qd_deck(twentieth: &str) -> Vec<String> {
    let mut deck = owned(&QD_P2_SHARED);
    deck.push(twentieth.to_string());
    deck
}

/// `act(state, { ...body, playerId: player }).state`.
fn act_as(state: &GameState, player: PlayerId, body: Value) -> GameState {
    let mut body = body;
    body["playerId"] = json!(player);
    act(state, body).state
}

/// Both players keep their opening hands; p1's first turn has begun.
fn keep_both(state: &GameState) -> GameState {
    let next = act_as(
        state,
        PlayerId::P1,
        json!({ "type": "mulligan", "keep": ids(&state.players.p1.hand) }),
    );
    act_as(
        &next,
        PlayerId::P2,
        json!({ "type": "mulligan", "keep": ids(&next.players.p2.hand) }),
    )
}

mod r225_a_quickdraw_card_is_counted_as_the_draw_it_replaces {
    use super::*;

    /// "r8-l10-void-3" puts #100 in p1's opening hand.
    fn void_game(p2_twentieth: &str) -> GameState {
        let decks = (owned(&QD_P1_DECK), p2_qd_deck(p2_twentieth));
        keep_both(&begin_game(&create_game(&game_args("r8-l10-void-3", decks))).state)
    }

    #[test]
    fn r225_r55_p1_s_ceaseless_void_does_not_price_in_whether_p2_s_opening_hand_holds_a_quickdraw_card_2_1_9_1()
     {
        register_all();
        let with_power = void_game(HEROIC_POWER);
        let without = void_game(CRAFT);

        // Same public course: p1's turn 1 has begun, p2 holds its 4 opening cards and The Coin (R244),
        // and p1 holds its Void.
        for state in [&with_power, &without] {
            assert_eq!(state.turn, 1);
            assert_eq!(state.active, PlayerId::P1);
            assert_eq!(
                view_for(state, PlayerId::P1).opponent.hand,
                HandView::Count { count: 5 }
            );
            let hand = view_for(state, PlayerId::P1).you.hand;
            assert!(matches!(&hand, HandView::Cards(cards) if cards.iter().any(|card| card.def_id == VOID)));
        }
        // The Heroic Power started in p2's hand "instead of a draw" (§6.2 Quickdraw).
        assert!(
            with_power
                .players
                .p2
                .hand
                .iter()
                .any(|card| card.def_id == HEROIC_POWER)
        );

        // §2.1: p2's opening hand is 4 cards either way, and whether one of them is a Quickdraw card is
        // p2's hand (§9.1). The Void's cost counts draws (R55). Were a Quickdraw card no draw, the cost p1
        // reads off its own hand would tell p1 how many Quickdraw cards p2 started with; R225 counts it
        // as the draw it replaces, so the cost is the same in both games.
        same_view(
            &view_for(&with_power, PlayerId::P1),
            &view_for(&without, PlayerId::P1),
        );
    }
}

// ---------------------------------------------------------------------------
// Quickdraw and setup resumed after a clause's question (R224)
// ---------------------------------------------------------------------------

const ASKING: &str = "r8-l10-asks";

fn asking(state: &mut GameState) {
    state
        .transient_defs
        .insert(ASKING.to_string(), fixture_def(ASKING, CardType::Spell));
    let script = asking_on_arrival("the clause's question");
    register_fixture_script(ASKING, script);
}

mod r225_r224_a_quickdraw_card_is_dealt_as_the_last_opening_draw {
    use super::*;

    /// p1's library holds one card whose start-of-game clause asks as it arrives in a hand (a fixture: no
    /// Core card asks), and with this seed p1's opening draw reaches it, so setup waits for p1's answer
    /// before it deals to p2 (R224, R151). The answer's action then carries p2's opening deal, which
    /// p1's view shows.
    fn paused_deal(seed: &str, p2_twentieth: &str) -> GameState {
        let mut game = create_game(&game_args(seed, (owned(&QD_P1_DECK), p2_qd_deck(p2_twentieth))));
        asking(&mut game);
        let card = new_instance(
            &mut game,
            ASKING,
            PlayerId::P1,
            Zone::Library { player: PlayerId::P1 },
        );
        game.players.p1.library[0] = card;
        begin_game(&game).state
    }

    fn first_pausing_seed() -> String {
        for at in 0..200 {
            let seed = format!("r8-l10-deal-{at}");
            let state = paused_deal(&seed, CRAFT);
            if let Some(pending) = &state.pending
                && pending.kind == PromptKind::Mode
                && pending.player_id == PlayerId::P1
            {
                return seed;
            }
        }
        panic!("no seed deals the asking card to p1");
    }

    /// `answered(p2Twentieth)`: the paused deal with p1's question answered.
    fn answered(seed: &str, p2_twentieth: &str) -> GameState {
        let state = paused_deal(seed, p2_twentieth);
        let pending = must(state.pending.clone(), "p1's cast question");
        assert_eq!(pending.kind, PromptKind::Mode);
        act_as(
            &state,
            PlayerId::P1,
            json!({ "type": "answer", "choiceId": pending.id, "selection": [{ "pick": "mode", "option": "ok" }] }),
        )
    }

    /// `typesOf(state)`: p1's view of the events, each as its type and, where it has one, its player.
    fn types_of(state: &GameState) -> Vec<String> {
        view_for(state, PlayerId::P1)
            .events
            .iter()
            .map(|event| {
                let value = serde_json::to_value(event).expect("an event serialises");
                let kind = value["type"].as_str().unwrap_or_default().to_string();
                match value.get("player") {
                    Some(player) => format!("{kind}:{}", player.as_str().unwrap_or_default()),
                    None => kind,
                }
            })
            .collect()
    }

    #[test]
    fn r225_r97_r224_the_events_of_p2_s_opening_deal_do_not_tell_p1_whether_p2_s_hand_holds_a_quickdraw_card_2_1_9_1()
     {
        register_all();
        let seed = first_pausing_seed();
        let with_power = answered(&seed, HEROIC_POWER);
        let without = answered(&seed, CRAFT);

        // Setup went on to the mulligans, and p2 holds its 4 opening cards in both games.
        for state in [&with_power, &without] {
            assert_eq!(mulligan_owed(state), vec![PlayerId::P1, PlayerId::P2]);
            assert_eq!(
                view_for(state, PlayerId::P1).opponent.hand,
                HandView::Count { count: 4 }
            );
        }
        assert!(
            with_power
                .players
                .p2
                .hand
                .iter()
                .any(|card| card.def_id == HEROIC_POWER)
        );

        // The deal's events reach p1's view (R168), redacted (R97). Were a Quickdraw card dealt with an
        // `addedToHand` alone, it would stand out among p2's `drawn` events and count p2's Quickdraw
        // cards, which are p2's hand (§9.1); R225 reports it as a draw, `drawn` then `addedToHand`.
        assert_eq!(types_of(&without), types_of(&with_power));
        same_view(
            &view_for(&with_power, PlayerId::P1),
            &view_for(&without, PlayerId::P1),
        );
    }

    /// p1's deck is 19 cards plus #98 Heroic Power (Quickdraw) or #99; the asking card is p1's.
    fn paused_own_deal(seed: &str, p1_twentieth: &str) -> GameState {
        let mut p1 = owned(&QD_P1_DECK[..19]);
        p1.push(p1_twentieth.to_string());
        let mut game = create_game(&game_args(seed, (p1, p2_qd_deck(CRAFT))));
        asking(&mut game);
        let card = new_instance(
            &mut game,
            ASKING,
            PlayerId::P1,
            Zone::Library { player: PlayerId::P1 },
        );
        game.players.p1.library[0] = card;
        begin_game(&game).state
    }

    #[test]
    fn r225_r224_p1_s_hand_and_library_counts_while_setup_waits_do_not_tell_p2_whether_p1_s_opening_hand_holds_a_quickdraw_card_2_1_9_1()
     {
        register_all();
        // A seed whose shuffle puts the asking card on top of p1's library, so it is p1's first draw: the
        // only card in p1's hand, where its clause arrived and asked.
        let mut seed: Option<String> = None;
        let mut at = 0;
        while at < 400 && seed.is_none() {
            let candidate = format!("r8-l10-own-deal-{at}");
            let state = paused_own_deal(&candidate, CRAFT);
            let hand: Vec<&str> = state
                .players
                .p1
                .hand
                .iter()
                .map(|card| card.def_id.as_str())
                .collect();
            if state.pending.as_ref().map(|pending| pending.kind) == Some(PromptKind::Mode)
                && hand.join(",") == ASKING
            {
                seed = Some(candidate);
            }
            at += 1;
        }
        let found = must(seed, "a seed that draws the asking card first");
        let with_power = paused_own_deal(&found, HEROIC_POWER);
        let without = paused_own_deal(&found, CRAFT);

        // Both games wait on p1's question, the asking card in p1's hand, p2 not yet dealt.
        for state in [&with_power, &without] {
            assert_eq!(
                state.pending.as_ref().map(|pending| pending.kind),
                Some(PromptKind::Mode)
            );
            assert_eq!(
                state.pending.as_ref().map(|pending| pending.player_id),
                Some(PlayerId::P1)
            );
            assert_eq!(state.players.p2.hand.len(), 0);
        }
        // R225: the Heroic Power replaces the last of p1's opening draws, so it is still in the library
        // while the first draw's clause asks.
        assert_eq!(
            with_power
                .players
                .p1
                .hand
                .iter()
                .map(|card| card.def_id.clone())
                .collect::<Vec<_>>(),
            vec![ASKING.to_string()]
        );
        assert!(
            with_power
                .players
                .p1
                .library
                .iter()
                .any(|card| card.def_id == HEROIC_POWER)
        );

        // p2 may count p1's hand and library (§10.8), but whether p1's deck holds a Quickdraw card is
        // p1's to keep (§9.1). Had the Heroic Power left the library for the hand before the other
        // draws, "instead of" a draw that has not happened yet, the counts would say so; it waits for
        // the last opening draw (R225), so both games count the same.
        same_view(
            &view_for(&with_power, PlayerId::P2),
            &view_for(&without, PlayerId::P2),
        );
    }
}

// ---------------------------------------------------------------------------
// Round 10: setup is no player's turn (§2.1, §2.2, R155, R241), and a clause that asks (R151)
// ---------------------------------------------------------------------------

/// `player` returns its first opening card, so R9's replacement draw takes the top card (§2.1).
fn mulligan_one(state: &GameState, player: PlayerId) -> GameState {
    let hand = ids(&state.players[player].hand);
    act(
        state,
        json!({ "type": "mulligan", "keep": &hand[1..], "playerId": player }),
    )
    .state
}

fn keep_all(state: &GameState, player: PlayerId) -> GameState {
    act(
        state,
        json!({ "type": "mulligan", "keep": ids(&state.players[player].hand), "playerId": player }),
    )
    .state
}

/// Play the game on, ending each turn at once (R82 may end one first), until `turn` has ended.
fn end_turns_through(state: &GameState, turn: i32) -> GameState {
    let mut next = state.clone();
    let mut guard = 0;
    while guard < 10 && next.result.is_none() && next.turn <= turn {
        if let Some(pending) = &next.pending {
            panic!("unexpected {} prompt on turn {}", pending.kind, next.turn);
        }
        next = act(&next, json!({ "type": "endTurn", "playerId": next.active })).state;
        guard += 1;
    }
    next
}

/// #23's return, verbatim in shape, on a Spell setup casts: the flag step 7 writes, or a play this turn.
fn setup_boomerang() -> Script {
    Script {
        cry: Some(hook(|_ctx| vec![])),
        end_of_turn: Some(hook(|ctx| {
            let Some(self_) = ctx.live_self() else {
                return vec![];
            };
            let returns = self_.return_to_hand_at_end_of_turn == Some(true)
                || was_played_this_turn(&*ctx.state, self_.controller, &self_.id);
            if returns {
                vec![effects::bounce(json_as(json!({ "target": { "of": "self" } })))]
            } else {
                vec![]
            }
        })),
        ..cast_on_setup_arrival()
    }
}

mod r155_r241_a_card_setup_casts_belongs_to_no_turn_of_its_caster_s_2_1_6_2 {
    use super::*;

    // No Core card casts in setup or carries an end-of-turn clause of a Spell it casts, so the card that
    // makes each case observable is a fixture; the rest of each game is real Core cards. Setup does not
    // draw a cast-on-draw card (R635), so the fixture casts itself as a replacement draw puts it in the
    // hand (`CAST_ON_SETUP_ARRIVAL`).
    #[test]
    fn r155_a_return_spell_cast_by_a_mulligan_s_replacement_draw_stays_in_the_graveyard_at_its_caster_s_first_turn_end_a_turn_it_was_not_played_on()
     {
        register_all();
        // Once for each seat: p1's first turn end is turn 1's, p2's is turn 2's.
        for seat in [PlayerId::P1, PlayerId::P2] {
            let mut state = begin_game(&create_game(&game_args(
                &format!("edge-r10-setup-return-{seat}"),
                (p1_deck(), p2_deck()),
            )))
            .state;
            assert_eq!(mulligan_owed(&state), vec![PlayerId::P1, PlayerId::P2]);
            let id = format!("edge-r10-setup-boomerang-{seat}");
            fixture(&mut state, &id, CardType::Spell, setup_boomerang());

            if seat == PlayerId::P2 {
                state = keep_all(&state, PlayerId::P1);
            }
            let boomerang = new_instance(&mut state, &id, seat, Zone::Library { player: seat });
            state.players[seat].library.insert(0, boomerang.clone());
            state = mulligan_one(&state, seat);
            if seat == PlayerId::P1 {
                state = keep_all(&state, PlayerId::P2);
            }
            // Both answers are in, so both resolved (R265): R9's replacement was cast during setup (§2.4,
            // R70) and landed in its caster's graveyard.
            assert!(
                state.players[seat]
                    .graveyard
                    .iter()
                    .any(|card| card.id == boomerang.id)
            );
            assert_eq!(state.turn, 1);

            // Through the caster's first turn end.
            state = end_turns_through(&state, if seat == PlayerId::P1 { 1 } else { 2 });
            assert_eq!(state.result, None);

            // R155: the return belongs to the turn the Spell was played on, and a Spell played outside its
            // controller's turn "stays in the graveyard rather than coming back at the end of a later turn
            // it was not played on". Setup is turn 0 and nobody's turn (§2.1, §2.2): the turn-scoped
            // riders a setup cast makes are already dead on turn 1 (`thisTurn` of turn 0), and its return
            // is over too.
            let in_hand = state.players[seat]
                .hand
                .iter()
                .any(|card| card.id == boomerang.id);
            assert!(
                !in_hand,
                "{seat}'s Spell cast during its mulligan came back to hand at the end of its first turn"
            );
        }
    }

    #[test]
    fn r241_an_end_of_turn_clause_armed_by_a_spell_p1_s_mulligan_casts_does_not_exile_p1_s_hand_at_the_end_of_turn_1()
     {
        register_all();
        let mut state = begin_game(&create_game(&game_args(
            "edge-r10-setup-exile",
            (p1_deck(), p2_deck()),
        )))
        .state;
        assert_eq!(mulligan_owed(&state), vec![PlayerId::P1, PlayerId::P2]);

        // /fullsend's end-of-turn clause, verbatim in shape, on a Spell setup casts.
        let id = "edge-r10-setup-late-exile";
        fixture(
            &mut state,
            id,
            CardType::Spell,
            Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::delay(json_as(json!({
                        "at": { "phase": "end", "player": "self" },
                        "step": "exile",
                        "hook": RESUME_HOOK,
                    })))]
                })),
                resume: IndexMap::from([(
                    "exile",
                    hook(|_ctx| vec![effects::exile_hand(json_as(json!({ "player": "self" })))]),
                )]),
                ..cast_on_setup_arrival()
            },
        );
        let cod = new_instance(
            &mut state,
            id,
            PlayerId::P1,
            Zone::Library { player: PlayerId::P1 },
        );
        state.players.p1.library.insert(0, cod.clone());

        // p1's answer is sealed until p2's is in (R265); then both resolve, p1's first.
        state = mulligan_one(&state, PlayerId::P1);
        state = keep_all(&state, PlayerId::P2);
        assert!(state.players.p1.graveyard.iter().any(|card| card.id == cod.id));
        // Setup's `active` names p1 only as a placeholder (§2.1): the cast was on no turn of p1's.
        assert_eq!(state.delayed, Vec::<DelayedEffect>::new());
        assert_eq!(state.turn, 1);
        let hand_on_turn_one = ids(&state.players.p1.hand);
        assert!(!hand_on_turn_one.is_empty());

        state = end_turns_through(&state, 1);
        assert_eq!(state.turn, 2);

        // R241: an end-of-turn clause is "the end of the turn the card was played on", and one made
        // outside its controller's turn "has no end of its controller's turn to wait for, so it is not
        // armed at all". The same card cast by p2's mulligan was never armed, and p1's was not played on
        // turn 1 either: its hand stays.
        let exiled: Vec<String> = hand_on_turn_one
            .iter()
            .filter(|card_id| state.players.p1.exile.iter().any(|card| &card.id == *card_id))
            .cloned()
            .collect();
        assert_eq!(
            exiled,
            Vec::<String>::new(),
            "the clause of a Spell p1 cast during setup exiled p1's hand at the end of turn 1"
        );
    }
}

mod r151_r113_a_start_of_game_clause_that_asks_at_2_1_step_4 {
    use super::*;

    #[test]
    fn r151_r113_a_start_of_game_clause_that_asks_holds_the_rest_of_setup_and_turn_1_until_it_is_answered_2_1_9_3()
     {
        register_all();
        let mut state = begin_game(&create_game(&game_args(
            "edge-r10-setup-start-asks",
            (p1_deck(), p2_deck()),
        )))
        .state;
        // "Start of game: choose one; then deal 3 damage to the enemy hero", at the bottom of p1's
        // library, where no opening draw reaches it: §2.1 step 4 runs it over the library too (R153).
        let id = "edge-r10-setup-start-asks";
        fixture(
            &mut state,
            id,
            CardType::Unit,
            Script {
                start_of_game: Some(hook(|_ctx| {
                    vec![
                        effects::choose_mode(json_as(json!({
                            "options": ["a", "b"],
                            "step": "picked",
                            "prompt": "the clause's question",
                        }))),
                        effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 3 }))),
                    ]
                })),
                resume: IndexMap::from([("picked", hook(|_ctx| vec![]))]),
                ..Script::default()
            },
        );
        let card = new_instance(
            &mut state,
            id,
            PlayerId::P1,
            Zone::Library { player: PlayerId::P1 },
        );
        state.players.p1.library.push(card);

        state = keep_all(&state, PlayerId::P1);
        state = keep_all(&state, PlayerId::P2);
        // §9.3: the question is state, and what comes after it — the rest of the clause and turn 1 —
        // waits for its answer rather than running over it.
        let pending = must(state.pending.clone(), "the clause's question");
        assert_eq!(pending.kind, PromptKind::Mode);
        assert_eq!(pending.player_id, PlayerId::P1);
        assert_eq!(state.turn, 0);
        assert_eq!(state.players.p2.hero.health, 30);

        let answered = act(&state, mode_answer(&pending.id, "a", PlayerId::P1));
        state = answered.state;
        // R122: the answer finishes the clause, then the setup it held: turn 1 is p1's (§2.1 step 5),
        // and it begins only after the clause's last effect.
        assert_eq!(state.players.p2.hero.health, 27);
        assert_eq!(state.turn, 1);
        assert_eq!(state.active, PlayerId::P1);
        let hit_at = answered
            .events
            .iter()
            .position(|event| matches!(event, GameEvent::Damage { amount: 3, .. }))
            .map(|at| at as i64)
            .unwrap_or(-1);
        let turn_at = answered
            .events
            .iter()
            .position(|event| matches!(event, GameEvent::TurnStarted { .. }))
            .map(|at| at as i64)
            .unwrap_or(-1);
        assert!(hit_at >= 0);
        assert!(turn_at > hit_at);
    }
}
