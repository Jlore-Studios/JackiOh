//! SPEC §8.12 S+ #142.2 Swarm Golem (R1420, R3535000, R3570001, R3570006): (4) Unit, Token (printed
//! Legendary), Mindustrial, 5/5 → 10/10, of the Stinkyscape+ set. Rush.
//!   Base:    "After this attacks summon a Swarm card that is Evolved."
//!   Radiant: "After this attacks summon a Radiant Swarm card that is Evolved."
//! Engine: Rush is a catalog keyword. `after_attack` (R1849001: once the state check closing each of its
//! combats has run, only while it stands on the field, R2285001) summons one random permanent of the Swarm
//! tag (R3570006: a Unit, Field Spell, Trap or Field Trap, never a Spell or a token, held sets' cards
//! included, R1420), placed per R64 with no Cry (R1), `summon_random` drawing only when a zone is open
//! (R129). It lands marked to enter evolved (`entersEvolved`, R3570001: only when its text prints an
//! Evolve hook, R3536002), so the state check after it lands runs its Evolve text; Radiant on the
//! Radiant face. Its maker is S+ #142 Call to Chaos (Mindustrial-Swarm Edition), entry 9 (#3570).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-142-2";

/// R3570006: the permanents a Golem's pool takes, as every random summon's does (R64's rows).
const PERMANENTS: [&str; 4] = ["Unit", "Field Spell", "Trap", "Field Trap"];

/// "After this attacks summon a [Radiant] Swarm card that is Evolved."
fn golem(radiant: bool) -> Script {
    Script {
        after_attack: Some(hook(move |_ctx| {
            vec![summon_random_with(json_as(json!({
                "query": { "set": CATALOG_SETS, "tags": [Tag::Swarm], "type": PERMANENTS },
                "radiant": radiant,
                "entersEvolved": true,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: golem(false),
        radiant: golem(true),
    }
}

// S+ #142.2 Swarm Golem — SPEC §8.12 row 142.2, R3570001, R3570006.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;

    const FILLER: &str = "core-005"; // Stockpile: a card in hand, so no turn ends by itself
    /// A defender that survives the Golem's hit and barely scratches it back, and one that kills it.
    const WALL: &str = "stand-in (1) 1/99";
    const KILLER: &str = "stand-in (1) 99/99";
    const WASP: &str = "stinkyscape-091"; // The Propagation's Wasp: "Evolve: Gain Flying"
    const SEED: &str = "golem-swarm";

    fn board(seed: &str, radiant: bool, defender: &str) -> Scenario {
        crate::scenario(json!({
            "seed": seed,
            "p1": { "hand": [FILLER], "field": [{ "def": ID, "radiant": radiant }], "library": crate::filler(4) },
            "p2": { "hand": [FILLER], "field": [defender], "library": crate::filler(4) },
        }))
    }

    /// What the Golem's attack summoned, off the Golem's own side.
    fn summoned(s: &Scenario) -> Vec<CardInstance> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned {
                    instance_id, def_id, ..
                } if def_id != ID => Some(s.card(instance_id).clone()),
                _ => None,
            })
            .collect()
    }

    fn swing(s: &mut Scenario) {
        let golem = s.unit_at(P1, 1);
        let target = s.unit_at(P2, 1);
        s.attack(&golem, &target);
    }

    /// The first game over the seeds whose Golem's swing summons `def_id`.
    fn landing(radiant: bool, def_id: &str) -> Scenario {
        for n in 0..400 {
            let mut s = board(&format!("{SEED}-{n}"), radiant, WALL);
            swing(&mut s);
            if summoned(&s).first().is_some_and(|card| card.def_id == def_id) {
                return s;
            }
        }
        panic!("no seed of {SEED} summoned {def_id}");
    }

    #[test]
    fn its_faces_are_5_5_and_10_10_with_rush() {
        for (radiant, stats) in [
            (false, json!({ "attack": 5, "health": 5 })),
            (true, json!({ "attack": 10, "health": 10 })),
        ] {
            let mut s = board(SEED, radiant, WALL);
            s.expect_stats(ID, stats);
            assert!(has_keyword(&s.stats(ID).keywords, KeywordKind::Rush));
        }
    }

    #[test]
    fn r3570006_after_it_attacks_it_summons_an_evolved_swarm_permanent() {
        // The Wasp prints an Evolve hook, so it lands marked and the state check after it evolves it.
        let s = landing(false, WASP);
        let wasp = summoned(&s).remove(0);
        assert_eq!(wasp.evolved, Some(true), "R3570001: evolved as it landed");
        assert!(
            has_keyword(&s.stats(&wasp).keywords, KeywordKind::Flying),
            "its Evolve text ran"
        );
        assert!(!wasp.radiant);
        // Every pick of a few games is a Swarm permanent, never a token or a Spell. The Golem summons
        // one, the first; a card it summons may then copy itself by its own Evolve text (S+ #161 Space
        // Invader: "Evolve: Summon a copy of this"), so every later summon is a copy of that one.
        for n in 0..12 {
            let mut s = board(&format!("{SEED}-pick-{n}"), false, WALL);
            swing(&mut s);
            let made = summoned(&s);
            assert!(!made.is_empty(), "one summon per attack");
            assert!(
                made[1..].iter().all(|copy| copy.def_id == made[0].def_id),
                "one summon per attack, then only its own copies: {:?}",
                made.iter().map(|card| card.def_id.as_str()).collect::<Vec<_>>()
            );
            let def = crate::card_def(&made[0].def_id);
            assert!(def.tags.contains(&Tag::Swarm), "{}", def.id);
            assert!(!def.token && def.type_ != CardType::Spell, "{}", def.id);
        }
    }

    #[test]
    fn r3570001_a_swarm_card_with_no_evolve_text_lands_as_it_is() {
        // #75 prints no Evolve hook: the mark is given to a card that can evolve, none to one that cannot.
        crate::register_all();
        let mut checked = 0;
        for n in 0..400 {
            let mut s = board(&format!("{SEED}-plain-{n}"), false, WALL);
            swing(&mut s);
            let made = summoned(&s);
            let Some(card) = made.first() else { continue };
            let can = jackioh_engine::evolution::can_evolve(s.state(), card);
            if !can && card.evolved != Some(true) {
                assert_ne!(card.enters_evolved, Some(true), "{}", card.def_id);
                checked += 1;
            }
            if checked >= 3 {
                break;
            }
        }
        assert!(checked >= 3, "plain Swarm cards were drawn");
    }

    #[test]
    fn r3570006_the_radiant_face_summons_a_radiant_one() {
        let mut s = board(SEED, true, WALL);
        swing(&mut s);
        let made = summoned(&s);
        assert_eq!(made.len(), 1);
        assert!(made[0].radiant);
        assert!(crate::card_def(&made[0].def_id).tags.contains(&Tag::Swarm));
    }

    #[test]
    fn r3570006_a_golem_killed_attacking_summons_nothing() {
        let mut s = board(SEED, false, KILLER);
        swing(&mut s);
        // A token that dies ceases to exist (R11); the kill is the record.
        assert!(
            s.events()
                .iter()
                .any(|event| matches!(event, GameEvent::Destroyed { def_id, .. } if def_id == ID))
        );
        assert!(summoned(&s).is_empty());
    }
}
