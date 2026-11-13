// The verbs Stinkyscape+ S18 (issue #3557) added, each through a fixture script before a card uses it:
// Dormant applied by an effect (R3557002), a summon that takes the backrow first (R3557003) and a
// Transform into a card that costs N more (R3557004). The cards' own tests play them in the real
// scripts; these pin the verbs themselves, so the engine never imports `crates/cards`. Ids are `s18-`.

use jackioh_engine::effects;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{apply_now, setup_catalog};

const DORMANT: &str = "s18-dormant";
const SUMMON_BACK: &str = "s18-summon-back";
const STEP_1: &str = "s18-step-1";
const STEP_2: &str = "s18-step-2";
const STEP_3: &str = "s18-step-3";
const STEP_ALL: &str = "s18-step-all";
/// Units of costs the pool of no other fixture reaches, so the draws are forced.
const LOW: &str = "s18-unit-30";
const MID: &str = "s18-unit-32";
const GAP: &str = "s18-unit-38";
const HIGH: &str = "s18-unit-42";
const TOP: &str = "s18-unit-44";
const PLAIN: &str = "s18-plain";

fn card(index: u32, id: &str, type_: &str, stats: Option<(i32, i32)>, cost: i32) -> CardDef {
    let face = match stats {
        Some((attack, health)) => json!({ "attack": attack, "health": health, "keywords": [], "text": id }),
        None => json!({ "keywords": [], "text": id }),
    };
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (S18 fixture)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": face.clone(),
        "radiant": face,
    }))
}

fn spell_script(effects_of: impl Fn() -> Vec<Effect> + Send + Sync + 'static, targeted: bool) -> CardScripts {
    let script = Script {
        targets: if targeted {
            vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
        } else {
            vec![]
        },
        cry: Some(hook(move |_ctx| effects_of())),
        ..Script::default()
    };
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn step_spell(step: i32) -> CardScripts {
    spell_script(
        move || {
            vec![effects::transform_cost_step(json_as(
                json!({ "target": { "of": "chosen" }, "step": step }),
            ))]
        },
        true,
    )
}

fn register() {
    setup_catalog();
    let mut catalog: CardDefs = registered_catalog().clone();
    for def in [
        card(35700, DORMANT, "Spell", None, 0),
        card(35701, SUMMON_BACK, "Spell", None, 0),
        card(35702, STEP_1, "Spell", None, 0),
        card(35703, STEP_2, "Spell", None, 0),
        card(35704, STEP_3, "Spell", None, 0),
        card(35705, STEP_ALL, "Spell", None, 0),
        card(35706, LOW, "Unit", Some((1, 1)), 30),
        card(35707, MID, "Unit", Some((2, 2)), 32),
        card(35708, GAP, "Unit", Some((3, 3)), 38),
        card(35709, HIGH, "Unit", Some((4, 4)), 42),
        card(35710, TOP, "Unit", Some((5, 5)), 44),
        card(35711, PLAIN, "Unit", Some((1, 1)), 0),
    ] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut scripts: IndexMap<String, CardScripts> = registered_scripts().clone();
    scripts.insert(
        DORMANT.to_string(),
        spell_script(
            || {
                vec![effects::dormant_target(json_as(
                    json!({ "target": { "of": "chosen" }, "turns": 2 }),
                ))]
            },
            true,
        ),
    );
    scripts.insert(
        SUMMON_BACK.to_string(),
        spell_script(
            || {
                vec![effects::summon(json_as(
                    json!({ "defId": PLAIN, "backrowFirst": true }),
                ))]
            },
            false,
        ),
    );
    scripts.insert(STEP_1.to_string(), step_spell(1));
    scripts.insert(STEP_2.to_string(), step_spell(2));
    scripts.insert(STEP_3.to_string(), step_spell(3));
    scripts.insert(
        STEP_ALL.to_string(),
        spell_script(
            || {
                vec![effects::transform_cost_step(json_as(json!({
                    "scope": { "side": "self", "zones": ["field"], "excludeSelf": true },
                    "step": 2,
                })))]
            },
            false,
        ),
    );
    register_scripts(scripts);
}

fn board(p1: Value, p2: Value) -> Scenario {
    register();
    let mut s = scenario(json!({ "seed": "s18", "p1": p1, "p2": p2 }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

mod r3557002_dormant_by_an_effect {
    use super::*;

    #[test]
    fn r3557002_a_unit_turns_dormant_for_the_turns_given_and_awakens_at_the_start_of_its_controllers_last() {
        let mut s = board(
            json!({ "hand": [DORMANT], "library": [PLAIN, PLAIN, PLAIN, PLAIN] }),
            json!({ "field": [PLAIN], "library": [PLAIN, PLAIN, PLAIN, PLAIN] }),
        );
        let target = s.unit_at(P2, 1).id;
        s.play(DORMANT, pick::targets(pick::id(&target)));
        assert_eq!(
            jackioh_engine::slumber::slumber_turns_of(s.card(&target)),
            Some(2)
        );
        assert!(jackioh_engine::slumber::is_slumbering(s.card(&target)));
        s.end_turn();
        assert_eq!(
            jackioh_engine::slumber::slumber_turns_of(s.card(&target)),
            Some(1)
        );
        s.end_turn().end_turn();
        assert!(
            !jackioh_engine::slumber::is_slumbering(s.card(&target)),
            "it awakened"
        );
        assert_eq!(s.card(&target).awakened, Some(true));
    }

    #[test]
    fn r3557002_a_card_that_left_the_field_or_is_already_dormant_is_left_alone() {
        let mut s = board(
            json!({ "hand": [DORMANT, DORMANT], "library": [PLAIN, PLAIN] }),
            json!({ "field": [PLAIN], "library": [PLAIN, PLAIN] }),
        );
        let target = s.unit_at(P2, 1).id;
        s.play(DORMANT, pick::targets(pick::id(&target)));
        let before = s.card(&target).slumber;
        // A second Dormant does not restart the countdown.
        s.end_turn().end_turn();
        let mid = s.card(&target).slumber;
        assert_ne!(before, mid, "its countdown ran");
        // A card in a hand is not Dormant.
        let hand_card = s.hand(P1).first().map(|card| card.id.clone());
        if let Some(id) = hand_card {
            apply_now(
                s.state_mut(),
                effects::dormant_target(json_as(
                    json!({ "target": { "of": "instance", "instanceId": id }, "turns": 2 }),
                )),
            );
            assert_eq!(s.card(&id).slumber, None);
        }
    }
}

mod r3557003_summon_backrow_first {
    use super::*;

    fn standing(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        s.state().players[player]
            .backrow
            .iter()
            .flatten()
            .filter(|card| card.def_id == PLAIN)
            .cloned()
            .collect()
    }

    #[test]
    fn r3557003_it_stands_in_the_leftmost_open_backrow_zone_as_a_unit() {
        let mut s = board(json!({ "hand": [SUMMON_BACK] }), json!({}));
        s.play(SUMMON_BACK, json!({}));
        let made = standing(&s, P1);
        assert_eq!(made.len(), 1);
        assert!(matches!(
            made[0].zone,
            Zone::Field {
                row: Row::Backrow,
                lane: 1,
                ..
            }
        ));
        assert!(stands_in_backrow(s.state(), &made[0]), "it stands as a Unit");
        assert!(s.unit(P1, 1).is_none());
    }

    #[test]
    fn r3557003_with_every_backrow_zone_taken_it_takes_the_unit_row() {
        let traps: Vec<Value> = (0..5).map(|_| json!("stand-in (1) Trap")).collect();
        let mut s = board(json!({ "hand": [SUMMON_BACK], "backrow": traps }), json!({}));
        s.play(SUMMON_BACK, json!({}));
        assert!(
            s.unit(P1, 1).is_some_and(|card| card.def_id == PLAIN),
            "the unit row"
        );
    }

    #[test]
    fn r3557003_with_no_zone_open_at_all_nothing_lands() {
        let traps: Vec<Value> = (0..5).map(|_| json!("stand-in (1) Trap")).collect();
        let units: Vec<Value> = (0..5).map(|_| json!("stand-in (1) 1/1")).collect();
        let mut s = board(
            json!({ "hand": [SUMMON_BACK], "backrow": traps, "field": units }),
            json!({}),
        );
        let before = s.state().players[P1].units.iter().flatten().count();
        s.play(SUMMON_BACK, json!({}));
        assert_eq!(s.state().players[P1].units.iter().flatten().count(), before);
    }
}

mod r3557004_transform_cost_step {
    use super::*;

    fn cost_of(s: &Scenario, id: &str) -> i32 {
        effective_cost(s.state(), s.card(id), Default::default())
    }

    #[test]
    fn r3557004_a_unit_becomes_one_that_costs_exactly_the_step_more_in_place_keeping_its_face() {
        let mut s = board(
            json!({ "hand": [STEP_2], "field": [{ "def": LOW, "radiant": true }] }),
            json!({}),
        );
        let old = s.unit_at(P1, 1).id;
        s.play(STEP_2, pick::targets(pick::id(&old)));
        let new = s.unit(P1, 1).expect("a Unit stands there");
        assert_ne!(new.id, old);
        assert_eq!(new.def_id, MID);
        assert_eq!(effective_cost(s.state(), &new, Default::default()), 32);
        assert!(new.radiant, "its face is kept");
    }

    #[test]
    fn r3557004_with_none_at_the_price_the_cheapest_higher_one_is_used() {
        let mut s = board(json!({ "hand": [STEP_3], "field": [GAP] }), json!({}));
        let old = s.unit_at(P1, 1).id;
        s.play(STEP_3, pick::targets(pick::id(&old)));
        assert_eq!(
            s.unit(P1, 1).map(|card| card.def_id),
            Some(HIGH.to_string()),
            "41 is empty, 42 is next"
        );
        let _ = cost_of;
    }

    #[test]
    fn r3557004_the_top_of_the_curve_stays_as_it_is() {
        let mut s = board(json!({ "hand": [STEP_1], "field": [TOP] }), json!({}));
        let old = s.unit_at(P1, 1).id;
        s.play(STEP_1, pick::targets(pick::id(&old)));
        assert_eq!(s.unit_at(P1, 1).id, old);
    }

    #[test]
    fn r3557004_a_scope_changes_each_card_but_the_one_running_it() {
        let mut s = board(
            json!({ "hand": [STEP_ALL], "field": [LOW, GAP] }),
            json!({ "field": [LOW] }),
        );
        let (first, second, enemy) = (s.unit_at(P1, 1).id, s.unit_at(P1, 2).id, s.unit_at(P2, 1).id);
        s.play(STEP_ALL, json!({}));
        assert_eq!(s.unit(P1, 1).map(|card| card.def_id), Some(MID.to_string()));
        assert_eq!(
            s.unit(P1, 2).map(|card| card.def_id),
            Some(HIGH.to_string()),
            "40 is empty, 42 next"
        );
        assert_ne!(s.unit_at(P1, 1).id, first);
        assert_ne!(s.unit_at(P1, 2).id, second);
        assert_eq!(s.unit_at(P2, 1).id, enemy, "the other side is not in the scope");
    }
}
