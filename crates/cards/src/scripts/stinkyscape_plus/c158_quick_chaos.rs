//! SPEC §8.12 S+ #158 Quick Chaos, a card of the Stinkyscape+ set (R1420, R3535000). (1) Spell, Rare.
//!   Base:    "Cast on draw: Cast a random Spell with random targets. Discard {discards|random card|random cards}." — 1.
//!   Radiant: "Cast on draw: Cast a random Radiant Spell with random targets."
//!
//! Cast on draw (§6.2, `StaticFlags.cast_on_draw`). `cast_random` draws one Spell from the collectible
//! Spells of the shipped sets (tokens out, R380, R382, R1420), never this card (R387), and casts it free
//! (R70) with every choice of the cast random, so nothing pauses for a prompt (R3557005). The Radiant
//! face casts it Radiant. R4783025: the base face then discards `discards` random cards of its caster's
//! own hand (R682), once the Spell has resolved; the number is the base face's alone (`tunedOn`, R1431).
//!
//! Issue #4783 (Stinkyscape+ Balance Changes VIII) gave the base face the discard; through v0.6.13c
//! (`script_v0_6_13c`, R767000) it discarded nothing and declared no numbers.

use jackioh_engine::effects::{CastRandomArgs, CastRandomCount, CastRandomQuery, cast_random};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-158";

/// "Cast a random Spell": one cast.
const CASTS: i32 = 1;

/// One face: the Spell, Radiant on the Radiant face, then, when `discards`, the declared discard.
fn quick_chaos(radiant: bool, discards: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |ctx| {
            let mut effects = vec![cast_random(CastRandomArgs {
                query: CastRandomQuery::Fixed(json_as(json!({ "type": [CardType::Spell] }))),
                count: Some(CastRandomCount::Fixed(CASTS)),
                radiant: Some(radiant),
                target_enemies: None,
                afterward: None,
            })];
            if discards {
                // R4783025, R682: random cards of the caster's own hand, after the Spell's cast.
                effects.push(discard_random(json_as(json!({ "count": param(&*ctx, "discards") }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: quick_chaos(false, true),
        radiant: quick_chaos(true, false),
    }
}

/// R767000: what it ran through v0.6.13c, before issue #4783: the base face discarded nothing.
pub fn script_v0_6_13c() -> CardScripts {
    CardScripts {
        base: quick_chaos(false, false),
        radiant: quick_chaos(true, false),
    }
}

// S+ #158 Quick Chaos — SPEC §8.12 row 158 (R3557005, R4783025).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::{ID, script_v0_6_13c};
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";
    /// How many seeds the discard is watched over.
    const SEEDS: usize = 12;

    fn drawn(seed: &str, radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [FILLER, FILLER, FILLER], "library": [{ "def": ID, "radiant": radiant }, FILLER, FILLER], "field": ["core-019"] },
            "p2": { "hand": [FILLER], "field": ["core-019"], "library": filler(4) },
        }));
        s.start_turn();
        s
    }

    /// The Spells cast during the draw, by definition.
    fn casts(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter(|event| event.event_type().as_str() == "cardPlayed")
            .map(crate::js)
            .filter_map(|event| event["defId"].as_str().map(str::to_string))
            .collect()
    }

    /// The cards of p1's discarded so far, in order.
    fn discards(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Discarded {
                    instance_id,
                    owner: PlayerId::P1,
                    ..
                } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    /// The draw of `seed` with Quick Chaos of `radiant`'s face on top, run on today's script and on the
    /// one kept for v0.6.13c: each run's discards of p1's cards, and whether the kept run left p1's turn
    /// going with cards in hand (its hand ids), the hand today's discard reads.
    fn both_runs(seed: &str, radiant: bool) -> (Vec<String>, Vec<String>, Option<Vec<String>>) {
        let (kept, held) = crate::under_kept(ID, script_v0_6_13c(), || {
            let s = drawn(seed, radiant);
            let going = s.state().active == P1 && !s.hand(P1).is_empty();
            let held: Vec<String> = s.hand(P1).iter().map(|card| card.id.clone()).collect();
            (discards(&s), going.then_some(held))
        });
        (discards(&drawn(seed, radiant)), kept, held)
    }

    #[test]
    fn r3557005_a_random_spell_is_cast_and_no_prompt_opens() {
        crate::register_all();
        for n in 0..10 {
            let mut s = drawn(&format!("r3557005-{n}"), false);
            s.expect_in_zone(ID, "graveyard");
            assert!(s.state().pending.is_none(), "{n}: nothing paused");
            let cast = casts(&s);
            assert!(cast.iter().any(|id| id == ID), "{n}: it was cast on draw");
            let others: Vec<&String> = cast.iter().filter(|id| id.as_str() != ID).collect();
            // The one random Spell is the first cast after it; a Spell that casts others (Call to Chaos)
            // leaves more cards played behind it, which are its own.
            assert!(!others.is_empty(), "{n}: one random Spell");
            let def = catalog::def_of(None, others[0]);
            assert_eq!(def.type_, CardType::Spell);
            assert!(!def.token);
            assert_ne!(def.id, ID);
        }
    }

    /// R4783025: against the same draw on the script kept for v0.6.13c, the base face discards one card
    /// more, after every discard the random Spell made, and it is a card of the hand that Spell left.
    #[test]
    fn r4783025_the_base_face_discards_a_random_card_of_its_hand_once_its_spell_has_resolved() {
        crate::register_all();
        let mut watched = 0;
        for n in 0..SEEDS {
            let (now, kept, held) = both_runs(&format!("r4783025-{n}"), false);
            let Some(held) = held else {
                continue;
            };
            watched += 1;
            assert_eq!(now.len(), kept.len() + 1, "{n}: one discard more: {now:?} against {kept:?}");
            assert_eq!(now[..kept.len()], kept[..], "{n}: the Spell's own discards come first");
            assert!(held.contains(&now[kept.len()]), "{n}: {} is no card of {held:?}", now[kept.len()]);
        }
        assert!(watched * 2 >= SEEDS, "PREMISE: most draws leave the turn going with a hand: {watched}");
    }

    #[test]
    fn r4783025_the_discard_is_its_declared_number() {
        let s = scenario(json!({ "p1": { "hand": [ID, FILLER] } }));
        let held = s.card(ID).clone();
        let discards = param_value(s.state(), Some(&held), "discards", ParamValueOptions::default());
        assert_eq!(discards, 1);
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_radiant_face_casts_a_radiant_spell() {
            crate::register_all();
            let s = drawn("r3557005-radiant", true);
            assert!(s.state().pending.is_none());
            let cast: Vec<CardInstance> = s
                .events()
                .iter()
                .filter(|event| event.event_type().as_str() == "cardPlayed")
                .map(crate::js)
                .filter_map(|event| event["instanceId"].as_str().and_then(|id| find_instance(s.state(), id).cloned()))
                .collect();
            assert!(cast.iter().any(|card| card.def_id != ID), "a random Spell was cast");
            assert!(cast.iter().all(|card| card.radiant), "every card cast is Radiant: {cast:?}");
        }

        #[test]
        fn r4783025_the_radiant_face_discards_nothing_of_its_own() {
            crate::register_all();
            for n in 0..SEEDS {
                let (now, kept, _) = both_runs(&format!("r4783025-radiant-{n}"), true);
                assert_eq!(now, kept, "{n}: only the Spell's discards");
            }
        }
    }

    /// R767000: the script kept for the replays of v0.6.13c and before (issue #4783), under its v0.6.13c
    /// definition, which declared no numbers.
    mod kept_v0_6_13c {
        use super::*;

        #[test]
        fn r767000_r3557005_its_base_face_cast_a_random_spell_and_discarded_nothing_of_its_own() {
            crate::under_kept_as(
                ID,
                script_v0_6_13c(),
                |def| def.params = None,
                || {
                    for n in 0..3 {
                        let mut s = drawn(&format!("r767000-158-{n}"), false);
                        s.expect_in_zone(ID, "graveyard");
                        assert!(s.state().pending.is_none(), "{n}: nothing paused");
                        assert!(casts(&s).iter().any(|id| id != ID), "{n}: a random Spell was cast");
                    }
                },
            );
        }
    }
}
