//! S+ #157 Phantom Shipyard (SPEC §8.12 row S+ #157; §6.1 Dormant, §6.3 Launch; R3539003,
//! R3539004): (4) Unit, Legendary, 0/0.
//!   Base:    "Start of turn: Fuse a random Starship Piece into this.\nDormant until launched.
//!            \nActivate 3: Exile the shipyard and launch as a Starship."
//!   Radiant: "Start of turn: Fuse a random Radiant Starship Piece into this.\nDormant until
//!            launched.\nActivate 3: Exile the shipyard and launch as a Starship."
//!
//! Engine: the shipyard is a hull (`starship_hull`) that fuses in a random Starship Piece — a Radiant
//! one on the Radiant card — at the start of its controller's turn (`starship_yard`,
//! `yard_at_turn_start`, R3539004). Its Activate (`shipyard`) needs at least one fused piece, then
//! exiles it and launches as a Starship holding every piece it fused in (`launch_as_starship`), for
//! nothing (R1852006). Any other awakening launches the shipyard itself. One script serves both
//! faces: the yard fuses Radiant pieces because the card is Radiant. "Activate 3" is three uses a
//! turn (R1857003).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-157";

/// R1857003: "Activate 3" is three uses a turn.
const USES: i32 = 3;
/// R3539004: the ability's id, which the shipyard's Dormant text names as its awakening.
const ABILITY: &str = "shipyard";

fn shipyard() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            slumber: Some(SlumberText {
                turns: None,
                wakes_by: Some(ABILITY.to_string()),
            }),
            starship_hull: Some(true),
            starship_yard: Some(true),
            ..StaticFlags::default()
        }),
        activations: vec![ActivationDecl {
            id: ABILITY.to_string(),
            label: "Exile the shipyard and launch as a Starship".to_string(),
            uses: ActivationUses::Count(USES),
            cost: None,
            targets: vec![],
            modes: vec![],
            can_activate: Some(condition_hook(|ctx| {
                !starship_pieces_of(ctx.self_).is_empty()
            })),
            has: None,
            run: hook(|_ctx| vec![launch_as_starship()]),
        }],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let face = shipyard();
    CardScripts {
        base: face.clone(),
        radiant: face,
    }
}

// S+ #157 Phantom Shipyard — SPEC §8.12 row S+ #157 (R3539003, R3539004).
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::{filler, scenario};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const SHIPYARD: &str = ID;
    const FILLER: &str = "core-005"; // Stockpile: a Spell
    const HIT_JOB: &str = "core-016"; // Hit Job: the removal
    const AP_ROUNDS: &str = "stinkyscape-100"; // AP Rounds: a Starship Piece

    /// The shipyard played into lane 2: a play is a real arrival, so it enters Dormant with its
    /// memory, as a setup-written zone never does.
    fn yarded(radiant: bool) -> Scenario {
        let hand = if radiant {
            json!([{ "def": SHIPYARD, "radiant": true }])
        } else {
            json!([SHIPYARD])
        };
        let mut s = scenario(json!({
            "seed": "shipyard",
            "p1": { "hand": hand, "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.state_mut().players.p1.auto_end_turn = Some(false);
        s.state_mut().players.p2.auto_end_turn = Some(false);
        s.play(SHIPYARD, json!({ "zone": 2 }));
        s
    }

    fn yard_id(s: &Scenario) -> String {
        s.unit(P1, 2).expect("the shipyard fills lane 2").id
    }

    fn held(s: &Scenario, id: &str) -> Vec<String> {
        starship_pieces_of(s.card(id))
            .into_iter()
            .map(|record| record.def_id)
            .collect()
    }

    #[test]
    fn r3539003_stays_on_the_field_at_0_0_while_dormant() {
        let mut s = yarded(false);
        let id = yard_id(&s);
        assert!(is_dormant_starship(s.card(&id)));
        s.end_turn();
        s.expect_stats(id.as_str(), json!({ "attack": 0, "health": 0 }));
    }

    #[test]
    fn r3539004_fuses_a_random_starship_piece_at_each_start_of_turn() {
        let mut s = yarded(false);
        let id = yard_id(&s);
        s.end_turn();
        s.end_turn();
        assert_eq!(held(&s, &id).len(), 1);
        let first = held(&s, &id);
        assert!(
            has_keyword(&crate::card_def(&first[0]).base.keywords, KeywordKind::StarshipPiece),
            "it fuses Starship Pieces"
        );
        s.end_turn();
        s.end_turn();
        assert_eq!(held(&s, &id).len(), 2);
    }

    #[test]
    fn radiant_fuses_radiant_pieces() {
        let mut s = yarded(true);
        let id = yard_id(&s);
        s.end_turn();
        s.end_turn();
        let records = starship_pieces_of(s.card(&id));
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].radiant, Some(true));
    }

    #[test]
    fn r3539004_activate_exiles_it_and_launches_a_starship_holding_its_pieces() {
        let mut s = yarded(false);
        let id = yard_id(&s);
        s.end_turn();
        s.end_turn();
        let pieces = held(&s, &id);
        assert_eq!(pieces.len(), 1);
        let mana = s.state().players.p1.mana.current;
        s.activate(id.as_str(), json!({ "ability": "shipyard" }));
        assert_eq!(s.state().players.p1.mana.current, mana, "the launch is free");
        assert_eq!(s.card(&id).zone.z(), ZoneName::Exile);
        let launched = launched_starship_of(s.state(), P1)
            .cloned()
            .expect("a fresh Starship launched");
        assert_eq!(hull_def_of(s.state(), &launched.def_id), DORMANT_STARSHIP_DEF_ID);
        assert_eq!(s.unit(P1, 2).map(|unit| unit.id), Some(launched.id.clone()));
        assert_eq!(held(&s, &launched.id), pieces);
        assert!(is_launched_starship(&launched));
    }

    #[test]
    fn r3539004_cannot_launch_before_it_holds_a_piece() {
        let mut s = yarded(false);
        let id = yard_id(&s);
        s.expect_refused_with(
            |s| s.activate(id.as_str(), json!({ "ability": "shipyard" })),
            "that ability can't be activated now",
        );
    }

    #[test]
    fn r3539004_a_dying_piece_fuses_into_it() {
        let mut s = scenario(json!({
            "seed": "shipyard-hit",
            "p1": {
                "hand": [HIT_JOB, SHIPYARD],
                "field": [{ "def": AP_ROUNDS, "lane": 3 }],
                "library": filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(SHIPYARD, json!({ "zone": 2 }));
        let id = yard_id(&s);
        let victim = s.unit(P1, 3).expect("the piece fills lane 3").id;
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": victim }] }));
        assert_eq!(held(&s, &id), vec![AP_ROUNDS.to_string()]);
    }

    #[test]
    fn r3539000_its_starship_has_no_launch_by_hand() {
        let s = yarded(false);
        let id = yard_id(&s);
        assert!(
            !legal_actions(s.state(), P1).iter().any(|action| {
                matches!(action, ActionBody::Activate { instance_id, ability, .. }
                    if instance_id == &id && ability.as_deref() == Some(STARSHIP_LAUNCH_ABILITY))
            }),
            "the yard offers no launch"
        );
        let ids: Vec<String> = jackioh_engine::subsystems::activate::abilities_of(s.state(), s.card(&id))
            .into_iter()
            .map(|decl| decl.id)
            .collect();
        assert_eq!(ids, vec!["shipyard".to_string()]);
    }
}
