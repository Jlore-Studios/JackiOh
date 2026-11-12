//! SPEC §8.12 S+ #142.1 Mindustrial Golem (R1420, R3535000, R3570006): (4) Unit, Token (printed Legendary),
//! Mindustrial, 5/5 → 10/10, of the Stinkyscape+ set. Rush, Ranged.
//!   Base:    "After this attacks summon a random Mindustrial card that can't attack."
//!   Radiant: "After this attacks summon a random Radiant Mindustrial card that can't attack."
//! Engine: Rush and Ranged are catalog keywords. `after_attack` (R1849001: once the state check closing
//! each of its combats has run, only while it stands on the field, R2285001) summons one random permanent
//! of the Mindustrial tag (R3570006: a Unit, Field Spell, Trap or Field Trap, never a Spell or a token,
//! held sets' cards included, R1420), placed per R64 with no Cry (R1), `summon_random` drawing only when a
//! zone is open (R129). It lands with Can't attack (`keywords`, the rider `summon` has), Radiant on the
//! Radiant face. Its maker is S+ #142 Call to Chaos (Mindustrial-Swarm Edition), entry 9 (#3570).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-142-1";

/// R3570006: the permanents a Golem's pool takes, as every random summon's does (R64's rows).
const PERMANENTS: [&str; 4] = ["Unit", "Field Spell", "Trap", "Field Trap"];

/// "After this attacks summon a random [Radiant] Mindustrial card that can't attack."
fn golem(radiant: bool) -> Script {
    Script {
        after_attack: Some(hook(move |_ctx| {
            vec![summon_random_with(json_as(json!({
                "query": { "set": CATALOG_SETS, "tags": [Tag::Mindustrial], "type": PERMANENTS },
                "radiant": radiant,
                "keywords": [{ "kind": "Can't attack" }],
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

// S+ #142.1 Mindustrial Golem — SPEC §8.12 row 142.1, R3570006.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;

    const FILLER: &str = "core-005"; // Stockpile: a card in hand, so no turn ends by itself
    /// A defender that survives the Golem's hit, and one that would kill it if it struck back.
    const WALL: &str = "stand-in (1) 99/99";
    const KILLER: &str = "stand-in (1) 99/99 Ranged";
    const SEED: &str = "golem-mindustrial";

    fn board(radiant: bool, defender: &str) -> Scenario {
        crate::scenario(json!({
            "seed": SEED,
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

    #[test]
    fn its_faces_are_5_5_and_10_10_with_rush_and_ranged() {
        for (radiant, stats) in [
            (false, json!({ "attack": 5, "health": 5 })),
            (true, json!({ "attack": 10, "health": 10 })),
        ] {
            let mut s = board(radiant, WALL);
            s.expect_stats(ID, stats);
            let view = s.stats(ID);
            assert!(has_keyword(&view.keywords, KeywordKind::Rush));
            assert!(has_keyword(&view.keywords, KeywordKind::Ranged));
        }
    }

    #[test]
    fn r3570006_after_it_attacks_it_summons_a_mindustrial_permanent_that_can_t_attack() {
        // Every pick of a few games is a Mindustrial permanent of the Mindustrial pool, never a token or a Spell.
        let mut units = 0;
        for n in 0..12 {
            let mut s = crate::scenario(json!({
                "seed": format!("{SEED}-{n}"),
                "p1": { "hand": [FILLER], "field": [ID], "library": crate::filler(4) },
                "p2": { "hand": [FILLER], "field": [WALL], "library": crate::filler(4) },
            }));
            swing(&mut s);
            let made = summoned(&s);
            assert_eq!(made.len(), 1, "one summon per attack");
            let def = crate::card_def(&made[0].def_id);
            assert!(def.tags.contains(&Tag::Mindustrial), "{}", def.id);
            assert!(!def.token && def.type_ != CardType::Spell, "{}", def.id);
            assert!(!made[0].radiant, "a base Golem's pick is base");
            if def.type_ == CardType::Unit {
                units += 1;
                assert!(
                    jackioh_engine::layers::unit_has(s.state(), &made[0], KeywordKind::CantAttack),
                    "{} can't attack",
                    def.id
                );
                let id = made[0].id.clone();
                s.expect_in_zone(&id, "field");
            }
        }
        assert!(units > 0, "a Unit was among the picks");
    }

    #[test]
    fn r3570006_the_radiant_face_summons_a_radiant_one() {
        let mut s = board(true, WALL);
        swing(&mut s);
        let made = summoned(&s);
        assert_eq!(made.len(), 1);
        assert!(made[0].radiant);
        assert!(crate::card_def(&made[0].def_id).tags.contains(&Tag::Mindustrial));
    }

    #[test]
    fn r3570006_a_golem_killed_attacking_summons_nothing() {
        let mut s = board(false, KILLER);
        swing(&mut s);
        // A token that dies ceases to exist (R11); the kill is the record.
        assert!(
            s.events()
                .iter()
                .any(|event| matches!(event, GameEvent::Destroyed { def_id, .. } if def_id == ID))
        );
        assert!(summoned(&s).is_empty());
    }

    #[test]
    fn r3570006_with_no_open_zone_it_summons_nothing() {
        let mut s = crate::scenario(json!({
            "seed": SEED,
            "p1": {
                "hand": [FILLER],
                "field": [ID, "stand-in (1) 1/1", "stand-in (1) 1/1", "stand-in (1) 1/1", "stand-in (1) 1/1"],
                "backrow": ["stand-in (1) Field Spell", "stand-in (1) Field Spell", "stand-in (1) Field Spell", "stand-in (1) Field Spell", "stand-in (1) Field Spell"],
                "library": crate::filler(4),
            },
            "p2": { "hand": [FILLER], "field": [WALL], "library": crate::filler(4) },
        }));
        let before = s.state().rng_cursor;
        swing(&mut s);
        assert!(summoned(&s).is_empty());
        assert_eq!(s.state().rng_cursor, before, "R129: nothing drawn");
    }
}
