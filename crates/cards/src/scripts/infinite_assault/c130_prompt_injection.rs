//! SPEC §8.13 IA #130 Prompt Injection ((2) Spell, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Your opponent's next Spell has its targets chosen randomly."
//!   Radiant: "Your opponent's next two Spells have their targets chosen randomly."
//!
//! One `used` modifier per affected Spell on the opponent (R4590001): only the declared targets of
//! that player's face-up Spell plays are drawn at random, each spending one modifier at §10.5 step 4.
//! Unit plays, face-down sets and the caster's own Spells choose as usual; a countered Spell spends
//! none. The modifier shows as a badge.

use jackioh_engine::effects::random_targets_next_spells;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-130";

/// "Your opponent's next Spell" (base) or "next two Spells" (Radiant).
const SPELLS: i32 = 1;
const RADIANT_SPELLS: i32 = 2;

fn injection(spells: i32) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![random_targets_next_spells(json_as(json!({
                "count": spells,
                "player": "enemy",
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: injection(SPELLS),
        radiant: injection(RADIANT_SPELLS),
    }
}

// IA #130 Prompt Injection — SPEC §8.13 row 130 (R4590001).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const BURN: &str = "classic-036"; // Burn: deal damage to any unit or hero
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "prompt-injection",
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": [VANILLA],
                "mana": 10,
                "library": crate::filler(3),
            },
            "p2": { "hand": [BURN, BURN, BURN, FILLER], "mana": 10, "library": crate::filler(6) },
        }))
    }

    /// How many plays of `BURN` p2 is offered, and whether the first carries targets.
    fn offered(s: &Scenario) -> (usize, bool) {
        let plays = s.plays_of(BURN);
        let targeted = plays.iter().any(|play| matches!(
            play,
            ActionBody::Play { targets: Some(_), .. }
        ));
        (plays.len(), targeted)
    }

    /// p2's turn: one Burn listed with no picks, played and drawn at random; back to p1's turn.
    fn burn_at_random(s: &mut Scenario) {
        s.end_turn();
        let (count, targeted) = offered(s);
        assert_eq!(count, 1, "one play, not one per target");
        assert!(!targeted, "no targets while the modifier holds");
        s.play(BURN, json!({}));
        s.end_turn();
    }

    /// p2's turn again: the next Burn is listed with its targets.
    fn chooses_again(s: &mut Scenario) {
        s.end_turn();
        let (count, targeted) = offered(s);
        assert!(count > 1 && targeted, "the Spell chooses again: {count} plays");
        s.end_turn();
    }

    mod base {
        use super::*;

        #[test]
        fn r4590001_the_next_spell_s_targets_are_drawn_and_the_following_spell_chooses_again() {
            let mut s = board(false);
            s.play(ID, json!({}));
            burn_at_random(&mut s);
            chooses_again(&mut s);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4590001_the_next_two_spells_targets_are_drawn_and_the_third_chooses_again() {
            let mut s = board(true);
            s.play(ID, json!({}));
            burn_at_random(&mut s);
            burn_at_random(&mut s);
            chooses_again(&mut s);
        }
    }
}
