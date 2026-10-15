//! R915000–R915006: custom cards in the engine (#915, #917, SPEC §12, `custom.rs`), and R962000: a
//! sandbox game spawns its own (#962, `sandbox.rs`). The engine depends on no Luau host, so these tests
//! lend it a Rust one (`FixtureHost`) that answers a fixture script per custom id; the card kit's own
//! tests are `crates/luau/tests/kit.rs`.

use std::rc::Rc;
use std::sync::Arc;

use jackioh_engine::custom::{
    CustomScriptHost, check_custom_setup, custom_def, custom_id_well_formed, is_custom_id,
    validate_custom_card, with_custom_scripts,
};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;
use jackioh_engine::validator::{
    CatalogSnapshot, DeckDraftInput, DeckInput, DraftRule, LoadoutDeck, LoadoutRule, check_deck_draft,
    validate_deck,
};
use jackioh_engine::{
    CustomCard, CustomFrame, CustomParam, GameEvent, KeywordKind, PrintedRarity, SetName, find_def, fold,
    hash_state, set_ships,
};

use crate::rules::fixtures::catalog::{vanilla_catalog, vanilla_deck};
use crate::rules::fixtures::combat::{self, combat_catalog, spikey_pillow};

const DRAWER: &str = "custom-drawer";
const FAILER: &str = "custom-failer";

fn frame() -> CustomFrame {
    CustomFrame {
        name: "Fixture Custom".to_string(),
        type_: CardType::Unit,
        cost: 2,
        rarity: PrintedRarity::Rare,
        attack: Some(2),
        health: Some(3),
        radiant_attack: None,
        radiant_health: None,
        keywords: vec![KeywordKind::Taunt],
        text: "Cry: Draw {cards}.".to_string(),
        params: vec![CustomParam {
            key: "cards".to_string(),
            base: 1,
            radiant: 2,
        }],
    }
}

fn custom(id: &str) -> CustomCard {
    CustomCard {
        id: id.to_string(),
        version: 1,
        frame: frame(),
        source: "-- a fixture: the host below is Rust".to_string(),
        builder: Some(json!({ "blocks": [] })),
    }
}

/// A host with a Rust script per custom id: the drawer's Cry draws its `cards`, the failer's says it
/// failed (as the Luau host does for a hook that errors).
struct FixtureHost;

impl CustomScriptHost for FixtureHost {
    fn scripts(&self, card: &CustomCard) -> Option<Arc<CardScripts>> {
        let cry = match card.id.as_str() {
            DRAWER => hook(|ctx| {
                let count = jackioh_engine::params::param(&*ctx, "cards");
                vec![jackioh_engine::effects::draw(json_as(json!({ "count": count })))]
            }),
            FAILER => hook(|_| {
                vec![jackioh_engine::effects::script_failed(
                    "cry failed on line 3: boom".to_string(),
                )]
            }),
            _ => return None,
        };
        let script = Script {
            cry: Some(cry),
            ..Script::default()
        };
        Some(Arc::new(CardScripts {
            base: script.clone(),
            radiant: script,
        }))
    }
}

fn setup() {
    register_catalog(vanilla_catalog(40, 1));
}

fn board(cards: &[&str]) -> Value {
    let customs: Vec<CustomCard> = cards.iter().map(|id| custom(id)).collect();
    let mut hand: Vec<Value> = cards.iter().map(|id| json!(id)).collect();
    hand.push(json!("fx-1"));
    json!({
        "custom": customs,
        "p1": { "hand": hand, "library": ["fx-2", "fx-3", "fx-4"] },
        "p2": { "hand": ["fx-5"], "library": ["fx-6"] },
    })
}

// ---------------------------------------------------------------------------------------------
// R915000: the id space and the definition
// ---------------------------------------------------------------------------------------------

#[test]
fn r915000_a_custom_id_is_the_prefix_and_a_tail_of_lowercase_letters_digits_and_hyphens() {
    for good in [
        "custom-a",
        "custom-0123456789abcdef0123456789abcdef",
        "custom-my-card-2",
    ] {
        assert!(custom_id_well_formed(good), "{good}");
        assert!(is_custom_id(good));
    }
    for bad in [
        "custom-",
        "custom-Big",
        "custom-a b",
        "custom-é",
        "core-001",
        "Custom-a",
        "t-1",
    ] {
        assert!(!custom_id_well_formed(bad), "{bad}");
    }
    assert!(!custom_id_well_formed(&format!(
        "custom-{}",
        "a".repeat(CUSTOM_ID_MAX_LENGTH)
    )));
    assert!(
        is_custom_id("custom-"),
        "the prefix alone is in the space, though no card's id"
    );
    assert!(!is_custom_id("core-001"));
}

#[test]
fn r915000_a_custom_card_s_definition_is_of_the_custom_set_which_never_ships() {
    let card = custom(DRAWER);
    let def = custom_def(&card);
    assert_eq!(def.id, DRAWER);
    assert_eq!(def.index, DRAWER);
    assert_eq!(def.set, SetName::Custom);
    assert!(!set_ships(SetName::Custom));
    assert!(
        !jackioh_engine::deckable(&def),
        "no random deck or pool reads it as a catalog card"
    );
    assert!(!def.token);
    assert!(def.tags.is_empty());
    assert_eq!(def.cost, CardCost::Fixed(2));
    assert_eq!((def.base.attack, def.base.health), (Some(2), Some(3)));
    assert_eq!(
        (def.radiant.attack, def.radiant.health),
        (Some(4), Some(6)),
        "twice the base by default"
    );
    assert_eq!(def.base.keywords, vec![Keyword::Taunt]);
    let params = def.params.expect("its number");
    assert_eq!(
        (params[0].key.as_str(), params[0].base, params[0].radiant),
        ("cards", 1, 2)
    );
}

#[test]
fn r915000_a_game_reads_its_custom_cards_definitions_and_nothing_else_does() {
    setup();
    let s = scenario(board(&[DRAWER]));
    assert_eq!(
        find_def(Some(s.state()), DRAWER).map(|def| def.name.as_str()),
        Some("Fixture Custom")
    );
    assert_eq!(find_def(None, DRAWER), None, "no catalog holds a custom card");
    assert!(registered_catalog().keys().all(|id| !is_custom_id(id)));
    let view = view_for(s.state(), P1);
    assert_eq!(
        view.defs
            .as_ref()
            .and_then(|defs| defs.get(DRAWER))
            .map(|def| def.set),
        Some(SetName::Custom),
        "the view hands the client the definition of a custom card it names"
    );
    let enemy = view_for(s.state(), P2);
    assert!(
        enemy.defs.as_ref().is_none_or(|defs| !defs.contains_key(DRAWER)),
        "nor one it may not read: the card is in P1's hand"
    );
}

// ---------------------------------------------------------------------------------------------
// R915001: the frame's limits
// ---------------------------------------------------------------------------------------------

#[test]
fn r915001_a_valid_card_has_no_issue_and_each_limit_names_its_field() {
    assert_eq!(validate_custom_card(&custom(DRAWER)), Vec::new());
    let fields = |change: &dyn Fn(&mut CustomCard)| -> Vec<String> {
        let mut card = custom(DRAWER);
        change(&mut card);
        validate_custom_card(&card)
            .into_iter()
            .map(|issue| issue.field)
            .collect()
    };
    assert_eq!(fields(&|c| c.id = "custom-".to_string()), vec!["id"]);
    assert_eq!(fields(&|c| c.version = 0), vec!["version"]);
    assert_eq!(fields(&|c| c.frame.name = "   ".to_string()), vec!["name"]);
    assert_eq!(
        fields(&|c| c.frame.name = "x".repeat(CUSTOM_NAME_MAX_LENGTH + 1)),
        vec!["name"]
    );
    assert_eq!(
        fields(&|c| c.frame.type_ = CardType::Trap),
        vec!["type", "attack", "keywords"]
    );
    assert_eq!(fields(&|c| c.frame.cost = CUSTOM_COST_MAX + 1), vec!["cost"]);
    assert_eq!(fields(&|c| c.frame.cost = -1), vec!["cost"]);
    assert_eq!(
        fields(&|c| c.frame.attack = Some(CUSTOM_ATTACK_MAX + 1)),
        vec!["attack"]
    );
    assert_eq!(fields(&|c| c.frame.health = Some(0)), vec!["health"]);
    assert_eq!(fields(&|c| c.frame.health = None), vec!["health"]);
    assert_eq!(
        fields(&|c| c.frame.radiant_attack = Some(CUSTOM_RADIANT_STAT_MAX + 1)),
        vec!["radiantAttack"]
    );
    assert_eq!(
        fields(&|c| c.frame.keywords = vec![KeywordKind::Stack]),
        vec!["keywords"]
    );
    assert_eq!(
        fields(&|c| c.frame.keywords = vec![KeywordKind::Taunt, KeywordKind::Taunt]),
        vec!["keywords"]
    );
    assert_eq!(
        fields(&|c| c.frame.keywords = vec![
            KeywordKind::Taunt,
            KeywordKind::Rush,
            KeywordKind::Charge,
            KeywordKind::Reborn,
            KeywordKind::Trample
        ]),
        vec!["keywords"]
    );
    assert_eq!(
        fields(&|c| c.frame.text = "x".repeat(CUSTOM_TEXT_MAX_LENGTH + 1)),
        vec!["text"]
    );
    assert_eq!(
        fields(&|c| c.frame.text = "Deal {dmg}.".to_string()),
        vec!["text"]
    );
    assert_eq!(
        fields(&|c| c.frame.text = "Draw {cards|card|cards}.".to_string()),
        Vec::<String>::new(),
        "a plural form names its key"
    );
    assert_eq!(
        fields(&|c| c.frame.params[0].key = "Bad Key".to_string()),
        vec!["params", "text"]
    );
    assert_eq!(
        fields(&|c| c.frame.params[0].radiant = CUSTOM_PARAM_MAX + 1),
        vec!["params"]
    );
    assert_eq!(
        fields(&|c| c.frame.params.push(CustomParam {
            key: "cards".to_string(),
            base: 1,
            radiant: 1
        })),
        vec!["params"]
    );
    assert_eq!(
        fields(&|c| c.source = "x".repeat(CUSTOM_SOURCE_MAX_BYTES + 1)),
        vec!["source"]
    );
    assert_eq!(
        fields(&|c| c.builder = Some(json!("x".repeat(CUSTOM_BUILDER_MAX_BYTES)))),
        vec!["builder"]
    );
    // A Spell has no stats and no keywords.
    assert_eq!(
        fields(&|c| {
            c.frame.type_ = CardType::Spell;
            c.frame.attack = None;
            c.frame.health = None;
            c.frame.keywords = Vec::new();
        }),
        Vec::<String>::new()
    );
    assert_eq!(
        fields(&|c| c.frame.type_ = CardType::Spell),
        vec!["attack", "keywords"]
    );
}

// ---------------------------------------------------------------------------------------------
// R915002: the host lent for a call
// ---------------------------------------------------------------------------------------------

#[test]
fn r915002_a_custom_card_runs_the_lent_host_s_script_and_none_without_one() {
    setup();
    let host: Rc<dyn CustomScriptHost> = Rc::new(FixtureHost);
    with_custom_scripts(host, || {
        let mut s = scenario(board(&[DRAWER]));
        s.play(DRAWER, json!({ "zone": 1 }));
        assert_eq!(
            s.state().players[P1].hand.len(),
            2,
            "the filler and the Cry's draw"
        );
    });
    // The same play with no host lent: the card plays as a vanilla card.
    let mut s = scenario(board(&[DRAWER]));
    s.play(DRAWER, json!({ "zone": 1 }));
    assert_eq!(
        s.state().players[P1].hand.len(),
        1,
        "only the filler: no script ran"
    );
}

#[test]
fn r915002_a_lent_host_is_put_back_when_its_call_ends() {
    struct Nothing;
    impl CustomScriptHost for Nothing {
        fn scripts(&self, _: &CustomCard) -> Option<Arc<CardScripts>> {
            None
        }
    }
    setup();
    let s = scenario(board(&[DRAWER]));
    let has_cry = || {
        jackioh_engine::scripts::scripts_for(s.state(), DRAWER)
            .base
            .cry
            .is_some()
    };
    assert!(!has_cry());
    with_custom_scripts(Rc::new(FixtureHost), || {
        assert!(has_cry());
        with_custom_scripts(Rc::new(Nothing), || assert!(!has_cry(), "the inner host answers"));
        assert!(has_cry(), "the outer host is back");
    });
    assert!(!has_cry(), "and none after");
}

// ---------------------------------------------------------------------------------------------
// R915004: a hook that failed says so
// ---------------------------------------------------------------------------------------------

#[test]
fn r915004_script_failed_names_the_card_its_controller_and_what_went_wrong() {
    setup();
    with_custom_scripts(Rc::new(FixtureHost), || {
        let mut s = scenario(board(&[FAILER]));
        s.play(FAILER, json!({ "zone": 1 }));
        let card = s.unit(P1, 1).expect("the card is played");
        let failed: Vec<&GameEvent> = s
            .events()
            .iter()
            .filter(|event| matches!(event, GameEvent::ScriptFailed { .. }))
            .collect();
        assert_eq!(
            failed,
            vec![&GameEvent::ScriptFailed {
                player: P1,
                instance_id: Some(card.id.clone()),
                def_id: FAILER.to_string(),
                message: "cry failed on line 3: boom".to_string(),
            }]
        );
        assert!(
            view_for(s.state(), P2)
                .events
                .iter()
                .any(|event| matches!(event, GameEvent::ScriptFailed { .. })),
            "both players see it"
        );
    });
}

// ---------------------------------------------------------------------------------------------
// R915005: practice only
// ---------------------------------------------------------------------------------------------

fn snapshot() -> CatalogSnapshot {
    CatalogSnapshot {
        version: "v-test".to_string(),
        cards: vanilla_catalog(40, 1),
        banned: None,
    }
}

#[test]
fn r915005_a_queued_deck_with_a_custom_card_is_refused_by_l7_first_and_by_nothing_else_for_it() {
    let mut cards = vanilla_deck(DECK_SIZE - 1, 1);
    cards.push(DRAWER.to_string());
    let collection = cards.iter().map(|id| (id.clone(), 1)).collect();
    let result = validate_deck(&DeckInput {
        deck: LoadoutDeck {
            name: Some("Mine".to_string()),
            cards,
        },
        catalog: snapshot(),
        collection,
    });
    assert!(!result.ok);
    assert_eq!(result.errors.len(), 1, "{:?}", result.errors);
    assert_eq!(result.errors[0].rule, LoadoutRule::L7);
    assert_eq!(result.errors[0].card_id.as_deref(), Some(DRAWER));
    assert_eq!(
        result.errors[0].message,
        "Mine holds the custom card \"custom-drawer\": a deck with a custom card plays only in practice."
    );
}

#[test]
fn r915005_a_saved_draft_keeps_a_well_formed_custom_id_and_refuses_a_malformed_one() {
    let deckable = |id: &str| id.starts_with("fx-");
    let draft = |cards: Vec<&str>| {
        check_deck_draft(&DeckDraftInput {
            name: "Draft".to_string(),
            cards: cards.into_iter().map(String::from).collect(),
            is_deckable: &deckable,
            portrait: None,
            is_portrait: None,
            name_max_length: 40,
        })
    };
    assert_eq!(draft(vec!["fx-1", DRAWER]), Vec::new());
    let issues = draft(vec!["fx-1", "custom-NOT"]);
    assert_eq!(
        issues.iter().map(|issue| issue.rule).collect::<Vec<_>>(),
        vec![DraftRule::D3]
    );
}

#[test]
fn r915005_a_game_is_dealt_a_custom_card_only_when_given_it_valid_and_once() {
    let mut deck = vanilla_deck(DECK_SIZE - 1, 1);
    deck.push(DRAWER.to_string());
    let decks = (deck.clone(), vanilla_deck(DECK_SIZE, 1));
    assert_eq!(check_custom_setup(&[custom(DRAWER)], &decks), Ok(()));
    assert!(check_custom_setup(&[], &decks).is_err_and(|error| error.contains("was not given")));
    assert!(
        check_custom_setup(&[custom(DRAWER), custom(DRAWER)], &decks)
            .is_err_and(|error| error.contains("given twice"))
    );
    let mut bad = custom(DRAWER);
    bad.frame.cost = 99;
    assert!(check_custom_setup(&[bad], &decks).is_err_and(|error| error.contains("the cost")));

    setup();
    let created = std::panic::catch_unwind(|| {
        create_game(&CreateGameOptions {
            seed: "r915005".to_string(),
            decks: decks.clone(),
            ..CreateGameOptions::default()
        })
    });
    assert!(
        created.is_err(),
        "a deck naming a custom card the game was not given is refused"
    );
}

// ---------------------------------------------------------------------------------------------
// R915006: the game carries its custom cards
// ---------------------------------------------------------------------------------------------

#[test]
fn r915006_a_game_keeps_its_custom_cards_without_the_builder_and_folds_the_same() {
    setup();
    let mut deck = vanilla_deck(DECK_SIZE - 1, 1);
    deck.push(DRAWER.to_string());
    let input = FoldArgs {
        seed: "r915006".to_string(),
        decks: (deck.clone(), deck),
        custom_cards: Some(vec![custom(DRAWER)]),
        log: Vec::new(),
        ..FoldArgs::default()
    };
    let start = fold(&input).state;
    let kept = start.custom_cards.get(DRAWER).expect("the game keeps the card");
    assert_eq!(kept.builder, None, "the builder's model is the creator's");
    assert_eq!(kept.source, custom(DRAWER).source);
    assert_eq!(start.custom_defs.get(DRAWER), Some(&custom_def(&custom(DRAWER))));
    assert_eq!(hash_state(&fold(&input).state), hash_state(&start));

    // A game with no custom card writes neither field, so it hashes as it did before them (D14).
    let plain = fold(&FoldArgs {
        seed: "r915006".to_string(),
        decks: (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 1)),
        log: Vec::new(),
        ..FoldArgs::default()
    })
    .state;
    let text = serde_json::to_string(&plain).expect("a state serialises");
    assert!(!text.contains("customCards") && !text.contains("customDefs"));

    // The cards are part of the hash: another source is another game.
    let mut edited = custom(DRAWER);
    edited.source.push_str(" -- edited");
    let other = fold(&FoldArgs {
        custom_cards: Some(vec![edited]),
        ..input.clone()
    })
    .state;
    assert_ne!(hash_state(&other), hash_state(&start));
}

#[test]
fn r915006_a_log_with_a_custom_card_s_play_folds_to_the_live_hash_under_the_same_host() {
    setup();
    with_custom_scripts(Rc::new(FixtureHost), || {
        let mut s = scenario(board(&[DRAWER]));
        s.play(DRAWER, json!({ "zone": 1 }));
        let live = s.state().clone();
        let roundtrip: GameState = serde_json::from_str(&serde_json::to_string(&live).expect("serialises"))
            .expect("a custom game survives JSON");
        assert_eq!(hash_state(&roundtrip), hash_state(&live));
    });
}

// ---------------------------------------------------------------------------------------------
// R962000: a sandbox game spawns the custom cards it carries
// ---------------------------------------------------------------------------------------------

/// The vanilla units and the combat fixtures, among them Spikey Pillow, whose aura takes 2 attack off
/// its controller's units.
fn sandbox_setup() {
    register_catalog(combat_catalog(vanilla_catalog(40, 1)));
    register_scripts(combat::scripts());
}

/// A sandbox game (R923000's flag, which the scenario has no option for) carrying `cards`, with
/// `p1_field` on P1's field and none of the cards dealt.
fn sandbox_board(cards: &[&str], p1_field: Value) -> Scenario {
    let customs: Vec<CustomCard> = cards.iter().map(|id| custom(id)).collect();
    let mut s = scenario(json!({
        "custom": customs,
        "p1": { "hand": ["fx-1"], "field": p1_field, "library": ["fx-2", "fx-3", "fx-4"] },
        "p2": { "hand": ["fx-5"], "library": ["fx-6"] },
    }));
    s.state_mut().sandbox = Some(true);
    s
}

fn spawn_into(def_id: &str, to: Value) -> ActionBody {
    ActionBody::Sandbox {
        op: json_as(json!({ "kind": "spawn", "player": "p1", "defId": def_id, "to": to })),
    }
}

/// The error a spawn of `def_id` into P1's hand is refused with, the state left as it was.
fn refused_spawn(state: &GameState, def_id: &str) -> String {
    let action = Action::new(spawn_into(def_id, json!({ "zone": "hand" })), P1, "refused");
    let out = reduce(state, &action);
    assert_eq!(&out.state, state, "a refusal leaves the state as it was");
    out.error
        .unwrap_or_else(|| panic!("{action:?} should have been refused"))
}

#[test]
fn r962000_a_sandbox_game_spawns_a_custom_card_it_carries_into_every_zone_placed_not_played() {
    sandbox_setup();
    with_custom_scripts(Rc::new(FixtureHost), || {
        let mut s = sandbox_board(&[DRAWER], json!([spikey_pillow.id]));
        let played = s.state().counters.played;

        // On the field: summoned, its Cry not run (no draw), sick as any summon, under the auras.
        s.act(
            spawn_into(DRAWER, json!({ "zone": "field", "row": "units", "lane": 2 })),
            P1,
            "spawn onto the field",
        );
        let unit = s.unit(P1, 2).expect("the custom card in lane 2");
        assert_eq!(unit.def_id, DRAWER);
        assert_eq!(unit.summoned_turn, Some(s.state().turn));
        let stats = s.stats(&unit);
        assert_eq!(
            (stats.attack, stats.health),
            (0, 3),
            "its own 2/3, less the 2 attack Spikey Pillow's aura takes"
        );
        assert!(stats.keywords.contains(&Keyword::Taunt));
        let events = s.last_events();
        assert!(
            events.iter().any(
                |event| matches!(event, GameEvent::Summoned { def_id, lane: 2, .. } if def_id == DRAWER)
            )
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, GameEvent::Drawn { .. })),
            "no Cry: {events:?}"
        );
        assert_eq!(s.hand(P1).len(), 1);

        // Into the hand, the graveyard and on top of the library.
        s.act(
            spawn_into(DRAWER, json!({ "zone": "hand" })),
            P1,
            "spawn into the hand",
        );
        assert_eq!(
            s.hand(P1).last().map(|card| card.def_id.clone()),
            Some(DRAWER.to_string())
        );
        s.act(
            spawn_into(DRAWER, json!({ "zone": "graveyard" })),
            P1,
            "spawn into the graveyard",
        );
        assert_eq!(
            s.state().players[P1]
                .graveyard
                .last()
                .map(|card| card.def_id.as_str()),
            Some(DRAWER)
        );
        s.act(
            spawn_into(DRAWER, json!({ "zone": "libraryTop" })),
            P1,
            "spawn on top of the library",
        );
        assert_eq!(
            s.state().players[P1]
                .library
                .first()
                .map(|card| card.def_id.as_str()),
            Some(DRAWER)
        );
        assert_eq!(s.state().counters.played, played, "no play is counted");
    });
}

#[test]
fn r962000_a_spawned_custom_card_plays_as_the_dealt_one_its_script_and_its_failure_included() {
    sandbox_setup();
    with_custom_scripts(Rc::new(FixtureHost), || {
        let mut s = sandbox_board(&[DRAWER, FAILER], json!([]));
        s.act(
            spawn_into(DRAWER, json!({ "zone": "hand" })),
            P1,
            "spawn the drawer",
        );
        s.play(DRAWER, json!({ "zone": 1 }));
        assert_eq!(s.hand(P1).len(), 2, "the filler and the Cry's draw");
        assert_eq!(s.unit(P1, 1).map(|card| card.def_id), Some(DRAWER.to_string()));

        s.act(
            spawn_into(FAILER, json!({ "zone": "hand" })),
            P1,
            "spawn the failer",
        );
        s.play(FAILER, json!({ "zone": 3 }));
        assert!(
            s.last_events()
                .iter()
                .any(|event| matches!(event, GameEvent::ScriptFailed { def_id, .. } if def_id == FAILER)),
            "a failure is said as R915004 says it"
        );
    });
}

#[test]
fn r962000_a_custom_card_the_game_does_not_carry_is_refused_and_no_tool_is_listed() {
    sandbox_setup();
    let s = sandbox_board(&[DRAWER], json!([]));
    assert_eq!(
        refused_spawn(s.state(), FAILER),
        "there is no card \"custom-failer\" to spawn",
        "a well-formed custom id the game was not given"
    );
    assert_eq!(
        refused_spawn(s.state(), "custom-"),
        "there is no card \"custom-\" to spawn"
    );
    for player in PLAYER_IDS {
        assert!(
            legal_actions(s.state(), player)
                .iter()
                .all(|body| !matches!(body, ActionBody::Sandbox { .. })),
            "legalActions lists no tool for {player}"
        );
    }
    let plain = scenario(board(&[DRAWER]));
    assert_eq!(
        refused_spawn(plain.state(), DRAWER),
        "the sandbox's tools work only in a sandbox game"
    );
}
