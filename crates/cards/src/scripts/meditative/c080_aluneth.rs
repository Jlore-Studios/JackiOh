//! M #80 Aluneth (SPEC §8.8 row 80). (3) Field Spell, tags Quickdraw, Legendary.
//!
//! Base:    "Indestructible, Untributable, Immutable\nEnd of turn: Draw 3 cards."
//! Radiant: "Indestructible, Immutable\nActivate: Exile this.\nEnd of turn: Draw 3 cards."
//! Engine:  "Quickdraw (R640) deals it in the opening hand. An Indestructible Field Spell that draws
//!          3 at its controller's end of turn, each draw its own (burning past the cap, fatigue past
//!          the deck). Immutable blocks Transform, Vanilla, Fuse-onto, Buff, Nerf and KY's Constant
//!          (R23, R386). NEW: the keyword Untributable (R1220): no Tribute cost may take it, and every
//!          Sacrifice of it does nothing; it is not in R21's pool, and a Nerf never removes it. The
//!          Radiant face drops Untributable and adds an Activate (R384) that exiles it, with no Death.
//!          Voice: \"Come, child. Let us wreak havoc.\" (R204). Tunes: none."
//!
//! Indestructible, Untributable and Immutable are catalog keywords (§10.4 layers, R1220); the draw is
//! one `draw` of 3, which burns past the cap and takes fatigue past the deck card by card (§2.4);
//! Quickdraw is the static flag `setup.rs` reads (R640), beside the catalog tag filters see.

use jackioh_engine::effects::{TuneDirection, applicable_changes, draw, exile, transform};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-080";

/// What the end of its controller's turn draws.
const DRAWS: i32 = 3;

fn aluneth(radiant: bool) -> Script {
    let mut script = Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        end_of_turn: Some(hook(|_ctx| vec![draw(json_as(json!({ "count": DRAWS })))])),
        ..Script::default()
    };
    if radiant {
        script.activations = vec![ActivationDecl {
            id: "exile".to_string(),
            label: "Exile this".to_string(),
            uses: ActivationUses::Count(1),
            cost: None,
            targets: vec![],
            modes: vec![],
            can_activate: None,
            has: None,
            run: hook(|_ctx| vec![exile(json_as(json!({ "target": { "of": "self" } })))]),
        }];
    }
    script
}

pub fn script() -> CardScripts {
    CardScripts {
        base: aluneth(false),
        radiant: aluneth(true),
    }
}

// M #80 Aluneth — SPEC §8.8 row 80, BUILD M10 row M 80: "Quickdraw: in the opening hand (R640); at
// your end of turn only, three draws, burning past the cap and taking fatigue past the deck;
// destroy effects leave it (Indestructible); no Tribute cost can use it and a Sacrifice of it does
// nothing (R1220); a Nerf, a Buff, a Transform, a Vanilla and KY's Constant leave it unchanged;
// exile, bounce and steal work; Untributable is never in R21's pool; radiant no Untributable, and
// Activate: exile this".
#[cfg(test)]
mod tests {
    use super::{DRAWS, ID};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008";
    const FILLER: &str = "core-016";
    const JAMMED: &str = "core-036"; // destroy target backrow card, Lock its zone

    fn spare() -> Value {
        json!({ "hand": [FILLER], "library": [VANILLA, VANILLA, VANILLA] })
    }

    /// Aluneth in p1's backrow, both sides with hands and libraries to draw into.
    fn fielded(seed: &str, radiant: bool) -> Scenario {
        crate::scenario(json!({
            "seed": seed,
            "p1": { "backrow": [{ "def": ID, "radiant": radiant }], "hand": [FILLER], "library": [VANILLA, VANILLA, VANILLA, VANILLA] },
            "p2": { "hand": [FILLER], "library": [VANILLA, VANILLA, VANILLA, VANILLA] },
        }))
    }

    mod m80_aluneth {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn quickdraw_deals_it_in_the_opening_hand() {
                let _preview = preview_sets(&[SetName::Meditative]);
                crate::register_all();
                // `setup` moves every library card carrying `quickdraw` into the opening hand; what
                // this card owes is the flag on both faces, read through the engine's own reader.
                let s = crate::scenario(json!({
                    "p1": { "library": [ID, { "def": ID, "radiant": true }] },
                }));
                let library = s.pile(P1, "library");
                assert_eq!(library.len(), 2);
                for card in &library {
                    assert_eq!(flags_of(s.state(), card).quickdraw, Some(true));
                }
            }

            #[test]
            fn draws_three_at_its_controllers_end_of_turn_only() {
                let mut s = fielded("aluneth-draws", false);
                assert_eq!(s.hand(P1).len(), 1);
                s.end_turn(); // p1's end of turn: three draws.
                assert_eq!(s.hand(P1).len(), 1 + DRAWS as usize);
                s.end_turn(); // p2's end of turn: none for p1.
                assert_eq!(s.hand(P1).len(), 1 + DRAWS as usize);
            }

            #[test]
            fn burns_past_the_hand_cap_and_takes_fatigue_past_the_deck() {
                crate::register_all();
                let mut s = crate::scenario(json!({
                    "seed": "aluneth-burn",
                    "p1": {
                        "backrow": [ID],
                        "hand": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                        "library": [VANILLA],
                    },
                    "p2": { "hand": [FILLER], "library": [VANILLA, VANILLA] },
                }));
                let health = s.state().players.p1.hero.health;
                s.end_turn();
                // Ten in hand already: the one library card burns the three draws away, and the two
                // draws past the empty deck are fatigue.
                assert_eq!(s.hand(P1).len(), 10);
                assert!(s.state().players.p1.hero.health < health);
            }

            #[test]
            fn a_destroy_leaves_it() {
                let mut s = crate::scenario(json!({
                    "seed": "aluneth-stays",
                    "active": "p2",
                    "p1": { "backrow": [ID], "hand": [FILLER], "library": [VANILLA, VANILLA] },
                    "p2": { "hand": [JAMMED, FILLER], "library": [VANILLA, VANILLA], "mana": 10 },
                }));
                let aluneth = s.card(ID).id.clone();
                s.play(
                    JAMMED,
                    json!({ "targets": [{ "pick": "instance", "instanceId": aluneth }] }),
                );
                s.expect_in_zone(ID, "field");
            }

            #[test]
            fn a_degrade_and_a_transform_leave_it_unchanged() {
                let s = fielded("aluneth-tune", false);
                let card = s.card(ID).clone();
                assert!(
                    applicable_changes(s.state(), &card, TuneDirection::Degrade).is_empty(),
                    "Immutable: no Degrade reaches it",
                );
                // A Transform is refused the same way: nothing about the card moves (R23).
                let mut state = s.state().clone();
                let mut events: Vec<GameEvent> = Vec::new();
                let mut rng = Rng::new(&state.seed, state.rng_cursor);
                {
                    let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
                    let mut ctx = make_context(
                        &mut sink,
                        None,
                        HookOptions {
                            controller: Some(P2),
                            ..HookOptions::default()
                        },
                    );
                    apply_effects(
                        &[transform(json_as(json!({
                            "instanceId": card.id,
                            "defId": VANILLA,
                        })))],
                        &mut ctx,
                    );
                }
                let card = find_instance(&state, &card.id).expect("still there");
                assert_eq!(card.def_id, ID);
            }

            #[test]
            fn r1220_a_sacrifice_of_it_does_nothing() {
                let s = fielded("aluneth-sacrifice", false);
                let mut state = s.state().clone();
                let mut events: Vec<GameEvent> = Vec::new();
                let mut rng = Rng::new(&state.seed, state.rng_cursor);
                {
                    let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
                    let card = find_instance(sink.state, &s.card(ID).id).cloned().expect("there");
                    sacrifice_now(&mut sink, &card);
                }
                assert!(find_instance(&state, &s.card(ID).id).is_some());
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn radiant_activate_exiles_it() {
                let mut s = fielded("aluneth-exile", true);
                s.activate(ID, json!({ "ability": "exile" }));
                s.expect_in_zone(ID, "exile");
            }

            #[test]
            fn the_face_has_no_untributable() {
                let def = crate::card_def(ID);
                let kinds = |keywords: &[Keyword]| keywords.iter().map(Keyword::kind).collect::<Vec<_>>();
                assert!(kinds(&def.base.keywords).contains(&KeywordKind::Untributable));
                assert!(!kinds(&def.radiant.keywords).contains(&KeywordKind::Untributable));
                assert!(kinds(&def.radiant.keywords).contains(&KeywordKind::Indestructible));
                assert!(kinds(&def.radiant.keywords).contains(&KeywordKind::Immutable));
            }
        }
    }
}
