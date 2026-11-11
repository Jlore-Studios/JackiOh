//! SPEC §8.12 S+ #129 Multiplicative Reconstructor (§2.4, §5.1, §6.3 Tribute; R62, R65, R78, R215, R382,
//! R1420, R3535000, R3563000, R3565000, R3565001): (3) Field Spell, Mindustrial, Rare, a card of the held
//! Stinkyscape+ set.
//!   Base:    "Start of turn: Add a random (2) cost Automa card to your hand, it has no Tribute condition."
//!   Radiant: "Start of turn: Add a random Radiant (2) cost Automa card to your hand, it has no Tribute
//!            condition."
//!
//! Engine: at the start of its controller's turn (R62), before their draw, one `addRandomFromCatalog` from
//! the Stinkyscape+ cards carrying any of the six Automa tags (R3565001), tokens out (§5.1, R382), the pool
//! naming its set so that it reaches them while the set is held (R1420), whose cost, read out of play (R65),
//! is (2): Fortress and Zenith. "It has no Tribute condition" is the add's `tribute` rider set to 0
//! (R3565000): the card asks no Tribute while it is in a hand or a deck, and the field's reset (R78) or a
//! graveyard (R215) gives its printed one back. The Radiant face adds it Radiant. A full hand burns it
//! (§2.4).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-129";

/// "(2) cost".
const COST: i32 = 2;
/// R3565000: "it has no Tribute condition".
const NO_TRIBUTE: i32 = 0;

/// R3565001: "Automa" is any of the six Automa tags.
const AUTOMA: [Tag; 6] = [
    Tag::AutomaAlpha,
    Tag::AutomaBeta,
    Tag::AutomaGamma,
    Tag::AutomaDelta,
    Tag::AutomaOmega,
    Tag::AutomaZeta,
];

/// R3565001: "a random (2) cost Automa card", the held set's.
fn automa_cards() -> CatalogQueryArgs {
    json_as(json!({ "set": SetName::StinkyscapePlus, "anyTags": AUTOMA, "cost": COST }))
}

fn reconstructor(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |_ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": automa_cards(),
                "radiant": radiant,
                "tribute": NO_TRIBUTE,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: reconstructor(false),
        radiant: reconstructor(true),
    }
}

// S+ #129 Multiplicative Reconstructor — SPEC §8.12 row S+ 129 (R3565000, R3565001; R62, R65, R1420, §2.4).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::play_choices::{legal_tribute_sets, tribute_cost_of};
    use jackioh_engine::testkit::*;

    use crate::{filler, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";
    /// Fortress and Zenith: the Automa cards that cost (2).
    const POOL: [&str; 2] = ["stinkyscapeplus-112", "stinkyscapeplus-117"];
    /// What each asks for when nothing has set it (R101).
    const PRINTED_TRIBUTE: i32 = 1;

    /// The Field Spell already standing in p1's backrow.
    fn reconstructing(radiant: bool, seed: &str) -> Scenario {
        weapon_board(
            seed,
            json!({
                "hand": [FILLER],
                "library": filler(6),
                "backrow": [{ "def": ID, "radiant": radiant }],
            }),
            json!({ "hand": [FILLER], "library": filler(6) }),
        )
    }

    /// The Stinkyscape+ cards p1 gained in the last step (the turn's draw reaches the hand too, as a Core
    /// filler).
    fn gained(s: &Scenario) -> Vec<CardInstance> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player: P1, instance_id, .. } => Some(s.card(instance_id).clone()),
                _ => None,
            })
            .filter(|card| crate::card_def(&card.def_id).set == SetName::StinkyscapePlus)
            .collect()
    }

    /// p1's next turn start, from their main phase; the one card it adds.
    fn next_turn(s: &mut Scenario) -> CardInstance {
        s.end_turn().end_turn();
        let mut cards = gained(s);
        assert_eq!(cards.len(), 1, "one card");
        cards.remove(0)
    }

    fn check_added(card: &CardInstance, radiant: bool) {
        assert!(POOL.contains(&card.def_id.as_str()), "{}", card.def_id);
        assert!(!crate::card_def(&card.def_id).is_token(), "{}", card.def_id);
        assert_eq!(card.zone.z(), ZoneName::Hand);
        assert_eq!(card.radiant, radiant, "{}", card.def_id);
    }

    /// The card asks no Tribute, and is played onto an empty field, which a Tribute would refuse.
    fn check_asks_no_tribute(s: &mut Scenario, card: &CardInstance) {
        assert_eq!(tribute_cost_of(s.state(), s.card(card)), 0, "{}", card.def_id);
        assert_eq!(legal_tribute_sets(s.state(), P1, s.card(card)), vec![Vec::<String>::new()]);
        s.state_mut().players.p1.mana.current = MAX_MANA;
        s.play(card.id.as_str(), json!({ "zone": 1 }));
        s.expect_in_zone(card.id.as_str(), "field");
    }

    #[test]
    fn r3565001_the_pool_is_fortress_and_zenith() {
        // The scenario registers the catalog, which the pool reads.
        let _registered = reconstructing(false, "s129-pool");
        let mut found: Vec<String> =
            crate::query::pool(ID, &automa_cards()).iter().map(|def| def.id.clone()).collect();
        found.sort();
        let expected: Vec<String> = POOL.iter().map(|id| id.to_string()).collect();
        assert_eq!(found, expected);
    }

    #[test]
    fn nothing_at_the_opponents_start_of_turn() {
        let mut s = reconstructing(false, "s129-opponent");
        s.end_turn();
        assert!(gained(&s).is_empty());
    }

    mod base {
        use super::*;

        #[test]
        fn r3565001_adds_a_2_cost_automa_card() {
            let mut kinds: IndexSet<String> = IndexSet::new();
            for i in 0..30 {
                let mut s = reconstructing(false, &format!("s129-{i}"));
                let card = next_turn(&mut s);
                assert!(s.state().pending.is_none(), "no prompt opens");
                check_added(&card, false);
                kinds.insert(card.def_id.clone());
            }
            assert_eq!(kinds.len(), POOL.len(), "both are added: {kinds:?}");
        }

        #[test]
        fn r3565000_it_asks_no_tribute() {
            for i in 0..4 {
                let mut s = reconstructing(false, &format!("s129-tribute-{i}"));
                let card = next_turn(&mut s);
                check_added(&card, false);
                check_asks_no_tribute(&mut s, &card);
            }
        }

        #[test]
        fn r3565000_the_tribute_comes_back_when_it_leaves_the_field() {
            let mut s = reconstructing(false, "s129-back");
            let card = next_turn(&mut s);
            check_asks_no_tribute(&mut s, &card);
            crate::run_as(
                &mut s,
                bounce(json_as(json!({ "target": { "of": "instance", "instanceId": card.id } }))),
                P1,
            );
            s.expect_in_zone(card.id.as_str(), "hand");
            assert_eq!(tribute_cost_of(s.state(), s.card(&card)), PRINTED_TRIBUTE);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3565001_radiant_adds_a_radiant_2_cost_automa_card() {
            for i in 0..10 {
                let mut s = reconstructing(true, &format!("s129-r{i}"));
                let card = next_turn(&mut s);
                check_added(&card, true);
            }
        }

        #[test]
        fn r3565000_radiant_it_asks_no_tribute() {
            for i in 0..4 {
                let mut s = reconstructing(true, &format!("s129-rtribute-{i}"));
                let card = next_turn(&mut s);
                check_added(&card, true);
                check_asks_no_tribute(&mut s, &card);
            }
        }
    }
}
