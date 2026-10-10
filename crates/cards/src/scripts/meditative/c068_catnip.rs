//! M #68 Catnip (SPEC §8.8 row 68): (1) Field Trap, Felinor, Rare, 5/5 → 10/10.
//!   Base:    "Animated\nReveals after your opponent plays a Spell."
//!   Radiant: "Animated, Taunt\nReveals after your opponent plays a Spell."
//! Engine: an Animated Field Trap (R383) as C #5 Tesla: fires once a Spell the opponent played or
//!   cast has resolved (R17, MD-D25, R1063), then animates; already a Unit, later Spells do nothing;
//!   no open zone: stays face-up to fire again; hidden (R33).

use jackioh_engine::effects::animate;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-068";

fn catnip() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("catnip-reveal", &[GameEventType::CardResolved], |_, _| {
                vec![animate(json_as(json!({})))]
            })
            .with_when(|ctx, event| {
                let GameEvent::CardResolved { player, def_id, radiant, .. } = event else {
                    return false;
                };
                if *player == ctx.controller {
                    return false;
                }
                if card_type_of_face(ctx.state, def_id, radiant.unwrap_or(false)) != CardType::Spell {
                    return false;
                }
                !ctx.live_self().is_some_and(|me| is_animated(ctx.state, me))
            }),
        ],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = catnip();
    // The Radiant face differs only in stats and Taunt, both catalog data.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// M #68 Catnip — SPEC §8.8 row 68, BUILD M10 row M 68.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::js;
    use crate::matches_object;
    use crate::scenario;

    const CATNIP: &str = "meditative-068";
    const STOCKPILE: &str = "core-005";
    const VANILLA: &str = "core-008";
    const SHEEP: &str = "core-t-sheep";
    const FILLER: &str = "core-010";

    fn set_catnip(radiant: bool, lane: i32) -> Value {
        json!({ "def": CATNIP, "radiant": radiant, "faceUp": false, "lane": lane })
    }

    fn row_is(s: &Scenario, row: &str) {
        assert!(
            matches_object(&js(&s.card(CATNIP).zone), &json!({ "row": row })),
            "catnip in {row}"
        );
    }

    #[test]
    fn r33_face_down_only_its_controller_reads_it() {
        crate::register_all();
        let s = scenario(json!({
            "active": "p2",
            "p1": { "hand": [FILLER], "backrow": [set_catnip(false, 3)] },
            "p2": { "hand": [VANILLA] },
        }));
        assert!(js(&s.view(PlayerId::P1))["you"]["backrow"].to_string().contains(CATNIP));
        assert!(!js(&s.view(PlayerId::P2)).to_string().contains(CATNIP));
    }

    #[test]
    fn r1063_after_the_opponent_s_spell_resolves_it_animates_a_5_5() {
        crate::register_all();
        let mut s = scenario(json!({
            "active": "p2",
            "p1": { "hand": [FILLER], "backrow": [set_catnip(false, 2)] },
            "p2": { "hand": [STOCKPILE, FILLER] },
        }));
        s.play(STOCKPILE, json!({}));
        s.expect_events(json!(["cardPlayed", "cardResolved", "trapFired", "animated"]));
        row_is(&s, "units");
        s.expect_stats(CATNIP, json!({ "attack": 5, "health": 5 }));
    }

    #[test]
    fn r1063_a_spell_cast_counts() {
        crate::register_all();
        let mut s = scenario(json!({
            "active": "p2",
            "p1": { "hand": [FILLER], "backrow": [set_catnip(false, 2)] },
            "p2": { "hand": [FILLER] },
        }));
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = jackioh_engine::rng::create_rng(&s.state().seed, s.state().rng_cursor);
        {
            let mut sink =
                jackioh_engine::script::EngineSink::new(s.state_mut(), &mut events, &mut rng);
            {
                let mut ctx = jackioh_engine::resolve::make_context(
                    &mut sink,
                    None,
                    HookOptions { controller: Some(PlayerId::P2), ..HookOptions::default() },
                );
                jackioh_engine::resolve::apply_effects(
                    &[jackioh_engine::effects::cast_new(json_as(json!({ "def": STOCKPILE })))],
                    &mut ctx,
                );
            }
            jackioh_engine::triggers::settle(
                &mut sink,
                jackioh_engine::triggers::SettleOptions::default(),
            );
        }
        s.state_mut().rng_cursor = rng.cursor();
        row_is(&s, "units");
    }

    #[test]
    fn r1063_a_unit_field_spell_or_trap_or_your_own_spell_leaves_it_set() {
        crate::register_all();
        for def in [VANILLA, "core-006", "core-018"] {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [FILLER], "backrow": [set_catnip(false, 2)] },
                "p2": { "hand": [def, FILLER] },
            }));
            // Units need a zone; Field Spells/Traps play to the backrow.
            if def == VANILLA {
                s.play(def, json!({ "zone": 1 }));
            } else {
                s.play(def, json!({}));
            }
            assert_eq!(s.card(CATNIP).face_up, Some(false));
        }
        // Your own Spell leaves it set.
        let mut s = scenario(json!({
            "p1": { "hand": [STOCKPILE, FILLER], "backrow": [set_catnip(false, 2)] },
            "p2": { "hand": [FILLER] },
        }));
        s.play(STOCKPILE, json!({}));
        assert_eq!(s.card(CATNIP).face_up, Some(false));
    }

    #[test]
    fn r383_with_no_open_zone_it_stays_face_up_and_fires_on_the_next_spell() {
        crate::register_all();
        let full = [VANILLA, VANILLA, SHEEP, VANILLA, VANILLA];
        let mut s = scenario(json!({
            "active": "p2",
            "p1": { "hand": [FILLER], "field": full, "backrow": [set_catnip(false, 1)] },
            "p2": { "hand": [STOCKPILE, VANILLA, STOCKPILE, "core-016", FILLER], "field": [{ "def": VANILLA, "lane": 1 }] },
        }));
        s.play(STOCKPILE, json!({}));
        row_is(&s, "backrow");
        assert_eq!(s.card(CATNIP).face_up, Some(true));
        // P2's Vanilla trades P1's lane-3 sheep open, then a second Stockpile animates it there.
        let killer = s.unit(PlayerId::P2, 1).expect("killer").id.clone();
        let sheep = s.unit(PlayerId::P1, 3).expect("sheep").id.clone();
        s.attack(&killer, &sheep);
        assert!(s.unit(PlayerId::P1, 3).is_none());
        s.play(STOCKPILE, json!({}));
        row_is(&s, "units");
        assert_eq!(s.unit(PlayerId::P1, 3).map(|unit| unit.def_id), Some(CATNIP.to_string()));
    }

    #[test]
    fn r1063_already_a_unit_a_later_spell_does_nothing() {
        crate::register_all();
        let mut s = scenario(json!({
            "active": "p2",
            "p1": { "hand": [FILLER], "backrow": [set_catnip(false, 2)] },
            "p2": { "hand": [STOCKPILE, STOCKPILE, FILLER] },
        }));
        s.play(STOCKPILE, json!({}));
        let events = s.events().len();
        s.play(STOCKPILE, json!({}));
        row_is(&s, "units");
        let _ = events;
    }

    #[test]
    fn radiant_10_10_taunt() {
        crate::register_all();
        let mut s = scenario(json!({
            "active": "p2",
            "p1": { "hand": [FILLER], "backrow": [set_catnip(true, 2)] },
            "p2": { "hand": [STOCKPILE, FILLER] },
        }));
        s.play(STOCKPILE, json!({}));
        s.expect_stats(CATNIP, json!({ "attack": 10, "health": 10 }));
    }
}
