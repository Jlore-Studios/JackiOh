//! FvH #57.2 Tackling (SPEC §8.14 row 57.2; §2.4, R4586005, R5238007, R3951001, R5255002): (0) Spell,
//! Other-Origins, Token (printed Epic), a token FvH #57 Otherworldly Hunting adds.
//!   Base:    "Look at the top three cards of your deck. Add one of them to your hand and discard the
//!            others."
//!   Radiant: "Look at the top three cards of your deck. Add one of them to your hand and make it
//!            Radiant. Discard the others."
//!
//! Engine: a `choosePick` over the top three of its caster's library in library order, offered to the
//! caster alone and drawing no rng (R4586005); a deck of fewer offers what it has, an empty one
//! nothing. The cards looked at ride the prompt's data, so the answer's step reads the same three
//! however its list is rebuilt. The pick moves to the hand by `addToHand` of its instance (Core #51 KY's
//! Private Tutor's move, which keeps its identity and is no steal), no draw (R5255002), and on the
//! Radiant face is made Radiant first (R74); a full hand burns it (§2.4). The others go to the
//! graveyard by `discardLibrary` with their ids (R5238007), which is no discard from a hand (R3951001).
//! "Three" and "one" are the card's words for the counts below, not declared numbers. Built by S05
//! (#5255).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-057-2";

/// The resume step answering the look's pick.
const TACKLE: &str = "tackle";
/// The data key holding the ids of the cards looked at, top first.
const LOOKED: &str = "looked";
/// How many cards off the top of the deck the look takes.
const LOOK: i32 = 3;
/// How many of them go to the hand.
const KEEP: i32 = 1;

/// The cards the look took, top first, as the Cry saw them.
fn looked_ids(ctx: &EffectContext<'_>) -> Vec<String> {
    ctx.data
        .get(LOOKED)
        .and_then(Value::as_array)
        .map(|ids| ids.iter().filter_map(|id| id.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

/// The cards the answer picked.
fn picked_ids(ctx: &EffectContext<'_>) -> Vec<String> {
    ctx.targets
        .iter()
        .filter_map(|selection| match selection {
            Selection::Instance { instance_id } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

fn tackling(radiant: bool) -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let looked: Vec<String> = zone_cards(ctx.state, ctx.controller, OffFieldZone::Library)
                .into_iter()
                .take(LOOK as usize)
                .map(|card| card.id)
                .collect();
            let mut args = json!({
                "step": TACKLE,
                "from": [{ "zone": "library", "player": "self", "top": LOOK }],
                "min": KEEP,
                "max": KEEP,
                "prompt": "Tackling: add one to your hand, the others to the graveyard",
            });
            args["data"][LOOKED] = json!(looked);
            vec![choose_pick(json_as(args))]
        })),
        resume: IndexMap::from([(
            TACKLE,
            hook(move |ctx| {
                let picked = picked_ids(ctx);
                // Always a list, even an empty one: absent, `discardLibrary` takes the whole deck.
                let others: Vec<String> = looked_ids(ctx)
                    .into_iter()
                    .filter(|id| !picked.contains(id))
                    .collect();
                vec![
                    add_to_hand(json_as(json!({ "instance": { "of": "chosen" }, "radiant": radiant }))),
                    discard_library(json_as(json!({ "instanceIds": others }))),
                ]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: tackling(false),
        radiant: tackling(true),
    }
}

// FvH #57.2 Tackling — SPEC §8.14 row 57.2 (R4586005, R5238007, R3951001, R5255002).
#[cfg(test)]
mod tests {
    use super::{ID, TACKLE};
    use jackioh_engine::testkit::*;

    use crate::{def_ids, scenario};

    const P1: PlayerId = PlayerId::P1;

    const TOP_A: &str = "stand-in (1) 1/1";
    const TOP_B: &str = "stand-in (2) 2/2";
    const TOP_C: &str = "stand-in (3) 3/3";
    const REST_D: &str = "stand-in (4) 4/4";
    const REST_E: &str = "stand-in (5) 5/5";
    /// A textless (0) Spell: something left to play, so no turn ends by itself.
    const FILLER: &str = "stand-in (0) Spell";

    /// p1 casts Tackling over `library` (top first); the look's pick is open.
    fn tackled(radiant: bool, library: Value) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": library },
            "p2": { "hand": [FILLER] },
        }));
        s.play(ID, json!({}));
        s
    }

    /// The open pick's cards, in the order it offers them.
    fn looked_at(s: &Scenario) -> Vec<String> {
        let pending = must(s.state().pending.clone(), "the look's pick");
        assert_eq!(pending.resume.step, TACKLE);
        let selections: Vec<Selection> = pending.options.iter().map(|option| option.selection.clone()).collect();
        pick::instance_ids(&selections)
    }

    /// The def ids of the cards `ids` name.
    fn defs_of(s: &Scenario, ids: &[String]) -> Vec<String> {
        ids.iter().map(|id| s.card(id.as_str()).def_id.clone()).collect()
    }

    /// p1's graveyard but Tackling itself.
    fn discarded_from_the_deck(s: &Scenario) -> Vec<String> {
        def_ids(&s.pile(P1, "graveyard")).into_iter().filter(|def| def != ID).collect()
    }

    /// p1's hand but the filler.
    fn kept(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id != FILLER).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4586005_the_top_three_are_offered_in_library_order() {
            let s = tackled(false, json!([TOP_A, TOP_B, TOP_C, REST_D, REST_E]));
            let looked = looked_at(&s);
            assert_eq!(defs_of(&s, &looked), [TOP_A, TOP_B, TOP_C]);
        }

        #[test]
        fn r5255002_the_pick_is_added_to_the_hand_and_the_others_go_to_the_graveyard() {
            let mut s = tackled(false, json!([TOP_A, TOP_B, TOP_C, REST_D, REST_E]));
            let looked = looked_at(&s);

            s.answer(pick::id(&looked[1]));

            let hand = kept(&s);
            assert_eq!(def_ids(&hand), [TOP_B]);
            assert!(!hand[0].radiant, "the base face leaves the pick as it was");
            assert_eq!(discarded_from_the_deck(&s), [TOP_A, TOP_C]);
            assert_eq!(def_ids(&s.pile(P1, "library")), [REST_D, REST_E]);
        }

        #[test]
        fn r5255002_r3951001_it_is_no_draw_and_no_discard() {
            let mut s = tackled(false, json!([TOP_A, TOP_B, TOP_C, REST_D]));
            let looked = looked_at(&s);

            s.answer(pick::id(&looked[0]));

            assert_eq!(s.count("drawn"), 0, "the pick is added, not drawn");
            assert_eq!(s.count("discarded"), 0, "nothing leaves a hand");
            assert_eq!(discarded_from_the_deck(&s), [TOP_B, TOP_C], "the other two reach the graveyard");
        }

        #[test]
        fn r4586005_a_deck_of_two_offers_both_and_discards_the_one_not_picked() {
            let mut s = tackled(false, json!([TOP_A, TOP_B]));
            let looked = looked_at(&s);
            assert_eq!(defs_of(&s, &looked), [TOP_A, TOP_B]);

            s.answer(pick::id(&looked[0]));

            assert_eq!(def_ids(&kept(&s)), [TOP_A]);
            assert_eq!(discarded_from_the_deck(&s), [TOP_B]);
            assert!(s.pile(P1, "library").is_empty());
        }

        #[test]
        fn r5238007_a_deck_of_one_keeps_it_and_discards_nothing() {
            let mut s = tackled(false, json!([TOP_A]));
            let looked = looked_at(&s);
            assert_eq!(defs_of(&s, &looked), [TOP_A]);

            s.answer(pick::id(&looked[0]));

            assert_eq!(def_ids(&kept(&s)), [TOP_A]);
            assert!(discarded_from_the_deck(&s).is_empty());
        }

        #[test]
        fn an_empty_deck_asks_nothing() {
            let s = tackled(false, json!([]));
            assert!(s.state().pending.is_none());
            assert!(kept(&s).is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5255002_the_pick_is_made_radiant_and_the_others_go_to_the_graveyard() {
            let mut s = tackled(true, json!([TOP_A, TOP_B, TOP_C, REST_D]));
            let looked = looked_at(&s);

            s.answer(pick::id(&looked[2]));

            let hand = kept(&s);
            assert_eq!(def_ids(&hand), [TOP_C]);
            assert!(hand[0].radiant, "the pick is Radiant");
            assert_eq!(discarded_from_the_deck(&s), [TOP_A, TOP_B]);
            assert_eq!(def_ids(&s.pile(P1, "library")), [REST_D]);
        }
    }
}
