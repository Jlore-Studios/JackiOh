//! Heroic Power (SPEC §8 #98; R43, R103, R752–R758; BUILD M3-T7's `heroPower.ts` row).
//!
//! R752: "Heroic Power costs (0) and playing it uses nothing. Each of its thirteen powers is an
//! 'Activate: Spend (X)' ability (R384): once per turn, its X paid in mana as it is activated, and the
//! card has only the one it rolled." This file is the engine's half: the table, the abilities, the
//! roll, and the powers whose behaviour needs nothing from the real catalog. `packages/cards`'
//! `098-heroic-power.test.ts` covers the real card again, with the powers that draw from the
//! catalog's pools (Witness Value, Stitching, KY Brainstorm, Pluck, Terminus Tricks, Cat Cafe).
//!
//! `hp-heroic` below is this file's stand-in for #98, wired as the real card is: `startOfGame` rolls
//! with `rollPower`, `activations` are `powerAbilities`, and `resume` points `POWER_RESUME` at
//! `heroPower`. Its catalog entry declares the `shot` number Steady Shot reads (R754).
//!
//! Port of `packages/engine/test/heroPower.test.ts`.

use std::sync::atomic::{AtomicU32, Ordering};

use serde::Serialize;

use jackioh_engine::effects::bounce;
use jackioh_engine::subsystems::activate::uses_this_turn;
use jackioh_engine::subsystems::hero_power::{
    HERO_POWER_NAMES, HERO_POWERS, POWER_KEY, POWER_RESUME, STEADY_SHOT_PARAM, ensure_power, hero_power,
    power_abilities, power_ability_of, power_by_name, power_of, roll_power, used_this_turn,
};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::harness::{
    events_of_type, in_hand, new_game, put, set_library, sink_for, slot,
};
use crate::rules::fixtures::scripts::{HERO_POWERS as FIXTURE_POWER_NAMES, heroic_power};

// ---------------------------------------------------------------------------
// Fixtures: this file's stand-in for #98 and the three tokens it summons by index.
// ---------------------------------------------------------------------------

/// TS `{ ...base, ...extra }`: the keys `extra` names replace the defaults'.
fn spread(mut base: Value, extra: Value) -> Value {
    if let (Value::Object(base), Value::Object(extra)) = (&mut base, extra) {
        for (key, value) in extra {
            base.insert(key, value);
        }
    }
    base
}

/// TS `def(name, type, extra)`; `index` is the value TS's running `nextIndex` (from 1600) gives it.
fn def(name: &str, type_: &str, index: u32, extra: Value) -> CardDef {
    json_as(spread(
        json!({
            "id": format!("hp-{name}"),
            "index": index.to_string(),
            "name": format!("{name} (heroPower)"),
            "set": "Core",
            "type": type_,
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 1,
            "base": { "keywords": [], "text": name },
            "radiant": { "keywords": [], "text": name },
        }),
        extra,
    ))
}

/// §8 #98's shape since R752: a Quickdraw Field Spell that costs (0) and declares Steady Shot's number.
fn heroic() -> CardDef {
    def(
        "heroic",
        "Field Spell",
        1601,
        json!({
            "cost": 0,
            "tags": ["Quickdraw"],
            "rarity": "Mythic",
            "params": [{ "key": STEADY_SHOT_PARAM, "base": 2, "radiant": 4, "better": "up", "step": STEADY_SHOT_RAISE, "min": 1 }],
        }),
    )
}

fn token(index: &str, name: &str, attack: i32, health: i32, tags: &[&str]) -> CardDef {
    let mut all: Vec<&str> = tags.to_vec();
    all.push("Token");
    json_as(json!({
        "id": format!("hp-token-{name}"),
        "index": index,
        "name": format!("{name} Token (heroPower)"),
        "set": "Core",
        "type": "Unit",
        "tags": all,
        "rarity": "Token",
        "token": true,
        "cost": 1,
        "base": { "attack": attack, "health": health, "keywords": [], "text": format!("{attack}/{health}") },
        "radiant": {
            "attack": attack * 2,
            "health": health * 2,
            "keywords": [],
            "text": format!("{}/{}", attack * 2, health * 2),
        },
    }))
}

fn rush_token() -> CardDef {
    token("T-rush", "rush", 3, 3, &[])
}

fn felinor_token() -> CardDef {
    token("T-felinor", "felinor", 1, 1, &["Felinor"])
}

fn ghoul_token() -> CardDef {
    token("T-ghoul", "ghoul", 0, 0, &[])
}

/// A 1-health unit with Armor 5, for Ping's Pierce and its Radiant Ghoul.
fn armored() -> CardDef {
    def(
        "armored",
        "Unit",
        1602,
        json!({
            "base": { "attack": 4, "health": 1, "keywords": [{ "kind": "Armor", "n": 5 }], "text": "Armor 5" },
            "radiant": { "attack": 8, "health": 2, "keywords": [{ "kind": "Armor", "n": 5 }], "text": "Armor 5" },
        }),
    )
}

fn defs() -> Vec<CardDef> {
    vec![heroic(), rush_token(), felinor_token(), ghoul_token(), armored()]
}

fn heroic_script(radiant: bool) -> Script {
    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    resume.insert(POWER_RESUME, hook(hero_power));
    Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        start_of_game: Some(hook(|_ctx| vec![roll_power()])),
        activations: power_abilities(radiant),
        resume,
        ..Script::default()
    }
}

/// TS `registerCatalog({ ...registeredCatalog(), ...DEFS })` and `registerScripts({ ...registeredScripts(), ...SCRIPTS })`.
fn register_fixtures() {
    let mut catalog = registered_catalog().clone();
    for entry in defs() {
        catalog.insert(entry.id.clone(), entry);
    }
    register_catalog(catalog);
    let mut scripts = registered_scripts();
    scripts.insert(
        heroic().id,
        CardScripts {
            base: heroic_script(false),
            radiant: heroic_script(true),
        },
    );
    register_scripts(scripts);
}

fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    register_fixtures();
    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;
    state.players.p1.mana = ManaState {
        current: 4,
        max: 4,
        next_turn_mod: 0,
        perm_mod: 0,
    };
    state
}

/// Write one key of a card's memory in the state (TS wrote through the live object) and hand back
/// the card as it now stands.
fn set_memory(state: &mut GameState, id: &str, key: &str, value: Value) -> CardInstance {
    let card = find_instance_mut(state, id).unwrap_or_else(|| panic!("no card {id}"));
    card.memory.insert(key.to_string(), value);
    card.clone()
}

/// A Heroic Power on the field with a chosen power, which is the state every activation needs.
fn powered(state: &mut GameState, name: &str, radiant: bool, lane: i32) -> CardInstance {
    let card = put(
        state,
        &heroic().id,
        slot(P1, Row::Backrow, lane),
        json!({ "radiant": radiant }),
    );
    set_memory(state, &card.id, POWER_KEY, json!(name))
}

static NONCE: AtomicU32 = AtomicU32::new(0);

fn act(state: &GameState, body: Value) -> ReduceResult {
    let n = NONCE.fetch_add(1, Ordering::Relaxed) + 1;
    let mut action = body;
    action["nonce"] = json!(format!("hp{n}"));
    reduce(state, &json_as::<Action>(action))
}

/// One `activate` of the card's power, as the client sends it (R752).
fn use_power(state: &GameState, card: &CardInstance, targets: Option<Value>) -> ReduceResult {
    let mut body = json!({ "type": "activate", "instanceId": card.id, "playerId": "p1" });
    if let Some(targets) = targets {
        body["targets"] = targets;
    }
    act(state, body)
}

/// Keep the turn from auto-ending (§2.5) under the assertions: a second card p1 could still play.
fn keep_turn(state: &mut GameState) {
    in_hand(state, &plain.id, P1, 1);
}

fn on_field_now(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id)
        .cloned()
        .unwrap_or_else(|| panic!("expected card {id}"))
}

fn json_of<T: Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// A power's stored name (R103) as its JSON string, whatever Rust type holds it.
fn name_of<T: Serialize>(name: T) -> String {
    json_of(name).as_str().unwrap_or_default().to_string()
}

/// `expect(HERO_POWER_NAMES).toContain(value)`.
fn is_power_name(value: Option<&Value>) -> bool {
    let names = json_of(HERO_POWER_NAMES);
    value.is_some_and(|value| names.as_array().is_some_and(|names| names.contains(value)))
}

fn events_json(events: &[GameEvent], kind: GameEventType) -> Vec<Value> {
    events_of_type(events, kind).into_iter().map(json_of).collect()
}

fn def_ids(cards: &[&CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.def_id.clone()).collect()
}

fn lists_activation_of(state: &GameState, card: &CardInstance) -> bool {
    legal_actions(state, P1)
        .iter()
        .any(|body| matches!(body, ActionBody::Activate { instance_id, .. } if *instance_id == card.id))
}

// ---------------------------------------------------------------------------
// The table and the abilities (R103, R752).
// ---------------------------------------------------------------------------

mod heroic_power_the_thirteen_powers_and_their_abilities_r103_r752 {
    use super::*;

    #[test]
    fn r103_keeps_the_eight_stored_names_and_adds_the_patchs_five_at_the_end_each_with_its_x() {
        assert_eq!(
            json_of(HERO_POWER_NAMES),
            json!([
                "recruit",
                "draw",
                "ping",
                "burn",
                "rush",
                "felinor",
                "discover",
                "stitching",
                "armor",
                "insect",
                "brainstorm",
                "pluck",
                "tricks",
            ])
        );
        let xs: Vec<i32> = HERO_POWERS.iter().map(|power| power.x).collect();
        assert_eq!(xs, vec![3, 1, 1, 1, 2, 1, 2, 2, 1, 2, 2, 2, 3]);
        let titles: Vec<String> = HERO_POWERS.iter().map(|power| power.title.to_string()).collect();
        assert_eq!(
            titles,
            vec![
                "Expedition Map",
                "Life Tap",
                "Ping",
                "Steady Shot",
                "Ranching",
                "Cat Cafe",
                "Witness Value",
                "Stitching",
                "Armor Up",
                "Die Insect",
                "KY Brainstorm",
                "Pluck",
                "Terminus Tricks",
            ]
        );
        // R757: Armor Up is the one power named otherwise on the Radiant face.
        let renamed: Vec<String> = HERO_POWERS
            .iter()
            .filter(|power| power.radiant_title != power.title)
            .map(|power| power.radiant_title.to_string())
            .collect();
        assert_eq!(renamed, vec!["Tank Up"]);
        assert!(power_by_name("nonsense").is_none());
    }

    #[test]
    fn r752_each_power_is_a_once_per_turn_activate_ability_paying_its_x_the_cards_only_while_it_rolled_it() {
        let mut state = game("r752-abilities");
        let card = powered(&mut state, "ping", false, 1);
        for radiant in [false, true] {
            let abilities = power_abilities(radiant);
            let ids: Vec<String> = abilities.iter().map(|decl| decl.id.clone()).collect();
            assert_eq!(json!(ids), json_of(HERO_POWER_NAMES));
            for (index, decl) in abilities.iter().enumerate() {
                let power = HERO_POWERS.get(index).expect("a power");
                assert_eq!(decl.uses, ActivationUses::Count(1));
                assert_eq!(decl.cost.and_then(|cost| cost.mana), Some(power.x));
                let title = if radiant { &power.radiant_title } else { &power.title };
                assert!(decl.label.starts_with(&format!("{title}: ")));
                let has = decl.has.as_ref().map(|has| {
                    has(HookArgs {
                        state: &state,
                        self_: &card,
                        radiant,
                    })
                });
                assert_eq!(has, Some(name_of(&power.name) == "ping"));
            }
        }
        // R81: only Ping declares a target, any unit or hero.
        let targeted: Vec<String> = power_abilities(false)
            .iter()
            .filter(|decl| !decl.targets.is_empty())
            .map(|decl| decl.id.clone())
            .collect();
        assert_eq!(targeted, vec!["ping"]);
        assert_eq!(
            power_ability_of(&state, &card).map(|decl| decl.id.clone()),
            Some("ping".to_string())
        );
        let card = {
            let live = find_instance_mut(&mut state, &card.id).expect("the Heroic Power");
            live.memory.shift_remove(POWER_KEY);
            live.clone()
        };
        assert!(power_ability_of(&state, &card).is_none());
    }

    #[test]
    fn r752_the_card_costs_0_and_playing_it_uses_nothing_its_power_is_then_one_activation_a_turn() {
        let mut state = game("r752-play");
        let held = in_hand(&mut state, &heroic().id, P1, 1);
        let card = held.first().expect("a Heroic Power in hand");
        let card = set_memory(&mut state, &card.id, POWER_KEY, json!("burn"));
        keep_turn(&mut state);
        assert_eq!(effective_cost(&state, &card, CostOptions::default()), 0);

        let played = act(
            &state,
            json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }),
        );
        assert_eq!(played.error, None);
        assert_eq!(
            events_json(&played.events, GameEventType::CardPlayed)
                .first()
                .map(|event| event["costPaid"].clone()),
            Some(json!(0))
        );
        assert_eq!(played.state.players.p1.mana.current, 4);
        assert_eq!(played.state.players.p2.hero.health, HERO_HEALTH);
        assert!(events_of_type(&played.events, GameEventType::Activated).is_empty());

        let on_field = on_field_now(&played.state, &card.id);
        assert!(!used_this_turn(&played.state, &on_field));
        let shot = use_power(&played.state, &on_field, None);
        assert_eq!(shot.error, None);
        assert_eq!(shot.state.players.p2.hero.health, HERO_HEALTH - 2);
        assert_eq!(shot.state.players.p1.mana.current, 3);
    }

    #[test]
    fn r752_a_use_pays_x_once_per_turn_is_refused_unaffordable_and_the_next_turn_is_a_new_use() {
        let mut state = game("r752-once");
        let card = powered(&mut state, "recruit", false, 1); // X 3
        set_library(&mut state, P1, &[plain.id.clone()]);
        keep_turn(&mut state);
        state.players.p1.mana.current = 2;
        assert_eq!(
            act(
                &state,
                json!({ "type": "activatePower", "instanceId": card.id, "playerId": "p1" })
            )
            .error
            .as_deref(),
            Some("that ability costs 3, more than your mana")
        );
        assert!(!lists_activation_of(&state, &card));

        state.players.p1.mana.current = 4;
        let recruit: ActionBody = json_as(json!({ "type": "activate", "instanceId": card.id, "ability": "recruit" }));
        assert!(legal_actions(&state, P1).contains(&recruit));
        let used = act(
            &state,
            json!({ "type": "activatePower", "instanceId": card.id, "playerId": "p1" }),
        );
        assert_eq!(used.error, None);
        assert_eq!(used.state.players.p1.mana.current, 1);
        assert_eq!(def_ids(&active_units_of(&used.state, P1)), vec![plain.id.clone()]);
        assert!(used_this_turn(&used.state, &on_field_now(&used.state, &card.id)));
        assert_eq!(
            use_power(&used.state, &card, None).error.as_deref(),
            Some("that ability has already been used this turn")
        );

        let mut later = used.state.clone();
        later.turn += 2;
        later.players.p1.mana.current = 4;
        assert!(!used_this_turn(&later, &on_field_now(&later, &card.id)));
        assert_eq!(use_power(&later, &card, None).error, None);
    }

    #[test]
    fn r752_the_power_is_not_the_opponents_works_only_from_the_field_and_the_alias_names_it_r384() {
        let mut state = game("r752-who");
        let card = powered(&mut state, "burn", false, 1);
        assert_eq!(
            act(
                &state,
                json!({ "type": "activate", "instanceId": card.id, "playerId": "p2" })
            )
            .error
            .as_deref(),
            Some("it is not your turn")
        );
        let held = in_hand(&mut state, &heroic().id, P1, 1);
        let held = held.first().expect("a Heroic Power in hand");
        let held = set_memory(&mut state, &held.id, POWER_KEY, json!("burn"));
        assert_eq!(
            act(
                &state,
                json!({ "type": "activatePower", "instanceId": held.id, "playerId": "p1" })
            )
            .error
            .as_deref(),
            Some("that card is not on the field")
        );
        keep_turn(&mut state);
        let via_alias = act(
            &state,
            json!({ "type": "activatePower", "instanceId": card.id, "playerId": "p1" }),
        );
        let via_activate = act(
            &state,
            json!({ "type": "activate", "instanceId": card.id, "ability": "burn", "playerId": "p1" }),
        );
        assert_eq!(via_alias.error, None);
        assert_eq!(
            via_alias.state.players.p2.hero.health,
            via_activate.state.players.p2.hero.health
        );
    }
}

// ---------------------------------------------------------------------------
// The powers that need nothing from the catalog (R753–R758).
// ---------------------------------------------------------------------------

mod heroic_power_the_powers_r753_r758 {
    use super::*;

    #[test]
    fn r753_life_tap_draws_1_and_deals_2_to_your_own_hero_radiant_draws_the_top_card_of_each_deck() {
        let mut state = game("r753-life-tap");
        let card = powered(&mut state, "draw", false, 1);
        set_library(&mut state, P1, &[plain.id.clone(), plain.id.clone()]);
        set_library(&mut state, P2, &[plain.id.clone(), plain.id.clone()]);
        keep_turn(&mut state);
        let hand = state.players.p1.hand.len();
        let after = use_power(&state, &card, None).state;
        assert_eq!(after.players.p1.hand.len(), hand + 1);
        assert_eq!(after.players.p1.hero.health, HERO_HEALTH - LIFE_TAP_DAMAGE);
        assert_eq!(
            events_of_type(&use_power(&state, &card, None).events, GameEventType::Damage).len(),
            1
        );

        let mut shining = game("r753-life-tap-radiant");
        let radiant = powered(&mut shining, "draw", true, 1);
        set_library(&mut shining, P1, &[plain.id.clone(), plain.id.clone()]);
        let theirs = set_library(&mut shining, P2, &[plain.id.clone(), plain.id.clone()]);
        keep_turn(&mut shining);
        let before = shining.players.p1.hand.len();
        let drawn = use_power(&shining, &radiant, None).state;
        assert_eq!(drawn.players.p1.hand.len(), before + 2);
        let top = theirs.first().expect("their top card");
        assert!(drawn.players.p1.hand.iter().any(|entry| entry.id == top.id));
        assert_eq!(drawn.players.p2.library.len(), 1);
        assert_eq!(drawn.players.p1.hero.health, HERO_HEALTH);
    }

    #[test]
    fn r754_steady_shot_deals_shot_to_the_enemy_hero_on_the_radiant_face_it_then_deals_2_more_each_use() {
        let mut state = game("r754-steady");
        let card = powered(&mut state, "burn", false, 1);
        keep_turn(&mut state);
        let once = use_power(&state, &card, None).state;
        assert_eq!(once.players.p2.hero.health, HERO_HEALTH - 2);
        assert_eq!(
            param_value(
                &once,
                Some(&on_field_now(&once, &card.id)),
                STEADY_SHOT_PARAM,
                Default::default()
            ),
            2
        );

        let mut shining = game("r754-steady-radiant");
        let radiant = powered(&mut shining, "burn", true, 1);
        keep_turn(&mut shining);
        let mut lost = 0;
        for expected in [4, 6, 8] {
            let result = use_power(&shining, &radiant, None);
            assert_eq!(result.error, None);
            lost += expected;
            assert_eq!(result.state.players.p2.hero.health, HERO_HEALTH - lost);
            let changed = events_json(&result.events, GameEventType::NumberChanged);
            let first = changed.first().expect("a numberChanged event");
            assert_eq!(first["key"], json!(STEADY_SHOT_PARAM));
            assert_eq!(first["value"], json!(expected + STEADY_SHOT_RAISE));
            shining = result.state.clone();
            shining.turn += 2;
            shining.players.p1.mana.current = 4;
        }
    }

    #[test]
    fn r756_ping_pierces_armor_on_the_radiant_face_a_unit_it_kills_leaves_a_ghoul_token_with_its_stats() {
        let mut state = game("r756-ping");
        let card = powered(&mut state, "ping", false, 1);
        let victim = put(&mut state, &armored().id, slot(P2, Row::Units, 1), Default::default());
        keep_turn(&mut state);
        // R81: the target is declared with the activation; legalActions lists one per target.
        let listed = legal_actions(&state, P1)
            .into_iter()
            .filter(|body| matches!(body, ActionBody::Activate { instance_id, .. } if *instance_id == card.id))
            .count();
        assert!(listed > 2);
        assert!(use_power(&state, &card, None).error.is_some());
        let hit = use_power(
            &state,
            &card,
            Some(json!([{ "pick": "instance", "instanceId": victim.id }])),
        );
        assert_eq!(hit.error, None);
        assert_eq!(
            events_json(&hit.events, GameEventType::Damage)
                .first()
                .map(|event| event["amount"].clone()),
            Some(json!(1))
        );
        assert!(hit.state.players.p2.units[0]
            .as_ref()
            .and_then(|pile| pile.first())
            .is_none());
        assert!(active_units_of(&hit.state, P1).is_empty());

        let mut shining = game("r756-ping-radiant");
        let radiant = powered(&mut shining, "ping", true, 1);
        let prey = put(&mut shining, &armored().id, slot(P2, Row::Units, 1), Default::default());
        keep_turn(&mut shining);
        let killed = use_power(
            &shining,
            &radiant,
            Some(json!([{ "pick": "instance", "instanceId": prey.id }])),
        );
        assert_eq!(killed.error, None);
        let ghouls = active_units_of(&killed.state, P1);
        assert_eq!(def_ids(&ghouls), vec![ghoul_token().id]);
        assert_eq!(
            ghouls.first().and_then(|ghoul| ghoul.stats_override),
            Some(AttackHealth { attack: 4, health: 1 })
        );

        // A hero hit, or a Unit the hit leaves standing, summons nothing.
        let face = use_power(&shining, &radiant, Some(json!([{ "pick": "hero", "player": "p2" }])));
        assert!(active_units_of(&face.state, P1).is_empty());
        let sturdy = put(&mut shining, &plain.id, slot(P2, Row::Units, 2), Default::default());
        let survived = use_power(
            &shining,
            &radiant,
            Some(json!([{ "pick": "instance", "instanceId": sturdy.id }])),
        );
        assert!(active_units_of(&survived.state, P1).is_empty());
    }

    #[test]
    fn r757_armor_ups_2_armor_holds_through_the_opponents_turn_and_is_gone_when_yours_begins() {
        let mut state = game("r757-armor");
        let card = powered(&mut state, "armor", false, 1);
        keep_turn(&mut state);
        let with_armor = use_power(&state, &card, None).state;
        assert_eq!(hero_armor_of(&with_armor, P1), ARMOR_UP_ARMOR);
        let their_turn = act(&with_armor, json!({ "type": "endTurn", "playerId": "p1" })).state;
        assert_eq!(their_turn.active, P2);
        assert_eq!(hero_armor_of(&their_turn, P1), ARMOR_UP_ARMOR);
        let mine = act(&their_turn, json!({ "type": "endTurn", "playerId": "p2" })).state;
        assert_eq!(mine.active, P1);
        assert_eq!(hero_armor_of(&mine, P1), 0);
    }

    #[test]
    fn r757_tank_up_keeps_4_armor_then_refreshes_into_a_different_power_that_may_be_used_this_turn() {
        let mut state = game("r757-tank-up");
        let card = powered(&mut state, "armor", true, 1);
        keep_turn(&mut state);
        let tanked = use_power(&state, &card, None);
        assert_eq!(tanked.error, None);
        let after = tanked.state;
        assert_eq!(after.players.p1.hero.armor, TANK_UP_ARMOR);
        let changed = on_field_now(&after, &card.id);
        let next = power_of(&changed).expect("a new power");
        assert_ne!(name_of(&next.name), "armor");
        assert_eq!(uses_this_turn(&after, &changed), 0);
        assert!(!used_this_turn(&after, &changed));
        // The armor stays into the next turn: Tank Up's is the hero's own (R757).
        let later = act(
            &act(&after, json!({ "type": "endTurn", "playerId": "p1" })).state,
            json!({ "type": "endTurn", "playerId": "p2" }),
        )
        .state;
        assert!(hero_armor_of(&later, P1) >= TANK_UP_ARMOR);
    }

    #[test]
    fn r758_die_insect_deals_8_to_a_random_enemy_the_radiant_faces_lucky_1_hits_a_unit_more_often() {
        /// `{ hero, unit }`: how often each was hit over 40 seeds.
        fn hits(radiant: bool) -> (i32, i32) {
            let (mut hero, mut unit) = (0, 0);
            for n in 0..40 {
                let mut state = game(&format!("r758-insect-{}-{n}", if radiant { "r" } else { "b" }));
                let card = powered(&mut state, "insect", radiant, 1);
                put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());
                keep_turn(&mut state);
                let result = use_power(&state, &card, None);
                assert_eq!(result.error, None);
                let damage = events_json(&result.events, GameEventType::Damage);
                assert_eq!(damage.len(), 1);
                assert_eq!(damage[0]["amount"], json!(DIE_INSECT_DAMAGE));
                if damage[0]["targetId"] == json!("hero-p2") {
                    hero += 1;
                } else {
                    unit += 1;
                }
            }
            (hero, unit)
        }
        let base = hits(false);
        let radiant = hits(true);
        assert!(base.0 > 0);
        assert!(base.1 > 0);
        assert!(radiant.1 > base.1);
    }
}

// ---------------------------------------------------------------------------
// The roll (R43, R78, R151).
// ---------------------------------------------------------------------------

mod heroic_power_rolling_the_power_r43_r78 {
    use super::*;

    #[test]
    fn r43_rolls_a_power_at_start_of_game_for_every_copy_in_either_players_hand_or_library() {
        let mut state = game("r43-roll-setup");
        let p1_hand = in_hand(&mut state, &heroic().id, P1, 1).remove(0);
        let p2_hand = in_hand(&mut state, &heroic().id, P2, 1).remove(0);
        set_library(&mut state, P1, &[heroic().id, plain.id.clone()]);
        set_library(&mut state, P2, &[plain.id.clone(), heroic().id]);
        let p1_library = state
            .players
            .p1
            .library
            .iter()
            .find(|card| card.def_id == heroic().id)
            .cloned()
            .expect("p1's library copy");
        let p2_library = state
            .players
            .p2
            .library
            .iter()
            .find(|card| card.def_id == heroic().id)
            .cloned()
            .expect("p2's library copy");

        let cards = [p1_hand, p2_hand, p1_library, p2_library];
        for card in &cards {
            assert!(card.memory.get(POWER_KEY).is_none());
        }
        finish_setup(&mut sink_for(&mut state));
        for card in &cards {
            let now = on_field_now(&state, &card.id);
            assert!(is_power_name(now.memory.get(POWER_KEY)));
            assert!(power_of(&now).is_some());
        }
    }

    #[test]
    fn r43_the_roll_comes_from_the_match_rng_so_the_same_seed_rolls_the_same_power() {
        fn rolled(seed: &str) -> Option<Value> {
            let mut state = game(seed);
            let card = in_hand(&mut state, &heroic().id, P1, 1).remove(0);
            finish_setup(&mut sink_for(&mut state));
            find_instance(&state, &card.id).and_then(|now| now.memory.get(POWER_KEY).cloned())
        }
        let first = rolled("r43-seeded");
        assert!(is_power_name(first.as_ref()));
        assert_eq!(rolled("r43-seeded"), first);

        let mut state = game("r43-two-copies");
        let cards = in_hand(&mut state, &heroic().id, P1, 6);
        finish_setup(&mut sink_for(&mut state));
        let powers: IndexSet<String> = cards
            .iter()
            .map(|card| {
                find_instance(&state, &card.id)
                    .and_then(|now| now.memory.get(POWER_KEY))
                    .map(Value::to_string)
                    .unwrap_or_default()
            })
            .collect();
        assert!(powers.len() > 1);
    }

    #[test]
    fn r43_the_m1_m3_fixture_heroic_power_also_rolls_at_start_of_game_setup_2_1() {
        let mut state = new_game("r43-fixture-roll", None);
        let card = in_hand(&mut state, &heroic_power().id, P2, 1).remove(0);
        assert!(card.memory.get("power").is_none());
        finish_setup(&mut sink_for(&mut state));
        let power = find_instance(&state, &card.id)
            .and_then(|now| now.memory.get("power").cloned())
            .unwrap_or(Value::Null);
        let names = json_of(FIXTURE_POWER_NAMES);
        assert!(names.as_array().is_some_and(|names| names.contains(&power)));
    }

    #[test]
    fn r43_ensure_power_rolls_for_an_instance_with_no_power_and_keeps_the_one_it_has() {
        let mut state = game("r43-ensure");
        let mut card = in_hand(&mut state, &heroic().id, P1, 1).remove(0);
        let mut sink = sink_for(&mut state);
        let rolled = name_of(&ensure_power(&mut sink, &mut card).expect("a rolled power").name);
        assert_eq!(card.memory.get(POWER_KEY), Some(&json!(rolled)));
        assert_eq!(
            ensure_power(&mut sink, &mut card).map(|power| name_of(&power.name)),
            Some(rolled.clone())
        );
        card.memory.insert(POWER_KEY.to_string(), json!("recruit"));
        assert_eq!(
            ensure_power(&mut sink, &mut card).map(|power| name_of(&power.name)),
            Some("recruit".to_string())
        );
    }

    #[test]
    fn r43_a_bounced_heroic_power_arrives_in_hand_with_a_power_again_r78_r151() {
        let mut state = game("r43-bounced");
        let card = powered(&mut state, "recruit", false, 1);
        let mut sink = sink_for(&mut state);
        {
            let mut ctx = make_context(
                &mut sink,
                Some(&card),
                HookOptions {
                    controller: Some(P1),
                    targets: Some(vec![Selection::Instance {
                        instance_id: card.id.clone(),
                    }]),
                    ..Default::default()
                },
            );
            apply_effects(
                &[bounce(json_as(json!({ "target": { "of": "chosen" } })))],
                &mut ctx,
            );
        }
        assert_eq!(
            find_instance(sink.state, &card.id).map(|now| now.zone.clone()),
            Some(Zone::Hand { player: P1 })
        );
        settle(&mut sink, SettleOptions::default());
        let now = on_field_now(sink.state, &card.id);
        assert!(is_power_name(now.memory.get(POWER_KEY)));
        assert_eq!(effective_cost(sink.state, &now, CostOptions::default()), 0);
    }

    #[test]
    fn r43_a_game_begun_with_a_heroic_power_in_the_deck_has_one_with_a_power_in_play_2_1() {
        game("r43-begin");
        let mut deck = vec![heroic().id];
        deck.extend(vanilla_deck(DECK_SIZE - 1, 1));
        let begun = begin_game(&create_game(&CreateGameArgs {
            seed: "r43-begin".into(),
            decks: (deck, vanilla_deck(DECK_SIZE, 21)),
            ..Default::default()
        }))
        .state;
        let keep: Vec<String> = begun.players.p1.hand.iter().map(|card| card.id.clone()).collect();
        let mut playing = act(&begun, json!({ "type": "mulligan", "keep": keep, "playerId": "p1" })).state;
        let keep: Vec<String> = playing.players.p2.hand.iter().map(|card| card.id.clone()).collect();
        playing = act(&playing, json!({ "type": "mulligan", "keep": keep, "playerId": "p2" })).state;
        let card = playing
            .players
            .p1
            .hand
            .iter()
            .chain(playing.players.p1.library.iter())
            .find(|entry| entry.def_id == heroic().id)
            .cloned()
            .expect("the Heroic Power");
        assert!(is_power_name(card.memory.get(POWER_KEY)));
        assert_eq!(effective_cost(&playing, &card, CostOptions::default()), 0);
    }
}
