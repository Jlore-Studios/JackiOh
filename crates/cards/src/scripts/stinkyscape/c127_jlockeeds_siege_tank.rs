//! Stinkyscape #127 Jlockeed's Siege Tank (SPEC §8.10 row 127). (3, Embiggened 4) Unit, Jlockeed, Rare;
//! 8/5 → 16/10.
//!   Both faces: "End of turn: This attacks a random enemy. / Embiggened (4): Starship Piece."
//!
//! End of turn (R62): a forced attack (R53, `forced_attack_random`) on one enemy, the hero or a Unit,
//! drawn from the match rng (R60) among those it may attack under §4.2 step 2, so Flying's restriction
//! holds (R783002); position, summoning sickness and Taunt are not read, no exertion is spent, and the
//! target strikes back. R1878001: the Embiggened version is a Starship Piece, a keyword its text lends
//! the card stamped embiggened (`conditionalKeywords`, as Core #56 Jilliax's Equip comes with its price,
//! R831013), so its death fuses a copy into its owner's Dormant Starship (R1852000). The Radiant face's

use jackioh_engine::effects::forced_attack_random;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-127";

/// R1878001: "Embiggened (4): Starship Piece.": the keyword holds only on a card stamped embiggened.
fn embiggened_piece(args: HookArgs<'_>) -> Vec<Keyword> {
    if args.self_.embiggened == Some(true) { vec![Keyword::StarshipPiece] } else { Vec::new() }
}

pub fn script() -> CardScripts {
    let base = Script {
        end_of_turn: Some(hook(|_ctx| {
            vec![forced_attack_random(json_as(json!({ "attacker": { "of": "self" } })))]
        })),
        conditional_keywords: Some(read_hook(embiggened_piece)),
        ..Script::default()
    };
    // One script: the Radiant face's 16/10 is catalog data, and its attack hits with it.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// Stinkyscape #127 Jlockeed's Siege Tank — SPEC §8.10 row 127, BUILD M12 row S 127 (R1852000,
// R53).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TANK: &str = "stinkyscape-127";
    const MENACE: &str = "core-019"; // (3) 9/9 Taunt.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const FILLER: &str = "core-005";

    fn standing(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [FILLER], "field": [{ "def": TANK, "lane": 1, "radiant": radiant }], "library": filler(6) },
            "p2": {
                "hand": [FILLER],
                "field": [{ "def": MENACE, "lane": 2 }, { "def": VANILLA, "lane": 4 }],
                "library": filler(6),
            },
        }))
    }

    fn attacks_by(s: &Scenario, id: &str) -> Vec<(String, bool)> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AttackDeclared { attacker_id, target_id, forced, .. } if attacker_id == id => {
                    Some((target_id.clone(), *forced))
                }
                _ => None,
            })
            .collect()
    }

    fn played(radiant: bool, embiggened: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": "siege-tank",
            "p1": {
                "hand": [{ "def": TANK, "radiant": radiant }, HIT_JOB, FILLER],
                "library": filler(6),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }));
        s.play(TANK, json!({ "zone": 1, "embiggen": embiggened }));
        s
    }

    fn is_piece(s: &Scenario, id: &str) -> bool {
        has_keyword(&s.stats(id).keywords, KeywordKind::StarshipPiece)
    }

    fn destroy(s: &mut Scenario, id: &str) {
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": id }] }));
    }

    #[test]
    fn r53_at_each_end_of_its_controllers_turn_it_attacks_a_random_enemy_taunt_not_binding() {
        let mut targets: IndexSet<String> = IndexSet::new();
        for seed in 0..16 {
            let mut s = standing(true, &format!("tank-{seed}"));
            let tank = s.unit(P1, 1).expect("the Tank").id;
            s.end_turn();
            let made = attacks_by(&s, &tank);
            assert_eq!(made.len(), 1, "seed {seed}: {made:?}");
            assert!(made[0].1, "forced");
            targets.insert(made[0].0.clone());
            s.end_turn();
            assert_eq!(attacks_by(&s, &tank).len(), 1, "seed {seed}");
        }
        assert!(targets.contains("hero-p2"), "{targets:?}");
        assert!(targets.len() == 3, "{targets:?}");
    }

    #[test]
    fn r53_no_exertion_is_spent_and_the_target_strikes_back() {
        let mut found = false;
        for seed in 0..32 {
            let mut s = standing(true, &format!("tank-menace-{seed}"));
            let tank = s.unit(P1, 1).expect("the Tank").id;
            let menace = s.unit(P2, 2).expect("the Menace").id;
            s.end_turn();
            if attacks_by(&s, &tank) != vec![(menace.clone(), true)] {
                continue;
            }
            s.expect_in_zone(menace.as_str(), "graveyard");
            s.expect_stats(tank.as_str(), json!({ "health": 1 }));
            assert!(!s.card(tank.as_str()).exertion.attacked);
            found = true;
            break;
        }
        assert!(found, "no seed drew the Menace");
    }

    #[test]
    fn r1878001_r1852000_played_at_4_it_is_a_starship_piece_and_its_death_fuses_it_into_the_dormant_starship() {
        for radiant in [false, true] {
            let mut s = played(radiant, true);
            let tank = s.unit(P1, 1).expect("the Tank").id;
            assert!(is_piece(&s, &tank), "radiant {radiant}");
            destroy(&mut s, &tank);
            s.expect_in_zone(tank.as_str(), "graveyard");
            let ship = dormant_starship_of(s.state(), P1).expect("a Dormant Starship").clone();
            let pieces: Vec<String> = starship_pieces_of(&ship).into_iter().map(|piece| piece.def_id).collect();
            assert_eq!(pieces, vec![TANK.to_string()], "radiant {radiant}");
        }
    }

    #[test]
    fn r1878001_played_at_3_it_is_no_piece_and_its_death_fuses_nothing() {
        for radiant in [false, true] {
            let mut s = played(radiant, false);
            let tank = s.unit(P1, 1).expect("the Tank").id;
            assert!(!is_piece(&s, &tank), "radiant {radiant}");
            destroy(&mut s, &tank);
            assert!(dormant_starship_of(s.state(), P1).is_none(), "radiant {radiant}");
        }
    }
}
