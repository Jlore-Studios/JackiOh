//! Audit (decks area): one failing test per reported finding. Each asserts the documented behaviour.

use jackioh_ai::{
    AI_DECK, AI_EVAL, AI_GATE, AI_GATE_BUDGET, AiDeckOptions, AiOptions, Matchup, NextSwing, SwingReader,
    build_ai_deck, evaluate, game_config, play_ai_turn,
};
use jackioh_engine::config::SWING_EVAL_SCALE;
use jackioh_engine::testkit::{
    Action, ActingCard, CardType, CreatedActing, PlayerId, Scenario, create_rng, def_of, hash_state, json,
    json_as, reduce, scenario,
};

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

fn units_in(deck: &[String]) -> usize {
    deck.iter()
        .filter(|id| def_of(None, id).type_ == CardType::Unit)
        .count()
}

/// ceil(size × minUnitShare), as `build_ai_deck_traced` rounds it (deck.rs:686).
fn units_needed(size: usize) -> usize {
    (size as f64 * AI_DECK.min_unit_share).ceil() as usize
}

/// Finding 1. SPEC §9.9 ("a hard floor beside the units' and the theme's"), R1370 ("a hard floor, as
/// the theme's `themeMinShare` and the units' `minUnitShare` are, so it holds whatever the rng does")
/// and deck.rs's header ("every deck `minUnitShare`"): a deck that leans on no set still holds
/// ceil(size × 0.45) Units. `narrow_unleaned` (deck.rs:410) narrows to Units only once the Units owed
/// alone reach the slots left, never counting the slots the theme still owes, so a theme with few or
/// no Units (Book, Fruit, Drone Core, Jelly Bean, Mineral, Call to Chaos, KY, Terminus) takes them.
#[test]
fn finding_1_the_quality_gates_own_ai_deck_misses_the_units_floor() {
    jackioh_cards::register_all();
    // Game 9 of the shipped ai-vs-greedy gate, the subject's (p1's) deck, dealt with default options.
    let config = game_config(Matchup::AiVsGreedy, 9, AI_GATE_BUDGET, AI_GATE.seed_series);
    let deck = &config.decks.0;
    assert!(
        units_in(deck) >= units_needed(deck.len()),
        "gate ai-vs-greedy game 9, p1: {} Units of {}, the floor is {}: {deck:?}",
        units_in(deck),
        deck.len(),
        units_needed(deck.len())
    );
}

#[test]
fn finding_1_a_deck_themed_on_a_tag_with_no_units_misses_the_units_floor() {
    jackioh_cards::register_all();
    let opts: AiDeckOptions = json_as(json!({ "theme": "Book" }));
    let short: Vec<String> = (1..=200)
        .filter_map(|n| {
            let seed = format!("audit:25:Book:{n}");
            let deck = build_ai_deck(&mut create_rng(&seed, 0), 25, &opts);
            (units_in(&deck) < units_needed(25)).then(|| format!("{seed}: {} Units", units_in(&deck)))
        })
        .collect();
    assert!(
        short.is_empty(),
        "{} of 200 Book-themed 25-card decks hold fewer than {} Units, e.g. {:?}",
        short.len(),
        units_needed(25),
        &short[..short.len().min(3)]
    );
}

fn step(reader: &mut SwingReader, s: &mut Scenario, verb: impl FnOnce(&mut Scenario)) {
    let before = s.state().clone();
    let from = s.events().len();
    verb(s);
    reader.step(&before, &s.events()[from..], s.state());
}

/// Finding 2. R4552000: "The row is written after the turn's start has resolved (its draw and its
/// start-of-turn triggers)". `SwingReader::step` (swing_turns.rs:127) writes the row in the step that
/// emits `turnStarted`, off the state that step left; when a start-of-turn prompt parks the start
/// (R113: Core #65 Masochism Mask asks every turn), that state is mid-start, before the draw and the
/// trigger's effect, and no later step writes the row again.
#[test]
fn finding_2_a_turn_whose_start_asks_has_its_row_read_before_the_start_resolved() {
    jackioh_cards::register_all();
    const MASK: &str = "core-065"; // Masochism Mask: "Start of turn: choose one: …"
    const PILLOW: &str = "core-065-1"; // Spikey Pillow, the token its third option summons
    let mut s = scenario(json!({
        "active": "p2",
        "p1": {
            "hand": ["core-019"],
            "backrow": [{ "def": MASK }],
            "library": ["core-019", "core-011", "core-061"],
        },
        "p2": { "hand": ["core-019"], "field": ["core-019"], "library": ["core-019", "core-011"] },
    }));
    let mut reader = SwingReader::new();
    step(&mut reader, &mut s, |s| {
        s.end_turn();
    });
    assert!(s.state().pending.is_some(), "the Mask asks at the start of p1's turn");
    step(&mut reader, &mut s, |s| {
        s.answer(json!("summon Spikey Pillow"));
    });
    // The start has resolved: the Pillow stands and the turn's card is drawn.
    let state = s.state();
    assert!(state.pending.is_none());
    assert_eq!(s.hand(P1).len(), 2, "the turn's draw has landed");

    let rows = &reader.turns()[P1];
    assert_eq!(rows.len(), 1, "one row for p1's turn");
    let made = ActingCard::Created(CreatedActing {
        card: PILLOW.to_string(),
        maker: Some(MASK.to_string()),
    });
    assert!(
        rows[0].acting.contains(&made),
        "the row lists the cards acting once the start resolved: {:?}",
        rows[0].acting
    );
    let difference =
        evaluate(state, P1, NextSwing::Seat, &AI_EVAL) - evaluate(state, P2, NextSwing::Enemy, &AI_EVAL);
    assert_eq!(rows[0].eval, (difference * SWING_EVAL_SCALE).round() as i64);
}

/// Finding 3. README, Matches: "`play_ai_turn` plays one AI turn out". Its nonces are `t{n}` from 0 on
/// every call (match_.rs:498), and `reduce` answers a nonce the state already applied with the state
/// unchanged and no error (engine reduce.rs:693, `NONCE_HISTORY` 64). So a second call on the state a
/// first call left "accepts" its first decisions without applying them.
#[test]
fn finding_3_a_second_play_ai_turn_on_the_same_game_applies_every_action_it_returns() {
    jackioh_cards::register_all();
    let start = scenario(json!({
        "seed": "audit-turns",
        "active": "p1",
        "turn": 9,
        "p1": { "hand": ["core-025", "core-011"], "mana": 4, "library": ["core-008", "core-011", "core-019"] },
        "p2": { "hand": ["core-025", "core-011"], "mana": 4, "library": ["core-008", "core-011", "core-019"] },
    }))
    .state()
    .clone();
    let mut ai = AiOptions::with_budget(create_rng("audit-turns", 0), AI_GATE_BUDGET);
    let first = play_ai_turn(&start, P1, &mut ai);
    assert!(first.actions.len() >= 2, "p1 plays and ends its turn");
    let second = play_ai_turn(&first.state, P2, &mut ai);

    let mut current = first.state.clone();
    for action in &second.actions {
        let next = reduce(&current, &Action::new(action.body.clone(), P2, action.nonce.clone()));
        assert!(next.error.is_none());
        assert_ne!(
            hash_state(&next.state),
            hash_state(&current),
            "p2's action {} ({:?}) changed nothing: its nonce was p1's",
            action.nonce,
            action.body
        );
        current = next.state;
    }
}
