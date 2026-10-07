//! C+ #31 Fusion Lab (SPEC §8.7 row 31; §6.3 Fuse, R23, R65, R77, R81, R102, R380, R387, R561).
//! (2) Field Spell, Epic.
//!   Base:    "Activate: Choose a card in your hand. Fuse a random card into it. Its cost doesn't
//!            change." (balance patch 1: the Cry and the end-of-turn trigger became one Activate)
//!   Radiant: "… Fuse a random Radiant card into it …"
//!
//! "A card in your hand" has no "random", so its controller chooses: the pick is declared with the
//! activation and travels in the `activate` action (R81). An Immutable card can't be chosen, since no
//! Fuse keeps one (R23), and an empty hand offers no activation.
//!
//! The Fuse is B5 E23's "fuse a random card into a card in your hand" (`fuseRandomInto`): one random
//! non-token card of every set (R380) but Fusion Lab — every ingredient's id, on a fused Lab (R387) — is
//! fused per R77 and R102 into the chosen card, which is the kept instance and stays in the hand, its
//! type the result's, with a `costOverride` of the cost it had (R65), so its cost doesn't change. On
//! the Radiant face the random card goes in on its Radiant face and lends it to both of the fusion's
//! forms, so its text and stats show whether or not the hand card is Radiant (R561). The opponent's
//! view names neither the hand card, the ingredient nor the fused id (R97, R179).

use jackioh_engine::effects::fuse_random_into;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-031";

/// The `targetChecks` predicate the ability's declared pick names: a card a Fuse may keep (R23).
const FUSABLE: &str = "fusable";

/// R23: a card a Fuse may keep is not Immutable.
fn fusable(state: &GameState, card: &CardInstance) -> bool {
    !unit_has(state, card, KeywordKind::Immutable)
}

/// R81: one card of the controller's own hand, declared with the activation.
fn hand_pick() -> Vec<TargetDecl> {
    vec![TargetDecl::hand(1, 1, json!({ "check": FUSABLE }))]
}

fn fusion_lab(radiant: bool) -> Script {
    // B5 E23: a random card of every set but this one into the chosen hand card, keeping its cost.
    let fuse = ActivationDecl {
        id: "fuse".to_string(),
        label: "Choose a card in your hand. Fuse a random card into it".to_string(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: hand_pick(),
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(move |_ctx| {
            vec![fuse_random_into(json_as(json!({
                "into": { "target": { "of": "chosen" } },
                "radiant": radiant,
                "keepCost": true,
            })))]
        }),
    };
    Script {
        target_checks: IndexMap::from([(
            FUSABLE,
            target_check(|args| args.candidate.is_some_and(|candidate| fusable(args.state, candidate))),
        )]),
        activations: vec![fuse],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: fusion_lab(false),
        radiant: fusion_lab(true),
    }
}

// C+ #31 Fusion Lab — SPEC §8.7 row 31, BUILD M9 Classic+ row C+ 31: "Field Spell (balance patch 1: the
// Cry and the end-of-turn trigger became one Activate, once): Activate fuses a random non-token card of
// any set but Fusion Lab (R387) into a hand card chosen with the activation (R81); the hand card is the
// kept instance, its type wins (R77) and it costs what it cost before (`costOverride`); an Immutable
// hand card is never offered (R23); an empty hand offers no activation; the opponent's view names
// neither the hand card, the ingredient nor the fused id (R97, R179); radiant the random card is fused
// in on its Radiant face and lends it to both of the fusion's forms, so its rider shows whether or not
// the hand card is Radiant" (R561).
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const LAB: &str = "classicplus-031";
    /// #19 Midrange Menace, (3) Unit: the hand card the fusion keeps.
    const MENACE: &str = "core-019";
    /// #5 Stockpile, (1) Spell: a second hand card.
    const STOCKPILE: &str = "core-005";
    const HIDDEN: &str = "hidden";

    /// An instance id, read before a Fuse renames the card's definition.
    fn id_of(s: &Scenario, card: &str) -> String {
        s.card(card).id.clone()
    }

    fn pick(id: &str) -> Value {
        json!([{ "pick": "instance", "instanceId": id }])
    }

    /// The kept card's fused id read back into [random ingredient, kept card].
    fn parts_of(s: &Scenario, id: &str) -> Vec<String> {
        match fused_id_parts(Some(s.state()), &s.card(id).def_id) {
            Some(parts) => parts,
            None => panic!("{id} was not fused"),
        }
    }

    fn fused_events(s: &Scenario, viewer: PlayerId) -> Vec<GameEvent> {
        s.view(viewer)
            .events
            .into_iter()
            .filter(|event| matches!(event, GameEvent::Fused { .. }))
            .collect()
    }

    use crate::js;

    fn hand_id_of(s: &Scenario, def_id: &str) -> String {
        s.hand(P1).into_iter().find(|card| card.def_id == def_id).map(|card| card.id).unwrap_or_default()
    }

    fn any_fused(s: &Scenario) -> bool {
        s.events().iter().any(|event| matches!(event, GameEvent::Fused { .. }))
    }

    mod c_n31_fusion_lab {
        use super::*;

        #[test]
        fn declares_one_hand_card_with_the_activation_r81_once_and_its_faces_differ_only_in_the_ingredient_s_face() {
            crate::register_all();
            assert_eq!(ID, LAB);
            assert_eq!(crate::card_def(ID).id, LAB);
            let scripts = script();
            assert_eq!(scripts.base.activations.len(), 1);
            assert_eq!(
                js(&scripts.base.activations[0].targets),
                json!([{ "kind": "hand", "min": 1, "max": 1, "filter": { "check": "fusable" } }])
            );
            assert!(matches!(scripts.base.activations[0].uses, ActivationUses::Count(1)));
            // TS `expect(base).not.toBe(radiant)`: each face is its own script, built by its own call.
            assert!(!std::sync::Arc::ptr_eq(
                &scripts.base.activations[0].run,
                &scripts.radiant.activations[0].run
            ));
        }

        mod base {
            use super::*;

            #[test]
            fn r77_its_activate_fuses_a_random_card_into_the_chosen_hand_card_which_stays_in_hand_keeps_its_type_and_its_cost() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [LAB, MENACE, STOCKPILE] } }));
                let kept = id_of(&s, MENACE);
                let other = id_of(&s, STOCKPILE);
                let cost_before = effective_cost(s.state(), s.card(&kept), Default::default());
                s.play(LAB, json!({}));
                s.activate(LAB, json!({ "targets": pick(&kept) }));
                let now = s.card(&kept).clone();
                assert_eq!(now.zone, Zone::Hand { player: P1 });
                let parts = parts_of(&s, &kept);
                let (ingredient, held) = (parts.first().cloned().unwrap_or_default(), parts.get(1).cloned());
                assert_eq!(held.as_deref(), Some(MENACE));
                assert_ne!(ingredient, LAB);
                assert!(!def_of(Some(s.state()), &ingredient).token);
                assert_eq!(def_of(Some(s.state()), &now.def_id).type_, CardType::Unit);
                assert_eq!(effective_cost(s.state(), &now, Default::default()), cost_before);
                assert_eq!(now.cost_override, Some(cost_before));
                assert_eq!(s.card(&other).def_id, STOCKPILE);
                s.expect_in_zone(LAB, "field");
            }

            #[test]
            fn r387_the_random_card_is_never_fusion_lab_whatever_the_seed() {
                crate::register_all();
                for seed in ["lab-a", "lab-b", "lab-c", "lab-d", "lab-e", "lab-f"] {
                    let mut s = scenario(json!({ "seed": seed, "p1": { "hand": [LAB, MENACE] } }));
                    let kept = id_of(&s, MENACE);
                    s.play(LAB, json!({}));
                    s.activate(LAB, json!({ "targets": pick(&kept) }));
                    assert_ne!(parts_of(&s, &kept)[0], LAB);
                }
            }

            #[test]
            fn the_end_of_turn_fuses_nothing_the_cry_and_the_trigger_are_gone() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [LAB, MENACE, STOCKPILE] }, "p2": { "hand": [STOCKPILE] } }));
                s.play(LAB, json!({}));
                let menace = id_of(&s, MENACE);
                s.end_turn();
                assert!(s.state().pending.is_none());
                assert!(!any_fused(&s));
                assert_eq!(s.card(&menace).def_id, MENACE);
                assert_eq!(s.state().active, P2);
            }

            #[test]
            fn the_activate_is_once_a_second_activation_is_refused() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [LAB, MENACE, STOCKPILE] } }));
                let menace = id_of(&s, MENACE);
                let stockpile = hand_id_of(&s, STOCKPILE);
                s.play(LAB, json!({}));
                s.activate(LAB, json!({ "targets": pick(&menace) }));
                s.expect_refused(|s| s.activate(LAB, json!({ "targets": pick(&stockpile) })));
                assert_eq!(s.card(&stockpile).def_id, STOCKPILE);
            }

            #[test]
            fn r113_the_played_state_survives_json_and_the_activation_replays_to_the_same_hash() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [LAB, MENACE, STOCKPILE] }, "p2": { "hand": [STOCKPILE] } }));
                let menace = id_of(&s, MENACE);
                s.play(LAB, json!({}));
                let thawed: GameState = serde_json::from_str(&serde_json::to_string(s.state()).unwrap()).unwrap();
                assert_eq!(&thawed, s.state());
                let declared: Action = json_as(json!({
                    "type": "activate",
                    "playerId": "p1",
                    "instanceId": s.card(LAB).id,
                    "ability": "fuse",
                    "targets": pick(&menace),
                    "nonce": "lab-roundtrip",
                }));
                let live = reduce(s.state(), &declared);
                let again = reduce(&thawed, &declared);
                assert!(live.error.is_none());
                assert_eq!(hash_state(&again.state), hash_state(&live.state));
                assert_eq!(again.events, live.events);
            }

            #[test]
            fn r23_an_immutable_hand_card_is_never_offered_its_declared_pick_is_refused_the_legal_card_still_fuses() {
                crate::register_all();
                // A Radiant Midrange Menace prints Immutable.
                let mut s = scenario(json!({
                    "p1": { "hand": [LAB, { "def": MENACE, "radiant": true }, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                let menace = id_of(&s, MENACE);
                let stockpile = hand_id_of(&s, STOCKPILE);
                s.play(LAB, json!({}));
                s.expect_refused_with(|s| s.activate(LAB, json!({ "targets": pick(&menace) })), "not a legal target");
                s.activate(LAB, json!({ "targets": pick(&stockpile) }));
                assert_eq!(parts_of(&s, &stockpile)[1], STOCKPILE);
                assert_eq!(s.card(&menace).def_id, MENACE);
            }

            #[test]
            fn r23_with_only_an_immutable_card_in_hand_the_activation_has_no_pick() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [LAB, { "def": MENACE, "radiant": true }] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                let menace = id_of(&s, MENACE);
                s.play(LAB, json!({}));
                s.expect_refused_with(|s| s.activate(LAB, json!({ "targets": pick(&menace) })), "not a legal target");
                s.end_turn();
                assert!(!any_fused(&s));
                assert!(s.state().pending.is_none());
                assert!(
                    s.events()
                        .iter()
                        .any(|event| matches!(event, GameEvent::TurnEnded { player, .. } if *player == P1))
                );
            }

            #[test]
            fn s8_7_an_empty_hand_activating_picks_nothing_and_fuses_nothing_and_the_end_of_turn_asks_nothing() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [LAB] }, "p2": { "hand": [STOCKPILE] } }));
                s.play(LAB, json!({}));
                s.activate(LAB, json!({}));
                assert!(!any_fused(&s));
                s.end_turn();
                assert!(s.state().pending.is_none());
                assert!(!any_fused(&s));
            }

            #[test]
            fn r97_r179_the_opponent_s_view_names_neither_the_hand_card_the_ingredient_nor_the_fused_id() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [LAB, MENACE] }, "p2": { "hand": [STOCKPILE] } }));
                let menace = id_of(&s, MENACE);
                s.play(LAB, json!({}));
                s.activate(LAB, json!({ "targets": pick(&menace) }));
                let theirs = fused_events(&s, P2);
                assert_eq!(theirs.len(), 1);
                match &theirs[0] {
                    GameEvent::Fused { instance_ids, result_instance_id, def_id } => {
                        assert_eq!(def_id, HIDDEN);
                        assert_eq!(result_instance_id, HIDDEN);
                        assert!(instance_ids.iter().all(|id| id == HIDDEN));
                    }
                    other => panic!("not a fused event: {other:?}"),
                }
                let view = serde_json::to_string(&s.view(P2)).unwrap();
                let fused_id = s.card(&menace).def_id.clone();
                assert!(!view.contains(&fused_id));
                let ingredient = parts_of(&s, &menace).first().cloned().unwrap_or_else(|| "none".to_string());
                assert!(!view.contains(&ingredient));
                assert!(!view.contains(&menace));
                let ours = fused_events(&s, P1);
                match ours.first() {
                    Some(GameEvent::Fused { def_id, .. }) => assert_eq!(def_id, &fused_id),
                    other => panic!("no fused event for p1: {other:?}"),
                }
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r561_the_random_card_goes_in_on_its_radiant_face_lent_to_both_forms_a_base_hand_card_shows_its_radiant_text() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": LAB, "radiant": true }, MENACE] } }));
                let menace = id_of(&s, MENACE);
                s.play(LAB, json!({}));
                s.activate(LAB, json!({ "targets": pick(&menace) }));
                let kept = s.card(&menace).clone();
                assert!(!kept.radiant);
                let specs = fused_id_specs(Some(s.state()), &kept.def_id).unwrap_or_default();
                let (ingredient, held) = (specs.first().cloned(), specs.get(1).cloned());
                assert_eq!(ingredient.as_ref().and_then(|spec| spec.radiant), Some(true));
                assert_eq!(js(&held), json!({ "defId": MENACE }));
                let fused_def = def_of(Some(s.state()), &kept.def_id).clone();
                let ingredient_id = ingredient.map(|spec| spec.def_id).unwrap_or_default();
                let lent = def_of(Some(s.state()), &ingredient_id).radiant.text.split('\n').next().unwrap_or("").to_string();
                assert!(fused_def.base.text.contains(&lent));
                assert!(fused_def.radiant.text.contains(&lent));
            }

            #[test]
            fn r561_a_radiant_hand_card_shows_the_same_radiant_ingredient() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": LAB, "radiant": true }, { "def": STOCKPILE, "radiant": true }] },
                }));
                let stockpile = id_of(&s, STOCKPILE);
                s.play(LAB, json!({}));
                s.activate(LAB, json!({ "targets": pick(&stockpile) }));
                let kept = s.card(&stockpile).clone();
                assert!(kept.radiant);
                let specs = fused_id_specs(Some(s.state()), &kept.def_id).unwrap_or_default();
                let ingredient = specs.first().cloned();
                assert_eq!(ingredient.as_ref().and_then(|spec| spec.radiant), Some(true));
                let ingredient_id = ingredient.map(|spec| spec.def_id).unwrap_or_default();
                let lent = def_of(Some(s.state()), &ingredient_id).radiant.text.split('\n').next().unwrap_or("").to_string();
                assert!(def_of(Some(s.state()), &kept.def_id).radiant.text.contains(&lent));
            }

            #[test]
            fn r77_its_activate_fuses_a_radiant_card_too_keeping_the_cost() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": LAB, "radiant": true }, MENACE, STOCKPILE] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                let stockpile = hand_id_of(&s, STOCKPILE);
                s.play(LAB, json!({}));
                let cost = effective_cost(s.state(), s.card(&stockpile), Default::default());
                s.activate(LAB, json!({ "targets": pick(&stockpile) }));
                let specs = fused_id_specs(Some(s.state()), &s.card(&stockpile).def_id).unwrap_or_default();
                assert_eq!(specs.first().and_then(|spec| spec.radiant), Some(true));
                assert_eq!(effective_cost(s.state(), s.card(&stockpile), Default::default()), cost);
            }
        }
    }
}
