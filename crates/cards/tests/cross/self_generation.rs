//! Port of `packages/cards/test/self-generation.test.ts` (v0.3.0 part 27.5).
//!
//! B4.1, R387: a card never Discovers or generates a copy of itself, unless its text names a pool
//! that holds it or makes copies of "this" (docs/classic-sets.md B4.1). This is rule 5's sweep: every
//! card with a script is played on a busy board under several seeds, every prompt it opens is
//! answered, and no card the play CREATED — an instance that did not exist before the play — carries
//! the played card's own definition, and no Discover it opens offers that definition. A card moved
//! rather than created (a card returning itself to hand, R25) is not generation and is not counted.
//!
//! The sweep reaches every set's scripts as they land: a new generating card that forgets its own
//! exclusion fails here, named. The exceptions are the texts that say so, each with its reason.

use jackioh_cards::{CATALOG, CATALOG_IDS};
use jackioh_engine::testkit::*;

/// B4.1 rules 3 and 4: the cards whose text makes a copy of itself, names itself, or names a pool
/// that holds it. Anything else that produces its own definition is a bug.
const NAMES_ITS_OWN_POOL: &[(&str, &str)] = &[
    (
        "core-012",
        "Duplicating Felinors summons a copy of this unit (rule 4: copies are not generation)",
    ),
    (
        "core-087",
        "Pocket Chaos names itself: \"add a Pocket Chaos with a base cost (1) less …\" (rule 3)",
    ),
    (
        "core-090",
        "CN-Viral Injection shuffles copies of itself (rule 4)",
    ),
    (
        "core-095",
        "Call to Chaos casts a random Call to Chaos, a pool it names (R28, rule 3)",
    ),
    (
        "classicplus-004",
        "Juhan Biggest Bat makes the cards beneath it copies of this (rule 4)",
    ),
    (
        "classicplus-046-1",
        "Felinor Flagbearer Prime fills the board with copies of this (rule 4)",
    ),
    (
        "classicplus-073",
        "Call to Chaos (Classic+ Edition) names the Call to Chaos pool (R28, rule 3)",
    ),
];

/// The seeds each card is played under.
const SEEDS: &[&str] = &["self-gen-a", "self-gen-b", "self-gen-c", "self-gen-d"];

/// The most prompts one play may open before the sweep gives up answering (a bound, not a rule).
const MAX_ANSWERS: usize = 40;

fn names_its_own_pool(id: &str) -> bool {
    NAMES_ITS_OWN_POOL.iter().any(|(own, _)| *own == id)
}

/// A busy board: units and backrow on both sides, libraries, graveyards and hands to reach into.
fn board(seed: &str, def_id: &str) -> Scenario {
    scenario(json!({
        "seed": seed,
        "p1": {
            "hand": [def_id, "core-010", "core-005"],
            "mana": 10,
            "field": [{ "def": "core-008", "lane": 1 }, { "def": "core-020", "lane": 2 }],
            "backrow": [{ "def": "core-006", "lane": 4 }],
            "library": ["core-011", "core-025", "core-013", "core-005", "core-041", "core-069", "core-004", "core-019"],
            "graveyard": ["core-002", "core-035"],
        },
        "p2": {
            "hand": ["core-005", "core-008"],
            "field": [{ "def": "core-019", "lane": 1 }, { "def": "core-011", "lane": 3 }],
            "library": ["core-008", "core-020", "core-005", "core-004"],
            "graveyard": ["core-056"],
        },
    }))
}

/// TS `/^c(\d+)$/`: an instance id's number, or -1 for anything else.
fn instance_number(id: &str) -> i64 {
    match id.strip_prefix('c') {
        Some(digits) if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) => {
            digits.parse::<i64>().unwrap_or(-1)
        }
        _ => -1,
    }
}

/// The instances an event says came into being or arrived somewhere, with their definitions.
fn named_cards(event: &GameEvent) -> Vec<(String, String)> {
    match event {
        GameEvent::AddedToHand {
            instance_id, def_id, ..
        }
        | GameEvent::Summoned {
            instance_id, def_id, ..
        }
        | GameEvent::ShuffledIn {
            instance_id, def_id, ..
        }
        | GameEvent::CardPlayed {
            instance_id, def_id, ..
        }
        | GameEvent::Burned {
            instance_id, def_id, ..
        } => vec![(instance_id.clone(), def_id.clone())],
        // A copy the full library refused was still generated.
        GameEvent::LibraryOverflow {
            instance_id,
            def_id,
            outcome,
            ..
        } => {
            if *outcome == LibraryOverflowOutcome::NotCreated {
                vec![(instance_id.clone(), def_id.clone())]
            } else {
                vec![]
            }
        }
        GameEvent::Transformed {
            new_instance_id,
            to_def_id,
            ..
        } => {
            vec![(new_instance_id.clone(), to_def_id.clone())]
        }
        _ => vec![],
    }
}

/// TS `sweep`'s closures (`step`, `offersItself`) over the state it drives.
struct Sweep {
    state: GameState,
    events: Vec<GameEvent>,
    nonce: u32,
    found: Vec<String>,
}

impl Sweep {
    fn step(&mut self, body: &ActionBody, player: PlayerId) -> bool {
        self.nonce += 1;
        let result = reduce(
            &self.state,
            &Action::new(body.clone(), player, format!("self-gen-{}", self.nonce)),
        );
        if result.error.is_some() {
            return false;
        }
        self.state = result.state;
        self.events.extend(result.events);
        true
    }

    fn offers_itself(&mut self, def_id: &str) {
        let Some(pending) = &self.state.pending else {
            return;
        };
        if pending.kind != PromptKind::Discover {
            return;
        }
        for option in &pending.options {
            if let Selection::Mode { option: offered } = &option.selection
                && offered == def_id
            {
                self.found.push(format!("Discover offered {def_id}"));
            }
        }
    }
}

/// What one play of `def_id` under `seed` generated of itself, and what its Discovers offered of it.
fn sweep(def_id: &str, seed: &str) -> Vec<String> {
    let s = board(seed, def_id);
    let own = s.card(def_id).id.clone();
    let plays: Vec<ActionBody> = legal_actions(s.state(), PlayerId::P1)
        .into_iter()
        .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == own))
        .collect();
    if plays.is_empty() {
        return vec![];
    }
    let number = instance_number(&own);
    let pick = if number >= 0 {
        plays.get(number as usize % plays.len())
    } else {
        None
    };
    let Some(pick) = pick.or(plays.first()).cloned() else {
        return vec![];
    };

    let first_new = i64::from(s.state().next_id);
    let mut run = Sweep {
        state: s.state().clone(),
        events: vec![],
        nonce: 0,
        found: vec![],
    };

    if !run.step(&pick, PlayerId::P1) {
        return vec![];
    }
    run.offers_itself(def_id);
    let mut answered = 0;
    while answered < MAX_ANSWERS && run.state.pending.is_some() && run.state.result.is_none() {
        let player = run
            .state
            .pending
            .as_ref()
            .map(|pending| pending.player_id)
            .expect("a prompt is open");
        let answers: Vec<ActionBody> = legal_actions(&run.state, player)
            .into_iter()
            .filter(|action| matches!(action, ActionBody::Answer { .. }))
            .collect();
        let Some(answer) = answers.get(answered % answers.len().max(1)).cloned() else {
            break;
        };
        if !run.step(&answer, player) {
            break;
        }
        run.offers_itself(def_id);
        answered += 1;
    }

    // The played card itself is not generated, under whatever id it wears: a card set face-down takes a
    // fresh id, which the events that set it name through `formerId` (R227).
    let mut own_ids: IndexSet<String> = IndexSet::new();
    own_ids.insert(own.clone());
    for event in &run.events {
        match event {
            GameEvent::CardPlayed {
                instance_id,
                former_id: Some(former),
                ..
            }
            | GameEvent::Summoned {
                instance_id,
                former_id: Some(former),
                ..
            } if own_ids.contains(former) => {
                own_ids.insert(instance_id.clone());
            }
            _ => {}
        }
    }
    let mut found = run.found;
    for event in &run.events {
        for (instance_id, card_def_id) in named_cards(event) {
            if card_def_id == def_id
                && !own_ids.contains(&instance_id)
                && instance_number(&instance_id) >= first_new
            {
                found.push(format!("{} created {instance_id} ({def_id})", event.event_type()));
            }
        }
    }
    found
}

/// Every card the sweep plays: non-token, with its own definition in the catalog.
fn swept() -> Vec<String> {
    CATALOG_IDS
        .iter()
        .filter(|id| match CATALOG.get(id.as_str()) {
            Some(def) => !def.token && !def.tags.contains(&Tag::Token) && !names_its_own_pool(id),
            None => false,
        })
        .cloned()
        .collect()
}

mod r387_a_card_never_generates_itself_b4_1_rule_5s_sweep {
    use super::*;

    // TS ran this one under `{ timeout: 120_000 }`; Rust's test runner has no per-test timeout.
    #[test]
    fn r387_no_cards_play_creates_or_discovers_its_own_definition_over_every_set_and_several_seeds() {
        jackioh_cards::register_all();
        let mut violations: Vec<String> = vec![];
        for def_id in swept() {
            for seed in SEEDS {
                for found in sweep(&def_id, seed) {
                    violations.push(format!("{def_id} @ {seed}: {found}"));
                }
            }
        }
        assert_eq!(violations, Vec::<String>::new());
    }

    #[test]
    fn r387_the_exceptions_are_exactly_the_cards_whose_text_copies_itself_or_names_its_own_pool() {
        jackioh_cards::register_all();
        for (id, _reason) in NAMES_ITS_OWN_POOL {
            assert!(CATALOG.get(*id).is_some(), "{id}");
        }
    }
}
