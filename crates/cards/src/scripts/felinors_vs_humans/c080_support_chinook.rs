//! FvH #80 Support Chinook (SPEC §8.14 row 80): (4) Unit, Jlockeed, Epic, 2/7 → 4/14.
//!   Base:    "Flying, Ranged, Durability 3\nActivate: Draw {draw|card|cards} and gain {mana} mana.\nActivate:
//!            Summon a random (2) Cost Human." (1, 1)
//!   Radiant: "Flying, Ranged, Durability 6\nActivate: Draw {draw|card|cards} and gain {mana} mana.\nActivate:
//!            Summon a random Radiant (2) Cost Human." (2, 2)
//!
//! Engine: Flying (R783002) and Ranged (R783000) are catalog data. Two Activates (R384), each once a turn
//! (the list's "Activate 1", printed "Activate:"): `draw` of `draw` and `gain_mana` of `mana`, this turn's
//! mana (§2.3); and `summon_random` of one non-token (2) Cost Human Unit of the sets that ship, Radiant on
//! the Radiant face, into its controller's leftmost open unit zone (R64, no Cry). Durability 3 (Radiant 6)
//! is the engine's keyword: one counter for both Activates, each use spending 1 (R5236000), refused at 0
//! while the Unit stays on the field (R5236001), never given back by Refresh (R5236003). Tunes: draw 1 ↑,
//! mana 1 ↑ (Radiant 2 each); Durability by the keyword's row (R5236005).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-080";

/// The two Activates' ids.
const RESUPPLY: &str = "resupply";
const AIRDROP: &str = "airdrop";
/// Each Activate is once a turn (R384).
const USES: i32 = 1;
/// "A random (2) Cost Human."
const HUMAN_COST: i32 = 2;

fn resupply() -> ActivationDecl {
    ActivationDecl {
        id: RESUPPLY.to_string(),
        label: "Draw and gain mana".to_string(),
        uses: ActivationUses::Count(USES),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(|ctx| {
            vec![
                draw(json_as(json!({ "count": param(&*ctx, "draw") }))),
                gain_mana(json_as(json!({ "amount": param(&*ctx, "mana") }))),
            ]
        }),
    }
}

fn airdrop(radiant: bool) -> ActivationDecl {
    ActivationDecl {
        id: AIRDROP.to_string(),
        label: if radiant {
            "Summon a random Radiant (2) Cost Human".to_string()
        } else {
            "Summon a random (2) Cost Human".to_string()
        },
        uses: ActivationUses::Count(USES),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(move |_ctx| {
            vec![summon_random(json_as(json!({
                "query": { "type": "Unit", "cost": HUMAN_COST, "tags": ["Human"] },
                "radiant": radiant,
            })))]
        }),
    }
}

fn face(radiant: bool) -> Script {
    Script {
        activations: vec![resupply(), airdrop(radiant)],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// FvH #80 Support Chinook — SPEC §8.14 row 80 (R384, R64, R5236000, R5236001, R5236003).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, run_as, scenario};

    const P1: PlayerId = PlayerId::P1;

    /// A textless Spell each seat can always play, so no turn ends by itself (§2.5).
    const SPELL: &str = "stand-in (0) Spell";
    /// Its printed numbers: base, then Radiant.
    const DRAW: usize = 1;
    const MANA: i32 = 1;
    const DURABILITY: i32 = 3;
    const RADIANT_DRAW: usize = 2;
    const RADIANT_MANA: i32 = 2;
    const RADIANT_DURABILITY: i32 = 6;
    /// How many seeds the random Human is drawn under.
    const SEEDS: usize = 6;

    fn chinook(seed: &str, radiant: bool) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [SPELL],
                "field": [{ "def": ID, "radiant": radiant }],
                "mana": 3,
                "library": filler(6),
            },
            "p2": { "hand": [SPELL], "library": filler(6) },
        }))
    }

    fn ability(name: &str) -> Value {
        json!({ "ability": name })
    }

    /// The Units the last step summoned for p1: (instance id, def id, lane).
    fn summoned(s: &Scenario) -> Vec<(String, String, i32)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { player, instance_id, def_id, row, lane, .. }
                    if *player == P1 && *row == Row::Units =>
                {
                    Some((instance_id.clone(), def_id.clone(), *lane))
                }
                _ => None,
            })
            .collect()
    }

    /// One use of the second Activate summoned one non-token (2) Cost Human Unit into lane 2.
    fn one_human(s: &Scenario, seed: &str) -> String {
        let landed = summoned(s);
        assert_eq!(landed.len(), 1, "{seed}: {landed:?}");
        let (instance_id, def_id, lane) = &landed[0];
        let def = crate::card_def(def_id);
        assert_eq!(def.type_, CardType::Unit, "{seed}: {def_id}");
        assert_eq!(query_cost(&def), HUMAN_COST, "{seed}: {def_id}");
        assert!(def.tags.contains(&Tag::Human), "{seed}: {def_id}");
        assert!(!def.token, "{seed}: {def_id}");
        assert_eq!(*lane, 2, "{seed}");
        instance_id.clone()
    }

    /// The first Activate: the hand grows by `drawn` and the mana by `gained`.
    fn resupplies(s: &mut Scenario, drawn: usize, gained: i32) {
        let hand = s.hand(P1).len();
        let mana = s.state().players[P1].mana.current;
        s.activate(ID, ability(RESUPPLY));
        assert_eq!(s.hand(P1).len(), hand + drawn);
        s.expect_mana(P1, mana + gained);
    }

    mod base {
        use super::*;

        #[test]
        fn r384_its_first_activate_draws_1_and_gains_1_mana_once_a_turn() {
            let mut s = chinook("fvh-080-resupply", false);
            resupplies(&mut s, DRAW, MANA);
            s.expect_refused(|s| s.activate(ID, ability(RESUPPLY)));
        }

        #[test]
        fn r64_its_second_activate_summons_a_random_2_cost_human() {
            for n in 0..SEEDS {
                let seed = format!("fvh-080-base-{n}");
                let mut s = chinook(&seed, false);
                s.activate(ID, ability(AIRDROP));
                let human = one_human(&s, &seed);
                assert!(!s.card(human.as_str()).radiant, "{seed}");
            }
        }

        #[test]
        fn r5236000_r5236001_both_activates_spend_one_durability_3_and_at_0_both_are_refused_while_it_stays() {
            let mut s = chinook("fvh-080-walk", false);
            resupplies(&mut s, DRAW, MANA);
            s.activate(ID, ability(AIRDROP));
            assert_eq!(durability_left(s.state(), s.card(ID)), Some(DURABILITY - 2));
            run_as(&mut s, refresh_activates(), P1);
            s.activate(ID, ability(RESUPPLY));
            assert_eq!(durability_left(s.state(), s.card(ID)), Some(0));
            s.expect_refused_with(|s| s.activate(ID, ability(AIRDROP)), "Durability");
            s.expect_in_zone(ID, "field");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r384_its_first_activate_draws_2_and_gains_2_mana() {
            let mut s = chinook("fvh-080-radiant-resupply", true);
            resupplies(&mut s, RADIANT_DRAW, RADIANT_MANA);
            assert_eq!(durability_left(s.state(), s.card(ID)), Some(RADIANT_DURABILITY - 1));
        }

        #[test]
        fn r64_its_second_activate_summons_a_random_radiant_2_cost_human() {
            for n in 0..SEEDS {
                let seed = format!("fvh-080-radiant-{n}");
                let mut s = chinook(&seed, true);
                s.activate(ID, ability(AIRDROP));
                let human = one_human(&s, &seed);
                assert!(s.card(human.as_str()).radiant, "{seed}");
            }
        }
    }
}
