//! M #97 Jlockheed's Evil Blueprints (SPEC §8.8 row 97, R1280). (0) Spell, Mythic, Jlockeed.
//!   Base:    "Piece together the blueprint: Discover one of Empty Plot, Wishing Well, School, Mega
//!            Church, Bunker, University, The Great Wall, Prison or Jlockheed's Headquarters."
//!   Radiant: "Discover a Radiant one."
//!   Engine:  "A Discover (§6.3) of three different buildings drawn uniformly from the nine (M #97.1 to
//!            #97.9, a named pool with tokens, R60), shown to the chooser only (§10.8), the pick added to
//!            your hand, Radiant on the Radiant face, the hand cap burning it (§2.4)."
//!
//! The nine are token cards, in no other pool but C+ #23's (R382), offered uniformly without replacement
//! (R60).

use jackioh_engine::effects::{add_to_hand, chosen_options, discover_from_catalog};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-097";

/// The nine buildings of the blueprint (M #97.1 to M #97.9).
pub const BUILDINGS: [&str; 9] = [
    "meditative-097-1",
    "meditative-097-2",
    "meditative-097-3",
    "meditative-097-4",
    "meditative-097-5",
    "meditative-097-6",
    "meditative-097-7",
    "meditative-097-8",
    "meditative-097-9",
];

/// §6.3 Discover offers three different options.
const OPTIONS: i32 = 3;

/// The one resume step: the Discover's answer comes back here (§10.6, `prompts::RESUME_HOOK`).
const PICKED: &str = "picked";

fn blueprints(radiant: bool) -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![discover_from_catalog(json_as(json!({
                "step": PICKED,
                "count": OPTIONS,
                "query": { "defId": BUILDINGS, "withTokens": true },
                "prompt": "Jlockheed's Evil Blueprints: Discover a building",
            })))]
        })),
        resume: IndexMap::from([(
            PICKED,
            hook(move |ctx| {
                let Some(picked) = chosen_options(ctx).into_iter().next() else {
                    return vec![];
                };
                vec![add_to_hand(json_as(json!({ "defId": picked, "radiant": radiant })))]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: blueprints(false),
        radiant: blueprints(true),
    }
}

// M 97 Jlockheed's Evil Blueprints — SPEC §8.8 row 97, BUILD M10 row M 97: "Discovers one of three
// different buildings out of the nine (M 97.1 to M 97.9), shown to you only (MD-F11), the pick to
// your hand, the hand cap burning it; every building can be offered, and never a card outside the
// nine; the opponent sees a prompt and a hidden card; radiant the building is Radiant".
#[cfg(test)]
mod tests {
    use super::{BUILDINGS, ID, script};
    use crate::js;
    use jackioh_engine::testkit::*;
    use std::collections::BTreeSet;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-010"; // (0) Spell.
    const SPARE: &str = "core-005"; // (1) Spell: library spare.

    const SEEDS: usize = 200;

    fn open_pending(state: &GameState) -> &PendingChoice {
        state.pending.as_ref().expect("pending prompt is open")
    }

    fn option_ids(state: &GameState) -> Vec<String> {
        state
            .pending
            .as_ref()
            .map(|pending| {
                pending
                    .options
                    .iter()
                    .filter_map(|opt| match &opt.selection {
                        Selection::Mode { option } => Some(option.clone()),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn setup(seed: &str, radiant: bool) -> Scenario {
        crate::scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": [SPARE, SPARE] },
            "p2": { "hand": [FILLER], "library": [SPARE, SPARE] },
        }))
    }

    #[test]
    fn is_a_0_cost_spell_tagged_jlockeed_mythic() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.id, "meditative-097");
        assert_eq!(js(&def.cost), json!(0));
        assert_eq!(def.rarity, Rarity::Mythic);
        assert_eq!(def.tags, vec![Tag::Jlockeed]);
        assert_eq!(def.type_, CardType::Spell);
        let s = script();
        assert!(s.base.cry.is_some());
        assert!(s.radiant.cry.is_some());
    }

    mod base {
        use super::*;

        #[test]
        fn r1280_offers_three_different_options_all_from_the_nine() {
            crate::register_all();
            let mut s = setup("bp-three", false);
            s.play(ID, json!({}));

            let pending = open_pending(s.state());
            assert_eq!(pending.kind, PromptKind::Discover);
            assert_eq!(pending.player_id, P1);
            let options = option_ids(s.state());
            assert_eq!(options.len(), 3);

            let unique: BTreeSet<_> = options.iter().collect();
            assert_eq!(unique.len(), 3, "options are 3 distinct buildings");

            for opt in &options {
                assert!(
                    BUILDINGS.contains(&opt.as_str()),
                    "option {opt} must be one of the nine buildings"
                );
            }
        }

        #[test]
        fn r1280_across_seed_sweep_every_building_is_offered_and_nothing_else_ever_is() {
            crate::register_all();
            let mut seen = BTreeSet::new();

            for n in 0..SEEDS {
                let mut s = setup(&format!("bp-sweep-{n}"), false);
                s.play(ID, json!({}));

                let options = option_ids(s.state());
                for opt in options {
                    assert!(
                        BUILDINGS.contains(&opt.as_str()),
                        "option {opt} must be one of the nine buildings"
                    );
                    seen.insert(opt);
                }
            }

            assert_eq!(seen.len(), 9, "all 9 buildings must be offered across 200 seeds");
        }

        #[test]
        fn r1280_p2_view_shows_no_options() {
            crate::register_all();
            let mut s = setup("bp-p2-view", false);
            s.play(ID, json!({}));

            let p2_view = view_for(s.state(), P2);
            match p2_view.pending {
                Some(PendingView::Elsewhere(elsewhere)) => {
                    assert!(!elsewhere.for_you);
                    assert_eq!(elsewhere.pending_for, P1);
                }
                other => panic!("expected PendingView::Elsewhere, got {other:?}"),
            }
        }

        #[test]
        fn r1280_pick_goes_to_hand_plain_on_base_face() {
            crate::register_all();
            let mut s = setup("bp-pick-base", false);
            s.play(ID, json!({}));

            let picked = option_ids(s.state())[0].clone();
            s.answer(json!(picked));

            let hand = s.hand(P1);
            let added = hand.iter().find(|c| c.def_id == picked).expect("picked card in hand");
            assert!(!added.radiant, "base face adds plain card");
        }

        #[test]
        fn r1280_a_full_hand_burns_it() {
            crate::register_all();
            let mut s = crate::scenario(json!({
                "seed": "bp-burn",
                "p1": {
                    "hand": [
                        { "def": ID },
                        FILLER, FILLER, FILLER, FILLER, FILLER,
                        FILLER, FILLER, FILLER, FILLER, FILLER
                    ],
                    "library": [SPARE, SPARE]
                },
                "p2": { "hand": [FILLER], "library": [SPARE, SPARE] },
            }));

            // P1 hand has 11 cards (ID + 10 FILLER). Playing ID consumes 1 card, leaving 10 in hand.
            s.play(ID, json!({}));
            assert_eq!(s.hand(P1).len(), 10, "hand is full (10 cards) after play");

            let picked = option_ids(s.state())[0].clone();
            s.answer(json!(picked));

            assert_eq!(s.hand(P1).len(), 10, "hand remains at cap (10)");
            let burned = s
                .last_events()
                .iter()
                .any(|e| matches!(e, GameEvent::Burned { def_id, .. } if def_id == &picked));
            assert!(burned, "card was burned per §2.4 and R11");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1280_pick_goes_to_hand_radiant_on_radiant_face() {
            crate::register_all();
            let mut s = setup("bp-pick-radiant", true);
            s.play(ID, json!({}));

            let picked = option_ids(s.state())[0].clone();
            s.answer(json!(picked));

            let hand = s.hand(P1);
            let added = hand.iter().find(|c| c.def_id == picked).expect("picked card in hand");
            assert!(added.radiant, "radiant face adds radiant card");
        }
    }
}
