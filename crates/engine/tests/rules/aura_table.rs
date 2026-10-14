//! #892: `layers::AuraTable` asks each aura source's hook once for a whole sweep, and a unit read
//! through it is the unit `layers::unit_view` reads alone (§10.4 layer 5): the same sources in the
//! same order (each side's acting units by lane, its carried Units, then its backrow by lane), none
//! from a Vanilla'd source (§6.1, §6.3), none onto a card dormant under a Stack (§3.2, R13), a
//! carried Unit both a source and a target (R446), and the last set-attack in aura order holding.
//! `legal_actions` and `view_for` each build one table, which the hook counts below prove.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{big_dfender, plain, spikey_pillow, stacker, taunter};
use crate::rules::fixtures::field::{FIELD_DEFS, FIELD_SCRIPTS, tower};
use crate::rules::fixtures::harness::setup_catalog;

// Local Field Spells, indexed from 9501 so they collide with no other file's catalog (BUILD §0).

fn def(id: &str, index: u32) -> CardDef {
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (aura table fixture)"),
        "set": "Core",
        "type": "Field Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": id },
        "radiant": { "keywords": [], "text": id },
    }))
}

const COUNTED: &str = "at-counted";
const SET_TWO: &str = "at-set-two";
const SET_FIVE: &str = "at-set-five";

/// An aura giving every card on the field `mod_`, its hook counting each call in `calls`.
fn field_aura(mod_: StatMod, calls: Option<Arc<AtomicUsize>>) -> CardScripts {
    let script = Script {
        aura: Some(aura_hook(move |_args| {
            if let Some(calls) = &calls {
                calls.fetch_add(1, SeqCst);
            }
            vec![AuraEntry {
                applies: Box::new(|unit: &CardInstance| matches!(unit.zone, Zone::Field { .. })),
                mod_: mod_.clone(),
            }]
        })),
        ..Script::default()
    };
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

/// p1's and p2's sides over the fixture catalog, the field fixtures and this file's three auras.
fn board(seed: &str, calls: &Arc<AtomicUsize>, p1: Value, p2: Value) -> Scenario {
    setup_catalog();
    let mut catalog = registered_catalog().clone();
    for card in FIELD_DEFS.iter() {
        catalog.insert(card.id.clone(), card.clone());
    }
    for (id, index) in [(COUNTED, 9501), (SET_TWO, 9502), (SET_FIVE, 9503)] {
        catalog.insert(id.to_string(), def(id, index));
    }
    register_catalog(catalog);
    let mut scripts = registered_scripts().clone();
    scripts.extend(FIELD_SCRIPTS.clone());
    scripts.insert(
        COUNTED.to_string(),
        field_aura(
            StatMod {
                attack: Some(1),
                ..StatMod::default()
            },
            Some(Arc::clone(calls)),
        ),
    );
    for (id, value) in [(SET_TWO, 2), (SET_FIVE, 5)] {
        scripts.insert(
            id.to_string(),
            field_aura(
                StatMod {
                    set_attack: Some(value),
                    ..StatMod::default()
                },
                None,
            ),
        );
    }
    register_scripts(scripts);
    let side = |mut setup: Value| {
        setup["library"] = json!([plain.id, plain.id]);
        setup
    };
    scenario(json!({ "seed": seed, "p1": side(p1), "p2": side(p2) }))
}

fn view(attack: i32, max_health: i32, keywords: Vec<Keyword>, armor: i32, position: Position) -> UnitView {
    UnitView {
        attack,
        max_health,
        health: max_health,
        keywords,
        armor,
        position,
    }
}

fn unit_at(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
    s.unit(player, lane)
        .unwrap_or_else(|| panic!("no unit at {player:?}'s lane {lane}"))
}

#[test]
fn a_table_reads_every_unit_as_a_single_read_does_across_sources_dormant_carried_and_vanilla() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut s = board(
        "aura-table-parity",
        &calls,
        json!({
            "field": [
                plain.id,
                plain.id,
                { "def": stacker.id, "stack": true },
                { "def": taunter.id, "position": "DEF" },
                big_dfender.id,
            ],
            "backrow": [COUNTED, tower.id, COUNTED],
            "hand": [plain.id],
        }),
        json!({ "field": [spikey_pillow.id, plain.id] }),
    );
    // R446: the hand's plain body goes onto the Tower, a carried Unit that is a source and a target.
    s.play(plain.id.as_str(), json!({ "zone": 2, "row": "backrow" }));
    // §6.1, §6.3: the second counted aura is Vanilla'd, so it projects nothing.
    let vanilla = s.backrow(P1, 3).expect("the second counted aura").id;
    s.card_mut(vanilla.as_str()).vanilla = true;

    let state = s.state();
    let dormant = zones::dormant_units_of(state, P1)
        .first()
        .copied()
        .cloned()
        .expect("the plain body under the Stacker");
    let carried = zones::carried_at(
        state,
        ZoneSlot {
            player: P1,
            row: Row::Backrow,
            lane: 2,
        },
    )
    .cloned()
    .expect("the carried plain body");
    let stack = || vec![Keyword::Stack];
    let expected: Vec<(&str, CardInstance, UnitView)> = vec![
        ("p1 lane 1 plain", unit_at(&s, P1, 1), view(4, 3, stack(), 0, Position::Atk)),
        ("p1 lane 2 stacker", unit_at(&s, P1, 2), view(6, 7, stack(), 0, Position::Atk)),
        ("p1 dormant plain", dormant, view(3, 3, vec![], 0, Position::Atk)),
        (
            "p1 lane 3 taunter",
            unit_at(&s, P1, 3),
            view(
                3,
                5,
                vec![
                    Keyword::Taunt,
                    Keyword::Armor { n: 2 },
                    Keyword::Stack,
                    Keyword::Armor { n: 1 },
                ],
                3,
                Position::Def,
            ),
        ),
        ("p1 lane 4 big_dfender", unit_at(&s, P1, 4), view(1, 8, stack(), 0, Position::Atk)),
        ("p1 carried plain", carried, view(4, 3, stack(), 0, Position::Atk)),
        ("p2 lane 1 spikey_pillow", unit_at(&s, P2, 1), view(0, 2, vec![], 0, Position::Atk)),
        ("p2 lane 2 plain", unit_at(&s, P2, 2), view(2, 3, vec![], 0, Position::Atk)),
    ];
    for (what, card, want) in &expected {
        let single = layers::unit_view(state, card);
        assert_eq!(&single, want, "{what} read alone");
    }
}

#[test]
fn the_last_set_attack_in_aura_order_holds_through_a_table() {
    let calls = Arc::new(AtomicUsize::new(0));
    let s = board(
        "aura-table-set-attack",
        &calls,
        json!({ "field": [plain.id], "backrow": [SET_TWO] }),
        json!({ "field": [plain.id], "backrow": [SET_TWO, SET_FIVE] }),
    );
    let state = s.state();
    // p2's backrow comes last in aura order, and its lane 2 last within it: a reordered side or lane
    // would leave a set-two holding.
    for player in [P1, P2] {
        let card = unit_at(&s, player, 1);
        assert_eq!(layers::unit_view(state, &card).attack, 5, "{player:?}'s plain read alone");
        assert_eq!(layers::unclamped_attack(state, &card), 5, "{player:?}'s plain unclamped");
    }
}

#[test]
fn each_aura_hook_runs_once_per_table_legal_actions_and_view_for() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut s = board(
        "aura-table-counts",
        &calls,
        json!({ "field": [plain.id, plain.id, plain.id], "backrow": [COUNTED, COUNTED] }),
        json!({ "field": [plain.id, taunter.id] }),
    );
    let vanilla = s.backrow(P1, 2).expect("the second counted aura").id;
    s.card_mut(vanilla.as_str()).vanilla = true;
    let state = s.state();
    let wall = unit_at(&s, P2, 2).id;

    calls.store(0, SeqCst);
    let attacks: Vec<String> = legal_actions(state, P1)
        .into_iter()
        .filter_map(|action| match action {
            ActionBody::Attack { target_id, .. } => Some(target_id),
            _ => None,
        })
        .collect();
    eprintln!("legal_actions: {} hook calls", calls.load(SeqCst));
    assert_eq!(attacks, vec![wall.clone(), wall.clone(), wall.clone()]);

    calls.store(0, SeqCst);
    view_for(state, P1);
    eprintln!("view_for: {} hook calls", calls.load(SeqCst));

    calls.store(0, SeqCst);
    layers::unit_view(state, &unit_at(&s, P1, 1));
    assert_eq!(calls.load(SeqCst), 1, "one single read");
}
