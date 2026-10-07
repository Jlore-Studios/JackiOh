// `shuffleRandomFromCatalog` (effects/shuffleRandom.ts): random catalog cards shuffled into a library,
// Radiant and enchanted (Classic+ #40 Appropriations' Education, E39, R60, R80, R387), through a
// test-only Book that shuffles three. The real card is proved in packages/cards
// (test/classic-plus/040-appropriations.test.ts).
//
// Port of `packages/engine/test/shuffle-random.test.ts`.

use jackioh_engine::effects::shuffle_random_from_catalog;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{new_game, set_library};
use crate::rules::fixtures::prompt_harness::cast_now;

fn book(id: &str, extra: Value) -> CardDef {
    let mut def = json!({
        "id": id,
        "index": id,
        "name": id,
        "set": "Core",
        "type": "Spell",
        "tags": ["Book"],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": id },
        "radiant": { "keywords": [], "text": id },
    });
    if let (Some(into), Some(from)) = (def.as_object_mut(), extra.as_object()) {
        for (key, value) in from {
            into.insert(key.clone(), value.clone());
        }
    }
    json_as(def)
}

/// The shuffling card is a Book itself, so R387 has something to keep out.
fn shuffler() -> CardDef {
    book("sr-shuffler", json!({}))
}

fn pages() -> CardDef {
    book("sr-pages", json!({}))
}

fn token_book() -> CardDef {
    book(
        "sr-token-book",
        json!({ "tags": ["Book", "Token"], "token": true, "rarity": "Token" }),
    )
}

fn scripts() -> Vec<(String, CardScripts)> {
    vec![(
        shuffler().id,
        CardScripts {
            base: Script {
                cry: Some(hook(|_ctx| {
                    vec![shuffle_random_from_catalog(json_as(json!({
                        "query": { "tags": ["Book"] },
                        "count": 3,
                        "radiant": true,
                        "enchantments": [{ "kind": "castOnDraw" }, { "kind": "targetEnemies" }],
                    })))]
                })),
                ..Script::default()
            },
            radiant: Script {
                cry: Some(hook(|_ctx| vec![])),
                ..Script::default()
            },
        },
    )]
}

fn board(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for def in [shuffler(), pages(), token_book()] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut registry = registered_scripts().clone();
    for (id, script) in scripts() {
        registry.insert(id, script);
    }
    register_scripts(registry);
    state.active = PlayerId::P1;
    state.phase = Phase::Main;
    set_library(&mut state, PlayerId::P1, &[]);
    state
}

fn count_of(events: &[GameEvent], kind: GameEventType) -> usize {
    events.iter().filter(|event| event.event_type() == kind).count()
}

mod shuffle_random_from_catalog_e39 {
    use super::*;

    #[test]
    fn r60_r387_shuffles_that_many_random_pool_cards_in_never_the_running_cards_own_nor_a_token_each_radiant_and_enchanted()
     {
        let mut state = board("sr-basic");
        cast_now(&mut state, &shuffler().id, None, None);
        let library = &state.players.p1.library;
        let defs: Vec<String> = library.iter().map(|card| card.def_id.clone()).collect();
        assert_eq!(defs, vec![pages().id, pages().id, pages().id]);
        for card in library {
            assert!(card.radiant);
            assert!(has_enchantment(card, "castOnDraw"));
            assert!(has_enchantment(card, "targetEnemies"));
            assert!(card.known_as.is_some());
        }
    }

    #[test]
    fn e39_a_card_it_made_is_cast_as_it_is_drawn() {
        let mut state = board("sr-draw");
        cast_now(&mut state, &shuffler().id, None, None);
        let mut events = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        draw(&mut sink, PlayerId::P1, 1);
        settle(&mut sink, Default::default());
        assert_eq!(count_of(sink.events, GameEventType::CardPlayed), 3);
        assert_eq!(sink.state.players.p1.library.len(), 0);
    }

    #[test]
    fn r80_a_full_library_turns_the_rest_away() {
        let mut state = board("sr-cap");
        let mut library = Vec::new();
        for _ in 0..LIBRARY_CAP - 1 {
            library.push(new_instance(
                &mut state,
                &pages().id,
                PlayerId::P1,
                Zone::Library { player: PlayerId::P1 },
            ));
        }
        state.players.p1.library = library;
        let events = cast_now(&mut state, &shuffler().id, None, None);
        assert_eq!(state.players.p1.library.len() as i32, LIBRARY_CAP);
        assert_eq!(count_of(&events, GameEventType::LibraryOverflow), 2);
    }
}
