//! #76 Field of Dreams (SPEC §8.3, R31, R4, R11, R50, §10.5).
//!
//! Base: "Replace your hand with the same number of Reminisce; exile this". The radiant cell is
//! "Radiant Reminisce", which restates only what the copies are, so every other clause is kept
//! (§8 Conventions) and the two faces differ by one flag.
//!
//! R31 is the whole card: "Replaced cards go to the GY (ruling), so Reminisce can find them." So the
//! replacement is a DISCARD of the whole hand, not an exile — the old hand lands in the graveyard,
//! which is exactly the pool #72 Reminisce discovers from (R50 reads the actual graveyard, so the
//! spell tokens among them are eligible too). A unit-token card in that hand ceases to exist instead
//! of reaching the graveyard, which is `discard`'s own R11 rule and not this card's business.
//!
//! N is the hand size AT RESOLUTION. §10.5 step 4 has already moved Field of Dreams out of the hand
//! into `resolving`, so it never counts itself, and the effects below are built before any of them
//! applies — the count is taken before the hand is emptied. N = 0 (the last card in hand) discards
//! nothing, adds nothing and still exiles this.
//!
//! The hand cap never bites here (R4): N ≤ HAND_CAP − 1 because Field of Dreams itself held a slot,
//! and the N copies arrive into a hand the discard has just emptied, so nothing is ever burned.
//! `add_to_hand` applies the cap regardless, so the rule is enforced either way.
//!
//! Missing verbs (see the report): `discard_hand({ player })` — a deterministic whole-hand discard.
//! `discard_random({ count: N })` would be wrong, not merely ugly: it burns N rng draws to reach a
//! deterministic outcome and so shifts `rng_cursor`, which breaks replay parity (§9.3, §10.7).

use jackioh_engine::effects::{add_to_hand, discard_hand, exile};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-076";

/// R31: the card the hand is replaced with — #72 Reminisce, by catalog id (TS read it off
/// `cardDef("core-072")`, never restating it; the catalog check holds the id to the catalog).
const REMINISCE: &str = "core-072";

/// §10.9: a hook may READ state to compute an effect's arguments; it never writes. This is the only
/// read this card makes — N, the caller's hand size at resolution — and it goes through the engine's
/// read-only `zone_count` (engine/src/query.rs), which is what keeps BUILD M3-T1's and REVIEW B1.7's
/// `state.players` grep over `src` clean.
fn hand_size_of(ctx: &EffectContext<'_>) -> i32 {
    zone_count(ctx.state, ctx.controller, OffFieldZone::Hand)
}

/// The two faces differ only in whether the copies are Radiant (§5.2, R74).
fn field_of_dreams(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let count = hand_size_of(ctx);
            let mut effects = Vec::new();
            // R31: the replaced hand goes to the graveyard, where Reminisce can find it.
            effects.push(discard_hand(json_as(json!({ "player": "self" }))));
            for _ in 0..count {
                effects.push(add_to_hand(json_as(json!({ "defId": REMINISCE, "radiant": radiant }))));
            }
            // "exile this": the spell leaves from `resolving` and never reaches the graveyard.
            effects.push(exile(json_as(json!({ "target": { "of": "self" } }))));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: field_of_dreams(false),
        radiant: field_of_dreams(true),
    }
}

// #76 Field of Dreams — SPEC §8.3, R31, R4, R11, R50.
//
// BUILD M4-T4: "Hand of N → N Reminisce, old cards in GY (R31); exiled; radiant gives radiant
// Reminisce". Covered here for base and radiant separately, plus N = 0 and the R4 hand cap.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const FIELD_OF_DREAMS: &str = "core-076";
    const REMINISCE: &str = "core-072";

    /// Four cards that are only ever hand filler here; none of them is played.
    const FILLER: [&str; 4] = ["core-001", "core-002", "core-003", "core-004"];
    /// Nine, so the hand is exactly HAND_CAP (10) with Field of Dreams in it (R4).
    const NINE: [&str; 9] = [
        "core-001", "core-002", "core-003", "core-004", "core-005", "core-006", "core-007", "core-008", "core-009",
    ];

    /// A unit on each side and a library, so no turn auto-ends underneath the assertions (R82).
    fn keep_busy() -> Value {
        json!({ "field": ["core-019"], "library": ["core-008", "core-008"] })
    }

    /// TS `{ ...KEEP_BUSY, hand }`.
    fn busy_with_hand(hand: Value) -> Value {
        let mut side = keep_busy();
        side["hand"] = hand;
        side
    }

    /// `scenario(opts)` with the shipped cards registered first (the TS globalSetup's `registerAll()`).
    fn setup(opts: Value) -> Scenario {
        crate::register_all();
        scenario(opts)
    }

    /// Field of Dreams (or its radiant copy) followed by `rest`, as a hand fixture.
    fn hand_of(first: Value, rest: &[&str]) -> Value {
        let mut hand = vec![first];
        hand.extend(rest.iter().map(|id| json!(id)));
        Value::Array(hand)
    }

    fn hand_def_ids(s: &Scenario) -> Vec<String> {
        s.pile(P1, "hand").into_iter().map(|card| card.def_id).collect()
    }

    fn graveyard_def_ids(s: &Scenario) -> Vec<String> {
        s.pile(P1, "graveyard").into_iter().map(|card| card.def_id).collect()
    }

    fn sorted(ids: &[&str]) -> Vec<String> {
        let mut ids: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
        ids.sort();
        ids
    }

    fn sorted_owned(mut ids: Vec<String>) -> Vec<String> {
        ids.sort();
        ids
    }

    mod field_of_dreams_base {
        use super::*;

        #[test]
        fn r31_replaces_a_hand_of_n_with_n_reminisce_and_puts_the_replaced_cards_in_the_graveyard() {
            let mut s = setup(json!({
                "p1": busy_with_hand(hand_of(json!(FIELD_OF_DREAMS), &FILLER)),
                "p2": keep_busy(),
            }));

            s.play(FIELD_OF_DREAMS, json!({}));

            // N = 4: the four filler cards are replaced, Field of Dreams never counts itself (§10.5 step 4).
            assert_eq!(hand_def_ids(&s), [REMINISCE, REMINISCE, REMINISCE, REMINISCE]);
            // R31: "Replaced cards go to the GY", which is the pool Reminisce discovers from (R50).
            assert_eq!(sorted_owned(graveyard_def_ids(&s)), sorted(&FILLER));
        }

        #[test]
        fn r31_the_new_reminisce_are_not_radiant_on_the_base_face() {
            let mut s = setup(json!({
                "p1": busy_with_hand(hand_of(json!(FIELD_OF_DREAMS), &FILLER)),
                "p2": keep_busy(),
            }));

            s.play(FIELD_OF_DREAMS, json!({}));

            let radiant: Vec<bool> = s.pile(P1, "hand").into_iter().map(|card| card.radiant).collect();
            assert_eq!(radiant, [false, false, false, false]);
        }

        #[test]
        fn exiles_itself_rather_than_going_to_the_graveyard() {
            let mut s = setup(json!({
                "p1": busy_with_hand(hand_of(json!(FIELD_OF_DREAMS), &FILLER)),
                "p2": keep_busy(),
            }));
            let self_ = s.card(FIELD_OF_DREAMS).clone();

            s.play(FIELD_OF_DREAMS, json!({}));

            s.expect_in_zone(&self_, "exile");
            assert!(!graveyard_def_ids(&s).contains(&FIELD_OF_DREAMS.to_string()));
            // The order the card states: the hand is discarded, then replaced, then this is exiled.
            s.expect_events(json!(["cardPlayed", "discarded", "enteredGraveyard", "exiled"]));
        }

        #[test]
        fn n_0_an_empty_hand_gets_no_reminisce_and_field_of_dreams_is_still_exiled() {
            let mut s = setup(json!({
                "p1": busy_with_hand(json!([FIELD_OF_DREAMS])),
                "p2": keep_busy(),
            }));
            let self_ = s.card(FIELD_OF_DREAMS).clone();

            s.play(FIELD_OF_DREAMS, json!({}));

            assert!(hand_def_ids(&s).is_empty());
            assert!(graveyard_def_ids(&s).is_empty());
            s.expect_in_zone(&self_, "exile");
        }

        #[test]
        fn r4_a_full_hand_of_10_becomes_9_reminisce_and_nothing_is_burned() {
            let mut s = setup(json!({
                "p1": busy_with_hand(hand_of(json!(FIELD_OF_DREAMS), &NINE)),
                "p2": keep_busy(),
            }));

            s.play(FIELD_OF_DREAMS, json!({}));

            // N ≤ HAND_CAP − 1 because Field of Dreams held a slot, and the copies arrive into an empty
            // hand, so the cap never bites: exactly 9 in hand and exactly the 9 originals in the GY.
            assert_eq!(hand_def_ids(&s), vec![REMINISCE; 9]);
            assert_eq!(sorted_owned(graveyard_def_ids(&s)), sorted(&NINE));
        }
    }

    mod field_of_dreams_radiant {
        use super::*;

        #[test]
        fn gives_radiant_reminisce_and_still_buries_the_replaced_hand_r31() {
            let mut s = setup(json!({
                "p1": busy_with_hand(hand_of(json!({ "def": FIELD_OF_DREAMS, "radiant": true }), &FILLER)),
                "p2": keep_busy(),
            }));

            s.play(FIELD_OF_DREAMS, json!({}));

            let hand = s.pile(P1, "hand");
            let ids: Vec<&str> = hand.iter().map(|card| card.def_id.as_str()).collect();
            assert_eq!(ids, [REMINISCE, REMINISCE, REMINISCE, REMINISCE]);
            // §5.2, R74: "Radiant Reminisce" is the radiant flag on each created copy and nothing else.
            let radiant: Vec<bool> = hand.iter().map(|card| card.radiant).collect();
            assert_eq!(radiant, [true, true, true, true]);
            assert_eq!(sorted_owned(graveyard_def_ids(&s)), sorted(&FILLER));
        }

        #[test]
        fn radiant_still_exiles_itself_and_n_0_still_creates_nothing() {
            let mut s = setup(json!({
                "p1": busy_with_hand(json!([{ "def": FIELD_OF_DREAMS, "radiant": true }])),
                "p2": keep_busy(),
            }));
            let self_ = s.card(FIELD_OF_DREAMS).clone();

            s.play(FIELD_OF_DREAMS, json!({}));

            assert!(hand_def_ids(&s).is_empty());
            s.expect_in_zone(&self_, "exile");
        }
    }
}
