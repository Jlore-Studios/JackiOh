//! Stinkyscape #80 Galaxy (SPEC §8.10 row 80). Unit 10/10 → 20/20, cost 4, Legendary.
//!   Base:    "Tribute 2, Flying
//!            Cry: Discover 2 Radiant Miners and Equip them into this."
//!   Radiant: "Tribute 2, Flying
//!            Cry: Equip a Radiant copy of every Miner into this."
//!
//! Tribute 2 is the play validator's (§6.3, R101), read off `staticFlags.tribute`, the keyword line's
//! printed Tribute (R366). R1872003: the Cry is `equip_picks` (R1841002) from the five Miners, S #62 Miner v1
//! to S #66 Miner v5, named by id so that the pool reaches them whether or not the set ships (R1420),
//! each pick a fresh copy on its Radiant face (R469) fused straight into this (R1086, R77), its Activate
//! with it (R1751000): two Discovers of three in turn, the second without the first; the Radiant face
//! fuses all five, v1 first, asking nothing. Flying is a catalog keyword the engine plays. Since #2521 it
//! is no Starship Piece, and the Miners it takes, each a piece, leave it none (R1852001), so its death
//! fuses nothing (R1852000 reads the keyword).

use jackioh_engine::effects::equip_picks;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-080";

/// "Tribute 2" (R101).
const TRIBUTE: i32 = 2;

/// S #62 Miner v1 to S #66 Miner v5, in the order the Radiant face fuses them.
const MINERS: [&str; 5] = [
    "stinkyscape-062",
    "stinkyscape-063",
    "stinkyscape-064",
    "stinkyscape-065",
    "stinkyscape-066",
];

/// "Discover 2".
const PICKS: i32 = 2;

fn tribute() -> Option<StaticFlags> {
    Some(StaticFlags {
        tribute: Some(TRIBUTE),
        ..StaticFlags::default()
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: tribute(),
        cry: Some(hook(|_ctx| {
            vec![equip_picks(json_as(json!({
                "query": { "defId": MINERS },
                "times": PICKS,
                "radiant": true,
            })))]
        })),
        ..Script::default()
    };
    // R1872003: "a Radiant copy of every Miner", one by one, v1 to v5.
    let radiant = Script {
        static_flags: tribute(),
        cry: Some(hook(|_ctx| {
            MINERS
                .iter()
                .map(|miner| {
                    equip_picks(json_as(json!({
                        "query": { "defId": miner },
                        "pick": "random",
                        "radiant": true,
                    })))
                })
                .collect()
        })),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// Stinkyscape #80 Galaxy — SPEC §8.10 row 80, BUILD M12 row S 80: "Tribute 2: two of your Units sacrificed
// as it is played (R101); its Cry asks two Discovers in turn, each three of the five Radiant Miners it does
// not hold, and fuses each pick into it (R1841002); radiant 20/20 and all five Radiant Miners fused, no
// prompt". Since #2521 it is no Starship Piece, its Miners leaving it none (R1852001).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon_board;
    use jackioh_engine::effects::EQUIP_PICK_STEP;
    use jackioh_engine::play_choices::tribute_cost_of;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005"; // (1) Spell.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.

    /// p1 plays the Galaxy (Radiant on `radiant`) into lane 3, its two Vanillas the Tribute.
    fn played(radiant: bool) -> (Scenario, String) {
        let mut s = weapon_board(
            "s080",
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "field": [VANILLA, VANILLA], "mana": 10 }),
            json!({}),
        );
        let tributes = [1, 2].map(|lane| s.unit_at(P1, lane).id);
        s.play(ID, json!({ "zone": 3, "tributes": tributes }));
        let host = s.unit_at(P1, 3).id;
        (s, host)
    }

    fn radiant_health(id: &str) -> i32 {
        crate::card_def(id).radiant.health.unwrap_or_default()
    }

    #[test]
    fn r101_it_needs_tribute_2() {
        let mut s = weapon_board("s080-tribute", json!({ "hand": [ID], "field": [VANILLA, VANILLA] }), json!({}));
        assert_eq!(tribute_cost_of(s.state(), s.card(ID)), TRIBUTE);
        let first = s.unit_at(P1, 1).id;
        s.expect_refused(|s| s.play(ID, json!({ "zone": 3 })));
        s.expect_refused(|s| s.play(ID, json!({ "zone": 3, "tributes": [first.as_str()] })));
    }

    #[test]
    fn r1872003_its_cry_discovers_two_radiant_miners_in_turn_and_equips_each() {
        let (mut s, host) = played(false);
        let mut held: Vec<String> = Vec::new();
        for _ in 0..PICKS {
            let pending = s.state().pending.clone().expect("a Discover is open");
            assert_eq!((pending.kind, pending.player_id), (PromptKind::Discover, P1));
            assert_eq!(pending.resume.step, EQUIP_PICK_STEP);
            let offered: Vec<String> = pending
                .options
                .iter()
                .filter_map(|option| match &option.selection {
                    Selection::Mode { option } => Some(option.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(offered.len(), DISCOVER_OPTIONS as usize, "{offered:?}");
            assert!(offered.iter().all(|id| MINERS.contains(&id.as_str()) && !held.contains(id)), "{offered:?}");
            let before = s.stats(host.as_str()).health;
            s.answer(json!(offered[0]));
            // R469: a Radiant pick lends its Radiant face to the plain host.
            assert_eq!(s.stats(host.as_str()).health, before + radiant_health(&offered[0]));
            held.push(offered[0].clone());
        }
        assert!(s.state().pending.is_none());
        // R1852001: each Miner is a piece, the kept Galaxy none, so the fusion is none.
        assert!(!unit_has(s.state(), s.card(host.as_str()), KeywordKind::StarshipPiece));
        s.activate(host.as_str(), json!({ "ability": "mine" }));
        assert_eq!(resources_in_hand_of(s.state(), P1).len(), 1);
    }

    #[test]
    fn r1872003_radiant_20_20_and_all_five_radiant_miners_fused_v1_to_v5_asking_nothing() {
        let (mut s, host) = played(true);
        assert!(s.state().pending.is_none());
        let fusions = s
            .last_events()
            .iter()
            .filter(|event| event.event_type() == GameEventType::Fused)
            .count();
        assert_eq!(fusions, MINERS.len());
        let gained: i32 = MINERS.iter().map(|id| radiant_health(id)).sum();
        s.expect_stats(host.as_str(), json!({ "attack": 20, "health": 20 + gained }));
        let mut held: Vec<String> = self_def_ids(Some(s.state()), &s.card(host.as_str()).def_id)
            .into_iter()
            .filter(|id| MINERS.contains(&id.as_str()))
            .collect();
        held.sort();
        assert_eq!(held, MINERS.map(String::from).to_vec(), "one of each");
        // Fused one at a time from v1 to v5: each fusion's result holds the next Miner.
        let mut order: Vec<String> = Vec::new();
        for event in s.last_events() {
            if let GameEvent::Fused { def_id, .. } = event {
                let added: Vec<String> = self_def_ids(Some(s.state()), def_id)
                    .into_iter()
                    .filter(|id| MINERS.contains(&id.as_str()) && !order.contains(id))
                    .collect();
                assert_eq!(added.len(), 1, "{added:?}");
                order.extend(added);
            }
        }
        assert_eq!(order, MINERS.map(String::from).to_vec());
        assert!(!unit_has(s.state(), s.card(host.as_str()), KeywordKind::StarshipPiece));
    }

    /// `radiant`'s Galaxy standing in p1's lane 1 (no Cry: set up there), destroyed by a Hit Job.
    fn destroyed(radiant: bool) -> Scenario {
        let mut s = crate::scenario(json!({
            "p1": {
                "hand": [HIT_JOB, FILLER],
                "field": [{ "def": ID, "radiant": radiant }],
                "library": crate::filler(4),
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        let me = s.card(ID).clone();
        assert!(!unit_has(s.state(), &me, KeywordKind::StarshipPiece));
        s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));
        s.expect_in_zone(&me, "graveyard");
        s
    }

    #[test]
    fn its_death_fuses_nothing_it_is_no_starship_piece() {
        let s = destroyed(false);
        assert!(dormant_starship_of(s.state(), P1).is_none(), "no Dormant Starship");
        assert!(!s.events().iter().any(|event| matches!(event, GameEvent::StarshipFused { .. })));
    }

    #[test]
    fn radiant_its_death_fuses_nothing() {
        let s = destroyed(true);
        assert!(dormant_starship_of(s.state(), P1).is_none(), "no Dormant Starship");
    }
}
