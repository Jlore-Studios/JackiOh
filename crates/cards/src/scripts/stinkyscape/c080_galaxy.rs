//! Stinkyscape #80 Galaxy (SPEC §8.10 row 80). Unit 2/2 → 4/4 (5/5 → 10/10 until issue #3996), cost 4,
//! Legendary.
//!   Base:    "Tribute 3, Flying
//!            Cry: Discover 2 Radiant Miners and Equip them into this."
//!   Radiant: "Tribute 3, Flying
//!            Cry: Equip a Radiant copy of every Miner into this."
//!
//! Tribute 3 since issue #4440 (Tribute 2 before it, which `script_v0_6_9n` keeps for the replays of
//! v0.6.9n and before, R767000) is the play validator's (§6.3, R101), read off `staticFlags.tribute`, the
//! keyword line's printed Tribute (R366). R1872003: the Cry is `equip_picks` (R1841002) from the Miners (`query::miners`,
//! R3869002), S #62 Miner v1 to S #66 Miner v5 today (named by id through v0.6.2b),
//! each pick a fresh copy on its Radiant face (R469) fused straight into this (R1086, R77), its Activate
//! with it (R1751000): two Discovers of three in turn, the second without the first; the Radiant face
//! fuses all five, v1 first, asking nothing. Flying is a catalog keyword the engine plays. Since #2521 it
//! is no Starship Piece, and the Miners it takes, each a piece, leave it none (R1852001), so its death
//! fuses nothing (R1852000 reads the keyword).

use jackioh_engine::effects::equip_picks;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-080";

/// "Tribute 3" (R101), since issue #4440.
const TRIBUTE: i32 = 3;
/// "Tribute 2", through v0.6.9n.
const TRIBUTE_V0_6_9G: i32 = 2;

/// "Discover 2".
const PICKS: i32 = 2;

fn tribute(units: i32) -> Option<StaticFlags> {
    Some(StaticFlags {
        tribute: Some(units),
        ..StaticFlags::default()
    })
}

pub fn script() -> CardScripts {
    galaxy(TRIBUTE)
}

/// R767000: through v0.6.9n it was Tribute 2 (issue #4440); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_9n() -> CardScripts {
    galaxy(TRIBUTE_V0_6_9G)
}

/// Both faces, played for `units` Units.
fn galaxy(units: i32) -> CardScripts {
    let base = Script {
        static_flags: tribute(units),
        cry: Some(hook(|_ctx| {
            vec![equip_picks(json_as(json!({
                "query": crate::query::miners(),
                "times": PICKS,
                "radiant": true,
            })))]
        })),
        ..Script::default()
    };
    // R1872003: "a Radiant copy of every Miner", one by one, v1 to v5.
    let radiant = Script {
        static_flags: tribute(units),
        cry: Some(hook(|_ctx| {
            crate::query::pool_ids(&crate::query::miners())
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

// Stinkyscape #80 Galaxy — SPEC §8.10 row 80, BUILD M12 row S 80 (#2521; R101, R1841002, R1852001).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon_board;
    use jackioh_engine::effects::EQUIP_PICK_STEP;
    use jackioh_engine::play_choices::tribute_cost_of;
    use jackioh_engine::testkit::*;

    /// R3869002: the Miner pool today, S #62 Miner v1 to S #66 Miner v5, pinned so a change to it is
    /// deliberate: the five this card named by id through v0.6.2b, in its order.
    const MINERS: [&str; 5] = [
        "stinkyscape-062",
        "stinkyscape-063",
        "stinkyscape-064",
        "stinkyscape-065",
        "stinkyscape-066",
    ];

    const P1: PlayerId = PlayerId::P1;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-005"; // Stockpile: a Spell
    const HIT_JOB: &str = "core-016"; // Hit Job: Destroy a Unit.

    /// p1 plays the Galaxy (Radiant on `radiant`) into lane 4, its three Vanillas the Tribute.
    fn played(radiant: bool) -> (Scenario, String) {
        let mut s = weapon_board(
            "s080",
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "field": [VANILLA, VANILLA, VANILLA], "mana": 10 }),
            json!({}),
        );
        let tributes = [1, 2, 3].map(|lane| s.unit_at(P1, lane).id);
        s.play(ID, json!({ "zone": 4, "tributes": tributes }));
        let host = s.unit_at(P1, 4).id;
        (s, host)
    }

    fn radiant_health(id: &str) -> i32 {
        crate::card_def(id).radiant.health.unwrap_or_default()
    }

    /// #3142, issue #3996: 2/2 on its base face and 4/4 on its Radiant one: the base face before any Miner is
    /// fused into it (its Discover is open), the Radiant one as printed (it fuses all five at once, below).
    #[test]
    fn it_is_2_2_and_radiant_4_4() {
        let (s, host) = played(false);
        assert!(s.state().pending.is_some(), "the Discover is open: nothing is fused yet");
        let first = s.stats(host.as_str());
        assert_eq!((first.attack, first.health), (2, 2));
        let printed = crate::card_def(ID);
        assert_eq!((printed.radiant.attack, printed.radiant.health), (Some(4), Some(4)));
    }

    /// Issue #4440: Tribute 3, so two Units no longer pay it.
    #[test]
    fn r101_it_needs_tribute_3() {
        let mut s = weapon_board(
            "s080-tribute",
            json!({ "hand": [ID], "field": [VANILLA, VANILLA, VANILLA] }),
            json!({}),
        );
        assert_eq!(tribute_cost_of(s.state(), s.card(ID)), TRIBUTE);
        let [first, second, third] = [1, 2, 3].map(|lane| s.unit_at(P1, lane).id);
        s.expect_refused(|s| s.play(ID, json!({ "zone": 4 })));
        s.expect_refused(|s| s.play(ID, json!({ "zone": 4, "tributes": [first.as_str()] })));
        s.expect_refused(|s| s.play(ID, json!({ "zone": 4, "tributes": [first.as_str(), second.as_str()] })));
        s.play(ID, json!({ "zone": 4, "tributes": [first.as_str(), second.as_str(), third.as_str()] }));
        s.expect_in_zone(ID, "field");
    }

    /// R767000: through v0.6.9n two Units paid it (issue #4440).
    #[test]
    fn r767000_the_script_kept_for_v0_6_9n_needs_tribute_2() {
        crate::under_kept(ID, script_v0_6_9n(), || {
            let mut s = weapon_board(
                "s080-kept",
                json!({ "hand": [ID], "field": [VANILLA, VANILLA, VANILLA] }),
                json!({}),
            );
            assert_eq!(tribute_cost_of(s.state(), s.card(ID)), TRIBUTE_V0_6_9G);
            let [first, second] = [1, 2].map(|lane| s.unit_at(P1, lane).id);
            s.play(ID, json!({ "zone": 4, "tributes": [first.as_str(), second.as_str()] }));
            s.expect_in_zone(ID, "field");
            assert!(s.unit(P1, 3).is_some(), "the third Unit stays");
        });
    }

    #[test]
    fn r1872003_its_cry_discovers_two_radiant_miners_in_turn_and_equips_each() {
        let (mut s, host) = played(false);
        let mut held: Vec<String> = Vec::new();
        for _ in 0..PICKS {
            let pending = s.state().pending.clone().expect("a Discover is open");
            assert_eq!((pending.kind, pending.player_id), (PromptKind::Discover, P1));
            assert_eq!(pending.resume.step, EQUIP_PICK_STEP);
            let offered = s.prompt_modes();
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
    fn r1872003_radiant_4_4_and_all_five_radiant_miners_fused_v1_to_v5_asking_nothing() {
        // The Radiant face rolls nothing: it fuses the whole Miner pool in the pool's order (R3869002),
        // read as it is.
        let (mut s, host) = with_pool_shift(0, || played(true));
        assert!(s.state().pending.is_none());
        let fusions = s
            .last_events()
            .iter()
            .filter(|event| event.event_type() == GameEventType::Fused)
            .count();
        assert_eq!(fusions, MINERS.len());
        let gained: i32 = MINERS.iter().map(|id| radiant_health(id)).sum();
        s.expect_stats(host.as_str(), json!({ "attack": 4, "health": 4 + gained }));
        let mut held: Vec<String> = self_def_ids(Some(s.state()), &s.card(host.as_str()).def_id)
            .into_iter()
            .filter(|id| MINERS.contains(&id.as_str()))
            .collect();
        held.sort();
        assert_eq!(held, MINERS.map(String::from).to_vec(), "one of each");
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
