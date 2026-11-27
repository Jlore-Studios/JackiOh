//! A hero's Armor wears (issue #5183; SPEC §4.4 step 2, §6.1's Armor row). R5183000: each hit on a
//! hero that its Armor takes something off, a hit it takes whole included, wears 1 off the Armor the
//! hero holds itself, reported by `armorWorn`; a Pierce hit wears nothing, nor a hit a lethal
//! replacement moves, and a projection's hits wear it one by one as the pipeline does.

use jackioh_engine::damage::{ProjectedPools, project_hit};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::damage_combat::{gambit, playing};
use crate::rules::fixtures::harness::{new_game, put, slot};

fn game(seed: &str) -> GameState {
    new_game(seed, None)
}

/// One hit of `amount` on `player`'s hero, from no source, alone in its sink: what it dealt and the
/// events it emitted, as the wire writes them.
fn hit_hero(state: &mut GameState, player: PlayerId, amount: i32, flags: Option<Value>) -> (i32, Vec<Value>) {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    let dealt = deal_damage(
        &mut sink,
        DamageArgs {
            source: None,
            target: DamageTarget::Hero { player },
            amount,
            flags: flags.map(json_as),
        },
    );
    let wire = sink
        .events
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serializes"))
        .collect();
    (dealt, wire)
}

fn worn(events: &[Value]) -> Vec<Value> {
    events
        .iter()
        .filter(|event| event["type"] == "armorWorn")
        .cloned()
        .collect()
}

#[test]
fn r5183000_each_hit_the_armor_meets_wears_1_off_the_heros_own_armor_until_none_is_left() {
    let mut state = game("r5183000-wear");
    state.players.p2.hero.armor = 3;

    // Taken whole: no damage, and the Armor wears to 2.
    let (dealt, events) = hit_hero(&mut state, P2, 2, None);
    assert_eq!(dealt, 0);
    assert_eq!(
        events,
        vec![
            json!({ "type": "damageAbsorbed", "sourceId": null, "targetId": "hero-p2", "absorbed": 2, "combat": false }),
            json!({ "type": "armorWorn", "player": "p2", "armor": 2 }),
        ]
    );

    // Taken in part: 5 − 2 = 3 lands, and the Armor wears to 1.
    let (dealt, events) = hit_hero(&mut state, P2, 5, None);
    assert_eq!(dealt, 3);
    assert_eq!(
        events,
        vec![
            json!({ "type": "damage", "sourceId": null, "targetId": "hero-p2", "amount": 3, "combat": false, "absorbed": 2 }),
            json!({ "type": "armorWorn", "player": "p2", "armor": 1 }),
        ]
    );

    // The last point goes with the next hit, and the one after lands whole with nothing to wear.
    let (dealt, events) = hit_hero(&mut state, P2, 4, None);
    assert_eq!(dealt, 3);
    assert_eq!(
        worn(&events),
        vec![json!({ "type": "armorWorn", "player": "p2", "armor": 0 })]
    );
    let (dealt, events) = hit_hero(&mut state, P2, 4, None);
    assert_eq!(dealt, 4);
    assert!(worn(&events).is_empty());
    assert_eq!(state.players.p2.hero.armor, 0);
    assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 10);
}

#[test]
fn r5183000_a_hit_that_pierces_wears_nothing() {
    let mut state = game("r5183000-pierce");
    state.players.p1.hero.armor = 3;
    let (dealt, events) = hit_hero(&mut state, P1, 2, Some(json!({ "ignoreArmor": true })));
    assert_eq!(dealt, 2);
    assert!(worn(&events).is_empty());
    assert_eq!(state.players.p1.hero.armor, 3);
}

#[test]
fn r5183000_a_hit_a_lethal_replacement_moves_wears_nothing_on_the_hero_it_left() {
    let mut state = playing("r5183000-moved");
    put(&mut state, &gambit.id, slot(P2, Row::Backrow, 1), json!({}));
    state.players.p2.hero.armor = 3;
    state.players.p2.hero.health = 5;
    state.players.p1.hero.armor = 1;
    // 12 − 3 = 9 would bring p2 to 0: Final Gambit sends it to p1, whose Armor takes 1 and wears.
    let (dealt, events) = hit_hero(&mut state, P2, 12, None);
    assert_eq!(dealt, 11);
    assert_eq!(
        worn(&events),
        vec![json!({ "type": "armorWorn", "player": "p1", "armor": 0 })]
    );
    assert_eq!(state.players.p2.hero.armor, 3);
    assert_eq!(state.players.p1.hero.armor, 0);
}

#[test]
fn r5183000_a_projection_wears_the_armor_hit_by_hit_as_the_pipeline_does() {
    let mut state = game("r5183000-projection");
    state.players.p2.hero.armor = 2;
    let hero = DamageTarget::Hero { player: P2 };
    let mut pools = ProjectedPools::default();
    // Three hits of 2: the first meets Armor 2, the second 1, the third none.
    let projected: Vec<i32> = (0..3)
        .map(|_| project_hit(&state, None, &hero, 2, &mut pools))
        .collect();
    assert_eq!(projected, vec![0, 1, 2]);
    assert_eq!(state.players.p2.hero.armor, 2, "a projection writes no state");

    // The pipeline deals the same three hits the same way.
    let dealt: Vec<i32> = (0..3).map(|_| hit_hero(&mut state, P2, 2, None).0).collect();
    assert_eq!(dealt, projected);
}
