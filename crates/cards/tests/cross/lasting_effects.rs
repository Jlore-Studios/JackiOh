//! A permanent's lasting effect lasts while it is on the field (SPEC §5.1, §5.2, R30, R169, R209).
//! Found by the polish-4 edge-case hunt, rounds 2 and 4 (docs/polish/4-edge-cases.md, lenses L1, L2
//! and L8); every case here failed before its fix.
//!
//! #79 Twinspell's "the next Spell you play gains Echo +1" is a player modifier, which the engine used
//! to end only by consuming it. So a Twinspell that left the field — bounced by radiant #52, destroyed
//! by #36 — kept echoing its old controller's next Spell, and one bounced and replayed stacked a
//! second rider. And the rider's amount was fixed by the face that installed it, so a Twinspell
//! radiant #49 stole and made Radiant still granted +1. Round 4: the rider was installed by a Cry,
//! which #79 does not print (R169), so a Twinspell summoned onto the field (§6.2: no Cry) granted
//! nothing, and neither did Twinspell's text fused by #85 onto the other player's Field Spell.
//!
//! Port of `packages/cards/test/lasting-effects.test.ts` (SURFACE §4.1, §8).

use jackioh_engine::subsystems::fuse::{FuseArgs, fuse};
use jackioh_engine::testkit::*;

const STOCKPILE: &str = "core-005";
const VANILLA: &str = "core-008";
const MAGIC_JAMMED: &str = "core-036";
const MIND_CONTROL: &str = "core-049";
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

mod r209_twinspells_grant_ends_when_twinspell_leaves_the_field {
    use super::*;

    #[test]
    fn r209_r30_a_twinspell_radiant_silly_silas_bounces_to_its_controllers_hand_takes_its_grant_with_it_so_the_next_spell_resolves_once()
     {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": {
                "hand": [TWINSPELL, { "def": SILAS, "radiant": true }, STOCKPILE, VANILLA],
                "mana": 10,
                "library": LIBRARY,
            },
            "p2": { "hand": [VANILLA], "field": [{ "def": VANILLA, "lane": 3 }], "library": LIBRARY },
        }));
        let twin = g.card(TWINSPELL).clone();
        g.play(TWINSPELL, json!({ "zone": 5 }));
        assert_eq!(echo_riders(&g, PlayerId::P1).len(), 1);

        // Backrow ring, rotating right from p1's seat: p1's lane 5 would cross to p2's lane 5, so the
        // radiant face bounces Twinspell to its controller's hand costing 0 instead (§8 #52, R14, R747).
        g.play(SILAS, json!({ "zone": 1, "modes": ["right"] }));
        g.expect_in_zone(&twin, "hand");

        // Twinspell is no longer on the field, so "the next Spell you play gains Echo +1" is no longer
        // anybody's: the rider is gone and Stockpile resolves once (draw 2).
        assert!(echo_riders(&g, PlayerId::P1).is_empty());
        let before = g.hand("p1").len();
        g.play(STOCKPILE, json!({}));
        assert_eq!(stockpile_resolutions(g.last_events()), 1);
        assert_eq!(g.hand("p1").len(), before - 1 + 2);
    }

    #[test]
    fn r209_r30_r174_a_twinspell_bounced_by_radiant_silly_silas_and_replayed_grants_echo_1_once_not_twice() {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": {
                "hand": [TWINSPELL, { "def": SILAS, "radiant": true }, STOCKPILE, VANILLA],
                "mana": 10,
                "library": LIBRARY,
            },
            "p2": { "hand": [VANILLA], "field": [{ "def": VANILLA, "lane": 3 }], "library": LIBRARY },
        }));
        let twin = g.card(TWINSPELL).clone();
        g.play(TWINSPELL, json!({ "zone": 5 }));
        g.play(SILAS, json!({ "zone": 1, "modes": ["right"] }));
        g.expect_in_zone(&twin, "hand");

        // Replayed (costing 0): standing on the field again, it has the one grant this Twinspell now has.
        g.play(&twin, json!({ "zone": 4 }));
        assert_eq!(echo_riders(&g, PlayerId::P1).len(), 1);

        let before = g.hand("p1").len();
        g.play(STOCKPILE, json!({}));
        // One Twinspell, Echo +1: two resolutions, four cards.
        assert_eq!(stockpile_resolutions(g.last_events()), 2);
        assert_eq!(g.hand("p1").len(), before - 1 + 4);
        g.expect_in_zone(&twin, "graveyard");
    }

    #[test]
    fn r209_r30_a_twinspell_the_opponent_destroys_with_magic_jammed_leaves_no_grant_behind_so_the_next_spell_gains_no_echo()
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
        g.end_turn();
        assert_eq!(g.state().active, PlayerId::P2);

        g.play(
            MAGIC_JAMMED,
            json!({ "targets": [{ "pick": "instance", "instanceId": twin.id }] }),
        );
        g.expect_in_zone(&twin, "graveyard");
        assert!(echo_riders(&g, PlayerId::P1).is_empty());

        g.end_turn();
        assert_eq!(g.state().active, PlayerId::P1);
        let before = g.hand("p1").len();
        g.play(STOCKPILE, json!({}));
        // One resolution of "draw 2" (the Stockpile itself left the hand).
        assert_eq!(stockpile_resolutions(g.last_events()), 1);
        assert_eq!(g.hand("p1").len(), before - 1 + 2);
    }
}

mod r209_twinspells_grant_follows_its_current_face {
    use super::*;

    #[test]
    fn r209_a_twinspell_stolen_by_radiant_snom_bunny_mind_control_grants_echo_2_to_the_thiefs_next_spell() {
        jackioh_cards::register_all();
        let library = [LIBRARY, LIBRARY].concat();
        let mut g = scenario(json!({
            "active": "p2",
            "turn": 8,
            "p1": {
                "hand": [{ "def": MIND_CONTROL, "radiant": true }, STOCKPILE, VANILLA, "core-t-coin"],
                "mana": 10,
                "library": library,
            },
            "p2": { "hand": [TWINSPELL, VANILLA], "field": [{ "def": VANILLA, "lane": 3 }], "library": LIBRARY },
        }));
        let twin = g.card(TWINSPELL).clone();
        g.play(TWINSPELL, json!({ "zone": 2 }));
        g.end_turn();
        assert_eq!(g.state().active, PlayerId::P1);
        // #49 costs (4) since patch v0.2.0 (issue #40): The Coin, played before Twinspell is p1's, pays
        // for Stockpile after it.
        g.play("core-t-coin", json!({}));

        // "Steal target enemy permanent; it also becomes Radiant". Twinspell is p1's now, and Radiant
        // on the field, where §5.2 has its text be the radiant one from then on: "Echo +2".
        g.play(
            MIND_CONTROL,
            json!({ "targets": [{ "pick": "instance", "instanceId": twin.id }] }),
        );
        assert_eq!(g.card(&twin).controller, PlayerId::P1);
        assert!(g.card(&twin).radiant);
        // R169's badge says so too.
        assert!(
            g.view("p1")
                .you
                .modifiers
                .iter()
                .any(|modifier| modifier.label == "Next Spell gains Echo +2")
        );

        // Stockpile (draw 2, heal 2) resolves 1 + 2 = 3 times: six cards for the one played.
        let before = g.hand("p1").len();
        g.play(STOCKPILE, json!({}));
        assert_eq!(stockpile_resolutions(g.last_events()), 3);
        assert_eq!(g.hand("p1").len(), before - 1 + 6);
        g.expect_in_zone(&twin, "graveyard");
    }
}

mod r209_twinspells_grant_is_the_permanents_however_it_came_to_stand_on_the_field {
    use super::*;

    #[test]
    fn r209_r41_r169_twinspells_summoned_by_22s_death_grant_their_echo_like_played_ones_so_the_next_spell_resolves_three_times()
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
        // and all, into p1's unit row (R41, R64). A summon fires no Cry (§6.2).
        g.play(
            HIT_JOB,
            json!({ "targets": [{ "pick": "instance", "instanceId": cube.id }] }),
        );
        let copies = g
            .last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Summoned { def_id, .. } if *def_id == eaten.def_id))
            .count();
        assert_eq!(copies, 2);
        assert_eq!(echo_riders(&g, PlayerId::P1).len(), 2);

        // Each copy standing on p1's side says "the next Spell you play gains Echo +1", and that text is
        // no Cry: Stockpile gains Echo +2 and resolves three times.
        g.play(STOCKPILE, json!({}));
        assert_eq!(stockpile_resolutions(g.last_events()), 3);
    }

    #[test]
    fn r209_r77_twinspell_fused_by_85_onto_the_trap_controllers_field_spell_grants_its_echo_to_that_controller()
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

        // "The next Spell you play gains Echo +1" now stands on p1's side, and "you" is its controller
        // (§8 Conventions): p1's next Spell, Stockpile, resolves twice.
        g.end_turn();
        assert_eq!(g.state().active, PlayerId::P1);
        g.play(STOCKPILE, json!({}));
        assert_eq!(stockpile_resolutions(g.last_events()), 2);
    }
}
