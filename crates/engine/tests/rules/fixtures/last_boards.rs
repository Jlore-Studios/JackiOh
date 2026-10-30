//! Fixture cards for B5 E30's last boards (SPEC §8.7 C+ #29, R417, R564): C+ #29's two faces in the
//! smallest script that has them, and a Trap and a Field Trap to put face-down. The engine never
//! imports `crates/cards` (CLAUDE.md).

use std::sync::LazyLock;

use jackioh_engine::effects;
use jackioh_engine::testkit::*;

use super::catalog::{spell_def, vanilla_deck};
use super::harness::{act_logged, setup_catalog};

/// C+ #29's shape: base Discovers from the caster's last board, Radiant adds 3 random; each costs (0).
pub const PORTAL: &str = "lb-portal";
/// How many the Radiant face adds (the real card reads `param(ctx, "cards")`).
pub const PORTAL_RADIANT_CARDS: i32 = 3;
pub const TRAP: &str = "lb-trap";
pub const FIELD_TRAP: &str = "lb-field-trap";

fn defs() -> Vec<CardDef> {
    vec![
        spell_def(9701, json!({ "id": PORTAL, "name": "LB Portal" })),
        spell_def(9702, json!({ "id": TRAP, "name": "LB Trap", "type": "Trap" })),
        spell_def(
            9703,
            json!({ "id": FIELD_TRAP, "name": "LB Field Trap", "type": "Field Trap" }),
        ),
    ]
}

fn scripts_table() -> IndexMap<String, CardScripts> {
    let mut table = IndexMap::new();
    table.insert(
        PORTAL.to_string(),
        CardScripts {
            base: Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::discover_from_last_board(json_as(
                        json!({ "step": "picked" }),
                    ))]
                })),
                resume: IndexMap::from([(
                    "picked",
                    hook(|_ctx| {
                        vec![effects::add_from_last_board(json_as(
                            json!({ "costOverride": LAST_BOARD_CARD_COST }),
                        ))]
                    }),
                )]),
                ..Script::default()
            },
            radiant: Script {
                cry: Some(hook(|_ctx| {
                    vec![effects::add_random_from_last_board(json_as(json!({
                        "count": PORTAL_RADIANT_CARDS,
                        "costOverride": LAST_BOARD_CARD_COST,
                    })))]
                })),
                ..Script::default()
            },
        },
    );
    table
}

/// This file's definitions, by id.
pub fn catalog() -> CardDefs {
    defs().into_iter().map(|card| (card.id.clone(), card)).collect()
}

/// This file's scripts, by id.
pub fn scripts() -> IndexMap<String, CardScripts> {
    scripts_table()
}

/// The vanilla fixture catalog plus this file's cards.
pub fn register_last_boards() {
    setup_catalog();
    let mut all_defs = registered_catalog().clone();
    all_defs.extend(catalog());
    register_catalog(all_defs);
    let mut all_scripts = registered_scripts().clone();
    all_scripts.extend(scripts());
    register_scripts(all_scripts);
}

/// p1's deck holds the Portal; p2's is vanilla.
pub static LB_DECKS: LazyLock<(Vec<String>, Vec<String>)> = LazyLock::new(|| {
    let mut first = vec![PORTAL.to_string()];
    first.extend(vanilla_deck(DECK_SIZE - 1, 1));
    (first, vanilla_deck(DECK_SIZE, 21))
});

pub struct PortalGame {
    pub seed: String,
    pub state: GameState,
    pub log: Vec<Action>,
}

/// A real game: the first seed from `seed_base` whose opening deal puts the Portal in p1's hand,
/// past both mulligans (everything kept), in p1's first main phase.
pub fn portal_game(seed_base: &str, last_boards: Option<LastBoardInput>) -> PortalGame {
    register_last_boards();
    for at in 0..200 {
        let seed = format!("{seed_base}-{at}");
        let mut state = begin_game(&create_game(&CreateGameOptions {
            seed: seed.clone(),
            decks: LB_DECKS.clone(),
            last_boards: last_boards.clone(),
            ..CreateGameOptions::default()
        }))
        .state;
        if !state.players.p1.hand.iter().any(|card| card.def_id == PORTAL) {
            continue;
        }
        let mut log: Vec<Action> = Vec::new();
        for player in [PlayerId::P1, PlayerId::P2] {
            let keep: Vec<String> = state.players[player]
                .hand
                .iter()
                .map(|card| card.id.clone())
                .collect();
            state = act_logged(
                &state,
                json!({ "type": "mulligan", "keep": keep, "playerId": player }),
                &mut log,
            )
            .state;
        }
        return PortalGame { seed, state, log };
    }
    panic!("no seed from {seed_base} deals p1 the Portal");
}
