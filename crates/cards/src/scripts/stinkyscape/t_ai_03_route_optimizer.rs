//! S T-AI-3 Route Optimizer (SPEC §7, §8.10 row T-AI-3; R65, R482, R1878000, R1881001): (1) Spell, AI,
//! Token (no printed rarity), an AI generated token of the Stinkyscape set, built by S21.
//!   Base:    "Discover a Starship Piece. It costs (1) less."
//!   Radiant: "Discover a Radiant Starship Piece. It costs (1) less."
//! Engine: a Discover (§6.3), three different options from `query::starship_pieces()` (R1878000: every
//!   non-token card printing Starship Piece, named by its set so R1420's gate would leave it whole), the pick
//!   added to its caster's hand with a `costMod` of −1 (R65), which travels with it (R78); a full hand
//!   burns it (§2.4). A token never is one (R1881001). Radiant: the pick is added Radiant. Declares no
//!   numbers (R482).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-ai-03";

/// The resume step that adds the pick.
const PICKED: &str = "route-optimizer-picked";

/// "It costs (1) less" (R65).
const DISCOUNT: i32 = -1;

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![discover_from_catalog(DiscoverFromCatalogArgs {
                step: PICKED.to_string(),
                query: Some(crate::query::starship_pieces()),
                prompt: Some("Discover a Starship Piece".to_string()),
                ..json_as(json!({ "step": PICKED }))
            })]
        })),
        resume: IndexMap::from([(
            PICKED,
            hook(move |ctx| {
                let Some(picked) = chosen_options(ctx).into_iter().next() else {
                    return vec![];
                };
                vec![add_to_hand(AddToHandArgs {
                    def_id: Some(picked),
                    radiant: radiant.then_some(true),
                    cost_mod: Some(DISCOUNT),
                    ..AddToHandArgs::default()
                })]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S T-AI-3 Route Optimizer — SPEC §8.10 row T-AI-3, BUILD M12 row S T-AI-3 (R1881001).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const OPTIMIZER: &str = "stinkyscape-t-ai-03";
    const FILLER: &str = "core-005";

    fn cast(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": OPTIMIZER, "radiant": radiant }, FILLER], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(OPTIMIZER, json!({}));
        s
    }

    fn offered(s: &Scenario) -> Vec<String> {
        s.state()
            .pending
            .as_ref()
            .expect("the Discover is open")
            .options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Mode { option } => Some(option.clone()),
                _ => None,
            })
            .collect()
    }

    fn prints_piece(def_id: &str) -> bool {
        has_keyword(&crate::card_def(def_id).base.keywords, KeywordKind::StarshipPiece)
    }

    #[test]
    fn r1881001_it_offers_three_different_starship_pieces_never_a_token() {
        let mut seen: IndexSet<String> = IndexSet::new();
        for i in 0..12 {
            let s = cast(false, &format!("optimizer-{i}"));
            let offer = offered(&s);
            assert_eq!(offer.len(), 3, "{offer:?}");
            let distinct: IndexSet<&String> = offer.iter().collect();
            assert_eq!(distinct.len(), 3, "{offer:?}");
            for def_id in &offer {
                assert!(prints_piece(def_id), "{def_id} prints Starship Piece");
                assert!(!crate::card_def(def_id).is_token(), "{def_id} is no token");
            }
            seen.extend(offer);
        }
        assert!(seen.len() > 3, "{seen:?}");
    }

    #[test]
    fn r65_the_pick_reaches_the_hand_costing_1_less() {
        let mut s = cast(false, "optimizer-pick");
        let pick = offered(&s)[0].clone();
        s.answer(json!(pick));
        let card = s.hand(P1).into_iter().find(|card| card.def_id == pick).expect("the pick");
        assert!(!card.radiant);
        assert_eq!(card.cost_mod, -1);
    }

    #[test]
    fn radiant_the_pick_is_radiant() {
        let mut s = cast(true, "optimizer-radiant");
        let pick = offered(&s)[0].clone();
        s.answer(json!(pick));
        let card = s.hand(P1).into_iter().find(|card| card.def_id == pick).expect("the pick");
        assert!(card.radiant);
        assert_eq!(card.cost_mod, -1);
    }

    #[test]
    fn r4_a_full_hand_burns_the_pick() {
        let mut hand = vec![json!(OPTIMIZER)];
        hand.extend((0..HAND_CAP).map(|_| json!(FILLER)));
        let mut s = scenario(json!({
            "seed": "optimizer-full",
            "p1": { "hand": hand, "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(OPTIMIZER, json!({}));
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
        let pick = offered(&s)[0].clone();
        s.answer(json!(pick));
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
        assert!(
            s.hand(P1).iter().all(|card| card.def_id == FILLER),
            "the pick is not in hand"
        );
        assert!(
            s.pile(P1, "graveyard").iter().any(|card| card.def_id == pick),
            "it burned"
        );
    }
}
