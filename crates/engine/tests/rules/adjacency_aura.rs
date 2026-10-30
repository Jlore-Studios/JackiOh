//! R2197001: an adjacency aura (TG #21 Flame Tongue Totem) read through §10.4's layer 5 on fixture
//! scripts: only a pile's top projects or receives an aura (R13), a source standing in the backrow reaches
//! only standing Units (R1081000), and the bonus follows the zones as they stand, stored nowhere.

use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{plain, stacker};
use crate::rules::fixtures::harness::{both, live, new_game, put, slot, spread};

const BONUS: i32 = 3;

fn def(overrides: Value) -> CardDef {
    json_as(spread(
        json!({
            "set": "Core",
            "type": "Unit",
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 1,
            "base": { "keywords": [], "text": "" },
            "radiant": { "keywords": [], "text": "" },
        }),
        overrides,
    ))
}

fn totem() -> CardDef {
    def(json!({
        "id": "aa-totem",
        "index": "21971",
        "name": "Totem (adjacency fixture)",
        "base": { "attack": 0, "health": 4, "keywords": [], "text": "Aura: Adjacent Units have +3 Attack." },
        "radiant": { "attack": 0, "health": 8, "keywords": [], "text": "Aura: Adjacent Units have +3 Attack." },
    }))
}

/// The same Totem with Flying, so it can stand in a backrow zone (R1081000).
fn flying_totem() -> CardDef {
    let flying = json!({ "attack": 0, "health": 4, "keywords": [{ "kind": "Flying" }], "text": "Flying" });
    def(json!({
        "id": "aa-flying-totem",
        "index": "21972",
        "name": "Flying Totem (adjacency fixture)",
        "base": flying,
        "radiant": flying,
    }))
}

fn flyer() -> CardDef {
    let flying = json!({ "attack": 2, "health": 2, "keywords": [{ "kind": "Flying" }], "text": "Flying" });
    def(json!({
        "id": "aa-flyer",
        "index": "21973",
        "name": "Flyer (adjacency fixture)",
        "base": flying,
        "radiant": flying,
    }))
}

/// A Trap with no script: a backrow card that is no Unit.
fn trap() -> CardDef {
    def(json!({
        "id": "aa-trap",
        "index": "21974",
        "name": "Trap (adjacency fixture)",
        "type": "Trap",
    }))
}

/// R2197001: "Adjacent Units have +N Attack": the Units in the zones beside the source, its side and row.
fn adjacency_aura() -> AuraHook {
    aura_hook(|a| {
        let Some(at) = zones::slot_of(a.state, a.self_) else {
            return Vec::new();
        };
        let beside = zones::adjacent(at);
        let source = a.self_;
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                unit.id != source.id
                    && zones::slot_of(a.state, unit).is_some_and(|at| beside.contains(&at))
                    && zones::in_units_row_or_standing(a.state, unit)
                    && zones::is_adjacent_card(a.state, source, unit)
            }),
            mod_: StatMod {
                attack: Some(BONUS),
                ..StatMod::default()
            },
        }]
    })
}

fn board(seed: &str) -> GameState {
    let state = new_game(seed, None);
    let mut catalog = catalog::registered_catalog().clone();
    for def in [totem(), flying_totem(), flyer(), trap()] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut scripts = scripts::registered_scripts();
    for id in [totem().id, flying_totem().id] {
        scripts.insert(
            id,
            both(Script {
                aura: Some(adjacency_aura()),
                ..Script::default()
            }),
        );
    }
    register_scripts(scripts);
    state
}

fn attack(state: &GameState, id: &str) -> i32 {
    layers::unit_view(state, &live(state, id)).attack
}

fn stack_on(state: &mut GameState, def_id: &str, at: ZoneSlot) -> CardInstance {
    let mut card = state::new_instance(state, def_id, at.player, Zone::Hand { player: at.player });
    let placed = zones::place_on_field(
        state,
        &mut card,
        at,
        zones::PlaceOnFieldOptions { stack: Some(true) },
    );
    assert!(placed, "could not stack {def_id} on {} {}", at.row, at.lane);
    live(state, &card.id)
}

fn p1(row: Row, lane: i32) -> ZoneSlot {
    slot(PlayerId::P1, row, lane)
}

fn p2(row: Row, lane: i32) -> ZoneSlot {
    slot(PlayerId::P2, row, lane)
}

mod r2197001_adjacency_aura {
    use super::*;

    #[test]
    fn r2197001_the_units_in_lanes_n_minus_1_and_n_plus_1_have_it_and_no_other() {
        let mut state = board("r2197001-lanes");
        let source = put(&mut state, &totem().id, p1(Row::Units, 3), json!({}));
        let left = put(&mut state, &plain.id, p1(Row::Units, 2), json!({}));
        let right = put(&mut state, &plain.id, p1(Row::Units, 4), json!({}));
        let far = put(&mut state, &plain.id, p1(Row::Units, 5), json!({}));
        let across = put(&mut state, &plain.id, p2(Row::Units, 3), json!({}));
        let enemy_beside = put(&mut state, &plain.id, p2(Row::Units, 2), json!({}));
        let printed = plain.base.attack.unwrap_or(0);
        assert_eq!(attack(&state, &left.id), printed + BONUS);
        assert_eq!(attack(&state, &right.id), printed + BONUS);
        assert_eq!(
            layers::unit_view(&state, &live(&state, &left.id)).max_health,
            plain.base.health.unwrap_or(0)
        );
        assert_eq!(attack(&state, &source.id), 0, "never itself");
        assert_eq!(attack(&state, &far.id), printed, "two lanes away");
        assert_eq!(attack(&state, &across.id), printed, "never across sides");
        assert_eq!(
            attack(&state, &enemy_beside.id),
            printed,
            "an enemy Unit is never adjacent"
        );
    }

    #[test]
    fn r2197001_r13_only_a_piles_top_has_it_and_a_source_beneath_a_pile_gives_none() {
        let mut state = board("r2197001-piles");
        put(&mut state, &totem().id, p1(Row::Units, 2), json!({}));
        let under = put(&mut state, &plain.id, p1(Row::Units, 1), json!({}));
        let top = stack_on(&mut state, &stacker.id, p1(Row::Units, 1));
        assert_eq!(attack(&state, &top.id), stacker.base.attack.unwrap_or(0) + BONUS);
        assert_eq!(
            attack(&state, &under.id),
            plain.base.attack.unwrap_or(0),
            "a dormant card gets none"
        );

        let mut buried = board("r2197001-buried-source");
        let neighbour = put(&mut buried, &plain.id, p1(Row::Units, 3), json!({}));
        put(&mut buried, &totem().id, p1(Row::Units, 2), json!({}));
        assert_eq!(
            attack(&buried, &neighbour.id),
            plain.base.attack.unwrap_or(0) + BONUS
        );
        stack_on(&mut buried, &stacker.id, p1(Row::Units, 2));
        assert_eq!(
            attack(&buried, &neighbour.id),
            plain.base.attack.unwrap_or(0),
            "a buried source projects none"
        );
    }

    #[test]
    fn r2197001_r1081000_a_source_standing_in_the_backrow_reaches_only_the_units_standing_beside_it() {
        let mut state = board("r2197001-backrow");
        let source = put(&mut state, &flying_totem().id, p1(Row::Backrow, 2), json!({}));
        assert!(zones::stands_in_backrow(&state, &source));
        let wing = put(&mut state, &flyer().id, p1(Row::Backrow, 1), json!({}));
        put(&mut state, &trap().id, p1(Row::Backrow, 3), json!({}));
        let front = put(&mut state, &plain.id, p1(Row::Units, 2), json!({}));
        assert_eq!(attack(&state, &wing.id), flyer().base.attack.unwrap_or(0) + BONUS);
        assert_eq!(
            attack(&state, &front.id),
            plain.base.attack.unwrap_or(0),
            "another row is not adjacent"
        );
    }

    #[test]
    fn r2197001_it_follows_the_zones_as_they_stand_and_is_stored_nowhere() {
        let mut state = board("r2197001-moves");
        let source = put(&mut state, &totem().id, p1(Row::Units, 2), json!({}));
        let neighbour = put(&mut state, &plain.id, p1(Row::Units, 3), json!({}));
        let printed = plain.base.attack.unwrap_or(0);
        assert_eq!(attack(&state, &neighbour.id), printed + BONUS);

        // The neighbour moves two lanes away: the bonus is gone at once.
        let moved = live(&state, &neighbour.id);
        zones::remove_from_field(&mut state, &moved, Default::default());
        let moved = put(&mut state, &plain.id, p1(Row::Units, 5), json!({}));
        assert_eq!(attack(&state, &moved.id), printed);

        // A new neighbour has it; the source leaving takes it, and nothing was written on the neighbour.
        let next = put(&mut state, &plain.id, p1(Row::Units, 1), json!({}));
        assert_eq!(attack(&state, &next.id), printed + BONUS);
        let source = live(&state, &source.id);
        zones::remove_from_field(&mut state, &source, Default::default());
        assert_eq!(attack(&state, &next.id), printed);
        assert_eq!(
            live(&state, &next.id).buffs,
            AttackHealth { attack: 0, health: 0 }
        );
    }
}
