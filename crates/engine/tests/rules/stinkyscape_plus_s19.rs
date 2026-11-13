// The verbs Stinkyscape+ S19 (issue #3567) added, each through a fixture script before a card
// uses it: an exile by name letters (R3567000) and losing one keyword (R3567004). The cards'
// own tests play them in the real scripts; these pin the verbs themselves, so the engine never
// imports `crates/cards`. Ids are `s19-`.

use jackioh_engine::effects;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{apply_now, setup_catalog};

const EXILE: &str = "s19-exile";
const MOOSE: &str = "s19-moose";
const YAK: &str = "s19-yak";
const CN_MOOSE: &str = "s19-cn";
const FLYER: &str = "s19-flyer";
const WALL: &str = "s19-wall";

fn card(
    index: u32,
    id: &str,
    name: &str,
    type_: &str,
    stats: Option<(i32, i32)>,
    keywords: Value,
    tags: Value,
) -> CardDef {
    let face = match stats {
        Some((attack, health)) => {
            json!({ "attack": attack, "health": health, "keywords": keywords, "text": id })
        }
        None => json!({ "keywords": keywords, "text": id }),
    };
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": name,
        "set": "Core",
        "type": type_,
        "tags": tags,
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face.clone(),
        "radiant": face,
    }))
}

fn spell_script(effects_of: impl Fn() -> Vec<Effect> + Send + Sync + 'static) -> CardScripts {
    let script = Script {
        cry: Some(hook(move |_ctx| effects_of())),
        ..Script::default()
    };
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn register() {
    setup_catalog();
    let mut catalog: CardDefs = registered_catalog().clone();
    for def in [
        card(35670, MOOSE, "Moose", "Unit", Some((1, 1)), json!([]), json!([])),
        card(35671, YAK, "Yak", "Unit", Some((1, 1)), json!([]), json!([])),
        card(
            35672,
            CN_MOOSE,
            "CN Moose",
            "Unit",
            Some((1, 1)),
            json!([]),
            json!(["CN"]),
        ),
        card(
            35673,
            FLYER,
            "Flyer",
            "Unit",
            Some((1, 1)),
            json!([{ "kind": "Flying" }]),
            json!([]),
        ),
        card(
            35674,
            WALL,
            "Wall",
            "Unit",
            Some((1, 1)),
            json!([{ "kind": "Flying" }, { "kind": "Immutable" }]),
            json!([]),
        ),
        card(35675, EXILE, "Axe", "Spell", None, json!([]), json!([])),
    ] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut scripts: IndexMap<String, CardScripts> = registered_scripts().clone();
    scripts.insert(
        EXILE.to_string(),
        spell_script(|| {
            vec![effects::exile_named(json_as(
                json!({ "letters": "MOUSE", "spareTags": ["CN"] }),
            ))]
        }),
    );
    register_scripts(scripts);
}

fn board(p1: Value, p2: Value) -> Scenario {
    register();
    let mut s = scenario(json!({ "seed": "s19", "p1": p1, "p2": p2 }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

fn zone_ids(s: &Scenario, player: PlayerId, zone: &str) -> Vec<String> {
    s.pile(player, zone).iter().map(|card| card.id.clone()).collect()
}

mod r3567000_exile_named {
    use super::*;

    #[test]
    fn r3567000_exiles_every_named_card_in_every_zone_of_both_players() {
        let mut s = board(
            json!({
                "hand": [EXILE, MOOSE],
                "field": [MOOSE],
                "library": [MOOSE, YAK],
                "graveyard": [MOOSE],
            }),
            json!({
                "hand": [MOOSE],
                "field": [MOOSE, YAK, CN_MOOSE],
                "library": [MOOSE, CN_MOOSE],
                "graveyard": [MOOSE, YAK],
            }),
        );
        // The spell's own name holds no M, O, U, S or E, so nothing played is spared by name.
        let spell = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == EXILE)
            .expect("the spell")
            .id;
        s.play(spell.as_str(), json!({}));
        for player in [P1, P2] {
            for id in s.hand(player) {
                assert_ne!(id.def_id.as_str(), MOOSE, "no Moose stays in a hand");
            }
            assert!(
                s.state().players[player]
                    .library
                    .iter()
                    .all(|card| card.def_id != MOOSE),
                "no Moose stays in a library"
            );
            assert!(
                s.state().players[player]
                    .graveyard
                    .iter()
                    .all(|card| card.def_id != MOOSE),
                "no Moose stays in a graveyard"
            );
        }
        let exiled: Vec<String> = [P1, P2]
            .iter()
            .flat_map(|player| zone_ids(&s, *player, "exile"))
            .collect();
        assert_eq!(exiled.len(), 8, "every Moose, both sides, every zone");
        // The unnamed Yak and the CN-tagged Moose stay where they were.
        assert!(s.unit(P2, 2).is_some_and(|unit| unit.def_id.as_str() == YAK));
        assert!(s.unit(P2, 3).is_some_and(|unit| unit.def_id.as_str() == CN_MOOSE));
        assert!(
            s.state().players[P1]
                .library
                .iter()
                .any(|card| card.def_id == YAK)
        );
        assert!(
            s.state().players[P2]
                .library
                .iter()
                .any(|card| card.def_id == CN_MOOSE)
        );
        assert!(
            s.state().players[P2]
                .graveyard
                .iter()
                .any(|card| card.def_id == YAK)
        );
    }

    #[test]
    fn r3567000_the_running_card_itself_is_spared() {
        let mut s = board(json!({ "hand": [EXILE] }), json!({}));
        let spell = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == EXILE)
            .expect("the spell")
            .id;
        s.play(spell.as_str(), json!({}));
        s.expect_in_zone(spell.as_str(), "graveyard");
    }
}

mod r3567004_lose_keyword {
    use super::*;

    fn lose_flying(state: &mut GameState, id: &str) {
        apply_now(
            state,
            effects::lose_keyword(json_as(json!({
                "target": { "of": "instance", "instanceId": id },
                "kind": "Flying",
            }))),
        );
    }

    #[test]
    fn r3567004_a_unit_loses_printed_and_granted_flying_and_keeps_the_loss_in_hand() {
        let mut s = board(json!({ "field": [FLYER] }), json!({}));
        let flyer = s.unit_at(P1, 1).id;
        s.card_mut(&flyer).granted_keywords.push(Keyword::Flying);
        lose_flying(s.state_mut(), &flyer);
        assert!(
            !jackioh_engine::layers::unit_has(s.state(), s.card(&flyer), KeywordKind::Flying),
            "printed and granted Flying are gone"
        );
        assert!(
            !jackioh_engine::layers::keywords_of(s.state(), s.card(&flyer))
                .iter()
                .any(|keyword| keyword.kind() == KeywordKind::Flying)
        );
        apply_now(
            s.state_mut(),
            effects::bounce(json_as(json!({
                "target": { "of": "instance", "instanceId": flyer },
            }))),
        );
        let hand = s.hand(P1);
        assert_eq!(hand.len(), 1, "it bounced");
        assert!(
            !jackioh_engine::layers::keywords_of(s.state(), &hand[0])
                .iter()
                .any(|keyword| keyword.kind() == KeywordKind::Flying),
            "the loss rides the tuning into the hand"
        );
    }

    #[test]
    fn r3567004_an_immutable_unit_keeps_flying() {
        let mut s = board(json!({ "field": [WALL] }), json!({}));
        let wall = s.unit_at(P1, 1).id;
        lose_flying(s.state_mut(), &wall);
        assert!(
            jackioh_engine::layers::unit_has(s.state(), s.card(&wall), KeywordKind::Flying),
            "an Immutable Unit keeps Flying"
        );
    }
}
