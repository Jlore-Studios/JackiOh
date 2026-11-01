//! M #101 Gachaholic (SPEC §8.8 row 101): (1) Unit, CN, Human, Common, 1/1 → 2/2. The designer's #98.
//!
//! Base:    "Cry: Add a random Luck-based card to your hand. Give it Lucky {lucky}."
//! Radiant: "Activate: Add a random Luck-based card to your hand. Give it Lucky {lucky}."
//! Engine:
//! - **The pull:** `add_random_from_catalog` over `query::luck_based()` (MD-G1, R1444): one uniform pick
//!   of the non-token cards that print Lucky or flip a coin, of every set that ships (R1420), on its
//!   base face at its printed cost, hidden from the opponent (R97); a full hand burns it (§2.4).
//! - **Given Lucky:** the verb's `lucky` rider (MD-G2, R1438) gives the card Lucky {lucky} once it is in
//!   the hand, a granted keyword that adds to the Lucky it prints and that its roll reads.
//! - **Radiant:** the same pull as an Activate (R384): once a turn, in your main phase, at no cost,
//!   the turn it is played included. It has no Cry.
//! - **Tuned:** `lucky` is `cry: "base"` (R2261001): on the field a base Gachaholic's Cry has run, so
//!   no Nerf or Buff draws it there, while a Radiant one's Activate still reads it.

use jackioh_engine::effects::{AddRandomFromCatalogArgs, add_random_from_catalog};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-101";

fn pull(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    vec![add_random_from_catalog(AddRandomFromCatalogArgs {
        query: Some(crate::query::luck_based()),
        lucky: Some(param(&*ctx, "lucky")),
        ..AddRandomFromCatalogArgs::default()
    })]
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(pull)),
        ..Script::default()
    };

    let radiant = Script {
        activations: vec![ActivationDecl {
            id: "pull".to_string(),
            label: "Add a random Luck-based card to your hand".to_string(),
            uses: ActivationUses::Count(1),
            cost: None,
            targets: Vec::new(),
            modes: Vec::new(),
            can_activate: None,
            has: None,
            run: hook(pull),
        }],
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// M #101 Gachaholic — SPEC §8.8 row 101, BUILD M10 row M 101, R386.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use crate::js;
    use jackioh_engine::effects::TuneDirection;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-016";
    const PEPTIDES: &str = "meditative-036";

    /// The pool from the sets that ship: the non-token cards that print Lucky or flip a coin (MD-G1,
    /// R1444), the Meditative set's among them since it ships: #36 CN Peptides, #86 Mayor Medinamogger
    /// and #102 Catboy Maid SSR+ (#65 Keymaster Keenus left it with patch v0.3.7, R781004, and Core #42
    /// Eugenics with patch v0.3.10, R831014). Re-pinned for the Stinkyscape release (#1884): its #77
    /// Yuma, #112 Job Application, #145 Fortify Flesh, #153 Stimmycheck and #166 and #167, the Scanning
    /// Probes, join it; and for the Training Grounds release (#2204): its #49 Reoccurring Dream, the
    /// snapshot of Core #23; and for #2521: Stinkyscape #53, #57 and #61, the Flaks, whose Radiant faces
    /// print Lucky 1.
    const SHIPPED_POOL: [&str; 20] = [
        "core-004",
        "core-023",
        "classic-065",
        "classicplus-025",
        "classicplus-053",
        "classicplus-065",
        "classicplus-066",
        "meditative-036",
        "meditative-086",
        "meditative-102",
        "stinkyscape-053",
        "stinkyscape-057",
        "stinkyscape-061",
        "stinkyscape-077",
        "stinkyscape-112",
        "stinkyscape-145",
        "stinkyscape-153",
        "stinkyscape-166",
        "stinkyscape-167",
        "traininggrounds-049",
    ];

    /// p1 holds Gachaholic (Radiant on `radiant`) and `others` fillers.
    fn holding(seed: &str, radiant: bool, others: usize) -> Scenario {
        crate::register_all();
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend((0..others).map(|_| json!(FILLER)));
        crate::scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": [FILLER, FILLER], "mana": 1 },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }))
    }

    /// The cards p1 holds past the fillers it kept.
    fn added(s: &Scenario, kept: usize) -> Vec<CardInstance> {
        s.hand(P1).into_iter().skip(kept).collect()
    }

    /// The keys of the declared numbers a Buff would move on it now.
    fn buffable(s: &Scenario) -> Vec<String> {
        steppable_params(s.state(), s.card(ID), TuneDirection::Upgrade)
            .into_iter()
            .map(|item| item.param.key)
            .collect()
    }

    mod m101_gachaholic {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r1444_its_cry_adds_one_shipped_luck_based_card_hidden_on_its_base_face_at_its_printed_cost() {
                let mut seen: Vec<String> = Vec::new();
                for n in 0..24 {
                    let mut s = holding(&format!("gachaholic-{n}"), false, 1);
                    s.play(ID, json!({}));
                    let added = added(&s, 1);
                    assert_eq!(added.len(), 1);
                    let card = &added[0];
                    let def = crate::card_def(&card.def_id);
                    assert!(SHIPPED_POOL.contains(&card.def_id.as_str()), "{}", card.def_id);
                    assert!(is_luck_based(&def));
                    assert!(!card.radiant);
                    let HandView::Cards(hand) = s.view(P1).you.hand else {
                        panic!("own hand is a list")
                    };
                    let shown = hand
                        .iter()
                        .find(|held| held.instance_id == card.id)
                        .expect("its owner sees it");
                    assert_eq!(shown.cost, query_cost(&def));
                    // R97: the opponent learns that a card reached the hand, never which.
                    let theirs = js(&s.view(P2));
                    assert_eq!(theirs["opponent"]["hand"], json!({ "count": 2 }));
                    assert!(!theirs.to_string().contains(&card.def_id), "{}", card.def_id);
                    if !seen.contains(&card.def_id) {
                        seen.push(card.def_id.clone());
                    }
                }
                assert!(seen.len() > 1, "the pick varies by seed");
            }

            #[test]
            fn r1438_the_card_it_adds_has_lucky_1() {
                let mut s = holding("gachaholic-lucky", false, 1);
                s.play(ID, json!({}));
                let card = added(&s, 1).remove(0);
                assert_eq!(card.granted_keywords, vec![Keyword::Lucky { n: 1 }]);
                let printed =
                    tuning::numbered_sum(&crate::card_def(&card.def_id).base.keywords, KeywordKind::Lucky);
                assert_eq!(lucky_on(s.state(), &card), printed.unwrap_or(0) + 1);
            }

            #[test]
            fn s2_4_a_full_hand_burns_it() {
                // Ten fillers behind it: the hand is full again as the Cry resolves.
                let mut s = holding("gachaholic-full", false, 10);
                s.play(ID, json!({}));
                assert_eq!(s.hand(P1).len() as i32, HAND_CAP);
                assert!(added(&s, 10).is_empty());
                let burned = s
                    .pile(P1, "graveyard")
                    .into_iter()
                    .find(|card| SHIPPED_POOL.contains(&card.def_id.as_str()))
                    .expect("the pulled card is burned");
                assert!(burned.granted_keywords.is_empty());
                s.expect_events(json!(["burned"]));
            }

            #[test]
            fn r386_a_buff_gives_lucky_2() {
                let mut s = holding("gachaholic-buffed", false, 1);
                assert_eq!(crate::upgrade_number(&mut s, ID, "lucky"), 2);
                s.play(ID, json!({}));
                let card = added(&s, 1).remove(0);
                assert_eq!(card.granted_keywords, vec![Keyword::Lucky { n: 2 }]);
            }

            #[test]
            fn r2261001_on_the_field_its_cry_s_lucky_is_out_of_a_buff_s_reach() {
                let mut s = holding("gachaholic-in-play", false, 1);
                assert_eq!(buffable(&s), ["lucky"]);
                s.play(ID, json!({}));
                assert!(buffable(&s).is_empty());
            }

            #[test]
            fn r1420_it_reaches_cn_peptides_since_the_set_shipped() {
                let found = (0..200).any(|n| {
                    let mut s = holding(&format!("gachaholic-preview-{n}"), false, 1);
                    s.play(ID, json!({}));
                    let card = added(&s, 1).remove(0);
                    assert!(is_luck_based(&crate::card_def(&card.def_id)), "{}", card.def_id);
                    assert_ne!(card.def_id, ID);
                    card.def_id == PEPTIDES
                });
                assert!(found, "a pull reaches Meditative #36 CN Peptides");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn radiant_has_no_cry() {
                let scripts = script();
                assert!(scripts.radiant.cry.is_none());
                assert!(scripts.base.activations.is_empty());
                let mut s = holding("gachaholic-radiant-play", true, 1);
                s.play(ID, json!({}));
                assert!(added(&s, 1).is_empty());
            }

            #[test]
            fn r2261001_on_the_field_its_activate_s_lucky_is_still_in_a_buff_s_reach() {
                let mut s = holding("gachaholic-radiant-in-play", true, 1);
                s.play(ID, json!({}));
                assert_eq!(buffable(&s), ["lucky"]);
            }

            #[test]
            fn r384_radiant_activates_once_per_turn_the_turn_it_is_played() {
                let mut s = holding("gachaholic-radiant", true, 1);
                s.play(ID, json!({}));
                s.activate(ID, json!({}));
                let pulled = added(&s, 1);
                assert_eq!(pulled.len(), 1);
                assert!(
                    SHIPPED_POOL.contains(&pulled[0].def_id.as_str()),
                    "{}",
                    pulled[0].def_id
                );
                assert_eq!(pulled[0].granted_keywords, vec![Keyword::Lucky { n: 1 }]);
                assert!(!pulled[0].radiant);
                s.expect_refused(|s| s.activate(ID, json!({})));
                assert_eq!(added(&s, 1).len(), 1);
            }
        }
    }
}
