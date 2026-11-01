//! SPEC §8.10 Stinkyscape 39 The Narcor Project, a card of the Stinkyscape set.
//!   (3) Field Spell, Legendary (#2521: was (4)).
//!   Base:    "Brittle 4\nEnd of turn: Gain a Narcor.\nActivate: Discard a Narcor to lower this card's
//!            Brittle by 1.\nDeath: If Brittle destroyed this, for the rest of the game, gain two Narcors
//!            at the end of each of your turns."
//!   Radiant: "… Gain a Red Narcor.\nActivate: Discard a Narcor or a Red Narcor to lower this card's
//!            Brittle by 1.\nDeath: If Brittle destroyed this, …, gain two Red Narcors …"
//!
//! Brittle 4 is the printed keyword, the engine's (§6.1, R385, R638). End of turn (R62): `gain_resources`
//! of one Narcor (S #T-narcor, a one-id pool, R1840002) into its controller's hand, a full hand burning
//! it (R2250000). Activate (§6.2, R384), offered only while the hand holds a Narcor (a `canActivate`
//! read of the hand, `hand_holds_any`): `discard_resource` of the one longest there, a hand discard that
//! pays its Credits (R2250000, R2250001), then `lower_brittle` of 1 on this card, which crumbles it at
//! once when it leaves the count at 0 (R1861001), a Brittle death like the tick's. Death: when Brittle
//! destroyed it (`brittle_spent` on the Death snapshot, R1861001), a rest-of-game player effect on its
//! controller, C+ #52's `forRestOfGame` with `atEnd` (R1861002): at the end of each of their turns, the
//! one it died on included, two Narcors, whatever became of the card (R127). Another destroy, or an
//! exile, gives nothing more. Radiant: Red Narcors (S #T-red-narcor), and the Activate takes the oldest
//! Narcor or Red Narcor. Its numbers are printed in its words, none declared.

use jackioh_engine::effects::{discard_resource, for_rest_of_game, gain_resources, lower_brittle};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-039";

const NARCOR: &str = "stinkyscape-t-narcor";
const RED_NARCOR: &str = "stinkyscape-t-red-narcor";
/// "Gain a Narcor" at each end of its controller's turn.
const END_OF_TURN_NARCORS: i32 = 1;
/// "Gain two Narcors at the end of each of your turns", for the rest of the game.
const REST_OF_GAME_NARCORS: i32 = 2;
/// "Lower this card's Brittle by 1."
const LOWERED_BY: i32 = 1;
/// The Activate's id.
const DISCARD_NARCOR: &str = "discard-narcor";
/// The rest-of-game effect's step.
const NARCORS: &str = "narcors";
/// What the Activate takes: a Narcor, or on the Radiant face a Narcor or a Red Narcor.
const TAKES: &[&str] = &[NARCOR];
const RADIANT_TAKES: &[&str] = &[NARCOR, RED_NARCOR];

/// The Narcors it gains: Narcors, Red Narcors on the Radiant face.
fn gain(radiant: bool, count: i32) -> Effect {
    let kind = if radiant { RED_NARCOR } else { NARCOR };
    gain_resources(json_as(json!({ "count": count, "pool": [kind] })))
}

fn discard_narcor(radiant: bool) -> ActivationDecl {
    let takes = if radiant { RADIANT_TAKES } else { TAKES };
    ActivationDecl {
        id: DISCARD_NARCOR.to_string(),
        label: if radiant {
            "Discard a Narcor or a Red Narcor: lower Brittle by 1"
        } else {
            "Discard a Narcor: lower Brittle by 1"
        }
        .to_string(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: Some(condition_hook(move |c| hand_holds_any(c.state, c.controller, takes))),
        has: None,
        run: hook(move |_ctx| {
            vec![
                discard_resource(json_as(json!({ "defIds": takes }))),
                lower_brittle(json_as(json!({ "target": { "of": "self" }, "n": LOWERED_BY }))),
            ]
        }),
    }
}

fn face(radiant: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |_ctx| vec![gain(radiant, END_OF_TURN_NARCORS)])),
        activations: vec![discard_narcor(radiant)],
        death: Some(hook(move |ctx| {
            if !ctx.self_.as_ref().is_some_and(brittle_spent) {
                return vec![];
            }
            // The badge (R169) is the Death line, as the catalog the game runs under prints it (R767000).
            let face = if radiant {
                FaceKind::Radiant
            } else {
                FaceKind::Base
            };
            let text = fill_params(def_of(Some(&*ctx.state), ID), face, None);
            let label = text.lines().last().unwrap_or_default().to_string();
            vec![for_rest_of_game(json_as(json!({
                "step": NARCORS,
                "label": label,
                "atEnd": true,
            })))]
        })),
        delayed: Some(hook(move |_ctx| vec![gain(radiant, REST_OF_GAME_NARCORS)])),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S 39 The Narcor Project — SPEC §8.10 row 39, BUILD M12 row S 39: "at its controller's end of turn
// gains a Narcor into the hand; its Activate is offered only with a Narcor in the
// hand, discards the oldest and lowers Brittle by 1; driven to 0 it crumbles at once and from then on
// its controller gains two Narcors at the end of each of their turns, the one it died on included, the
// card long gone (R1861001, R1861002); destroyed otherwise, nothing more; radiant Red Narcors, the
// Activate taking a Narcor or a Red Narcor".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::{DISCARD_NARCOR, ID, NARCOR, RED_NARCOR};
    use crate::{def_ids, filler, js, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";
    const NETHER: &str = "core-088"; // (4) Spell: destroy all permanents
    const REKNITE: &str = "stinkyscape-t-reknite";

    /// p1's Project in the backrow, a hand of Twisting Nether, a filler and `held`.
    fn project(radiant: bool, held: Value) -> Scenario {
        let mut hand = vec![json!(NETHER), json!(FILLER)];
        hand.extend(held.as_array().cloned().unwrap_or_default());
        scenario(json!({
            "p1": {
                "hand": hand,
                "library": filler(6),
                "backrow": [{ "def": ID, "radiant": radiant }],
            },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }))
    }

    fn in_hand(s: &Scenario) -> Vec<String> {
        def_ids(&crate::resources_in_hand(s, P1))
    }

    fn brittle(s: &Scenario) -> Option<i32> {
        active_brittle_count(s.card(ID))
    }

    /// The Activates `legal_actions` offers p1 on the Project.
    fn offered(s: &Scenario) -> usize {
        let id = s.card(ID).id.clone();
        legal_actions(s.state(), P1)
            .into_iter()
            .filter(|body| match body {
                ActionBody::Activate { instance_id, .. } | ActionBody::ActivatePower { instance_id, .. } => {
                    *instance_id == id
                }
                _ => false,
            })
            .count()
    }

    fn activate(s: &mut Scenario) -> &mut Scenario {
        s.activate(ID, json!({ "ability": DISCARD_NARCOR }))
    }

    /// The Project's count set to 1, as three earlier Activates would have left it.
    fn at_one(s: &mut Scenario) {
        s.card_mut(ID)
            .brittle
            .as_mut()
            .expect("Brittle 4 is in force")
            .count = 1;
    }

    #[test]
    fn end_of_turn_gains_a_narcor_into_the_hand() {
        let mut s = project(false, json!([]));
        let hand = s.hand(P1).len();
        s.end_turn();
        assert_eq!(in_hand(&s), vec![NARCOR], "its controller's end of turn");
        assert_eq!(s.hand(P1).len(), hand + 1, "a hand card like any other (R2250000)");
        s.end_turn();
        assert_eq!(in_hand(&s), vec![NARCOR], "the opponent's end of turn gains nothing");
        assert!(crate::resources_in_hand(&s, P2).is_empty());
    }

    #[test]
    fn the_activate_needs_a_narcor_and_discards_the_oldest() {
        let mut s = project(false, json!([REKNITE]));
        assert_eq!(offered(&s), 0, "no Narcor in the hand");
        s.expect_refused(activate);

        let mut s = project(false, json!([NARCOR, REKNITE, NARCOR]));
        let oldest = crate::resources_in_hand(&s, P1)[0].id.clone();
        assert_eq!(offered(&s), 1);
        activate(&mut s);
        assert_eq!(in_hand(&s), vec![REKNITE, NARCOR], "the oldest Narcor is gone");
        assert!(
            s.pile(P1, "graveyard").iter().any(|card| card.id == oldest),
            "discarded from the hand (R2250000)"
        );
        assert_eq!(brittle(&s), Some(3), "Brittle 4 lowered by 1");
        s.expect_refused(activate);
    }

    #[test]
    fn r1861001_r1861002_lowered_to_0_it_crumbles_then_two_narcors_each_of_its_ends() {
        let mut s = project(false, json!([NARCOR]));
        at_one(&mut s);
        activate(&mut s);
        assert!(
            s.events()
                .iter()
                .any(|event| matches!(event, GameEvent::Crumbled { def_id, .. } if def_id == ID))
        );
        s.expect_in_zone(ID, "graveyard");
        assert!(in_hand(&s).is_empty());
        let label = "Death: If Brittle destroyed this, for the rest of the game, gain two Narcors at the end of each of your turns.";
        for (viewer, side) in [(P1, "you"), (P2, "opponent")] {
            let view = js(&s.view(viewer));
            let labels: Vec<&str> = view[side]["modifiers"]
                .as_array()
                .map(|mods| mods.iter().filter_map(|entry| entry["label"].as_str()).collect())
                .unwrap_or_default();
            assert!(labels.contains(&label), "a badge in {viewer}'s view: {labels:?}");
        }

        s.end_turn();
        assert_eq!(
            in_hand(&s),
            vec![NARCOR, NARCOR],
            "at the end of the turn it died on"
        );
        s.end_turn();
        assert_eq!(in_hand(&s).len(), 2, "none at the end of the opponent's turn");
        s.end_turn();
        assert_eq!(
            in_hand(&s),
            vec![NARCOR; 4],
            "two more at the end of each of its controller's turns"
        );
    }

    #[test]
    fn r1861001_ticked_to_0_it_crumbles_and_pays_too() {
        let mut s = project(false, json!([]));
        at_one(&mut s);
        s.card_mut(ID).brittle.as_mut().expect("in force").since = 0;
        s.start_turn();
        s.expect_in_zone(ID, "graveyard");
        s.end_turn();
        assert_eq!(in_hand(&s), vec![NARCOR, NARCOR]);
    }

    #[test]
    fn r1861001_twisting_nether_gives_nothing_more() {
        let mut s = project(false, json!([]));
        s.play(NETHER, json!({}));
        s.expect_in_zone(ID, "graveyard");
        s.end_turn();
        assert!(
            in_hand(&s).is_empty(),
            "destroyed otherwise, no Narcors at its controller's end of turn"
        );
        s.end_turn().end_turn();
        assert!(in_hand(&s).is_empty());
    }

    #[test]
    fn radiant_red_narcors_and_the_activate_takes_a_red_narcor() {
        let mut s = project(true, json!([]));
        s.end_turn();
        assert_eq!(in_hand(&s), vec![RED_NARCOR]);
        assert!(!crate::resources_in_hand(&s, P1)[0].radiant, "a Red Narcor, not a Radiant one");

        let mut s = project(true, json!([RED_NARCOR]));
        assert_eq!(offered(&s), 1, "a Red Narcor pays the Radiant Activate");
        activate(&mut s);
        assert!(in_hand(&s).is_empty());
        assert_eq!(brittle(&s), Some(3));

        let mut s = project(true, json!([NARCOR]));
        at_one(&mut s);
        activate(&mut s);
        s.expect_in_zone(ID, "graveyard");
        s.end_turn();
        assert_eq!(
            in_hand(&s),
            vec![RED_NARCOR, RED_NARCOR],
            "two Red Narcors at each end of its turns"
        );
    }
}
