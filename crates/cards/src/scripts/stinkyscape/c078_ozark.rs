//! Stinkyscape #78 Ozark (SPEC §8.10 row 78). Unit 4/4 → 8/8, cost 3, Epic.
//!   Base:    "Cry: Equip a random Miner into this."
//!   Radiant: "Cry: Equip a random Radiant Miner into this, twice."
//!
//! R1872001: the Cry is `equip_picks` (R1841002), a random pick from the five Miners, S #62 Miner v1 to
//! S #66 Miner v5, named by id so that the pool reaches them whether or not the set ships (R1420): a
//! fresh copy fused straight into this, as an Equip play fuses its card (R1086, R77), with no hand, no
//! play and no Cry, its Activate with it (R1751000). The Radiant face picks twice, each pick on its
//! Radiant face (R469), the second from the Miners it does not hold yet. Off the field, Immutable, or
//! holding every Miner, nothing (R129). Since #2521 it is no Starship Piece, and the Miners it takes,
//! each a piece, leave it none (R1852001: a Fuse keeps the keyword only from the card it keeps), so its
//! death fuses nothing (R1852000 reads the keyword).

use jackioh_engine::effects::equip_picks;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-078";

/// S #62 Miner v1 to S #66 Miner v5.
const MINERS: [&str; 5] = [
    "stinkyscape-062",
    "stinkyscape-063",
    "stinkyscape-064",
    "stinkyscape-065",
    "stinkyscape-066",
];

/// "twice", on the Radiant face.
const RADIANT_PICKS: i32 = 2;

/// R1872001: "Equip a random [Radiant] Miner into this[, twice]".
fn prospect(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            let mut args = json!({ "query": { "defId": MINERS }, "pick": "random" });
            if radiant {
                args["times"] = json!(RADIANT_PICKS);
                args["radiant"] = json!(true);
            }
            vec![equip_picks(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: prospect(false),
        radiant: prospect(true),
    }
}

// Stinkyscape #78 Ozark — SPEC §8.10 row 78, BUILD M12 row S 78 (R1841002). Since #2521 it is no Starship
// Piece, its Miners leaving it none (R1852001), and its death fuses nothing.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon_board;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-005"; // (1) Spell.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.

    /// p1 plays the Ozark (Radiant on `radiant`) into lane 1; returns the scenario and its id.
    fn prospected(radiant: bool, seed: &str) -> (Scenario, String) {
        let mut s = weapon_board(seed, json!({ "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 }), json!({}));
        s.play(ID, json!({ "zone": 1 }));
        let host = s.unit_at(P1, 1).id;
        (s, host)
    }

    fn miners_in(s: &Scenario, host: &str) -> Vec<String> {
        self_def_ids(Some(s.state()), &s.card(host).def_id)
            .into_iter()
            .filter(|id| MINERS.contains(&id.as_str()))
            .collect()
    }

    fn fusions(s: &Scenario) -> usize {
        s.last_events()
            .iter()
            .filter(|event| event.event_type() == GameEventType::Fused)
            .count()
    }

    #[test]
    fn r1872001_its_cry_equips_one_random_miner_with_its_activate_and_leaves_it_no_piece() {
        let (mut s, host) = prospected(false, "s078");
        assert!(s.state().pending.is_none(), "a random pick asks nothing");
        assert_eq!(fusions(&s), 1);
        let miners = miners_in(&s, &host);
        assert_eq!(miners.len(), 1, "{miners:?}");
        // R77: a plain host takes its Miner's base health.
        let gained = crate::card_def(&miners[0]).base.health.unwrap_or_default();
        s.expect_stats(host.as_str(), json!({ "attack": 4, "health": 4 + gained }));
        // R1852001: the Miner is a piece, the kept Ozark none, so the fusion is none.
        assert!(!unit_has(s.state(), s.card(host.as_str()), KeywordKind::StarshipPiece));
        // The Miner's Activate came with it.
        s.activate(host.as_str(), json!({ "ability": "mine" }));
        assert_eq!(resources_in_hand_of(s.state(), P1).len(), 1);
        // R60: the pick replays from the seed.
        let (again, other) = prospected(false, "s078");
        assert_eq!(miners_in(&again, &other), miners);
    }

    #[test]
    fn r1841002_the_pick_is_drawn_from_all_five_miners() {
        let mut seen: IndexSet<String> = IndexSet::new();
        for seed in 0..24 {
            let (s, host) = prospected(false, &format!("s078-{seed}"));
            seen.extend(miners_in(&s, &host));
        }
        assert_eq!(seen.len(), MINERS.len(), "{seen:?}");
    }

    #[test]
    fn radiant_8_8_and_two_different_radiant_miners() {
        for seed in 0..8 {
            let (mut s, host) = prospected(true, &format!("s078-radiant-{seed}"));
            assert!(s.state().pending.is_none());
            assert_eq!(fusions(&s), 2);
            let miners = miners_in(&s, &host);
            assert_eq!(miners.len(), 2, "{miners:?}");
            assert_ne!(miners[0], miners[1]);
            let gained: i32 = miners
                .iter()
                .map(|id| crate::card_def(id).radiant.health.unwrap_or_default())
                .sum();
            s.expect_stats(host.as_str(), json!({ "attack": 8, "health": 8 + gained }));
            assert!(!unit_has(s.state(), s.card(host.as_str()), KeywordKind::StarshipPiece));
        }
    }

    #[test]
    fn its_death_fuses_nothing_it_is_no_starship_piece() {
        let mut s = crate::scenario(json!({
            "p1": { "hand": [HIT_JOB, FILLER], "field": [ID], "library": crate::filler(4) },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        let me = s.card(ID).clone();
        assert!(!unit_has(s.state(), &me, KeywordKind::StarshipPiece));
        s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));
        s.expect_in_zone(&me, "graveyard");
        assert!(dormant_starship_of(s.state(), P1).is_none(), "no Dormant Starship");
        assert!(!s.events().iter().any(|event| matches!(event, GameEvent::StarshipFused { .. })));
    }

    #[test]
    fn its_death_holding_a_miner_fuses_nothing() {
        // R1852001: the Miner it holds is a piece, but the fusion keeps the Ozark, which is none.
        let mut s = weapon_board(
            "s078-death",
            json!({ "hand": [ID, HIT_JOB], "mana": 10 }),
            json!({}),
        );
        s.play(ID, json!({ "zone": 1 }));
        let host = s.unit_at(P1, 1);
        assert_eq!(miners_in(&s, &host.id).len(), 1);
        s.play(HIT_JOB, json!({ "targets": pick::instance(&host) }));
        s.expect_in_zone(&host, "graveyard");
        assert!(dormant_starship_of(s.state(), P1).is_none(), "no Dormant Starship");
    }
}
