//! SPEC §8.12 S+ #139 Operation Omega (§2.4, §5.1, §6.3 Add to hand; R60, R65, R78, R215, R382, R1181003, R1420, R3535000,
//! R3563000, R3565000, R3565001): (2 embiggen 4) Spell, Mindustrial, Legendary, a card of the Stinkyscape+ set.
//!   Base:    "Add 2 Automa-Ω cards to your hand.
//!            Embiggened (4): They have Tribute 2."
//!   Radiant: "Add 2 Radiant Automa-Ω cards to your hand.
//!            Embiggened (4): They have Tribute 2."
//!
//! Engine: its Cry is one `addRandomFromCatalog` of `automa` cards (R386, 2 on both faces) from the
//! Stinkyscape+ cards tagged Automa-Ω (R3565001), tokens out (§5.1, R382: no Reignclipse), the pool naming its set so
//! that it reaches them while the set is held (R1420): Reign, Eclipse, Oct, Omura and Corvus, each a draw from the match rng (R60),
//! unique while `UNIQUE_RANDOM_DRAWS` holds (R3563000). They keep their printed Tribute (3) unless the
//! play was embiggened, when the add's `tribute` rider sets it to 2 in place of it (R3565000), kept in the
//! hand and the deck until the field's reset (R78) or a graveyard (R215) takes it. The Radiant face adds
//! them Radiant. A full hand burns the overflow (§2.4).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-139";

/// R3565000: "Embiggened: they have Tribute 2".
const EMBIGGENED_TRIBUTE: i32 = 2;

/// R3565001: "Automa-Ω cards", the held set's.
fn automa_cards() -> CatalogQueryArgs {
    json_as(json!({ "set": SetName::StinkyscapePlus, "tags": [Tag::AutomaOmega] }))
}

fn definition(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![add_random_from_catalog(AddRandomFromCatalogArgs {
                query: Some(automa_cards()),
                count: Some(param(ctx, "automa")),
                radiant: Some(radiant),
                tribute: ctx.embiggened.then_some(EMBIGGENED_TRIBUTE),
                ..AddRandomFromCatalogArgs::default()
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: definition(false),
        radiant: definition(true),
    }
}

// S+ #139 Operation Omega — SPEC §8.12 row S+ 139 (R3565000, R3565001; R60, R1181003, R1420, §2.4).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::play_choices::tribute_cost_of;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";
    /// "Add 2 Automa-Ω cards", declared as `automa`.
    const COUNT: usize = 2;
    /// The embiggened price.
    const EMBIGGENED_MANA: i32 = 4;
    /// Reign, Eclipse, Oct, Omura and Corvus: the Automa-Ω cards that are no token.
    const POOL: [&str; 5] = ["stinkyscapeplus-114", "stinkyscapeplus-119", "stinkyscapeplus-123", "stinkyscapeplus-125", "stinkyscapeplus-127"];
    /// What each asks for when nothing has set it (R101).
    const PRINTED_TRIBUTE: i32 = 3;

    fn defining(radiant: bool, seed: &str, others: usize) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend((0..others).map(|_| json!(FILLER)));
        scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": filler(4), "mana": EMBIGGENED_MANA },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// The cards the cast added to p1's hand, in order.
    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player: P1, instance_id, .. } => Some(s.card(instance_id).clone()),
                _ => None,
            })
            .collect()
    }

    fn check_added(cards: &[CardInstance], radiant: bool) {
        assert_eq!(cards.len(), COUNT, "{COUNT} cards");
        for card in cards {
            assert!(POOL.contains(&card.def_id.as_str()), "{}", card.def_id);
            let def = crate::card_def(&card.def_id);
            assert!(def.tags.contains(&Tag::AutomaOmega) && !def.is_token(), "{}", card.def_id);
            assert_eq!(card.zone.z(), ZoneName::Hand);
            assert_eq!(card.radiant, radiant, "{}", card.def_id);
        }
    }

    #[test]
    fn declared_automa_is_2() {
        let s = defining(false, "s139-declared", 0);
        let def = crate::card_def(ID);
        let param = def.params.as_ref().and_then(|params| params.first()).expect("a declared number");
        assert_eq!(param.key, "automa");
        assert_eq!((param.base, param.radiant), (COUNT as i32, COUNT as i32));
        assert_eq!(param_value(s.state(), Some(s.card(ID)), "automa", ParamValueOptions::default()), COUNT as i32);
    }

    #[test]
    fn r3565001_the_pool_is_the_five_automa_omega_cards() {
        // The scenario registers the catalog, which the pool reads.
        let _registered = defining(false, "s139-pool", 0);
        let mut found: Vec<String> =
            crate::query::pool(ID, &automa_cards()).iter().map(|def| def.id.clone()).collect();
        found.sort();
        let mut expected: Vec<String> = POOL.iter().map(|id| id.to_string()).collect();
        expected.sort();
        assert_eq!(found, expected);
    }

    mod base {
        use super::*;

        #[test]
        fn r3565001_adds_2_automa_omega_cards_keeping_their_tribute() {
            let mut kinds: IndexSet<String> = IndexSet::new();
            for i in 0..30 {
                let mut s = defining(false, &format!("s139-{i}"), 0);
                s.state_mut().players.p1.mana.current = 2;
                s.play(ID, json!({}));
                assert!(s.state().pending.is_none(), "no prompt opens");
                let cards = added(&s);
                check_added(&cards, false);
                for card in &cards {
                    assert_eq!(tribute_cost_of(s.state(), card), PRINTED_TRIBUTE, "{}", card.def_id);
                    assert!(!card.memory.contains_key(TRIBUTE_SET_KEY), "{}", card.def_id);
                    kinds.insert(card.def_id.clone());
                }
            }
            assert!(kinds.len() >= 3, "the cards vary: {kinds:?}");
        }

        #[test]
        fn r3565000_embiggened_they_have_tribute_2() {
            for i in 0..10 {
                let mut s = defining(false, &format!("s139-big-{i}"), 0);
                s.play(ID, json!({ "embiggen": true }));
                let cards = added(&s);
                check_added(&cards, false);
                for card in &cards {
                    assert_eq!(tribute_cost_of(s.state(), card), EMBIGGENED_TRIBUTE, "{}", card.def_id);
                }
            }
        }

        #[test]
        fn s2_4_a_full_hand_burns_the_overflow() {
            let mut s = defining(false, "s139-full", HAND_CAP as usize - 1);
            s.state_mut().players.p1.mana.current = 2;
            s.play(ID, json!({}));
            assert_eq!(added(&s).len(), 1, "one fits");
            let burned = s.last_events().iter().filter(|event| matches!(event, GameEvent::Burned { .. })).count();
            assert_eq!(burned, COUNT - 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3565001_radiant_adds_2_radiant_automa_omega_cards() {
            for i in 0..10 {
                let mut s = defining(true, &format!("s139-r{i}"), 0);
                s.state_mut().players.p1.mana.current = 2;
                s.play(ID, json!({}));
                let cards = added(&s);
                check_added(&cards, true);
                for card in &cards {
                    assert_eq!(tribute_cost_of(s.state(), card), PRINTED_TRIBUTE, "{}", card.def_id);
                }
            }
        }

        #[test]
        fn r3565000_radiant_embiggened_they_have_tribute_2() {
            for i in 0..10 {
                let mut s = defining(true, &format!("s139-rbig-{i}"), 0);
                s.play(ID, json!({ "embiggen": true }));
                let cards = added(&s);
                check_added(&cards, true);
                for card in &cards {
                    assert_eq!(tribute_cost_of(s.state(), card), EMBIGGENED_TRIBUTE, "{}", card.def_id);
                }
            }
        }
    }
}
