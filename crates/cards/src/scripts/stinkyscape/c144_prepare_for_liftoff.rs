//! S #144 Prepare For Liftoff (SPEC §8.10 row 144; R1, R17, R64, R1838000, R1852000, R1852002,
//! R1852006, R1859012): (1) Trap, Common, Tradable, Flashback, a card of the Stinkyscape set.
//!
//! Base:    "Tradable, Flashback\nReveals when one of your Starship Pieces dies: Your Starship launch
//!          costs (0)."
//! Radiant: "Tradable, Flashback\nReveals when one of your Starship Pieces dies: Summon it from your
//!          graveyard. Your Starship launch costs (0)."
//! Engine: Tradable (R1838000) and Flashback (§6.1, set again from its owner's graveyard once) are
//! catalog keywords. Set face-down, it reveals (R17) when "one of your Starship Pieces" dies: a piece
//! whose copy fuses into their Dormant Starship (`starshipFused` for them, R1852000, R1852002), one that
//! died on their side, into their graveyard, a stolen one included (R1859012, R2618000). Then
//! `starship_launch_costs { cost: 0 }` (R1852006): their next launch by the Activate is free. The
//! Radiant face first summons that piece out of its owner's graveyard into the leftmost open unit zone
//! (R64), a summon with no Cry (R1), its copy staying fused in the Starship. Declares no numbers: (0)
//! is the verb's price.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-144";

/// R1852006's "(0)": what the next launch costs.
const LAUNCH_COST: i32 = 0;

/// The piece whose copy `event` fuses into the controller's Starship, by id: one of their Starship
/// Pieces has died (R1859012).
fn fused_piece(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<String> {
    match event {
        GameEvent::StarshipFused { player, piece_id, .. } if *player == ctx.controller => {
            Some(piece_id.clone())
        }
        _ => None,
    }
}

fn liftoff(radiant: bool) -> Script {
    let reveal = TriggerDef::new("prepare-for-liftoff", &[GameEventType::StarshipFused], move |ctx, event| {
        let Some(piece) = fused_piece(ctx, event) else {
            return vec![];
        };
        let mut effects = Vec::new();
        let in_graveyard = find_instance(&*ctx.state, &piece)
            .is_some_and(|card| card.zone.z() == ZoneName::Graveyard && card.owner == ctx.controller);
        if radiant && in_graveyard {
            effects.push(summon(json_as(json!({
                "instance": { "of": "instance", "instanceId": piece },
            }))));
        }
        effects.push(starship_launch_costs(StarshipLaunchCostArgs { cost: LAUNCH_COST }));
        effects
    })
    .with_when(|ctx, event| fused_piece(ctx, event).is_some());
    Script {
        triggers: vec![reveal],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: liftoff(false),
        radiant: liftoff(true),
    }
}

// S #144 Prepare For Liftoff — SPEC §8.10 row 144, BUILD M12 row S 144 (R1838000, R1852006,
// R1859012).
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::launch_starship;
    use jackioh_engine::prelude::{STARSHIP_LAUNCH_COST, dormant_starship_of, launch_cost, starship_pieces_of};
    use jackioh_engine::testkit::*;
    use jackioh_engine::{is_tradable, trades_left};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const AP_ROUNDS: &str = "stinkyscape-100"; // (1) 3/1 Starship Piece, Equip, Pierce: keywords only.
    const HIT_JOB: &str = "core-016"; // (3) Spell: destroy a Unit.
    const FEELINGS: &str = "classic-032"; // (2) Spell: steal an enemy Unit in a lane with your (1) Unit.
    const COMMANDER: &str = "stinkyscape-137"; // (4) Unit, Cry: launch your Dormant Starship.

    /// p1's Prepare For Liftoff set (Radiant if `radiant`) in backrow lane 1, and an AP Rounds of p1's in
    /// lane 2 and of p2's in lane 1, p1 holding two Hit Jobs.
    fn setup(seed: &str, radiant: bool) -> Scenario {
        board(seed, radiant, "p1", json!({}), json!({}))
    }

    fn board(seed: &str, radiant: bool, active: &str, p1: Value, p2: Value) -> Scenario {
        let mut s = crate::scenario(json!({
            "seed": seed,
            "active": active,
            "p1": crate::merged(json!({
                "hand": [HIT_JOB, HIT_JOB],
                "field": [{ "def": AP_ROUNDS, "lane": 2 }],
                "backrow": [{ "def": ID, "lane": 1, "faceUp": false, "radiant": radiant }],
                "mana": 10,
                "library": crate::filler(4),
            }), p1),
            "p2": crate::merged(json!({
                "hand": crate::filler(1),
                "field": [{ "def": AP_ROUNDS, "lane": 1 }],
                "mana": 10,
                "library": crate::filler(4),
            }), p2),
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    /// `player` steals the enemy Unit in `lane` with Felinor Feelings, their own AP Rounds, a (1) Unit,
    /// standing in that lane: the stolen card's id.
    fn steal(s: &mut Scenario, player: PlayerId, lane: i32) -> String {
        let id = s.unit_at(opponent_of(player), lane).id;
        s.play(FEELINGS, json!({ "targets": pick::id(&id) }));
        assert_eq!(s.card(id.as_str()).controller, player, "stolen");
        id
    }

    fn set_from_graveyard(s: &mut Scenario) {
        let trap = s.card(ID).id.clone();
        s.act(
            ActionBody::Play {
                instance_id: trap,
                zone: Some(ZoneChoice {
                    row: Row::Backrow,
                    lane: 1,
                }),
                x: None,
                embiggen: None,
                equip: None,
                tributes: None,
                targets: None,
                modes: None,
                plague: None,
                face_down: None,
            },
            P1,
            "set Prepare For Liftoff from the graveyard",
        );
    }

    fn offered(s: &Scenario) -> bool {
        let trap = s.card(ID).id.clone();
        legal_actions(s.state(), P1)
            .iter()
            .any(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == trap))
    }

    fn hit(s: &mut Scenario, player: PlayerId, lane: i32) -> String {
        let id = s.unit_at(player, lane).id;
        s.play(HIT_JOB, json!({ "targets": pick::id(&id) }));
        id
    }

    fn trap_fired(s: &Scenario) -> bool {
        s.last_events()
            .iter()
            .any(|event| event.event_type() == GameEventType::TrapFired)
    }

    #[test]
    fn r1859012_its_controllers_piece_dying_into_their_starship_makes_their_next_launch_free() {
        let mut s = setup("r1859012-base", false);
        assert_eq!(launch_cost(s.state(), P1), STARSHIP_LAUNCH_COST);
        let piece = hit(&mut s, P1, 2);
        assert!(trap_fired(&s));
        s.expect_in_zone(ID, "graveyard");
        assert_eq!(launch_cost(s.state(), P1), 0);
        s.expect_in_zone(piece.as_str(), "graveyard");
        let ship = dormant_starship_of(s.state(), P1).expect("p1's Starship").id.clone();
        s.state_mut().players.p1.mana.current = 0;
        launch_starship(&mut s, &ship);
        assert!(s.card(ship.as_str()).starship.as_ref().is_some_and(|memory| memory.launched.is_some()));
        assert_eq!(launch_cost(s.state(), P1), STARSHIP_LAUNCH_COST, "the launch spent it");
    }

    #[test]
    fn r1859012_an_enemy_piece_dying_leaves_it_set() {
        let mut s = setup("r1859012-enemy", false);
        hit(&mut s, P2, 1);
        assert!(dormant_starship_of(s.state(), P2).is_some(), "p2's piece fused into p2's Starship");
        assert!(!trap_fired(&s));
        let trap = s.card(ID);
        assert!(matches!(trap.zone, Zone::Field { row: Row::Backrow, .. }) && trap.face_up != Some(true));
        assert_eq!(launch_cost(s.state(), P1), STARSHIP_LAUNCH_COST);
    }

    #[test]
    fn r1859012_radiant_the_piece_comes_back_from_the_graveyard_its_copy_staying_fused() {
        let mut s = setup("r1859012-radiant", true);
        let piece = hit(&mut s, P1, 2);
        assert!(trap_fired(&s));
        let back = s.card(piece.as_str()).clone();
        assert!(matches!(back.zone, Zone::Field { player: P1, row: Row::Units, .. }), "{:?}", back.zone);
        let ship = dormant_starship_of(s.state(), P1).expect("p1's Starship");
        assert_eq!(starship_pieces_of(ship).len(), 1);
        assert_eq!(launch_cost(s.state(), P1), 0);
        let Zone::Field { lane, .. } = back.zone else {
            panic!("the piece is on the field");
        };
        hit(&mut s, P1, lane);
        let ship = dormant_starship_of(s.state(), P1).expect("p1's Starship");
        assert_eq!(starship_pieces_of(ship).len(), 2);
    }

    #[test]
    fn r1859012_r2618000_a_piece_it_owns_dying_under_the_enemy_leaves_it_set() {
        // p2 steals p1's AP Rounds and destroys it: it dies under p2's control into p2's graveyard as
        // p2's own card (R2618000), and its copy fuses into p2's Starship (R1852002), so it is not one
        // of p1's Starship Pieces.
        let mut s = board(
            "r1859012-stolen-own",
            false,
            "p2",
            json!({ "hand": [] }),
            json!({ "hand": [FEELINGS, HIT_JOB], "field": [{ "def": AP_ROUNDS, "lane": 2 }] }),
        );
        let piece = steal(&mut s, P2, 2);
        s.play(HIT_JOB, json!({ "targets": pick::id(&piece) }));
        assert_eq!(s.card(piece.as_str()).zone, Zone::Graveyard { player: P2 });
        assert!(dormant_starship_of(s.state(), P2).is_some(), "the copy fused into p2's Starship");
        assert!(dormant_starship_of(s.state(), P1).is_none());
        assert!(!trap_fired(&s));
        let trap = s.card(ID);
        assert!(matches!(trap.zone, Zone::Field { row: Row::Backrow, .. }) && trap.face_up != Some(true));
        assert_eq!(launch_cost(s.state(), P1), STARSHIP_LAUNCH_COST);
    }

    #[test]
    fn r1859012_r2618000_an_enemy_piece_dying_under_its_controller_fuses_into_its_starship_and_reveals_it() {
        // p1 steals p2's AP Rounds and destroys it: it dies under p1's control into p1's graveyard as
        // p1's own card (R2618000), and its copy fuses into p1's Starship, so it is one of p1's pieces.
        let mut s = board(
            "r1859012-stolen-enemy",
            false,
            "p1",
            json!({ "hand": [FEELINGS, HIT_JOB] }),
            json!({ "field": [{ "def": AP_ROUNDS, "lane": 2 }] }),
        );
        let piece = steal(&mut s, P1, 2);
        s.play(HIT_JOB, json!({ "targets": pick::id(&piece) }));
        assert_eq!(s.card(piece.as_str()).zone, Zone::Graveyard { player: P1 });
        assert!(dormant_starship_of(s.state(), P1).is_some(), "the copy fused into p1's Starship");
        assert!(dormant_starship_of(s.state(), P2).is_none());
        assert!(trap_fired(&s));
        s.expect_in_zone(ID, "graveyard");
        assert_eq!(launch_cost(s.state(), P1), 0);
    }

    #[test]
    fn r1859012_a_launch_by_an_effect_leaves_the_free_launch_held() {
        let mut s = board(
            "r1859012-effect-launch",
            false,
            "p1",
            json!({ "hand": [HIT_JOB, COMMANDER] }),
            json!({}),
        );
        hit(&mut s, P1, 2);
        assert_eq!(launch_cost(s.state(), P1), 0);
        let ship = dormant_starship_of(s.state(), P1).expect("p1's Starship").id.clone();
        // Starfleet Commander's Cry awakens the Starship, a launch that spends no price (R1852006).
        s.play(COMMANDER, json!({ "zone": 4 }));
        assert!(s.card(ship.as_str()).starship.as_ref().is_some_and(|memory| memory.launched.is_some()));
        assert_eq!(launch_cost(s.state(), P1), 0, "still held for the next launch by the Activate");
    }

    #[test]
    fn both_faces_trade_from_the_hand_for_a_draw_once_a_turn() {
        for radiant in [false, true] {
            let mut s = board(
                "r1838000-liftoff-trade",
                radiant,
                "p1",
                json!({ "hand": [{ "def": ID, "radiant": radiant }], "backrow": [] }),
                json!({}),
            );
            let trap = s.card(ID).clone();
            assert!(is_tradable(s.state(), &trap), "radiant {radiant}: the face prints Tradable");
            let (hand, deck) = (s.hand("p1").len(), s.pile("p1", "library").len());
            s.trade(&trap.id);
            s.expect_events(json!(["traded", "shuffledIn", "drawn"]));
            assert_eq!(s.hand("p1").len(), hand);
            assert_eq!(s.pile("p1", "library").len(), deck);
            assert_eq!(trades_left(s.state(), P1), 0, "the turn's one Trade is made");
        }
    }

    #[test]
    fn flashback_sets_it_again_once_from_the_graveyard_where_it_reveals_again() {
        let mut s = board(
            "liftoff-flashback",
            false,
            "p1",
            json!({ "field": [{ "def": AP_ROUNDS, "lane": 2 }, { "def": AP_ROUNDS, "lane": 3 }] }),
            json!({}),
        );
        hit(&mut s, P1, 2);
        assert!(trap_fired(&s));
        s.expect_in_zone(ID, "graveyard");
        assert!(offered(&s), "Flashback offers it from the graveyard");

        set_from_graveyard(&mut s);
        let trap = s.card(ID).clone();
        assert!(matches!(trap.zone, Zone::Field { player: P1, row: Row::Backrow, lane: 1 }), "{:?}", trap.zone);
        assert!(trap.face_up != Some(true), "set face-down");
        assert_eq!(trap.flashback_spent, Some(true));

        hit(&mut s, P1, 3);
        assert!(trap_fired(&s));
        s.expect_in_zone(ID, "graveyard");
        assert!(!offered(&s), "not again");
    }
}
