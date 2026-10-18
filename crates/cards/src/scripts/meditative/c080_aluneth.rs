//! M #80 Aluneth (SPEC §8.8 row 80). (2) Field Spell, tags Quickdraw, Legendary.
//!
//! Base:    "Indestructible, Untributable, Immutable\nEnd of turn: Draw 3. They cost ({discount}) less."
//! Radiant: "Indestructible, Immutable\nActivate: Exile this.\nEnd of turn: Draw 3. They cost ({discount})
//!          less."
//! Engine:  "Quickdraw (R640) deals it in the opening hand. An Indestructible Field Spell that draws
//!          3 at its controller's end of turn, each draw its own (burning past the cap, fatigue past
//!          the deck), and each card a draw puts in the hand costs (1) less. Immutable blocks
//!          Transform, Vanilla, Fuse-onto, Buff, Nerf and KY's Constant (R23, R386). NEW: the keyword
//!          Untributable (R1220): no Tribute cost may take it, and every Sacrifice of it does nothing;
//!          it is not in R21's pool, and a Nerf never removes it. The Radiant face drops Untributable
//!          and adds an Activate (R384) that exiles it, with no Death.
//!          Voice: \"Come, child. Let us wreak havoc.\" (R204). Tunes: none."
//!
//! Indestructible, Untributable and Immutable are catalog keywords (§10.4 layers, R1220); the draws are
//! three `drawPriced` draws, C+ #65.2's: each burns past the cap and takes fatigue past the deck (§2.4),
//! and each prices only the card it put in the hand, `costMod` −`discount`, the declared number (patch
//! v0.3.12, #1194); a card cast on draw, a burned card and a fatigue draw get nothing. Quickdraw is the
//! static flag `setup.rs` reads (R640), beside the catalog tag filters see. `script_v0_3_7b` keeps the
//! undiscounted draw for the replays of v0.3.7b and before (R767000).

use jackioh_engine::effects::{draw, draw_priced, exile};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-080";

/// What the end of its controller's turn draws.
const DRAWS: i32 = 3;

/// "Draw 3. They cost ({discount}) less.": one `drawPriced` per draw (§2.4), each pricing the card it
/// put in the hand (C+ #65.2's), `discount` the declared number (R386).
fn draws_discounted(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let discount = -param(ctx, "discount");
    (0..DRAWS)
        .map(|_| draw_priced(json_as(json!({ "costMod": discount }))))
        .collect()
}

/// R767000: "Draw 3." as v0.3.7b and every version before it printed it, with no discount.
fn draws_v0_3_7b(_ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    vec![draw(json_as(json!({ "count": DRAWS })))]
}

fn aluneth(radiant: bool, draws: fn(&mut EffectContext<'_>) -> Vec<Effect>) -> Script {
    let mut script = Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        end_of_turn: Some(hook(draws)),
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
        base: aluneth(false, draws_discounted),
        radiant: aluneth(true, draws_discounted),
    }
}

/// R767000: the scripts v0.3.7b and every version before it ran, whose end of turn drew 3 with no
/// discount (patch v0.3.12, #1194); `KEPT_SCRIPTS` folds those versions' replays under them.
pub fn script_v0_3_7b() -> CardScripts {
    CardScripts {
        base: aluneth(false, draws_v0_3_7b),
        radiant: aluneth(true, draws_v0_3_7b),
    }
}

// M #80 Aluneth — SPEC §8.8 row 80, BUILD M10 row M 80: "Quickdraw: in the opening hand (R640); at
// your end of turn only, three draws, burning past the cap and taking fatigue past the deck, each card
// they put in the hand costing (1) less (the declared `discount`) and no other draw's;
// destroy effects leave it (Indestructible); no Tribute cost can use it and a Sacrifice of it does
// nothing (R1220); a Nerf, a Buff, a Transform, a Vanilla and KY's Constant leave it unchanged;
// exile, bounce and steal work; Untributable is never in R21's pool; radiant no Untributable, and
// Activate: exile this".
#[cfg(test)]
mod tests {
    use super::{DRAWS, ID, script};
    use jackioh_engine::effects::{TuneDirection, applicable_changes, transform};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008";
    const MENACE: &str = "core-019"; // (3) Unit
    const FILLER: &str = "core-016";
    const JAMMED: &str = "core-036"; // destroy target backrow card, Lock its zone
    const NETHER: &str = "core-088"; // Twisting Nether: destroy all permanents

    /// Aluneth in p1's backrow, both sides with hands and libraries to draw into.
    fn fielded(seed: &str, radiant: bool) -> Scenario {
        crate::scenario(json!({
            "seed": seed,
            "p1": { "backrow": [{ "def": ID, "radiant": radiant }], "hand": [FILLER], "library": [VANILLA, VANILLA, VANILLA, VANILLA] },
            "p2": { "hand": [FILLER], "library": [VANILLA, VANILLA, VANILLA, VANILLA] },
        }))
    }

    /// The costs of the Menaces in p1's hand, lowest first.
    fn menace_costs(s: &Scenario) -> Vec<i32> {
        let mut costs: Vec<i32> = s
            .hand(P1)
            .iter()
            .filter(|card| card.def_id == MENACE)
            .map(|card| effective_cost(s.state(), card, Default::default()))
            .collect();
        costs.sort();
        costs
    }

    /// Aluneth's end of turn draws three (3) Menaces: each costs (1) less; p1's next turn's own draw,
    /// a fourth, does not.
    fn discounted_draws(radiant: bool) {
        crate::register_all();
        let mut s = crate::scenario(json!({
            "seed": "aluneth-discount",
            "p1": {
                "backrow": [{ "def": ID, "radiant": radiant }],
                "hand": [FILLER],
                "library": [MENACE, MENACE, MENACE, MENACE, MENACE],
            },
            "p2": { "hand": [FILLER], "library": [VANILLA, VANILLA, VANILLA, VANILLA] },
        }));
        s.end_turn();
        assert_eq!(menace_costs(&s), vec![2, 2, 2]);
        s.end_turn();
        assert_eq!(menace_costs(&s), vec![2, 2, 2, 3]);
    }

    mod m80_aluneth {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn quickdraw_deals_it_in_the_opening_hand() {
                let _preview = preview_sets(&[SetName::Meditative]);
                crate::register_all();
                // R640: `setup` deals every library card carrying `quickdraw` into the opening hand,
                // the base face and the Radiant one alike.
                let scripts = script();
                assert_eq!(scripts.base.flags().quickdraw, Some(true));
                assert_eq!(scripts.radiant.flags().quickdraw, Some(true));
                // Plain Core Units, then Aluneth: one copy of each (§2.6 L3).
                let others = [
                    "core-002", "core-005", "core-006", "core-008", "core-011", "core-012", "core-013", "core-015",
                    "core-016", "core-019", "core-020", "core-025", "core-026", "core-032", "core-036", "core-043",
                    "core-044", "core-053", "core-055",
                ];
                let mut deck: Vec<String> = others.iter().map(|id| id.to_string()).collect();
                deck.push(ID.to_string());
                let created = create_game(&CreateGameOptions {
                    seed: "aluneth-quickdraw".to_string(),
                    decks: (deck.clone(), deck),
                    ..CreateGameOptions::default()
                });
                let state = begin_game(&created).state;
                assert!(state.players.p1.hand.iter().any(|card| card.def_id == ID));
                assert!(state.players.p1.library.iter().all(|card| card.def_id != ID));
            }

            #[test]
            fn draws_three_at_its_controllers_end_of_turn_only() {
                let mut s = fielded("aluneth-draws", false);
                assert_eq!(s.hand(P1).len(), 1);
                s.end_turn(); // p1's end of turn: three draws.
                assert_eq!(s.hand(P1).len(), 1 + DRAWS as usize);
                s.end_turn(); // p2's end of turn: none for p1, only its own turn's draw (§2.2).
                assert_eq!(s.hand(P1).len(), 1 + DRAWS as usize + 1);
            }

            #[test]
            fn each_card_it_draws_costs_1_less_and_no_other_draw_does() {
                discounted_draws(false);
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
            fn r781001_a_spell_cannot_target_it_and_a_destroy_of_every_permanent_leaves_it() {
                let mut s = crate::scenario(json!({
                    "seed": "aluneth-stays",
                    "active": "p2",
                    "p1": { "backrow": [ID], "hand": [FILLER], "library": [VANILLA, VANILLA] },
                    "p2": { "hand": [JAMMED, NETHER, FILLER], "library": [VANILLA, VANILLA], "mana": 10 },
                }));
                let aluneth = s.card(ID).id.clone();
                // Immutable: a Spell can't target it in play (R781001)…
                s.expect_refused(|s| {
                    s.play(
                        JAMMED,
                        json!({ "targets": [{ "pick": "instance", "instanceId": aluneth }] }),
                    )
                });
                // …and an effect on every permanent reaches it, which Indestructible survives.
                s.play(NETHER, json!({}));
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
            fn each_card_it_draws_costs_1_less_too() {
                discounted_draws(true);
            }

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
