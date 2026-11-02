//! Stinkyscape #130 Jlockeed's Starship Retrofitting Program (SPEC §8.10 row 130). (1) Spell, Jlockeed,
//! Epic.
//!   Base:    "Add a random Jlockeed card to your hand. It gains Starship Piece."
//!   Radiant: "Add a random Radiant Jlockeed card to your hand. It gains Starship Piece and costs
//!            ({discount}) less." — discount 1
//!
//! The pool is the non-token cards tagged Jlockeed (R278) but this one (R387), from the sets that ship
//! (R380, R1420). R1878003: the card is given Starship Piece as it reaches the hand (the
//! `addRandomFromCatalog` keywords rider), which stays the card's own until it leaves the field (R78), so
//! played as a Unit its death fuses a copy into its owner's Dormant Starship (R1852000); a Spell that
//! resolves never dies, so it fuses nothing. A full hand burns it (§2.4). Radiant: the card is Radiant and
//! costs `discount` less, never below (0) (R65).

use jackioh_engine::prelude::*;

use crate::query::CardQuery;

pub const ID: &str = "stinkyscape-130";

fn retrofit(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut args = AddRandomFromCatalogArgs {
                query: Some(CardQuery {
                    tags: Some(vec![Tag::Jlockeed]),
                    ..CardQuery::default()
                }),
                keywords: Some(vec![Keyword::StarshipPiece]),
                ..AddRandomFromCatalogArgs::default()
            };
            if radiant {
                args.radiant = Some(true);
                args.cost_mod = Some(-param(&*ctx, "discount"));
            }
            vec![add_random_from_catalog(args)]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: retrofit(false),
        radiant: retrofit(true),
    }
}

// Stinkyscape #130 Jlockeed's Starship Retrofitting Program — SPEC §8.10 row 130, BUILD M12 row S
// 130 (R1852000, R387, R824000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const PROGRAM: &str = "stinkyscape-130";
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const FILLER: &str = "core-005";

    fn holding(radiant: bool, seed: &str, fillers: usize) -> Scenario {
        let mut hand = vec![json!({ "def": PROGRAM, "radiant": radiant }), json!(HIT_JOB)];
        hand.extend((0..fillers).map(|_| json!(FILLER)));
        scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": filler(6), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }))
    }

    fn added(s: &Scenario) -> Option<CardInstance> {
        s.last_events().iter().find_map(|event| match event {
            GameEvent::AddedToHand { player: P1, instance_id, .. } => Some(s.card(instance_id).clone()),
            _ => None,
        })
    }

    #[test]
    fn r278_r387_r1878003_adds_a_random_jlockeed_card_but_this_one_with_starship_piece() {
        for i in 0..16 {
            let mut s = holding(false, &format!("retrofit-{i}"), 1);
            s.play(PROGRAM, json!({}));
            let card = added(&s).expect("a Jlockeed card");
            assert!(crate::card_def(&card.def_id).tags.contains(&Tag::Jlockeed), "{}", card.def_id);
            assert_ne!(card.def_id, PROGRAM);
            assert!(!crate::card_def(&card.def_id).is_token());
            assert_eq!(card.granted_keywords, vec![Keyword::StarshipPiece], "{}", card.def_id);
            assert!(!card.radiant);
        }
    }

    #[test]
    fn r1878003_r1852000_played_as_a_unit_it_is_a_starship_piece_and_its_death_fuses_it_into_the_dormant_starship() {
        let seed = seed_where("retrofit-unit", |seed| {
            let mut s = holding(false, seed, 1);
            s.play(PROGRAM, json!({}));
            added(&s).is_some_and(|card| crate::card_def(&card.def_id).type_ == CardType::Unit)
        });
        let mut s = holding(false, &seed, 1);
        s.play(PROGRAM, json!({}));
        let card = added(&s).expect("a Jlockeed Unit");
        s.play(card.id.as_str(), json!({ "zone": 1 }));
        let unit = s.unit(P1, 1).expect("the Unit").id;
        assert_eq!(unit, card.id);
        assert!(has_keyword(&s.stats(unit.as_str()).keywords, KeywordKind::StarshipPiece));
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": unit }] }));
        let ship = dormant_starship_of(s.state(), P1).expect("a Dormant Starship").clone();
        let pieces: Vec<String> = starship_pieces_of(&ship).into_iter().map(|piece| piece.def_id).collect();
        assert_eq!(pieces, vec![card.def_id]);
    }

    #[test]
    fn s2_4_a_full_hand_burns_it() {
        let mut s = holding(false, "retrofit-full", 9);
        s.play(PROGRAM, json!({}));
        assert!(added(&s).is_none());
        let burned = s
            .last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Burned { .. }))
            .count();
        assert_eq!(burned, 1);
    }

    #[test]
    fn radiant_the_card_is_radiant_and_costs_1_less() {
        for i in 0..8 {
            let mut s = holding(true, &format!("retrofit-radiant-{i}"), 1);
            s.play(PROGRAM, json!({}));
            let card = added(&s).expect("a Jlockeed card");
            assert!(card.radiant);
            assert_eq!(card.cost_mod, -1);
            assert_eq!(card.granted_keywords, vec![Keyword::StarshipPiece]);
            let printed = query_cost(&crate::card_def(&card.def_id));
            assert_eq!(effective_cost(s.state(), &card, Default::default()), (printed - 1).max(0));
        }
    }

    #[test]
    fn radiant_r824000_the_discount_reads_through_param() {
        let mut s = holding(true, "retrofit-discount", 1);
        step_param(s.card_mut(PROGRAM), "discount", 1);
        s.play(PROGRAM, json!({}));
        assert_eq!(added(&s).expect("a Jlockeed card").cost_mod, -2);
    }
}
