// The verb Stinkyscape+ S14 (issue #3565) added, through a fixture script before a card uses it: an
// effect that adds a card to a hand may set its Tribute X (R3565000, Stinkyscape+ #129's "it has no
// Tribute condition", #138's "they have Tribute 1"). The cards' own tests play it in the real
// scripts; these pin the verb itself. Ids are `s14-`.

use jackioh_engine::effects;
use jackioh_engine::testkit::PlayerId::P1;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{run_effect, setup_catalog};

const TRIBUTED: &str = "s14-tributed";
const PLAIN: &str = "s14-plain";
const PRINTED_TRIBUTE: i32 = 2;
const SET_TRIBUTE: i32 = 1;

fn unit_def(index: u32, id: &str, attack: i32, health: i32, cost: i32) -> CardDef {
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (S14 fixture)"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": { "attack": attack, "health": health, "keywords": [], "text": id },
        "radiant": { "attack": attack, "health": health, "keywords": [], "text": id },
    }))
}

fn flags_script(flags: Option<StaticFlags>) -> CardScripts {
    let script = Script {
        static_flags: flags,
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
        unit_def(35650, TRIBUTED, 4, 4, 1),
        unit_def(35651, PLAIN, 1, 3, 1),
    ] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut scripts: IndexMap<String, CardScripts> = registered_scripts().clone();
    scripts.insert(
        TRIBUTED.to_string(),
        flags_script(Some(StaticFlags {
            tribute: Some(PRINTED_TRIBUTE),
            ..StaticFlags::default()
        })),
    );
    scripts.insert(PLAIN.to_string(), flags_script(None));
    register_scripts(scripts);
}

fn board(p1: Value) -> Scenario {
    register();
    let mut s = scenario(json!({ "seed": "s14", "p1": p1, "p2": {} }));
    for player in [P1, PlayerId::P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

/// The id of the card the effect added to p1's hand (or burned, when the hand is full): `tribute` rides
/// the add as the effect's rider.
fn add_tributed(s: &mut Scenario, tribute: Option<i32>) -> String {
    let events = run_effect(
        s.state_mut(),
        effects::add_random_from_catalog(effects::AddRandomFromCatalogArgs {
            query: Some(json_as(json!({ "defId": [TRIBUTED] }))),
            tribute,
            ..Default::default()
        }),
        P1,
    );
    events
        .iter()
        .find_map(|event| match event {
            GameEvent::AddedToHand { instance_id, .. } | GameEvent::Burned { instance_id, .. } => {
                Some(instance_id.clone())
            }
            _ => None,
        })
        .expect("the card was added to the hand or burned")
}

fn tribute_of(s: &Scenario, id: &str) -> i32 {
    tribute_cost_of(s.state(), find_instance(s.state(), id).expect("the card"))
}

/// Plays the card in unit zone 1 with no Tribute chosen, so a card that asks one is refused.
fn play(s: &mut Scenario, id: &str) {
    s.play(id, json!({ "zone": 1 }));
}

mod r3565000_an_effect_may_set_a_cards_tribute_as_it_adds_it_to_a_hand {
    use super::*;

    #[test]
    fn r3565000_tribute_0_asks_no_tribute_in_hand() {
        let mut s = board(json!({ "mana": 1 }));
        let id = add_tributed(&mut s, Some(0));
        let card = find_instance(s.state(), &id).expect("the card");
        assert_eq!(tribute_cost_of(s.state(), card), 0);
        assert_eq!(
            legal_tribute_sets(s.state(), P1, card),
            vec![Vec::<String>::new()]
        );
        // Played with no Unit on the field to sacrifice.
        play(&mut s, &id);
        assert!(
            s.state().players[P1]
                .units
                .iter()
                .flatten()
                .flatten()
                .any(|unit| unit.def_id == TRIBUTED)
        );
    }

    #[test]
    fn r3565000_a_set_tribute_replaces_the_printed_one() {
        let mut s = board(json!({ "mana": 1 }));
        let unset = add_tributed(&mut s, None);
        let set = add_tributed(&mut s, Some(SET_TRIBUTE));
        let zero = add_tributed(&mut s, Some(0));
        assert_eq!(tribute_of(&s, &unset), PRINTED_TRIBUTE);
        assert_eq!(tribute_of(&s, &set), SET_TRIBUTE);
        let tribute_listed = |id: &str| {
            numbered_keywords_on(s.state(), find_instance(s.state(), id).expect("the card"))
                .into_iter()
                .find(|numbered| numbered.key == NumberedKey::Tribute)
        };
        assert_eq!(
            tribute_listed(&unset).and_then(|numbered| numbered.printed),
            Some(PRINTED_TRIBUTE)
        );
        assert_eq!(
            tribute_listed(&set).and_then(|numbered| numbered.printed),
            Some(SET_TRIBUTE)
        );
        // A set 0 is no number a Nerf draws.
        assert_eq!(tribute_listed(&zero), None);
    }

    #[test]
    fn r3565000_the_field_reset_takes_it() {
        let mut s = board(json!({ "mana": 1 }));
        let id = add_tributed(&mut s, Some(0));
        play(&mut s, &id);
        let on_field = find_instance(s.state(), &id).expect("on the field");
        assert_eq!(on_field.zone.z(), ZoneName::Field);
        assert!(on_field.memory.contains_key(TRIBUTE_SET_KEY));
        run_effect(
            s.state_mut(),
            effects::bounce(json_as(
                json!({ "target": { "of": "instance", "instanceId": id } }),
            )),
            P1,
        );
        let back = find_instance(s.state(), &id).expect("back in the hand");
        assert_eq!(back.zone.z(), ZoneName::Hand);
        assert!(!back.memory.contains_key(TRIBUTE_SET_KEY));
        assert_eq!(tribute_of(&s, &id), PRINTED_TRIBUTE);
    }

    #[test]
    fn r3565000_a_burned_card_takes_none() {
        let mut s = board(json!({ "hand": vec![PLAIN; HAND_CAP as usize] }));
        let id = add_tributed(&mut s, Some(0));
        let burned = find_instance(s.state(), &id).expect("the burned card");
        assert_eq!(burned.zone.z(), ZoneName::Graveyard);
        assert!(!burned.memory.contains_key(TRIBUTE_SET_KEY));
    }
}
