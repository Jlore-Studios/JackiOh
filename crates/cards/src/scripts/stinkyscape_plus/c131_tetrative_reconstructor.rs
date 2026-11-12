//! SPEC §8.12 S+ #131 Tetrative Reconstructor (§2.4, §5.1, §6.3 Tribute; R62, R65, R78, R101, R215, R382, R1420, R3535000,
//! R3563000, R3565000, R3565001): (4) Field Spell, Mindustrial, Legendary, a card of the Stinkyscape+ set.
//!   Base:    "Tribute 4
//!            Start of turn: Add a random Automa-Ω card to your hand, it has no Tribute condition."
//!   Radiant: "Tribute 4
//!            Start of turn: Add a random Radiant Automa-Ω card to your hand, it has no Tribute condition."
//!
//! Tribute 4 is the play validator's (§6.3, R101), read off `staticFlags.tribute`: four Units of its player's
//! destroyed to pay it. At the start of its controller's turn (R62), before their draw, one
//! `addRandomFromCatalog` from the Stinkyscape+ cards tagged Automa-Ω (R3565001), tokens out (§5.1, R382:
//! no Reignclipse), the pool naming its set so that it reaches them while the set is held (R1420): Reign, Eclipse, Oct, Omura and Corvus.
//! "It has no Tribute condition" is the add's `tribute` rider set to 0 (R3565000): the card asks no Tribute
//! while it is in a hand or a deck, and the field's reset (R78) or a graveyard (R215) gives its printed one
//! back. The Radiant face adds it Radiant. A full hand burns it (§2.4).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-131";

/// "Tribute 4" (R101).
const TRIBUTE: i32 = 4;
/// R3565000: "it has no Tribute condition".
const NO_TRIBUTE: i32 = 0;

/// R3565001: "a random Automa-Ω card", the held set's.
fn automa_cards() -> CatalogQueryArgs {
    json_as(json!({ "set": SetName::StinkyscapePlus, "tags": [Tag::AutomaOmega] }))
}

fn reconstructor(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE),
            ..StaticFlags::default()
        }),
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

// S+ #131 Tetrative Reconstructor — SPEC §8.12 row S+ 131 (R3565000, R3565001; R62, R101, R1420, §2.4).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::play_choices::{legal_tribute_sets, tribute_cost_of};
    use jackioh_engine::testkit::*;

    use crate::{filler, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";
    const VANILLA: &str = "stand-in (0) 1/1";
    /// Reign, Eclipse, Oct, Omura and Corvus: the Automa-Ω cards that are no token.
    const POOL: [&str; 5] = ["stinkyscapeplus-114", "stinkyscapeplus-119", "stinkyscapeplus-123", "stinkyscapeplus-125", "stinkyscapeplus-127"];
    /// What each asks for when nothing has set it (R101).
    const PRINTED_TRIBUTE: [i32; 5] = [3, 3, 3, 3, 3];

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

    /// The Field Spell in p1's hand, Units on the field to pay its Tribute.
    fn to_play(radiant: bool, seed: &str) -> Scenario {
        weapon_board(
            seed,
            json!({
                "hand": [{ "def": ID, "radiant": radiant }],
                "library": filler(6),
                "field": vec![VANILLA; TRIBUTE as usize],
                "mana": 10,
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
        let def = crate::card_def(&card.def_id);
        assert!(def.tags.contains(&Tag::AutomaOmega) && !def.is_token(), "{}", card.def_id);
        assert_eq!(card.zone.z(), ZoneName::Hand);
        assert_eq!(card.radiant, radiant, "{}", card.def_id);
    }

    /// The card asks no Tribute, and is played onto an empty field, which its printed one would refuse.
    fn check_asks_no_tribute(s: &mut Scenario, card: &CardInstance) {
        let at = POOL.iter().position(|id| *id == card.def_id).expect("a pool card");
        assert!(PRINTED_TRIBUTE[at] > 0, "{} prints a Tribute", card.def_id);
        assert_eq!(tribute_cost_of(s.state(), s.card(card)), 0, "{}", card.def_id);
        assert_eq!(legal_tribute_sets(s.state(), P1, s.card(card)), vec![Vec::<String>::new()]);
        s.state_mut().players.p1.mana.current = MAX_MANA;
        s.play(card.id.as_str(), json!({ "zone": 1 }));
        s.expect_in_zone(card.id.as_str(), "field");
    }

    #[test]
    fn r101_tribute_4_is_required() {
        for radiant in [false, true] {
            let mut s = to_play(radiant, "s131-tribute");
            assert_eq!(tribute_cost_of(s.state(), s.card(ID)), TRIBUTE, "radiant {radiant}");
            // No tribute is refused, and so is one short; the Units in lanes 1..=4 pay it.
            s.expect_refused(|s| s.play(ID, json!({ "zone": 5 })));
            let fodder: Vec<String> = (1..=TRIBUTE).map(|lane| s.unit_at(P1, lane).id).collect();
            let one_short: Vec<String> = fodder[1..].to_vec();
            if !one_short.is_empty() {
                s.expect_refused(|s| s.play(ID, json!({ "zone": 5, "tributes": one_short })));
            }
            s.play(ID, json!({ "zone": 5, "tributes": fodder }));
            s.expect_in_zone(ID, "field");
            for id in &fodder {
                s.expect_in_zone(id.as_str(), "graveyard");
            }
        }
    }

    #[test]
    fn r3565001_the_pool_is_the_five_automa_omega_cards() {
        // The scenario registers the catalog, which the pool reads.
        let _registered = reconstructing(false, "s131-pool");
        let mut found: Vec<String> =
            crate::query::pool(ID, &automa_cards()).iter().map(|def| def.id.clone()).collect();
        found.sort();
        let mut expected: Vec<String> = POOL.iter().map(|id| id.to_string()).collect();
        expected.sort();
        assert_eq!(found, expected);
    }

    #[test]
    fn nothing_at_the_opponents_start_of_turn() {
        let mut s = reconstructing(false, "s131-opponent");
        s.end_turn();
        assert!(gained(&s).is_empty());
    }

    mod base {
        use super::*;

        #[test]
        fn r3565001_adds_an_automa_omega() {
            let mut kinds: IndexSet<String> = IndexSet::new();
            for i in 0..40 {
                let mut s = reconstructing(false, &format!("s131-{i}"));
                let card = next_turn(&mut s);
                assert!(s.state().pending.is_none(), "no prompt opens");
                check_added(&card, false);
                kinds.insert(card.def_id.clone());
            }
            assert!(kinds.len() >= 3, "the cards vary: {kinds:?}");
        }

        #[test]
        fn r3565000_it_asks_no_tribute() {
            for i in 0..4 {
                let mut s = reconstructing(false, &format!("s131-no-tribute-{i}"));
                let card = next_turn(&mut s);
                check_added(&card, false);
                check_asks_no_tribute(&mut s, &card);
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3565001_radiant_adds_a_radiant_automa_omega() {
            for i in 0..10 {
                let mut s = reconstructing(true, &format!("s131-r{i}"));
                let card = next_turn(&mut s);
                check_added(&card, true);
            }
        }

        #[test]
        fn r3565000_radiant_it_asks_no_tribute() {
            for i in 0..4 {
                let mut s = reconstructing(true, &format!("s131-rno-tribute-{i}"));
                let card = next_turn(&mut s);
                check_added(&card, true);
                check_asks_no_tribute(&mut s, &card);
            }
        }
    }
}
