//! S #144 Prepare For Liftoff (SPEC §8.10 row 144; R1, R17, R64, R1838000, R1852000, R1852002,
//! R1852006, R1859012): (1) Trap, Common, Tradable, Flashback, a card of the held Stinkyscape set
//! (R1420).
//!
//! Base:    "Tradable, Flashback\nReveals when one of your Starship Pieces dies: Your Starship launch
//!          costs (0)."
//! Radiant: "Tradable, Flashback\nReveals when one of your Starship Pieces dies: Summon it from your
//!          graveyard. Your Starship launch costs (0)."
//! Engine: Tradable (R1838000) and Flashback (§6.1, set again from its owner's graveyard once) are
//! catalog keywords. Set face-down, it reveals (R17) when "one of your Starship Pieces" dies: a piece
//! its controller owns, whose copy fuses into their Dormant Starship (`starshipFused` for them,
//! R1852000, R1852002), wherever it died, a stolen one included (R1859012). Then
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

// S #144 Prepare For Liftoff — SPEC §8.10 row 144, BUILD M12 row S 144: "Tradable in the hand (one Trade
// a turn, R1838000); set, it reveals when a Starship Piece it owns dies, fusing into their Starship
// (R1859012), and their next launch by the Activate costs (0) (R1852006), a launch by an effect spending
// none; Flashback lets it be set again once from the graveyard; radiant the dead piece is summoned back
// from the graveyard first, its fused copy staying".
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::launch_starship;
    use jackioh_engine::prelude::{STARSHIP_LAUNCH_COST, dormant_starship_of, launch_cost, starship_pieces_of};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const STARTOS: &str = "stinkyscape-068"; // (1) 3/3 Starship Piece, Flying: keywords only.
    const HIT_JOB: &str = "core-016"; // (3) Spell: destroy a Unit.

    /// p1's Prepare For Liftoff set (Radiant if `radiant`) in backrow lane 1, and a Startos of p1's in
    /// lane 2 and of p2's in lane 1, p1 holding two Hit Jobs.
    fn setup(seed: &str, radiant: bool) -> Scenario {
        let mut s = crate::scenario(json!({
            "seed": seed,
            "active": "p1",
            "p1": {
                "hand": [HIT_JOB, HIT_JOB],
                "field": [{ "def": STARTOS, "lane": 2 }],
                "backrow": [{ "def": ID, "lane": 1, "faceUp": false, "radiant": radiant }],
                "mana": 10,
                "library": crate::filler(4),
            },
            "p2": {
                "hand": crate::filler(1),
                "field": [{ "def": STARTOS, "lane": 1 }],
                "library": crate::filler(4),
            },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
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
        // The base face leaves the piece in the graveyard, its copy in the Starship.
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
        // Its next death fuses again.
        let Zone::Field { lane, .. } = back.zone else {
            panic!("the piece is on the field");
        };
        hit(&mut s, P1, lane);
        let ship = dormant_starship_of(s.state(), P1).expect("p1's Starship");
        assert_eq!(starship_pieces_of(ship).len(), 2);
    }
}
