//! Forced attacks and whose side their target is on, and when a run is over (SPEC §4.2, §6.3 Forced
//! attack, R53, R96, R173, R174). Found by the polish-4 edge-case hunt (docs/polish/4-edge-cases.md,
//! lenses L1 and L5); every case here failed before its fix.
//!
//!  - R173: a forced attack is made on an enemy. When #86 "Miss" Mrow dies to #9 Moths to the
//!    Flame's strike back and its Death steals Moths, the rest of the run is on Moths's new side and
//!    does not attack it; radiant #60 Bear Honeypot's tokens do not attack a played unit that #52
//!    Silly Silas has already rotated onto their own side.
//!  - R174: a run stops once its target has left the field, even when Reborn puts it straight back;
//!    and (round 4, lens L2) an attacker the run named that died in an earlier combat of the run and
//!    is back through Reborn is passed over, like one that is gone (R96).
//!
//! Port of `packages/cards/test/forced-attacks.test.ts` (SURFACE §4.1, §8).

use jackioh_engine::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

const VANILLA: &str = "core-008";
const MOTHS: &str = "core-009";
const STOCKPILE: &str = "core-005";
const TIMMY: &str = "core-011";
const HIT_JOB: &str = "core-016";
const SILAS: &str = "core-052";
const HONEYPOT: &str = "core-060";
const RIGHT_HOUSE: &str = "core-003";
const MROW: &str = "core-086";
const BIG_FELINOR: &str = "core-043";
const SURGERY: &str = "core-063";
const FIENDER: &str = "core-092";
const LIBRARY: [&str; 6] = [VANILLA; 6];

use super::scenario;

fn unit_at(g: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
    match g.unit(player, lane) {
        Some(card) => card.clone(),
        None => panic!("setup: {player} should hold a unit in lane {lane}"),
    }
}

/// One `attackDeclared` event's fields.
#[derive(Clone, Debug, PartialEq)]
struct Declared {
    attacker_id: String,
    target_id: String,
    forced: bool,
}

/// TS `declared(events)`: the `attackDeclared` events, in order.
fn declared(events: &[GameEvent]) -> Vec<Declared> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::AttackDeclared {
                attacker_id,
                target_id,
                forced,
            } => Some(Declared {
                attacker_id: attacker_id.clone(),
                target_id: target_id.clone(),
                forced: *forced,
            }),
            _ => None,
        })
        .collect()
}

mod r173_a_forced_attack_is_made_on_an_enemy {
    use super::*;

    #[test]
    fn r173_once_moths_to_the_flame_is_stolen_by_the_attackers_side_mid_run_the_rest_of_the_run_does_not_attack_it_8_9_86()
     {
        let mut g = scenario(json!({
            "seed": "hunt-cw-moths-mrow",
            "p1": { "hand": [VANILLA], "field": [{ "def": MOTHS, "lane": 3 }], "library": [TIMMY, TIMMY] },
            "p2": { "hand": [VANILLA], "field": [{ "def": MROW, "lane": 1 }, { "def": VANILLA, "lane": 2 }], "library": [TIMMY] },
        }));
        let moths = unit_at(&g, P1, 3);
        let mrow = unit_at(&g, P2, 1);
        let vanilla = unit_at(&g, P2, 2);

        // p1's start of turn: "every enemy unit attacks this". Mrow goes first and dies to the strike
        // back; its Death takes control of the unit that destroyed it — Moths — for p2 (R361).
        g.start_turn();
        g.expect_in_zone(&mrow.id, "graveyard");
        assert_eq!(g.card(&moths.id).controller, P2);

        // Moths is p2's own unit now, so p2's Mr. Vanilla is passed over in silence (R96's way).
        let attackers: Vec<String> = declared(g.events())
            .into_iter()
            .map(|event| event.attacker_id)
            .collect();
        assert_eq!(attackers, vec![mrow.id.clone()]);
        assert_eq!(g.card(&vanilla.id).damage, 0);
        g.expect_stats(&moths.id, json!({ "health": 13 }));
    }

    #[test]
    fn r173_radiant_bear_honeypots_tokens_do_not_attack_a_played_unit_that_has_crossed_to_their_own_side_8_60_52()
     {
        let mut g = scenario(json!({
            "p1": { "hand": [SILAS, VANILLA] },
            "p2": { "hand": [VANILLA], "backrow": [{ "def": HONEYPOT, "radiant": true, "lane": 3, "faceUp": false }] },
        }));

        // Silas enters p1's lane 5 and rotates right: he crosses to p2's lane 5 and is p2's (§3.1).
        let silas = g.card(SILAS).clone();
        g.play(&silas.id, json!({ "zone": 5, "modes": ["right"] }));
        assert!(g.events().contains(&GameEvent::ControlChanged {
            instance_id: silas.id.clone(),
            controller: P2,
            row: Row::Units,
            lane: 5,
            former_id: None,
        }));

        // p2's trap answers p1's play (radiant: any card) and fills p2's board with Rush Tokens. "If it
        // was a Unit, they attack it" — but the unit is on their own side now.
        assert!(
            g.events()
                .iter()
                .any(|event| matches!(event, GameEvent::TrapFired { .. }))
        );
        let at_silas: Vec<Declared> = declared(g.events())
            .into_iter()
            .filter(|event| event.target_id == silas.id)
            .collect();
        assert_eq!(at_silas, Vec::<Declared>::new());
        g.expect_in_zone(&silas.id, "field");
        assert_eq!(g.card(&silas.id).damage, 0);
    }
}

mod r174_a_forced_run_and_a_target_that_left_the_field {
    use super::*;

    #[test]
    fn r174_r53_r83_a_forced_run_stops_once_its_target_has_left_the_field_even_though_reborn_brings_it_back()
    {
        // p1's Radiant Bear Honeypot answers p2's 1-cost Right-house defender (1/1, Taunt, Divine
        // Shield, Reborn): five Rush Tokens attack it. The first spends its shield, the second kills
        // it; Reborn returns it at 1 health, and the other three tokens do not attack the body that
        // came back.
        let mut g = scenario(json!({
            "seed": "hunt-cw-reborn-run",
            "active": "p2",
            "p1": { "backrow": [{ "def": HONEYPOT, "radiant": true, "faceUp": false, "lane": 3 }] },
            "p2": { "hand": [RIGHT_HOUSE, STOCKPILE], "library": [TIMMY, HIT_JOB] },
        }));

        g.play(RIGHT_HOUSE, json!({ "zone": 1 }));

        assert_eq!(
            declared(g.events()).iter().filter(|event| event.forced).count(),
            2
        );
        assert_eq!(
            g.events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Destroyed { .. }))
                .count(),
            1
        );
        let defender = unit_at(&g, P2, 1);
        assert_eq!(defender.def_id, RIGHT_HOUSE);
        g.expect_stats(&defender.id, json!({ "health": 1 }));
    }

    #[test]
    fn r174_r96_r83_a_felinor_fiender_that_dies_mid_run_to_moths_to_the_flame_and_comes_back_through_reborn_does_not_attack_in_that_run()
     {
        // On this seed Plastic Surgery's random keyword for the Fiender is Reborn (§6.1's pool, R21).
        let mut g = scenario(json!({
            "seed": "r4-fiender-reborn-33", // R346's Pierce moved the roll off "-7", R49's Deft off "-8"
            "active": "p2",
            "turn": 10,
            "p1": { "field": [{ "def": MOTHS, "lane": 1 }], "hand": [STOCKPILE], "library": LIBRARY },
            "p2": {
                "hand": [SURGERY, STOCKPILE],
                // Big Felinor feeds the Fiender's layer 2 (§10.4) and dies to Moths' strike back; the
                // Fiender's damage is then more than it has without it.
                "field": [
                    { "def": BIG_FELINOR, "lane": 1, "damage": 9 },
                    { "def": FIENDER, "lane": 2, "damage": 12 },
                ],
                "library": LIBRARY,
            },
        }));
        let fiender = unit_at(&g, P2, 2);
        g.play(
            SURGERY,
            json!({ "targets": [{ "pick": "instance", "instanceId": fiender.id }] }),
        );
        let kinds: Vec<KeywordKind> = g.stats(&fiender.id).keywords.iter().map(Keyword::kind).collect();
        assert!(kinds.contains(&KeywordKind::Reborn));

        // p1's start of turn: every p2 unit attacks Moths, in lane order (R53). Big Felinor dies in
        // its combat, the Fiender dies in the check after it and comes back at 1 health (§4.5 step 4).
        g.end_turn();
        let back = g.events().iter().position(
            |event| matches!(event, GameEvent::Summoned { instance_id, .. } if *instance_id == fiender.id),
        );
        let Some(back) = back else {
            panic!("the Fiender should come back through Reborn (a summoned event)");
        };

        // The body that came back is a new arrival (R83, R174): the run named the unit that died, so
        // the body is passed over like any attacker that is gone (R96), and stands at 1 health.
        let by_fiender: Vec<Declared> = declared(&g.events()[back..])
            .into_iter()
            .filter(|event| event.attacker_id == fiender.id)
            .collect();
        assert_eq!(by_fiender, Vec::<Declared>::new());
        g.expect_in_zone(&fiender.id, "field");
        assert_eq!(g.card(&fiender.id).reborn_spent, Some(true));
        g.expect_stats(&fiender.id, json!({ "health": 1 }));
    }
}
