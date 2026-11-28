//! SPEC §8.14 FvH #65 Felinor Forager ((1) Unit, Felinor, Common, 2/1 → 4/2), a card of the held Felinors vs
//! Humans set (R5235000).
//!   Base:    "Death: Add a random Fruit to your hand."
//!   Radiant: "Reborn\nDeath: Add a random Fruit to your hand."
//!
//! The Death is `add_random_from_catalog` of one card of the Fruit pool (R382, the sets that ship), not
//! Radiant on either face, as the text says; a full hand burns it (R4). The Radiant face's Reborn is catalog
//! data, so the body it returns adds another as it dies. Declares no numbers. Built by S07 (#5257).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-065";

pub fn script() -> CardScripts {
    let base = Script {
        death: Some(hook(|_ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "tags": ["Fruit"] },
                "count": 1,
            })))]
        })),
        ..Script::default()
    };
    // The same Death: the Radiant face's Reborn is catalog data and its Fruit is not Radiant.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #65 Felinor Forager — SPEC §8.14 row 65.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill
    const FILLER: &str = "core-005"; // Stockpile: a card to hold

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [HIT_JOB, HIT_JOB],
                "field": [{ "def": ID, "radiant": radiant }],
                "mana": 10,
                "library": crate::filler(4),
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }))
    }

    fn kill(s: &mut Scenario) {
        let me = must(
            active_units_of(s.state(), P1)
                .into_iter()
                .find(|unit| unit.def_id == ID)
                .cloned(),
            "the Felinor Forager on p1's field",
        );
        s.play(HIT_JOB, pick::targets(pick::instance(&me)));
    }

    /// The cards p1's hand gained in the last step, by `addedToHand`.
    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.last_events_of(GameEventType::AddedToHand)
            .into_iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player: P1, instance_id, .. } => Some(s.card(instance_id.as_str()).clone()),
                _ => None,
            })
            .collect()
    }

    fn is_fruit(s: &Scenario, card: &CardInstance) -> bool {
        def_of(Some(s.state()), &card.def_id).tags.contains(&Tag::Fruit)
    }

    mod base {
        use super::*;

        #[test]
        fn its_death_adds_one_random_fruit_that_is_not_radiant() {
            let mut s = board(false);
            kill(&mut s);
            let fruits = nonempty(added(&s), "the Fruit its Death added");
            assert_eq!(fruits.len(), 1);
            for fruit in &fruits {
                assert!(is_fruit(&s, fruit), "{} is a Fruit", fruit.def_id);
                assert!(!fruit.radiant);
                assert_eq!(fruit.zone.z(), ZoneName::Hand);
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn its_death_adds_a_base_face_fruit_and_the_reborn_bodys_death_adds_another() {
            let mut s = board(true);
            kill(&mut s);
            let first = nonempty(added(&s), "the first death's Fruit");
            assert_eq!(first.len(), 1);
            assert!(first.iter().all(|fruit| is_fruit(&s, fruit) && !fruit.radiant));
            kill(&mut s);
            let second = nonempty(added(&s), "the Reborn body's Fruit");
            assert_eq!(second.len(), 1);
            assert!(second.iter().all(|fruit| is_fruit(&s, fruit) && !fruit.radiant));
        }
    }
}
