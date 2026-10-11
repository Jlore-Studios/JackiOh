//! M #75 Ever Growing Tree (SPEC §8.8 row 75, R1087): (4) Field Spell, Legendary.
//!
//! Base:    "Activate: Fuse a random Field Spell into this."
//! Radiant: "Activate: Fuse a random Radiant Field Spell into this."
//! Engine: Activate (R384, once a turn). `fuse_random_into` with the Tree as the kept instance
//! (R77, R102): one random non-token Field Spell of every set, never this card or an ingredient
//! already in it (R387); on the Radiant face the pick goes in on its Radiant face (R469). A Cry
//! never runs, auras and turn lines run, fused Activates share the Tree's count, and a fused
//! keyword applies at once (R1087).

use jackioh_engine::effects::fuse_random_into;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-075";

fn tree(radiant: bool) -> Script {
    // C+ #31 Fusion Lab's shape: one Activate, once a turn, fusing a random card into self.
    let grow = ActivationDecl {
        id: "grow".to_string(),
        label: "Fuse a random Field Spell into this".to_string(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(move |_ctx| {
            vec![fuse_random_into(json_as(json!({
                "into": { "target": { "of": "self" } },
                "query": { "type": "Field Spell" },
                "radiant": radiant,
            })))]
        }),
    };
    Script {
        activations: vec![grow],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: tree(false),
        radiant: tree(true),
    }
}

// M #75 Ever Growing Tree — SPEC §8.8 row 75, BUILD M10 row M 75: "Activate, once a turn in your
// main phase: one random non-token Field Spell (never itself or one already fused in) is fused
// into it, its aura and turn lines working from then on, its Cry never running; a fused Activate
// shares the Tree's once-a-turn use; a fused keyword applies at once (R1087); the fused definition
// survives a JSON round trip (R468); radiant the Field Spell goes in on its Radiant face (R469)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const TREE: &str = "meditative-075";
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    /// TS `JSON.parse(JSON.stringify(state))`.
    fn round_trip(state: &GameState) -> GameState {
        serde_json::from_value(crate::js(state)).expect("the state round-trips")
    }

    /// The fused Tree's ingredient defs, read off the backrow card: after the activation the
    /// Tree's def id is composite (`pick+tree`), so no name lookup finds it.
    fn fused_parts(s: &Scenario) -> Vec<String> {
        let def_id = state_backrow_tree(s.state());
        match fused_id_parts(Some(s.state()), &def_id) {
            Some(parts) => parts,
            None => panic!("the Tree was not fused"),
        }
    }

    fn fused(s: &Scenario) -> bool {
        s.events().iter().any(|event| matches!(event, GameEvent::Fused { .. }))
    }

    mod m75_ever_growing_tree {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn activate_fuses_a_random_field_spell_never_itself() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "tree-grow",
                    "p1": { "hand": [TREE, FILLER], "library": filler(3) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(TREE, json!({}));
                let tree = s.card(TREE).id.clone();
                s.activate(tree.as_str(), json!({}));

                assert!(fused(&s));
                let parts = fused_parts(&s);
                assert_eq!(parts.len(), 2);
                // R387: the Tree is never its own pick — it goes in as the target, last.
                assert_ne!(parts[0], TREE);
                assert_eq!(parts[1], TREE);
                // Once a turn: a second activation is refused.
                s.expect_refused(|s| s.activate(tree.as_str(), json!({})));
            }

            #[test]
            fn r1087_r468_the_fused_definition_survives_a_json_round_trip() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "tree-round-trip",
                    "p1": { "hand": [TREE, FILLER], "library": filler(3) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(TREE, json!({}));
                s.activate(TREE, json!({}));
                let before = fused_parts(&s);

                let state = round_trip(s.state());
                let after = fused_id_parts(Some(&state), &state_backrow_tree(&state));
                assert_eq!(after, Some(before));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r1087_r469_the_pick_goes_in_on_its_radiant_face() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "tree-radiant",
                    "p1": {
                        "hand": [{ "def": TREE, "radiant": true }, FILLER],
                        "library": filler(3),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(TREE, json!({}));
                let tree = s.card(TREE).id.clone();
                s.activate(tree.as_str(), json!({}));

                assert!(fused(&s));
                // R469's mark: the pick went in Radiant.
                let def_id =
                    find_instance(s.state(), &tree).expect("the fused Tree").def_id.clone();
                assert!(def_id.contains('*'), "the fused id names a Radiant ingredient");
                s.expect_refused(|s| s.activate(tree.as_str(), json!({})));
            }
        }
    }

    /// The Tree's def id on the backrow of `state`.
    fn state_backrow_tree(state: &GameState) -> String {
        state.players[P1]
            .backrow
            .iter()
            .flatten()
            .find(|card| card.def_id.starts_with("t-") || card.def_id == TREE)
            .map(|card| card.def_id.clone())
            .expect("the Tree on the backrow")
    }
}
