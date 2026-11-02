//! `redact` and fused cards (R185, R77, R179): keep definition chain for visible cards, drop hidden.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN};

/// R179's ids: `t-1` fused from core-008 and core-011, and `t-2` from that and core-020.
const INNER: &str = "t-1:core-008+core-011";

/// `t-2:(${INNER})+core-020`.
fn outer() -> String {
    format!("t-2:({INNER})+core-020")
}

/// A transient def as `subsystems.fuse` builds one: a real card's faces under a fused id.
fn fused_def(id: &str) -> CardDef {
    let mut def = registered_catalog()["core-008"].clone();
    def.id = id.to_string();
    def.index = id.to_string();
    def.name = format!("fused {id}");
    def
}

/// Where p2's fusion of a fusion sits.
#[derive(Clone, Copy)]
enum Where {
    Field,
    Hand,
}

/// p2 holds the `t-2` fusion of a fusion, somewhere.
fn with_chain(where_: Where) -> GameState {
    jackioh_cards::register_all();
    let p2 = match where_ {
        Where::Field => json!({ "field": ["core-019"] }),
        Where::Hand => json!({ "hand": ["core-019"] }),
    };
    let mut state = scenario(json!({ "active": "p1", "p1": { "field": ["core-011"] }, "p2": p2 }))
        .state()
        .clone();
    state.transient_defs.insert(INNER.to_string(), fused_def(INNER));
    state.transient_defs.insert(outer(), fused_def(&outer()));
    let side = &state.players[HUMAN];
    let card_id = match where_ {
        Where::Field => side
            .units
            .iter()
            .flatten()
            .flatten()
            .next()
            .map(|card| card.id.clone()),
        Where::Hand => side.hand.first().map(|card| card.id.clone()),
    }
    .unwrap_or_else(|| panic!("the scenario placed no p2 card"));
    find_instance_mut(&mut state, &card_id)
        .expect("the p2 card")
        .def_id = outer();
    state
}

mod redact_keeps_what_a_fused_card_is_built_from_r185 {
    use super::*;

    /// R185 a visible fusion of a fusion keeps its older ingredient, and the redacted state still runs
    #[test]
    fn r185_a_visible_fusion_of_a_fusion_keeps_its_older_ingredient_and_the_redacted_state_still_runs() {
        let public = redact(&with_chain(Where::Field), AI);
        let mut keys: Vec<String> = public.transient_defs.keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, vec![INNER.to_string(), outer()]);
        // Neither panics on the redacted state (TS: `not.toThrow()`).
        let _ = legal_actions(&public, AI);
        let _ = view_for(&public, AI);
    }

    /// R185 a fusion in the hidden hand takes its whole chain with it
    #[test]
    fn r185_a_fusion_in_the_hidden_hand_takes_its_whole_chain_with_it() {
        let public = redact(&with_chain(Where::Hand), AI);
        assert!(public.transient_defs.is_empty());
        let _ = legal_actions(&public, AI);
    }

    /// R1850003 a fused card held beneath another keeps its definitions, so its release still runs
    #[test]
    fn r1850003_a_fused_card_held_beneath_another_keeps_its_chain_and_its_release_still_runs() {
        let mut state = with_chain(Where::Field);
        // As `hold_beneath` takes it: off the field, its lane emptied.
        let card = state.players[HUMAN]
            .units
            .iter_mut()
            .find_map(Option::take)
            .and_then(|mut pile| pile.pop())
            .expect("p2's unit");
        let holder = state.players[AI]
            .units
            .iter()
            .flatten()
            .flatten()
            .next()
            .expect("p1's unit");
        state.held = Some(vec![HeldCard {
            holder_id: holder.id.clone(),
            holder_mark: 0,
            to: HUMAN,
            card,
            exile_on_release: None,
        }]);
        let public = redact(&state, AI);
        let mut keys: Vec<String> = public.transient_defs.keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, vec![INNER.to_string(), outer()]);
        let _ = legal_actions(&public, AI);
        let _ = view_for(&public, AI);
    }

    /// R1850000 a copier on the field keeps the fused text it copied, so its layers still read
    #[test]
    fn r1850000_a_copier_keeps_the_fused_units_definition_it_copied() {
        let mut state = with_chain(Where::Hand);
        let copier = state.players[AI]
            .units
            .iter_mut()
            .flatten()
            .flatten()
            .next()
            .expect("p1's unit");
        copier.memory.insert(
            jackioh_engine::subsystems::UNIT_TEXT_KEY.to_string(),
            json!({ "defId": outer(), "radiant": false }),
        );
        let public = redact(&state, AI);
        let mut keys: Vec<String> = public.transient_defs.keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, vec![INNER.to_string(), outer()]);
        let _ = legal_actions(&public, AI);
        let _ = view_for(&public, AI);
    }
}
