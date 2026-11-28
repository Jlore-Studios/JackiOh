//! FvH #115 Call to Chaos Vending Machine (SPEC §8.14 row 115): (3) Field Spell, Legendary.
//!   Base:    "Durability 3\nActivate: Pay (2) to cast a random Call to Chaos."
//!   Radiant: "Durability 3\nActivate: Cast a random Call to Chaos."
//!
//! Engine: one Activate, once a turn (R384; the list's "Activate 1" prints as "Activate:"), its cost
//! `cost.mana` 2 on the base face and none on the Radiant. Durability 3 is the engine's keyword: each use
//! spends 1 (R5236000), a countered one too (R5236003); at 0 it is refused (R5236001) and, after the
//! activation resolves, the settled state check moves it to its owner's graveyard by a plain move
//! (R5236002); Refresh never gives it back (R5236003), and a card that leaves the field comes back full
//! (R5236004). The run: `cast_random` of one card from `call_to_chaos_editions()` (R3869001), its base face
//! on both of the machine's faces (the Radiant text says no Radiant), every choice random (R656), a link of
//! the chain (R28). It prints no Call to Chaos tag, so no edition casts it. Its printed 2 and 3 are a
//! price and a keyword count, fixed (R1748003). Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-115";

/// "Activate" is once a turn (R384).
const USES: i32 = 1;
/// "Pay (2)": the base face's mana price.
const PRICE: i32 = 2;
/// "Cast a random Call to Chaos": one cast.
const CASTS: i32 = 1;

fn face(radiant: bool) -> Script {
    let vend = ActivationDecl {
        id: "vend".to_string(),
        label: if radiant {
            "Cast a random Call to Chaos".to_string()
        } else {
            "Pay (2) to cast a random Call to Chaos".to_string()
        },
        uses: ActivationUses::Count(USES),
        cost: (!radiant).then(|| ActivationCost {
            mana: Some(PRICE),
            ..ActivationCost::default()
        }),
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(|_ctx| {
            vec![cast_random(CastRandomArgs {
                query: crate::query::call_to_chaos_editions().into(),
                count: Some(CastRandomCount::Fixed(CASTS)),
                // The base face of the edition on either face: the Radiant text says no Radiant.
                radiant: Some(false),
                target_enemies: None,
                afterward: None,
            })]
        }),
    };
    Script {
        activations: vec![vend],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// FvH #115 Call to Chaos Vending Machine — SPEC §8.14 row 115 (R384, R3869001, R656, R5236000). An
// edition can wreck the board, so each test reads the step that cast it and nothing after.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    /// A textless Spell each seat can always play, so no turn ends by itself (§2.5).
    const SPELL: &str = "stand-in (0) Spell";
    /// Its printed Durability.
    const DURABILITY: i32 = 3;
    /// How many seeds the random edition is drawn under.
    const SEEDS: usize = 5;

    fn machine(seed: &str, radiant: bool, mana: i32) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [SPELL],
                "backrow": [{ "def": ID, "radiant": radiant, "faceUp": true }],
                "mana": mana,
                "library": filler(6),
            },
            "p2": { "hand": [SPELL], "library": filler(6) },
        }))
    }

    /// Every Call to Chaos of the shipped sets.
    fn editions() -> Vec<String> {
        crate::query::query(&crate::query::call_to_chaos_editions())
            .into_iter()
            .map(|def| def.id.clone())
            .collect()
    }

    /// The first card the last step cast: (instance id, def id).
    fn first_cast(s: &Scenario) -> Option<(String, String)> {
        s.last_events().iter().find_map(|event| match event {
            GameEvent::CardPlayed { instance_id, def_id, .. } => Some((instance_id.clone(), def_id.clone())),
            _ => None,
        })
    }

    /// The Durability the last step left on the machine, as its `counterChanged` reports it.
    fn durability_reported(s: &Scenario, machine: &str) -> Option<i32> {
        s.last_events().iter().find_map(|event| match event {
            GameEvent::CounterChanged { instance_id, counter: CounterKind::Durability, value, .. }
                if instance_id == machine =>
            {
                Some(*value)
            }
            _ => None,
        })
    }

    /// One use on `seed` cast one edition of the pool, on its base face, and spent 1 Durability.
    fn vends_one_edition(seed: &str, radiant: bool, mana: i32) {
        let pool = editions();
        assert!(!pool.iter().any(|id| id == ID), "it prints no Call to Chaos tag");
        let mut s = machine(seed, radiant, mana);
        let card = s.card(ID).id.clone();
        s.activate(&card, json!({}));
        let (instance_id, def_id) = first_cast(&s).unwrap_or_else(|| panic!("{seed}: nothing was cast"));
        assert!(pool.contains(&def_id), "{seed}: {def_id} is no Call to Chaos");
        assert!(!s.card(instance_id.as_str()).radiant, "{seed}: the edition's base face");
        assert_eq!(durability_reported(&s, &card), Some(DURABILITY - 1), "{seed}");
    }

    #[test]
    fn declares_no_tuned_numbers() {
        assert!(crate::card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn r3869001_r5236000_it_pays_2_and_casts_a_random_call_to_chaos_spending_1_durability() {
            for n in 0..SEEDS {
                vends_one_edition(&format!("fvh-115-base-{n}"), false, PRICE);
            }
        }

        #[test]
        fn r384_with_less_than_2_mana_it_is_refused() {
            let mut s = machine("fvh-115-poor", false, PRICE - 1);
            s.expect_refused(|s| s.activate(ID, json!({})));
            assert_eq!(durability_left(s.state(), s.card(ID)), Some(DURABILITY));
            assert!(first_cast(&s).is_none());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3869001_r5236000_with_no_mana_it_casts_a_random_call_to_chaos_on_its_base_face() {
            for n in 0..SEEDS {
                vends_one_edition(&format!("fvh-115-radiant-{n}"), true, 0);
            }
        }
    }
}
