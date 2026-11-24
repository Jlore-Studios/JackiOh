//! Audit of the evaluation's worth and price side (`crates/ai/src/evaluate.rs`, the root probes and the
//! reads of their prices). Each test asserts the documented behaviour and fails on the current code.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

const AI: PlayerId = PlayerId::P1;

const VANILLA: &str = "core-008";
/// S #90 After Burner: (2) 4/8 Starship Piece, "When launched, your Starship gains random keywords": no
/// Activate of its own.
const AFTER_BURNER: &str = "stinkyscape-090";
/// Core #19 Midrange Menace: (3) 9/9 Taunt, no other text.
const MENACE: &str = "core-019";
/// S #75 Badger: (1) 2/3, "When this attacks, gain a random Stinkyscape Resource."
const BADGER: &str = "stinkyscape-075";
/// M #11 Double Header: a (4) Tradable card (`crates/ai/tests/ai/tradable.rs`).
const DOUBLE_HEADER: &str = "meditative-011";
const LIBRARY: &[&str] = &[VANILLA, VANILLA, VANILLA, VANILLA];

fn board(seed: &str, mut p1: Value, mut p2: Value) -> GameState {
    jackioh_cards::register_all();
    p1["library"] = json!(LIBRARY);
    p2["library"] = json!(LIBRARY);
    scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }))
        .state()
        .clone()
}

/// `state` after the sandbox's destroy of `instance_id` (R923003: its death runs as any destroy's).
fn destroyed(state: &GameState, instance_id: &str) -> GameState {
    let mut world = state.clone();
    world.sandbox = Some(true);
    let op = SandboxOp::Destroy {
        instance_id: instance_id.to_string(),
    };
    let counter = create_node_counter(1000, None);
    let mut next = simulate(&world, AI, &ActionBody::Sandbox { op }, &counter, Prices::NONE)
        .expect("nodes")
        .expect("the destroy is accepted");
    next.sandbox = state.sandbox;
    next
}

/// Finding 1. R3575404: "A sleeper read this way is worth what R2380000 gives it, the unit it wakes as at
/// the waiting share, plus `AI_EVAL.activation` of what waking it is worth, in full", and
/// `crates/ai/tests/ai/starship_equip.rs` pins exactly that for S+ #157 Phantom Shipyard. A Dormant
/// Starship (the token a Starship Piece's death stands up, R1852000) wakes by the engine's launch
/// (R1852003), which is no Activate of its text: `slumber_worth` (evaluate.rs:1272) counts
/// `share * unit_worth(awake)` and puts back `(1 - share)` of the price, expecting the awake copy to have
/// counted the other `share` of it; but `activation_price` (evaluate.rs:507) asks
/// `abilities_of(state, awake)`, which is empty for an awake Starship whose pieces print no Activate, so
/// that share is never counted and the launch reads at half its price.
#[test]
fn a_dormant_starships_launch_counts_in_full() {
    let state = board(
        "audit-eval-starship",
        json!({ "field": [AFTER_BURNER], "hand": [VANILLA], "mana": 4 }),
        json!({ "hand": [VANILLA], "field": [VANILLA] }),
    );
    let piece = active_units_of(&state, AI)[0].id.clone();
    let state = destroyed(&state, &piece);
    let ship = units_on_field_of(&state, AI)
        .into_iter()
        .find(|card| is_dormant_starship(card))
        .expect("the piece's death stands up a Dormant Starship (R1852000)")
        .clone();
    assert!(acts_to_awaken(&state, &ship), "it keeps its launch (R1852003)");

    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let prices = activation_prices(&state, AI, &counter);
    let price = prices
        .iter()
        .find(|entry| entry.def_id == ship.def_id && entry.controller == AI)
        .unwrap_or_else(|| panic!("the Dormant Starship's launch is priced: {prices:?}"))
        .price;
    assert!(price > 0.0, "{price}");
    let priced = Prices {
        activations: prices,
        ..Prices::default()
    };
    let gained = evaluate_priced(&state, AI, NextSwing::Enemy, &AI_EVAL, &priced)
        - evaluate_priced(&state, AI, NextSwing::Enemy, &AI_EVAL, Prices::NONE);
    assert!(
        (gained - AI_EVAL.activation * price).abs() < 1e-9,
        "R3575404: the sleeper is worth `activation` of its launch in full, not a slumber share of it: \
         {gained} against {} ({} of the price {price})",
        AI_EVAL.activation * price,
        AI_EVAL.activation
    );
}

/// Finding 2. R2688000 prices a unit's attack text as its attack less the same attack by its hookless
/// twin, "its text given back before scoring" (`with_text_back`, evaluate.rs:3233, which sets
/// `vanilla = false`). `has_attack_hook` (evaluate.rs:524) also takes a unit that only holds Frenzy, and a
/// Vanilla'd unit keeps a Frenzy it was granted (`layers::card_keywords`: a Vanilla card loses its printed
/// keywords, never its granted ones). Such a unit has no text to give back: its twin is itself, so its
/// price is 0. The probe instead grants the twin the keywords the Vanilla took and scores it un-Vanilla'd,
/// so the price is minus the worth of the text the unit no longer has.
#[test]
fn a_vanilla_unit_that_holds_frenzy_has_no_text_to_price() {
    let mut state = board(
        "audit-eval-frenzy",
        json!({ "field": [MENACE], "hand": [VANILLA] }),
        json!({ "hand": [VANILLA], "field": [] }),
    );
    let id = active_units_of(&state, AI)[0].id.clone();
    {
        // A Vanilla (M #25 Blue-Eyes White Felinor, S+ #65 Thorium Wall) and then a random combat
        // keyword that rolled Frenzy (R3551004: the pool's fifteenth entry).
        let card = find_instance_mut(&mut state, &id).expect("the Menace");
        card.vanilla = true;
        card.granted_keywords.push(Keyword::Frenzy);
    }
    let menace = find_instance(&state, &id).expect("the Menace").clone();
    let view = unit_view(&state, &menace);
    assert!(has_keyword(&view.keywords, KeywordKind::Frenzy));
    assert!(
        !has_keyword(&view.keywords, KeywordKind::Taunt),
        "the Vanilla took its printed Taunt"
    );

    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let prices = attack_hook_prices(&state, AI, &counter);
    let price = prices
        .iter()
        .find(|entry| entry.def_id == MENACE && entry.controller == AI)
        .map_or(0.0, |entry| entry.price);
    assert!(
        price.abs() < 1e-9,
        "its attack on the bare hero runs no text and kills nothing, so it is worth what its twin's is: \
         {price} (the Taunt it lost weighs {})",
        AI_EVAL.keyword.taunt
    );
}

/// Finding 3. `hookless_twin` (evaluate.rs:3219): the twin "is granted the keywords it prints ... so it
/// fights as itself", and R2688000 prices the hooks alone ("the same attack by the unit's hookless
/// twin"). A keyword its controller's "your Units have X" lends (R1859007: S #137.1 to #137.4, the
/// Doctrines) never reaches a Vanilla card, so the twin fights without it: against a Taunt a lent
/// Poisonous kills for the unit and not for its twin, and the kill is booked as the worth of "When this
/// attacks, gain a random Resource". `crates/ai/tests/ai/attack_hooks.rs`
/// `r2688000_the_twin_is_scored_with_its_text_back` pins that a lent Poisonous moves no price, on a bare
/// hero only.
#[test]
fn a_lent_keyword_moves_no_attack_hook_price_against_a_taunt() {
    let bare = board(
        "audit-eval-lent",
        json!({ "field": [BADGER], "hand": [VANILLA] }),
        json!({ "hand": [VANILLA], "field": [MENACE] }),
    );
    let mut lent = bare.clone();
    lent.players[AI].mods.push(PlayerModifier {
        id: "audit-eval-lent".to_string(),
        expiry: ModifierExpiry::Never,
        kind: ModifierKind::UnitsHaveKeyword {
            keyword: Keyword::Poisonous,
            from_spell: None,
        },
    });
    let price_on = |state: &GameState| {
        let counter = create_node_counter(AI_BUDGET.nodes, None);
        let prices = attack_hook_prices(state, AI, &counter);
        prices
            .iter()
            .find(|entry| entry.def_id == BADGER && entry.controller == AI)
            .unwrap_or_else(|| panic!("the Badger's attack is priced: {prices:?}"))
            .price
    };
    let (with_lent, without) = (price_on(&lent), price_on(&bare));
    assert!(
        (with_lent - without).abs() < 1e-9,
        "the Badger's text gains one Resource either way: {with_lent} with the lent Poisonous against \
         {without} without"
    );
}

/// Finding 4. R4154400: "A card in the AI's own hand is dead while it is Tradable now and its own cost is
/// above the crystals its next refresh brings ... So neither this turn's crystals nor the next turn's
/// pay for it." `next_turn_crystals` (evaluate.rs:1006) adds a crystal whenever `mana.max` is under
/// `MAX_MANA`, but a seat that lost maximum mana for the rest of the game (S+ #95 JC Backed, S+ #82
/// Endless Expedition: `mana.perm_mod`, a public count) is under it for good and gains none
/// (`mana::max_mana_for`).
#[test]
fn a_tradable_card_is_dead_when_the_lowered_maximum_never_pays_for_it() {
    let mut state = board(
        "audit-eval-dead",
        json!({ "hand": [DOUBLE_HEADER, VANILLA], "mana": 0 }),
        json!({ "hand": [VANILLA], "field": [VANILLA] }),
    );
    {
        // "Lose 1 mana for the rest of the game", as `effects::change_max_mana` writes it.
        let side = &mut state.players[AI];
        side.mana.perm_mod -= 1;
        side.mana.max = jackioh_engine::max_mana_for(side);
    }
    let card = state.players[AI].hand[0].clone();
    let cost = jackioh_engine::own_cost(&state, &card).expect("a cost");
    assert!(is_tradable(&state, &card));
    // What the engine's next refresh brings: the maximum once one more turn has started.
    let mut next = state.players[AI].clone();
    next.turns_started += 1;
    let brings = jackioh_engine::max_mana_for(&next);
    assert!(
        cost > state.players[AI].mana.max && cost > brings,
        "neither this turn's {} crystals nor the next refresh's {brings} pay for its ({cost})",
        state.players[AI].mana.max
    );
    assert!(
        dead_in_hand(&state, AI, &card),
        "R4154400: its cost ({cost}) is above the crystals its next refresh brings ({brings})"
    );
}

// TEMP fuzz: the root probes over random-policy games: panics, determinism, budget.
#[test]
fn temp_fuzz_decision_prices() {
    jackioh_cards::register_all();
    let mut states: Vec<(GameState, PlayerId)> = Vec::new();
    for game in 0..6 {
        let seed = format!("audit-eval:fuzz:{game}");
        let deck = |seat: PlayerId| {
            build_ai_deck(
                &mut Rng::new(&format!("{seed}:deck:{}", seat.as_str()), 0),
                DECK_SIZE,
                &AiDeckOptions {
                    banned: Some(vec![]),
                    gated_sets: Some(vec![]),
                    ..AiDeckOptions::default()
                },
            )
        };
        let config = MatchConfig {
            seed: seed.clone(),
            decks: (deck(PlayerId::P1), deck(PlayerId::P2)),
            handicaps: None,
            controllers: PerPlayer {
                p1: SeatController::Random,
                p2: SeatController::Random,
            },
            max_actions: Some(500),
            setup: MatchSetup::default(),
        };
        let mut step = 0usize;
        let mut hooks = MatchHooks {
            after_action: Some(Box::new(
                |_before: &GameState, after: &GameState, _seat: PlayerId, _action: &ActionBody| {
                    step += 1;
                    if step % 7 == 0 && after.result.is_none() {
                        if let Some(seat) = seat_to_act(after) {
                            states.push((after.clone(), seat));
                        }
                    }
                },
            )),
            ..MatchHooks::default()
        };
        play_match(&config, &mut hooks);
    }
    let (mut panics, mut uneven, mut over, mut nan) = (0, 0, 0, 0);
    for (index, (state, seat)) in states.iter().enumerate() {
        let public = redact(state, *seat);
        let ground = det_ground(&public, *seat);
        let probe = probe_world(&public, *seat, &ground);
        let run = || {
            let counter = create_node_counter(AI_DECISION_PRICE_NODES, None);
            let prices = decision_prices(&probe, *seat, &counter);
            (prices, counter.used(), counter.sim_panic_tally(), counter.first_sim_panic())
        };
        let first = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run));
        let Ok((prices, used, tally, message)) = first else {
            panics += 1;
            println!("state {index}: decision_prices panicked");
            continue;
        };
        if tally.unwrap_or(0) > 0 {
            panics += 1;
            println!("state {index}: sim panic {message:?}");
        }
        if used > AI_DECISION_PRICE_NODES {
            over += 1;
        }
        let (again, _, _, _) = run();
        if again != prices {
            uneven += 1;
            println!("state {index}: two runs differ");
        }
        let all = prices
            .deaths
            .iter()
            .map(|p| p.price)
            .chain(prices.turn_hooks.iter().flat_map(|p| [p.price, p.opponent_turn]))
            .chain(prices.activations.iter().map(|p| p.price))
            .chain(prices.attack_hooks.iter().map(|p| p.price))
            .chain(prices.awakenings.iter().map(|p| p.price))
            .chain(prices.evolutions.iter().map(|p| p.price));
        if all.clone().any(|p| !p.is_finite()) {
            nan += 1;
        }
        for next in [NextSwing::Enemy, NextSwing::Seat] {
            for who in PLAYER_IDS {
                let score = evaluate_priced(state, who, next, &AI_EVAL, &prices);
                if !score.is_finite() {
                    nan += 1;
                }
            }
        }
    }
    println!("states {} panics {panics} uneven {uneven} over {over} nan {nan}", states.len());
}
