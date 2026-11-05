//! What became of #79 Twinspell's lasting effect (SPEC §5.1, §5.2, R209, R384, R3044000).
//! Found by the polish-4 edge-case hunt, rounds 2 and 4 (docs/polish/4-edge-cases.md, lenses L1, L2, L8).
//! Through v0.5.11p Twinspell's "the next Spell you play gains Echo +1" was a player modifier the
//! permanent installed standing on the field, which ended when it left (R209). Since #3044 it is an
//! Activate, once a turn (R384), whose grant is its player's for that turn, a rider with no source
//! (R3044000): it outlives the Twinspell that gave it (bounced by radiant #52), and Twinspell standing on
//! the field gives nothing until it is activated, by whoever controls it then (stolen by radiant #97.4,
//! whose Radiant face then stays), however it came to stand there (#22's summoned copies of a Unit
//! carrying its text, Twinspell's text fused by #85 onto the other player's Field Spell).
//! The engine's lasting grant that R209 describes is proved in `crates/engine/tests/rules/rulings_a.rs`.
//! Test paths: SURFACE §4.1, §8.

use jackioh_engine::subsystems::fuse::{FuseArgs, fuse};
use jackioh_engine::testkit::*;

const STOCKPILE: &str = "core-005";
const VANILLA: &str = "core-008";
const MAGIC_JAMMED: &str = "core-036";
/// Meditative #97.4 Mega Church, a Unit token: "Activate: Steal an enemy permanent. [Make it Radiant.]"
const CHURCH: &str = "meditative-097-4";
const SILAS: &str = "core-052";
const TWINSPELL: &str = "core-079";
const MANA_WELL: &str = "core-006";
const HIT_JOB: &str = "core-016";
const CUBE: &str = "core-022";
const UNLICENSED: &str = "core-085";
const LIBRARY: [&str; 8] = [
    VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA,
];

fn echo_riders(g: &Scenario, player: PlayerId) -> Vec<PlayerModifier> {
    g.state().players[player]
        .mods
        .iter()
        .filter(|modifier| matches!(modifier.kind, ModifierKind::EchoNextSpell { .. }))
        .cloned()
        .collect()
}

/// Stockpile heals its hero once per resolution (§8 #5), so its `healed` events count them.
fn stockpile_resolutions(events: &[GameEvent]) -> usize {
    events
        .iter()
        .filter(|event| event.event_type() == GameEventType::Healed)
        .count()
}

/// R428: p1's Unit in `unit_lane` fused with the backrow card in `backrow_lane`, carrying its text (R77).
fn unit_carrying(g: &mut Scenario, unit_lane: i32, backrow_lane: i32) -> CardInstance {
    let (Some(unit), Some(carried)) = (g.unit("p1", unit_lane), g.backrow("p1", backrow_lane)) else {
        panic!("setup: the Unit and the card it carries");
    };
    let mut rng = create_rng(&g.state().seed, g.state().rng_cursor);
    let mut events: Vec<GameEvent> = Vec::new();
    let mut sink = EngineSink::new(g.state_mut(), &mut events, &mut rng);
    let fused = fuse(
        &mut sink,
        FuseArgs {
            ingredients: vec![unit.clone(), carried],
            target: Some(unit),
            ..Default::default()
        },
    );
    fused.expect("setup: the fusion")
}

mod r209_r3044000_twinspells_grant_is_its_players_and_outlives_twinspell {
    use super::*;

    #[test]
    fn r209_r3044000_a_radiant_twinspell_bounced_by_radiant_silly_silas_leaves_its_grant_behind_so_the_next_spell_still_echoes()
     {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": {
                "hand": [{ "def": TWINSPELL, "radiant": true }, { "def": SILAS, "radiant": true }, STOCKPILE, VANILLA],
                "mana": 10,
                "library": LIBRARY,
            },
            "p2": { "hand": [VANILLA], "field": [{ "def": VANILLA, "lane": 3 }], "library": LIBRARY },
        }));
        let twin = g.card(TWINSPELL).clone();
        g.play(TWINSPELL, json!({ "zone": 5 }));
        assert!(echo_riders(&g, PlayerId::P1).is_empty());
        g.activate(TWINSPELL, json!({}));
        assert_eq!(echo_riders(&g, PlayerId::P1).len(), 1);

        // Backrow ring, rotating right from p1's seat: p1's lane 5 would cross to p2's lane 5, so the
        // radiant face bounces Twinspell to its controller's hand costing 0 instead (§8 #52, R14, R747).
        g.play(SILAS, json!({ "zone": 1, "modes": ["right"] }));
        g.expect_in_zone(&twin, "hand");

        // No lasting effect ends with it: the grant is p1's (R3044000).
        assert_eq!(echo_riders(&g, PlayerId::P1).len(), 1);
        let before = g.hand("p1").len();
        g.play(STOCKPILE, json!({}));
        assert_eq!(stockpile_resolutions(g.last_events()), 2);
        assert_eq!(g.hand("p1").len(), before - 1 + 4);
    }

    #[test]
    fn r3044000_r78_r384_a_radiant_twinspell_bounced_and_replayed_activates_afresh_so_the_next_spell_gains_both_grants()
     {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": {
                "hand": [{ "def": TWINSPELL, "radiant": true }, { "def": SILAS, "radiant": true }, STOCKPILE, VANILLA],
                "mana": 10,
                "library": LIBRARY,
            },
            "p2": { "hand": [VANILLA], "field": [{ "def": VANILLA, "lane": 3 }], "library": LIBRARY },
        }));
        let twin = g.card(TWINSPELL).clone();
        g.play(TWINSPELL, json!({ "zone": 5 }));
        g.activate(TWINSPELL, json!({}));
        g.play(SILAS, json!({ "zone": 1, "modes": ["right"] }));
        g.expect_in_zone(&twin, "hand");

        // Leaving the field reset its uses (R78, R384), so the replayed Twinspell may be activated again,
        // and each grant gives its own Echo +1.
        g.play(&twin, json!({ "zone": 4 }));
        g.activate(&twin, json!({}));
        assert_eq!(echo_riders(&g, PlayerId::P1).len(), 2);

        let before = g.hand("p1").len();
        g.play(STOCKPILE, json!({}));
        assert_eq!(stockpile_resolutions(g.last_events()), 3);
        assert_eq!(g.hand("p1").len(), before - 1 + 6);
        assert!(echo_riders(&g, PlayerId::P1).is_empty());
        g.expect_in_zone(&twin, "field");
    }

    #[test]
    fn r209_r3044000_a_twinspell_the_opponent_destroys_with_magic_jammed_before_it_was_activated_gave_nothing_so_the_next_spell_gains_no_echo()
     {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "active": "p2",
            "turn": 8,
            "p1": { "hand": [TWINSPELL, STOCKPILE, VANILLA], "mana": 10, "library": LIBRARY },
            "p2": { "hand": [MAGIC_JAMMED, VANILLA], "field": [{ "def": VANILLA, "lane": 3 }], "library": LIBRARY },
        }));
        let twin = g.card(TWINSPELL).clone();
        g.end_turn();
        assert_eq!(g.state().active, PlayerId::P1);
        g.play(TWINSPELL, json!({ "zone": 2 }));
        // Standing on the field it installs nothing.
        assert!(echo_riders(&g, PlayerId::P1).is_empty());
        g.end_turn();
        assert_eq!(g.state().active, PlayerId::P2);

        g.play(
            MAGIC_JAMMED,
            json!({ "targets": [{ "pick": "instance", "instanceId": twin.id }] }),
        );
        g.expect_in_zone(&twin, "graveyard");

        g.end_turn();
        assert_eq!(g.state().active, PlayerId::P1);
        let before = g.hand("p1").len();
        g.play(STOCKPILE, json!({}));
        assert_eq!(stockpile_resolutions(g.last_events()), 1);
        assert_eq!(g.hand("p1").len(), before - 1 + 2);
    }
}

mod r3044000_a_twinspell_is_activated_by_its_controller_on_the_face_it_wears {
    use super::*;

    #[test]
    fn r3044000_r384_a_twinspell_stolen_by_a_radiant_mega_church_is_the_thiefs_to_activate_and_its_radiant_face_stays()
     {
        jackioh_cards::register_all();
        let library = [LIBRARY, LIBRARY].concat();
        let mut g = scenario(json!({
            "active": "p2",
            "turn": 8,
            "p1": {
                "hand": [STOCKPILE, VANILLA],
                "field": [{ "def": CHURCH, "lane": 1, "radiant": true }],
                "mana": 10,
                "library": library,
            },
            "p2": { "hand": [TWINSPELL, VANILLA], "field": [{ "def": VANILLA, "lane": 3 }], "library": LIBRARY },
        }));
        let twin = g.card(TWINSPELL).clone();
        g.play(TWINSPELL, json!({ "zone": 2 }));
        g.end_turn();
        assert_eq!(g.state().active, PlayerId::P1);

        // Twinspell is p1's now, and Radiant on the field, where §5.2 has its text be the radiant one
        // from then on: its Activate leaves it on the field.
        g.activate(
            CHURCH,
            json!({ "targets": [{ "pick": "instance", "instanceId": twin.id }] }),
        );
        assert_eq!(g.card(&twin).controller, PlayerId::P1);
        assert!(g.card(&twin).radiant);
        g.activate(&twin, json!({}));
        assert!(echo_riders(&g, PlayerId::P2).is_empty());
        // R169's badge says so.
        assert!(
            g.view("p1")
                .you
                .modifiers
                .iter()
                .any(|modifier| modifier.label == "Next Spell gains Echo +1")
        );

        let before = g.hand("p1").len();
        g.play(STOCKPILE, json!({}));
        assert_eq!(stockpile_resolutions(g.last_events()), 2);
        assert_eq!(g.hand("p1").len(), before - 1 + 4);
        g.expect_in_zone(&twin, "field");
    }
}

mod r3044000_twinspells_activate_is_the_cards_however_it_came_to_stand_on_the_field {
    use super::*;

    #[test]
    fn r3044000_r41_twinspells_text_on_units_22_summons_is_activated_on_each_so_the_next_spell_resolves_three_times()
     {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": {
                "hand": [TWINSPELL, CUBE, HIT_JOB, STOCKPILE, VANILLA],
                "field": [{ "def": VANILLA, "lane": 2 }],
                "mana": 10,
                "library": LIBRARY,
            },
            "p2": { "hand": [VANILLA], "field": [{ "def": VANILLA, "lane": 3 }], "library": LIBRARY },
        }));
        g.play(TWINSPELL, json!({ "zone": 1 }));
        // R428: the Cube eats Units only, so its meal is a Mr. Vanilla carrying Twinspell's text (R77).
        let eaten = unit_carrying(&mut g, 2, 1);
        g.play(
            CUBE,
            json!({ "zone": 1, "targets": [{ "pick": "instance", "instanceId": eaten.id }] }),
        );
        let cube = g.unit("p1", 1).expect("setup: p1's Cube");
        g.expect_in_zone(&eaten, "graveyard");

        // Hit Job kills the Cube, whose Death summons two copies of the Unit it ate, Twinspell's text
        // and all, into p1's unit row (R41, R64). Neither installs anything standing there.
        g.play(
            HIT_JOB,
            json!({ "targets": [{ "pick": "instance", "instanceId": cube.id }] }),
        );
        let copies: Vec<String> = g
            .last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned {
                    instance_id, def_id, ..
                } if *def_id == eaten.def_id => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(copies.len(), 2);
        assert!(echo_riders(&g, PlayerId::P1).is_empty());

        // Each copy's Activate gives Echo +1, then sends that copy to the graveyard: a plain move, no
        // death (R3044000).
        for copy in &copies {
            g.activate(copy.as_str(), json!({}));
            g.expect_in_zone(copy.as_str(), "graveyard");
        }
        assert_eq!(echo_riders(&g, PlayerId::P1).len(), 2);
        assert!(
            g.last_events()
                .iter()
                .all(|event| event.event_type() != GameEventType::Destroyed)
        );

        g.play(STOCKPILE, json!({}));
        assert_eq!(stockpile_resolutions(g.last_events()), 3);
    }

    #[test]
    fn r3044000_r77_twinspell_fused_by_85_onto_the_trap_controllers_field_spell_is_that_controllers_to_activate()
     {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "active": "p2",
            "p1": {
                "backrow": [{ "def": UNLICENSED, "lane": 3 }, { "def": MANA_WELL, "lane": 1 }],
                "hand": [STOCKPILE, VANILLA],
                "library": LIBRARY,
            },
            "p2": { "hand": [TWINSPELL, VANILLA], "mana": 10, "library": LIBRARY },
        }));
        let well = g.backrow("p1", 1).expect("setup: p1's Mana Well");

        // p2 plays Twinspell; after its arrival (R61) p1's #85 fuses it onto p1's Mana Well, which keeps
        // its instance on p1's side and now carries Twinspell's text as well (R77).
        g.play(TWINSPELL, json!({ "zone": 2 }));
        assert!(matches!(
            g.card(&well).zone,
            Zone::Field {
                player: PlayerId::P1,
                ..
            }
        ));
        assert_ne!(g.card(&well).def_id, MANA_WELL);
        assert!(echo_riders(&g, PlayerId::P2).is_empty());

        // "Activate: The next Spell you play this turn gains Echo +1. Then this goes to the graveyard."
        // now stands on p1's side, and "you" is its controller (§8 Conventions): p1 activates it, the
        // fused Mana Well goes to the graveyard, and p1's next Spell, Stockpile, resolves twice.
        g.end_turn();
        assert_eq!(g.state().active, PlayerId::P1);
        g.activate(&well, json!({}));
        g.expect_in_zone(&well, "graveyard");
        g.play(STOCKPILE, json!({}));
        assert_eq!(stockpile_resolutions(g.last_events()), 2);
    }
}
