//! M #75 Ever Growing Tree (SPEC §8.8 row 75, R1087): (4) Field Spell, Legendary.
//!
//! Base:    "Activate: Fuse a random Field Spell into this."
//! Radiant: "Activate: Fuse a random Radiant Field Spell into this."
//! Engine: Activate (R384, once a turn). `fuse_random_into` with the Tree as the kept instance
//! (R77, R102): one random non-token Field Spell of every set, never this card or an ingredient
//! already in it (R387); on the Radiant face the pick goes in on its Radiant face (R469). A Cry
//! never runs, auras and turn lines run, fused Activates share the Tree's count, and a fused
//! keyword applies at once (R1087).

pub const ID: &str = "meditative-075";

// M #75 Ever Growing Tree — SPEC §8.8 row 75, BUILD M10 row M 75: "Activate, once a turn in your
// main phase: one random non-token Field Spell (never itself or one already fused in) is fused
// into it, its aura and turn lines working from then on, its Cry never running; a fused Activate
// shares the Tree's once-a-turn use; a fused keyword applies at once (R1087); the fused definition
// survives a JSON round trip (R468); radiant the Field Spell goes in on its Radiant face (R469)".
#[cfg(test)]
mod tests {
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

    const VANILLA: &str = "core-008"; // (1) 4/4.
    const MANA_WELL: &str = "core-006";
    const WEAPONS: &str = "core-014";
    const ANTI_ONESHOT: &str = "core-073";
    const GOING_LONG: &str = "core-084";
    const THRIVE: &str = "classic-081";
    const WARAXE: &str = "meditative-066";
    /// The Power to Thrive's Activate, as the fused Tree carries it.
    const THRIVE_ABILITY: &str = "thrive";

    /// A Tree on p1's backrow and a 4/4 beside it, with the catalog's Field Spells narrowed to the
    /// Tree and `keep`, so the random pick is the one the clause is about.
    fn narrowed(seed: &str, keep: &[&str]) -> Scenario {
        crate::register_all();
        let catalog: CardDefs = registered_catalog()
            .iter()
            .filter(|(id, def)| {
                def.type_ != CardType::FieldSpell || id.as_str() == TREE || keep.contains(&id.as_str())
            })
            .map(|(id, def)| (id.clone(), def.clone()))
            .collect();
        register_catalog(catalog);
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [FILLER, FILLER], "backrow": [TREE], "field": [VANILLA], "library": filler(4) },
            "p2": { "hand": [FILLER, FILLER], "library": filler(4) },
        }))
    }

    /// The fused Tree's ingredient defs, read by its instance id.
    fn parts_of(s: &Scenario, tree: &str) -> Vec<String> {
        let def_id = s.card(tree).def_id.clone();
        fused_id_parts(Some(s.state()), &def_id).unwrap_or_else(|| panic!("{def_id} is not fused"))
    }

    fn has_keyword(keywords: &[Keyword], kind: KeywordKind) -> bool {
        keywords.iter().any(|keyword| keyword.kind() == kind)
    }

    fn fused(s: &Scenario) -> bool {
        s.events()
            .iter()
            .any(|event| matches!(event, GameEvent::Fused { .. }))
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
            #[test]
            fn once_a_turn_in_your_main_phase_a_second_activation_or_one_on_their_turn_is_refused() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "tree-once",
                    "p1": { "hand": [FILLER], "backrow": [TREE], "library": filler(3) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                let tree = s.card(TREE).id.clone();
                // On the opponent's turn the Tree cannot be activated.
                s.end_turn();
                assert_eq!(s.state().active, PlayerId::P2);
                s.expect_refused(|s| s.activate(tree.as_str(), json!({})));
                // Back on its controller's turn: once, then refused.
                s.end_turn();
                s.activate(tree.as_str(), json!({}));
                assert!(fused(&s));
                s.expect_refused(|s| s.activate(tree.as_str(), json!({})));
            }

            #[test]
            fn never_a_field_spell_already_fused_in() {
                // Two Field Spells besides the Tree: the second activation must take the other one.
                let mut s = narrowed("tree-twice", &[MANA_WELL, GOING_LONG]);
                let tree = s.card(TREE).id.clone();
                s.activate(tree.as_str(), json!({}));
                s.end_turn();
                s.end_turn();
                s.activate(tree.as_str(), json!({}));
                // The second fusion nests the first composite: flatten it.
                let mut parts = Vec::new();
                let mut todo = parts_of(&s, &tree);
                while let Some(part) = todo.pop() {
                    match fused_id_parts(Some(s.state()), &part) {
                        Some(inner) => todo.extend(inner),
                        None => parts.push(part),
                    }
                }
                assert_eq!(parts.len(), 3, "two ingredients and the Tree: {parts:?}");
                assert!(parts.contains(&MANA_WELL.to_string()), "{parts:?}");
                assert!(parts.contains(&GOING_LONG.to_string()), "{parts:?}");
                assert_eq!(parts.iter().filter(|part| part.as_str() == TREE).count(), 1);
            }

            #[test]
            fn r1087_a_fused_aura_works_from_then_on() {
                // Jlockeed's Weapons: "Aura: Your Units have +4 Attack, Rush and First Strike."
                let mut s = narrowed("tree-aura", &[WEAPONS]);
                assert_eq!(s.stats(VANILLA).attack, 4);
                let tree = s.card(TREE).id.clone();
                s.activate(tree.as_str(), json!({}));
                assert_eq!(parts_of(&s, &tree)[0], WEAPONS);
                assert_eq!(
                    s.stats(VANILLA).attack,
                    8,
                    "the fused aura reaches the Unit at once"
                );
                s.end_turn();
                s.end_turn();
                assert_eq!(s.stats(VANILLA).attack, 8, "and keeps working");
            }

            #[test]
            fn r1087_a_fused_turn_line_works_from_then_on() {
                // Mana Well: "Start of turn: Gain 1 mana."
                let mut s = narrowed("tree-turn-line", &[MANA_WELL]);
                let tree = s.card(TREE).id.clone();
                s.activate(tree.as_str(), json!({}));
                assert_eq!(parts_of(&s, &tree)[0], MANA_WELL);
                s.end_turn();
                s.end_turn();
                let mana = &s.state().players[P1].mana;
                assert_eq!(mana.current, mana.max + 1, "the fused start-of-turn line gave 1");
            }

            #[test]
            fn r1087_a_fused_cry_never_runs() {
                // Anti-oneshot Armor: "Cry: Draw 1." Fused into a Tree already on the field, it
                // draws nothing.
                let mut s = narrowed("tree-cry", &[ANTI_ONESHOT]);
                let hand = s.hand(P1).len();
                let library = s.pile(P1, "library").len();
                let tree = s.card(TREE).id.clone();
                s.activate(tree.as_str(), json!({}));
                assert_eq!(parts_of(&s, &tree)[0], ANTI_ONESHOT);
                assert_eq!(s.hand(P1).len(), hand, "no card drawn");
                assert_eq!(s.pile(P1, "library").len(), library);
                assert!(
                    !s.events()
                        .iter()
                        .any(|event| matches!(event, GameEvent::Drawn { .. })),
                    "no draw event"
                );
            }

            #[test]
            fn r1087_a_fused_activate_shares_the_trees_once_a_turn_use() {
                // The Power to Thrive: "Activate: Choose one: heal, draw or gain mana."
                let mut s = narrowed("tree-shared", &[THRIVE]);
                let tree = s.card(TREE).id.clone();
                s.activate(tree.as_str(), json!({ "ability": "grow" }));
                assert_eq!(parts_of(&s, &tree)[0], THRIVE);
                // The Tree's use is spent, so the fused Activate is refused this turn.
                s.expect_refused(|s| {
                    s.activate(
                        tree.as_str(),
                        json!({ "ability": THRIVE_ABILITY, "modes": ["draw"] }),
                    )
                });
                // Next turn the fused Activate spends the one use, and the Tree's own is refused.
                s.end_turn();
                s.end_turn();
                let hand = s.hand(P1).len();
                s.activate(
                    tree.as_str(),
                    json!({ "ability": THRIVE_ABILITY, "modes": ["draw"] }),
                );
                assert_eq!(s.hand(P1).len(), hand + 1, "the fused Activate drew");
                s.expect_refused(|s| s.activate(tree.as_str(), json!({ "ability": "grow" })));
            }

            #[test]
            fn r1087_a_fused_keyword_applies_at_once() {
                // Fiery Waraxe (3/2): "Animated on your turn". Fused in, the Tree animates at
                // its controller's next start of turn with the summed stats (MD-D34).
                let _preview = preview_sets(&[SetName::Meditative]);
                let mut s = narrowed("tree-keyword", &[WARAXE]);
                let tree = s.card(TREE).id.clone();
                s.activate(tree.as_str(), json!({}));
                assert_eq!(parts_of(&s, &tree)[0], WARAXE);
                assert!(
                    has_keyword(&s.stats(tree.as_str()).keywords, KeywordKind::AnimatedOnYourTurn),
                    "the keyword is on the Tree at once"
                );
                assert!(!is_animated(s.state(), s.card(tree.as_str())));
                s.end_turn();
                s.end_turn();
                assert!(
                    is_animated(s.state(), s.card(tree.as_str())),
                    "animated at the next start"
                );
                assert_eq!(s.stats(tree.as_str()).attack, 3);
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
                let def_id = find_instance(s.state(), &tree)
                    .expect("the fused Tree")
                    .def_id
                    .clone();
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
