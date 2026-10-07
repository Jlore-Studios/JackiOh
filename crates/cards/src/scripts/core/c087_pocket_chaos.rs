//! #87 Pocket Chaos (SPEC §8.4, §10.6, R4, R11, R12, R33, R73, R81, R88).
//!
//! (4). Base: "Choose one: Swap hero health, boards or decks with your opponent. Then add a Pocket
//! Chaos with a base cost (1) less than this one's to your opponent's hand, unless its base cost
//! would be (0). Exile this." Radiant: "… Then you may add a Pocket Chaos …" (§8's cell "You may
//! skip adding it"; patch v0.1.1 removed the Radiant face's "draw 1"; patch v0.2.9 costs it at (4)
//! and prices the gift, R742).
//!
//! §8's Conventions: the radiant cell restates the "add a Pocket Chaos" clause, so the Choose one and
//! the exile are kept unchanged. The radiant difference is one: the gift becomes optional.
//!
//! BOTH choices are declared play choices, not prompts. R81's card list names #87, and §10.6 is
//! explicit: "A card's own play choices (zone, X, embiggen, Tribute, declared targets and modes) are
//! not prompts; they travel in the `play` action". `Script.modes` is a LIST of `ModeDecl`, and
//! `playChoices.ts` enumerates the cross product of every declaration and refuses a play that does
//! not answer each one (`refuseModes`: "Pocket Chaos needs a mode choice for each of its 2"), so the
//! radiant face declares a second mode for the gift rather than opening a prompt mid-resolution.
//! That is also the only reading that keeps the two faces consistent: making the skip a prompt would
//! pause a resolution that the base face never pauses, and §10.6 reserves `PendingChoice` for
//! choices made DURING resolution (Discover, chained steps, Echo repeats, casts, triggers).
//!
//! Reading the answers: `chosenOptions(ctx)` returns the play's `modes` in declaration order. The
//! two option sets are disjoint, so this file never indexes into that list — `swap()` picks out the
//! one of "health" | "board" | "library" it recognises and the gift clause looks for SKIP by name.
//! A play that somehow carries no swap mode fizzles that clause and the rest of the card still
//! resolves (§6.3, §8 Conventions).
//!
//! What each swap does is R73's, and `effects/swap.ts` owns it, so nothing here re-states it:
//!   - health: the two values change places, armor stays with its hero; not damage and not "lose
//!     health", so no pipeline (R18).
//!   - board: zone contents change sides lane by lane in BOTH rows (§3.1), read whole and then
//!     placed, so control changes for everything including face-down traps — which stay face-down
//!     and become readable by their new controller only (R33) — while ownership does not (R12).
//!     Locks are zone flags and stay with their zones, and a card whose destination is Locked or
//!     reserved bounces to its controller's hand (R88, R4, R11, R747).
//!   - library: the two piles change places whole and in order, and each swapped card's owner
//!     becomes the player now holding it — R12's one exception (R73). Fatigue stays with the player.
//!
//! The gift is a fresh, non-Radiant card: `addToHand` creates a new instance of this definition in
//! the opponent's hand, and a full hand burns it (§2.4, R4). Radiant Pocket Chaos gives away a base
//! copy — R57's "carries the radiant flag" is about copies of an existing card, and nothing in this
//! card's text or the radiant cell says the gift is Radiant. Its base cost is priced, not copied:
//! the gift arrives with a `costOverride` of the cast copy's base cost less GIFT_DISCOUNT (R742),
//! kept in every zone (R78), so a gift cast in turn prices the next one down until (0) ends the chain.
//!
//! `def.id` is the definition this file already owns, so the gift needs no id literal and no second
//! catalog lookup.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-087";

/// R742: the gift's base cost is this much less than the cast copy's.
const GIFT_DISCOUNT: i32 = 1;

/// The three things #87 swaps. The names are `effects/swap.ts`'s `SwapWhat` values, which is what
/// lets `swap()` with no argument read the play's answer itself (§6.3 Choose one). (TS `SWAP_MODE`.)
fn swap_mode() -> ModeDecl {
    ModeDecl {
        kind: PromptKind::Mode,
        options: vec!["health".to_string(), "board".to_string(), "library".to_string()],
    }
}

/// The radiant face's second choice: hand the copy over, or keep it out of the opponent's hand.
const GIFT: &str = "gift";
const SKIP: &str = "skip";

/// TS `GIFT_MODE`.
fn gift_mode() -> ModeDecl {
    ModeDecl {
        kind: PromptKind::Mode,
        options: vec![GIFT.to_string(), SKIP.to_string()],
    }
}

/// R742: the gift with its base cost. The copy's base cost (R65's start: its `costOverride`, else
/// its printed cost) is GIFT_DISCOUNT less than the cast copy's, so the gifts chain down
/// (4) → (3) → (2) → (1); a copy whose base cost would be (0) is never added.
fn gift(ctx: &EffectContext<'_>) -> Vec<Effect> {
    let Some(this) = ctx.self_.as_ref() else {
        return vec![];
    };
    let cost = this.cost_override.unwrap_or_else(|| printed_cost(&ctx.state, this)) - GIFT_DISCOUNT;
    if cost <= 0 {
        return vec![];
    }
    vec![add_to_hand(json_as(json!({ "defId": ID, "player": "enemy", "costOverride": cost })))]
}

/// `radiant_face` is the whole of the radiant text: an optional gift.
fn chaos(radiant_face: bool) -> Script {
    Script {
        modes: if radiant_face {
            vec![swap_mode(), gift_mode()]
        } else {
            vec![swap_mode()]
        },
        cry: Some(hook(move |ctx| {
            // Only an explicit SKIP skips: the base clause is to add it, and the radiant cell makes that
            // optional rather than reversing it, so an unanswered gift mode still hands the copy over.
            let skipped = radiant_face && chosen_options(ctx).iter().any(|option| option == SKIP);
            let mut effects = vec![swap(json_as(json!({})))];
            if !skipped {
                effects.extend(gift(ctx));
            }
            effects.push(exile(json_as(json!({ "target": { "of": "self" } }))));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = chaos(false);
    let radiant = chaos(true);
    CardScripts { base, radiant }
}

// #87 Pocket Chaos (SPEC §8.5, BUILD M4-T4 row 87): "Health swap, lane-preserving board swap
// including face-down traps with locks staying put, library swap that transfers ownership of the
// swapped cards (R73); opponent gains a Pocket Chaos; exiled; radiant may skip the gift". Patch
// v0.1.1 removed the draw R275 had added to the radiant face.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CHAOS: &str = "core-087";

    // Inert fixtures: every unit below has a Cry and nothing else, and the harness's `field` setup
    // never fires a Cry. Sheepish is a Trap that watches for a Unit the opponent plays, so a Spell
    // never sets it off — which makes it a stable face-down card to swap.
    const GARY: &str = "core-004"; // 1/1
    const RENO: &str = "core-053"; // 4/6
    const POSTDOC: &str = "core-061"; // 2/4
    const SHEEPISH: &str = "core-041"; // Trap
    const MANA_WELL: &str = "core-006"; // Field Spell; start-of-turn only

    // §2.5: one always-playable card per hand keeps a scenario on the turn it started on.
    const FILLER: &str = "core-005";

    const SEED: &str = "chaos-87";

    /// An engine value as the JSON the TS test compares it with.
    fn js<T: serde::Serialize>(v: &T) -> Value {
        serde_json::to_value(v).expect("serialises")
    }

    fn defs(cards: Vec<CardInstance>) -> Vec<String> {
        cards.into_iter().map(|card| card.def_id).collect()
    }

    fn gifts_in(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        s.hand(Some(player))
            .into_iter()
            .filter(|card| card.def_id == CHAOS)
            .collect()
    }

    fn drew(s: &Scenario) -> bool {
        s.events().iter().any(|event| matches!(event, GameEvent::Drawn { .. }))
    }

    fn slot(player: PlayerId, lane: i32) -> ZoneSlot {
        ZoneSlot {
            player,
            row: Row::Units,
            lane,
        }
    }

    mod n87_pocket_chaos_declared_play_choices_r81_s10_6 {
        use super::*;

        #[test]
        fn r81_declares_the_choose_one_as_a_play_time_mode_not_a_prompt() {
            crate::register_all();
            let base = script().base;
            assert_eq!(
                js(&base.modes),
                json!([{ "kind": "mode", "options": ["health", "board", "library"] }])
            );
            // The base face has exactly one choice: the gift is unconditional.
            assert_eq!(base.modes.len(), 1);
        }

        #[test]
        fn r81_the_radiant_face_declares_a_second_mode_for_the_optional_gift() {
            crate::register_all();
            assert_eq!(
                js(&script().radiant.modes),
                json!([
                    { "kind": "mode", "options": ["health", "board", "library"] },
                    { "kind": "mode", "options": ["gift", "skip"] },
                ])
            );
        }
    }

    mod n87_pocket_chaos_base {
        use super::*;

        #[test]
        fn r73_swaps_the_two_heroes_health_and_leaves_each_hero_s_armor_where_it_was() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [CHAOS, FILLER], "health": 12, "armor": 2, "mana": 10 }, // (4): keep mana after the play so the turn does not auto-end (R82)
                "p2": { "hand": [FILLER], "health": 25, "armor": 0 },
            }));
            let this = s.card(CHAOS).clone();

            s.play(&this, json!({ "modes": ["health"] }));

            s.expect_health(P1, 25).expect_health(P2, 12);
            // R73: "armor stays with its hero", and this is not damage or "lose health" (R18).
            assert_eq!(s.state().players.p1.hero.armor, 2);
            assert_eq!(s.state().players.p2.hero.armor, 0);
            s.expect_events(json!(["cardPlayed", "swapped"]));
        }

        #[test]
        fn then_adds_a_pocket_chaos_to_the_opponent_s_hand_and_exiles_this_one() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [CHAOS, FILLER], "health": 12, "mana": 10 }, // (4): keep mana after the play so the turn does not auto-end (R82)
                "p2": { "hand": [FILLER], "health": 25 },
            }));
            let this = s.card(CHAOS).clone();

            s.play(&this, json!({ "modes": ["health"] }));

            // The gift is a fresh, non-Radiant instance owned by the opponent.
            let gifts = gifts_in(&s, P2);
            assert_eq!(gifts.len(), 1);
            assert_eq!(gifts.first().map(|card| card.owner), Some(P2));
            assert_eq!(gifts.first().map(|card| card.radiant), Some(false));
            assert_ne!(gifts.first().map(|card| card.id.clone()), Some(this.id.clone()));
            // R742: its base cost is (1) less than the (4) cast copy's.
            assert_eq!(gifts.first().and_then(|card| card.cost_override), Some(3));

            // §8.5: "exile this". The play pipeline must not then send it on to the graveyard.
            s.expect_in_zone(&this, "exile")
                .expect_events(json!(["swapped", "addedToHand", "exiled"]));
            assert_eq!(s.state().counters.exiled, 1);
        }

        #[test]
        fn the_base_face_draws_nothing() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [CHAOS, FILLER], "library": [GARY, RENO] },
                "p2": { "hand": [FILLER] },
            }));

            s.play(CHAOS, json!({ "modes": ["health"] }));

            assert!(!drew(&s));
            assert_eq!(defs(s.pile(P1, "library")), vec![GARY.to_string(), RENO.to_string()]);
            assert_eq!(defs(s.hand(Some(P1))), vec![FILLER.to_string()]);
        }

        #[test]
        fn r73_swaps_the_board_lane_by_lane_in_both_rows_control_changes_ownership_does_not() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": {
                    "hand": [CHAOS, FILLER],
                    "field": [{ "def": GARY, "lane": 1 }],
                    "backrow": [{ "def": SHEEPISH, "lane": 2 }],
                },
                "p2": {
                    "hand": [FILLER],
                    "field": [{ "def": RENO, "lane": 3 }],
                    "backrow": [{ "def": MANA_WELL, "lane": 4 }],
                },
            }));
            let gary = s.card(GARY).clone();
            let reno = s.card(RENO).clone();
            let trap = s.card(SHEEPISH).clone();
            let well = s.card(MANA_WELL).clone();

            s.play(CHAOS, json!({ "modes": ["board"] }));

            // Lane-preserving: the same row and lane on the other side of the centre line.
            assert_eq!(s.unit(P2, 1).map(|card| card.id.clone()), Some(gary.id.clone()));
            assert!(s.unit(P1, 1).is_none());
            assert_eq!(s.unit(P1, 3).map(|card| card.id.clone()), Some(reno.id.clone()));
            assert!(s.unit(P2, 3).is_none());
            assert_eq!(s.backrow(P2, 2).map(|card| card.id.clone()), Some(trap.id.clone()));
            assert!(s.backrow(P1, 2).is_none());
            assert_eq!(s.backrow(P1, 4).map(|card| card.id.clone()), Some(well.id.clone()));
            assert!(s.backrow(P2, 4).is_none());

            // R12, R73: control changed for everything; ownership changed for nothing.
            assert_eq!(s.card(&gary).controller, P2);
            assert_eq!(s.card(&gary).owner, P1);
            assert_eq!(s.card(&reno).controller, P1);
            assert_eq!(s.card(&reno).owner, P2);
            assert_eq!(s.card(&trap).controller, P2);
            assert_eq!(s.card(&trap).owner, P1);
        }

        #[test]
        fn r33_a_swapped_face_down_trap_stays_face_down_and_only_its_new_controller_may_read_it() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [CHAOS, FILLER], "backrow": [{ "def": SHEEPISH, "lane": 2 }] },
                "p2": { "hand": [FILLER] },
            }));
            let trap = s.card(SHEEPISH).clone();

            s.play(CHAOS, json!({ "modes": ["board"] }));

            // `faceUp` is untouched by a swap: the card is still hidden, just on the other side.
            assert_ne!(s.card(&trap).face_up, Some(true));
            assert_eq!(s.card(&trap).controller, P2);

            // §10.8: readability keys on the controller, so p2 now reads it and p1 no longer does.
            let for_p2 = js(&s.view(Some(P2)).you.backrow.get(1).cloned().flatten());
            let for_p1 = js(&s.view(Some(P1)).opponent.backrow.get(1).cloned().flatten());
            assert_eq!(for_p2["faceDown"], json!(false));
            assert_eq!(for_p1["faceDown"], json!(true));
        }

        #[test]
        fn r88_locks_stay_with_their_zones_so_a_card_whose_destination_is_locked_bounces_home() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [CHAOS, FILLER], "field": [{ "def": GARY, "lane": 1 }] },
                "p2": { "hand": [FILLER] },
            }));
            let gary = s.card(GARY).clone();
            // A Locked zone is ordinary state (#36 Magic Jammed locks one); this seeds it directly.
            lock_zone(s.state_mut(), &slot(P2, 1));

            s.play(CHAOS, json!({ "modes": ["board"] }));

            // R88, following R14: an ordinary return to the controller's hand (R747).
            s.expect_in_zone(&gary, "hand");
            assert!(s.hand(Some(P1)).iter().any(|card| card.id == gary.id));
            assert!(s.unit(P2, 1).is_none());
            // The lock is a zone flag: it neither travelled nor was cleared.
            assert!(is_locked(s.state(), &slot(P2, 1)));
            assert!(!is_locked(s.state(), &slot(P1, 1)));
            s.expect_events(json!(["swapped", "bounced"]));
        }

        #[test]
        fn r73_swaps_the_libraries_whole_and_in_order_and_each_swapped_card_changes_owner_r12() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [CHAOS, FILLER], "library": [GARY], "mana": 10 }, // (4): keep mana after the play so the turn does not auto-end (R82)
                "p2": { "hand": [FILLER], "library": [RENO, POSTDOC] },
            }));

            s.play(CHAOS, json!({ "modes": ["library"] }));

            // The piles changed places whole, keeping their order: library[0] is still the next draw.
            assert_eq!(defs(s.pile(P1, "library")), vec![RENO.to_string(), POSTDOC.to_string()]);
            assert_eq!(defs(s.pile(P2, "library")), vec![GARY.to_string()]);

            // R12's one exception: the owner of a swapped library card becomes the player holding it.
            assert!(s.pile(P1, "library").iter().all(|card| card.owner == P1));
            assert!(s.pile(P1, "library").iter().all(|card| card.controller == P1));
            assert_eq!(s.pile(P2, "library").first().map(|card| card.owner), Some(P2));

            // §2.4: fatigue belongs to the player, not the library.
            assert_eq!(s.state().players.p1.fatigue_count, 0);
            assert_eq!(s.state().players.p2.fatigue_count, 0);
        }

        #[test]
        fn s6_3_a_play_naming_no_swap_mode_fizzles_that_clause_the_gift_and_the_exile_still_happen() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [CHAOS, FILLER], "health": 12, "mana": 10 }, // (4): keep mana after the play so the turn does not auto-end (R82)
                "p2": { "hand": [FILLER], "health": 25 },
            }));
            let this = s.card(CHAOS).clone();

            // `playChoices` refuses a play that answers a declared mode with nothing, which is the engine's
            // own guard; the script's fizzle is the second line of defence §8's Conventions ask for.
            s.expect_refused_with(
                |s| {
                    s.play(&this, json!({ "modes": [] }));
                },
                "mode",
            );
            s.expect_health(P1, 12).expect_health(P2, 25);
        }
    }

    mod n87_pocket_chaos_radiant {
        use super::*;

        #[test]
        fn may_skip_adding_it_the_swap_and_the_exile_still_happen_the_opponent_gains_nothing() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [{ "def": CHAOS, "radiant": true }, FILLER], "health": 12, "library": [GARY], "mana": 10 }, // (4): keep mana after the play
                "p2": { "hand": [FILLER], "health": 25 },
            }));
            let this = s.card(CHAOS).clone();

            s.play(&this, json!({ "modes": ["health", "skip"] }));

            s.expect_health(P1, 25).expect_health(P2, 12);
            assert_eq!(gifts_in(&s, P2).len(), 0);
            s.expect_in_zone(&this, "exile")
                .expect_events(json!(["swapped", "exiled"]));
        }

        #[test]
        fn patch_v0_1_1_the_radiant_face_draws_nothing_gift_or_not() {
            crate::register_all();
            for gift in ["gift", "skip"] {
                let mut s = scenario(json!({
                    "seed": SEED,
                    "p1": { "hand": [{ "def": CHAOS, "radiant": true }, FILLER], "library": [GARY, RENO], "mana": 10 }, // (4): keep mana after the play
                    "p2": { "hand": [FILLER] },
                }));

                s.play(CHAOS, json!({ "modes": ["health", gift] }));

                assert!(!drew(&s));
                assert_eq!(defs(s.hand(Some(P1))), vec![FILLER.to_string()]);
                assert_eq!(defs(s.pile(P1, "library")), vec![GARY.to_string(), RENO.to_string()]);
            }
        }

        #[test]
        fn gift_is_still_an_option_and_the_radiant_copy_hands_over_a_base_one() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [{ "def": CHAOS, "radiant": true }, FILLER], "library": [GARY], "mana": 10 }, // (4): keep mana after the play
                "p2": { "hand": [FILLER], "library": [RENO, POSTDOC] },
            }));
            let this = s.card(CHAOS).clone();

            s.play(&this, json!({ "modes": ["library", "gift"] }));

            let gifts = gifts_in(&s, P2);
            assert_eq!(gifts.len(), 1);
            // R57's "a copy carries the radiant flag" is about copies of an existing card; the gift is a
            // fresh card, and neither #87's text nor its radiant cell makes it Radiant.
            assert_eq!(gifts.first().map(|card| card.radiant), Some(false));
            // R742: the Radiant cast copy's base cost is still (4), so its gift costs (3) too.
            assert_eq!(gifts.first().and_then(|card| card.cost_override), Some(3));
            // R73: the libraries swapped, whole and in order, each card now its new holder's (R12).
            assert_eq!(defs(s.pile(P1, "library")), vec![RENO.to_string(), POSTDOC.to_string()]);
            assert!(s.pile(P1, "library").iter().all(|card| card.owner == P1));
            assert_eq!(defs(s.pile(P2, "library")), vec![GARY.to_string()]);
            s.expect_in_zone(&this, "exile");
        }

        #[test]
        fn skipping_the_gift_does_not_skip_the_board_swap_either() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": {
                    "hand": [{ "def": CHAOS, "radiant": true }, FILLER],
                    "field": [{ "def": GARY, "lane": 2 }],
                    "library": [RENO],
                },
                "p2": { "hand": [FILLER] },
            }));
            let this = s.card(CHAOS).clone();
            let gary = s.card(GARY).clone();

            s.play(&this, json!({ "modes": ["board", "skip"] }));

            assert_eq!(s.unit(P2, 2).map(|card| card.id.clone()), Some(gary.id.clone()));
            assert_eq!(s.card(&gary).controller, P2);
            assert_eq!(gifts_in(&s, P2).len(), 0);
        }
    }

    mod n87_pocket_chaos_r742_the_gift_s_base_cost_patch_v0_2_9_issue_n44 {
        use super::*;

        #[test]
        fn r742_costs_4_and_the_gift_s_base_cost_is_1_less_than_the_cast_copy_s() {
            crate::register_all();
            assert_eq!(js(&crate::card_def(ID).cost), json!(4));
        }

        #[test]
        fn r742_the_gifts_chain_down_4_3_2_1_and_a_1_cast_adds_nothing() {
            crate::register_all();
            let deck = [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER];
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [CHAOS, FILLER], "library": deck, "mana": 10 },
                "p2": { "hand": [FILLER], "library": deck, "mana": 10 },
            }));

            // (4) → (3).
            s.play(CHAOS, json!({ "modes": ["health"] }));
            let mut gifts = gifts_in(&s, P2);
            assert_eq!(gifts.len(), 1);
            assert_eq!(gifts.first().and_then(|card| card.cost_override), Some(3));

            // (3) → (2). Every cast copy exiles itself, so each side's hand holds only the new gift.
            s.end_turn();
            let gift = gifts.first().cloned().expect("the (3) gift");
            s.play(&gift, json!({ "modes": ["health"] }));
            gifts = gifts_in(&s, P1);
            assert_eq!(gifts.len(), 1);
            assert_eq!(gifts.first().and_then(|card| card.cost_override), Some(2));

            // (2) → (1).
            s.end_turn();
            let gift = gifts.first().cloned().expect("the (2) gift");
            s.play(&gift, json!({ "modes": ["health"] }));
            gifts = gifts_in(&s, P2);
            assert_eq!(gifts.len(), 1);
            assert_eq!(gifts.first().and_then(|card| card.cost_override), Some(1));

            // (1) → nothing: a copy whose base cost would be (0) is never added.
            s.end_turn();
            let gift = gifts.first().cloned().expect("the (1) gift");
            s.play(&gift, json!({ "modes": ["health"] }));
            assert_eq!(gifts_in(&s, P1).len(), 0);
        }
    }

    mod n87_pocket_chaos_r312_the_owners_library_lists {
        use super::*;

        #[test]
        fn r312_a_swapped_library_is_unknown_to_its_new_owner_on_both_sides() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [CHAOS, FILLER], "library": [GARY], "mana": 10 }, // (4): keep mana after the play so the turn does not auto-end (R82)
                "p2": { "hand": [FILLER], "library": [RENO, POSTDOC] },
            }));
            assert_eq!(
                js(&s.view(Some(P1)).you.own_library),
                json!({ "cards": [{ "defId": GARY, "radiant": false, "count": 1 }], "unknown": 0 })
            );

            s.play(CHAOS, json!({ "modes": ["library"] }));

            // Each player now holds the other's old library, and was shown none of it.
            assert_eq!(js(&s.view(Some(P1)).you.own_library), json!({ "cards": [], "unknown": 2 }));
            assert_eq!(js(&s.view(Some(P2)).you.own_library), json!({ "cards": [], "unknown": 1 }));
            let mine = serde_json::to_string(&s.view(Some(P1))).expect("serialises");
            assert!(!mine.contains(&format!("\"{RENO}\"")));
            assert!(!mine.contains(&format!("\"{POSTDOC}\"")));
            let theirs = serde_json::to_string(&s.view(Some(P2))).expect("serialises");
            assert!(!theirs.contains(&format!("\"{GARY}\"")));
        }
    }
}
