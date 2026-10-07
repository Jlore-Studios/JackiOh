//! A Tribute can pay for its own zone (docs/classic-sets.md B4.5; SPEC §3.2, R391): with its row full,
//! a Tribute card may be played into a zone its own Tribute empties — a pile of exactly one tributed
//! unit, without Reborn, not Locked — and `legalActions` pairs each zone with the paying sets that
//! leave it open, while the play's check reads the same pair (R90, R101, R210).
//!
//! Port of `packages/engine/test/tribute-zones.test.ts`.

use jackioh_engine::effects::{add_to_hand, chosen_options, discover_from_catalog, summon};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{events_of_type, in_hand, put, slot};
use crate::rules::fixtures::play_pipeline_b::{
    DISCOVER_POOL, grave_trap, pb_act, pb_playing, pb_reduce, plays_of, reborn_body, stack_body, titan, titan_two,
    tribute_field,
};

fn unit_def(name: &str, index: i32) -> CardDef {
    json_as(json!({
        "id": format!("pbt-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (tribute zones)"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "attack": 1, "health": 1, "keywords": [], "text": name },
        "radiant": { "attack": 2, "health": 2, "keywords": [], "text": name },
    }))
}

/// Its Death summons a Rush Token: a card R210's held zone must keep out.
fn summoner() -> CardDef {
    unit_def("summoner", 4611)
}
/// Its Death Discovers, so the play pauses at step 2 with its zone held.
fn asker() -> CardDef {
    unit_def("asker", 4612)
}

fn inline() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();
    let summons = || Script {
        death: Some(hook(|_ctx| vec![summon(json_as(json!({ "defId": "fx-token-rush" })))])),
        ..Script::default()
    };
    scripts.insert(
        summoner().id,
        CardScripts {
            base: summons(),
            radiant: summons(),
        },
    );
    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    resume.insert(
        "picked",
        hook(|ctx| match chosen_options(ctx).first() {
            None => vec![],
            Some(def_id) => vec![add_to_hand(json_as(json!({ "defId": def_id })))],
        }),
    );
    scripts.insert(
        asker().id,
        CardScripts {
            base: Script {
                death: Some(hook(|_ctx| {
                    vec![discover_from_catalog(json_as(
                        json!({ "step": "picked", "query": { "defId": DISCOVER_POOL } }),
                    ))]
                })),
                resume,
                ..Script::default()
            },
            radiant: Script::default(),
        },
    );
    scripts
}

fn playing(seed: &str) -> GameState {
    let state = pb_playing(seed);
    let mut catalog = registered_catalog().clone();
    catalog.insert(summoner().id, summoner());
    catalog.insert(asker().id, asker());
    register_catalog(catalog);
    let mut scripts = registered_scripts().clone();
    scripts.extend(inline());
    register_scripts(scripts);
    state
}

/// Five units in p1's row, lane by lane (a different def per lane where asked).
fn fill_row(state: &mut GameState, defs: &[(i32, String)]) -> Vec<CardInstance> {
    (1..=5)
        .map(|lane| {
            let def_id = defs
                .iter()
                .find(|(at, _)| *at == lane)
                .map_or_else(|| format!("fx-{lane}"), |(_, def_id)| def_id.clone());
            put(&mut *state, &def_id, slot(PlayerId::P1, Row::Units, lane), json!({}))
        })
        .collect()
}

/// TS `only(items)` (playPipelineB's): the first item, failing on none.
fn only<T: Clone>(items: &[T]) -> T {
    items.first().cloned().expect("expected at least one item")
}

/// A play as its JSON, so `play.zone?.lane` and `play.tributes` read as TS reads them.
fn play_json(play: &impl serde::Serialize) -> Value {
    serde_json::to_value(play).expect("a play serialises")
}

fn lanes(plays: &[Value]) -> Vec<Value> {
    plays.iter().map(|play| play["zone"]["lane"].clone()).collect()
}

fn plays(state: &GameState, card: &CardInstance) -> Vec<Value> {
    plays_of(state, &card.id, PlayerId::P1).iter().map(play_json).collect()
}

/// `whyChoicesRefused(state, "p1", card, { type: "play", instanceId, zone: { row: "units", lane }, tributes })`.
fn refused_at(state: &GameState, card: &CardInstance, lane: i32, tributes: &[&str]) -> Option<String> {
    let play: PlayAction = json_as(json!({
        "type": "play",
        "instanceId": card.id,
        "zone": { "row": "units", "lane": lane },
        "tributes": tributes,
    }));
    why_choices_refused(state, PlayerId::P1, card, &play).err().map(|error| error.message)
}

/// `toMatch(/text/)` on an error that must be there.
fn says(error: &Option<String>, text: &str) -> bool {
    error.as_deref().is_some_and(|message| message.contains(text))
}

fn top_id(state: &GameState, at: usize) -> Option<String> {
    active_units_of(state, PlayerId::P1).get(at).map(|card| card.id.clone())
}

mod r391_b4_5_a_tribute_can_pay_for_its_own_zone {
    use super::*;

    #[test]
    fn r391_with_a_full_row_each_zone_is_offered_with_the_paying_sets_that_empty_it_and_the_play_lands_there() {
        let mut state = playing("r391-full");
        let row = fill_row(&mut state, &[]);
        let card = only(&in_hand(&mut state, &titan().id, PlayerId::P1, 1));

        let offered = plays(&state, &card);
        // Tribute 1 on a full row: one play per lane, paying with that lane's unit.
        assert_eq!(
            offered
                .iter()
                .map(|play| json!([play["zone"]["lane"], play["tributes"]]))
                .collect::<Vec<_>>(),
            row.iter().enumerate().map(|(at, unit)| json!([at + 1, [unit.id]])).collect::<Vec<_>>()
        );
        // The check reads the pair: lane 2 with lane 3's unit does not empty lane 2.
        assert!(says(&refused_at(&state, &card, 2, &[&row[2].id]), "not open"));

        let before = state.players.p2.hero.health;
        let result = pb_reduce(
            &state,
            json!({ "type": "play", "instanceId": card.id, "zone": { "row": "units", "lane": 2 }, "tributes": [row[1].id], "playerId": "p1" }),
        );
        assert_eq!(result.error, None);
        let after = result.state;
        assert_eq!(top_id(&after, 1), Some(card.id.clone()));
        assert!(after.players.p1.graveyard.iter().any(|unit| unit.id == row[1].id));
        // Its Cry fired; the zone step 2 held is released.
        assert_eq!(after.players.p2.hero.health, before - 1);
        assert!(!is_reserved(&after, slot(PlayerId::P1, Row::Units, 2)));
    }

    #[test]
    fn r391_a_play_that_names_no_zone_takes_the_leftmost_open_one_or_on_a_full_row_the_leftmost_its_tribute_empties() {
        let mut state = playing("r391-default");
        let row = fill_row(&mut state, &[]);
        let card = only(&in_hand(&mut state, &titan().id, PlayerId::P1, 1));
        let after = pb_act(
            &state,
            json!({ "type": "play", "instanceId": card.id, "tributes": [row[3].id], "playerId": "p1" }),
        );
        assert_eq!(top_id(&after, 3), Some(card.id.clone()));
    }

    #[test]
    fn r391_r13_r64_a_pile_of_two_frees_nothing_a_unit_with_reborn_frees_nothing_and_a_locked_zone_stays_shut() {
        let mut state = playing("r391-not-freed");
        let row = fill_row(&mut state, &[(2, reborn_body().id.clone())]);
        // Lane 1: a Stack pile of two.
        let mut top = new_instance(&mut state, &stack_body().id, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        assert!(place_on_field(
            &mut state,
            &mut top,
            slot(PlayerId::P1, Row::Units, 1),
            json_as(json!({ "stack": true }))
        ));
        assert_eq!(
            state.players.p1.units[0].as_ref().map(|pile| pile.iter().map(|card| card.id.clone()).collect::<Vec<_>>()),
            Some(vec![top.id.clone(), row[0].id.clone()])
        );
        // Lane 3: Locked under its unit.
        lock_zone(&mut state, slot(PlayerId::P1, Row::Units, 3));

        assert!(!freed_by_tribute(&state, &slot(PlayerId::P1, Row::Units, 1), &[top.id.clone()]));
        assert!(!freed_by_tribute(&state, &slot(PlayerId::P1, Row::Units, 2), &[row[1].id.clone()]));
        assert!(!freed_by_tribute(&state, &slot(PlayerId::P1, Row::Units, 3), &[row[2].id.clone()]));
        assert!(freed_by_tribute(&state, &slot(PlayerId::P1, Row::Units, 4), &[row[3].id.clone()]));

        let card = only(&in_hand(&mut state, &titan().id, PlayerId::P1, 1));
        assert_eq!(lanes(&plays(&state, &card)), vec![json!(4), json!(5)]);
        for (lane, unit) in [(1, &top), (2, &row[1]), (3, &row[2])] {
            assert!(says(&refused_at(&state, &card, lane, &[&unit.id]), "not open"));
        }
    }

    #[test]
    fn r391_a_tribute_2_pairs_each_zone_with_the_sets_that_hold_its_unit_and_an_open_zone_with_every_set() {
        let mut state = playing("r391-two");
        let row: Vec<CardInstance> = (1..=4)
            .map(|lane| put(&mut state, &format!("fx-{lane}"), slot(PlayerId::P1, Row::Units, lane), json!({})))
            .collect();
        let card = only(&in_hand(&mut state, &titan_two().id, PlayerId::P1, 1));
        let offered = plays(&state, &card);
        // Six sets of two out of four; lane 5 is open for all six, and each full lane for the three
        // sets that hold its unit.
        assert_eq!(offered.iter().filter(|play| play["zone"]["lane"] == json!(5)).count(), 6);
        for (at, unit) in row.iter().enumerate() {
            let here: Vec<&Value> =
                offered.iter().filter(|play| play["zone"]["lane"] == json!(at + 1)).collect();
            assert_eq!(here.len(), 3);
            for play in here {
                assert!(play["tributes"].as_array().is_some_and(|tributes| tributes.contains(&json!(unit.id))));
            }
        }
        assert_eq!(
            legal_zones_for(&state, PlayerId::P1, &card, &[row[0].id.clone(), row[1].id.clone()])
                .iter()
                .map(|zone| zone.lane)
                .collect::<Vec<_>>(),
            vec![1, 2, 5]
        );
    }

    #[test]
    fn r391_r210_the_emptied_zone_is_held_for_the_play_a_tributed_units_death_summons_elsewhere_or_nowhere() {
        let mut state = playing("r391-held");
        let row = fill_row(&mut state, &[(3, summoner().id)]);
        let card = only(&in_hand(&mut state, &titan().id, PlayerId::P1, 1));
        let result = pb_reduce(
            &state,
            json!({ "type": "play", "instanceId": card.id, "zone": { "row": "units", "lane": 3 }, "tributes": [row[2].id], "playerId": "p1" }),
        );
        assert_eq!(result.error, None);
        assert_eq!(top_id(&result.state, 2), Some(card.id.clone()));
        // The Death's summon found no other open zone and fizzled (R64).
        let summoned: Vec<Value> = events_of_type(&result.events, GameEventType::Summoned)
            .into_iter()
            .map(|event| serde_json::to_value(event).expect("an event serialises")["defId"].clone())
            .collect();
        assert!(!summoned.contains(&json!("fx-token-rush")));
    }

    #[test]
    fn r391_a_death_that_asks_at_step_2_pauses_the_play_with_its_emptied_zone_still_held_across_json_and_the_play_lands_there() {
        let mut state = playing("r391-pause");
        let row = fill_row(&mut state, &[(5, asker().id)]);
        let card = only(&in_hand(&mut state, &titan().id, PlayerId::P1, 1));
        let paused = pb_act(
            &state,
            json!({ "type": "play", "instanceId": card.id, "zone": { "row": "units", "lane": 5 }, "tributes": [row[4].id], "playerId": "p1" }),
        );
        assert_eq!(paused.pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Discover));
        assert!(is_reserved(&paused, slot(PlayerId::P1, Row::Units, 5)));
        let round: GameState =
            serde_json::from_value(serde_json::to_value(&paused).expect("serialises")).expect("deserialises");
        let answers: Vec<ActionBody> = legal_actions(&paused, PlayerId::P1)
            .into_iter()
            .filter(|action| matches!(action, ActionBody::Answer { .. }))
            .take(1)
            .collect();
        let answer = only(&answers);
        let live = pb_act(
            &paused,
            ActionInput {
                body: answer.clone(),
                player_id: PlayerId::P1,
            },
        );
        let again = pb_act(
            &round,
            ActionInput {
                body: answer,
                player_id: PlayerId::P1,
            },
        );
        assert_eq!(hash_state(&again), hash_state(&live));
        assert_eq!(top_id(&live, 4), Some(card.id.clone()));
        assert!(!is_reserved(&live, slot(PlayerId::P1, Row::Units, 5)));
    }

    #[test]
    fn r391_a_backrow_card_with_a_tribute_cost_reads_the_rule_the_same_way_a_tribute_of_units_empties_no_backrow_zone() {
        let mut state = playing("r391-backrow");
        put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1), json!({}));
        for lane in [1, 2, 3, 4, 5] {
            put(&mut state, &grave_trap().id, slot(PlayerId::P1, Row::Backrow, lane), json!({}));
        }
        let card = only(&in_hand(&mut state, &tribute_field().id, PlayerId::P1, 1));
        assert!(plays(&state, &card).is_empty());
        let first_unit = top_id(&state, 0).expect("a unit");
        assert!(says(
            &pb_reduce(
                &state,
                json!({ "type": "play", "instanceId": card.id, "playerId": "p1", "tributes": [first_unit] }),
            )
            .error,
            "no free backrow zone"
        ));
        // With a backrow zone open it is played as ever, paying its Tribute.
        state.players.p1.backrow[4] = None;
        assert_eq!(lanes(&plays(&state, &card)), vec![json!(5)]);
    }
}
