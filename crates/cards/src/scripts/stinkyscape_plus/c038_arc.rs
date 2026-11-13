//! SPEC §8.12 S+ #38 Arc (Unit 0/5 → 0/10, Mindustrial, Common, cost 2), a card of the Stinkyscape+ set (R1420).
//!   Base:    "Can't Defend
//!            Activate {damage}: Deal {damage2} damage to a Unit. This randomly chains to an adjacent Unit. It
//!            repeats until no valid targets remain. It cannot chain to the same Unit twice in one Activate." —
//!            damage 2, damage2 2
//!   Radiant: the same text — damage 5, damage2 2
//!
//! Engine: the Activate is `damage` uses a turn (R384, R1857003: 2 on the base face, 5 on the Radiant face).
//! R3559001: its target is a declared Unit on either side and in either row, never Arc itself, and it is not
//! offered when there is no such Unit (R81, R430). The verb is chain lightning (R3554006): `damage2` to the
//! chosen Unit, then a jump at random to an unhit Unit beside any Unit hit so far — the next lane or the
//! backrow behind it — until none is left, so the whole connected row is hit once each, whatever the draws;
//! every draw comes from the game's Rng, and the chain may still reach Arc. Both faces share the script;
//! the numbers are the declared ones (R386), read through `param`.

use jackioh_engine::effects::chain_lightning;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-038";

/// "Activate 2": 2 uses a turn on the base face, a label and no param (R384, R482, R1857003).
const USES: i32 = 2;
/// "Activate 5": 5 uses a turn on the Radiant face.
const RADIANT_USES: i32 = 5;

/// R3559001: "Deal damage to a Unit, chaining", offered only when a Unit other than Arc stands.
fn arc(uses: i32) -> Script {
    let activation = ActivationDecl {
        id: "arc".into(),
        label: "Deal damage to a Unit, chaining to adjacent Units".into(),
        uses: ActivationUses::Count(uses),
        cost: None,
        // "a Unit": either side, never Arc itself (R3559001), never a hero.
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "any", "of": ["unit"], "excludeSelf": true }),
        )],
        modes: vec![],
        // R430: nothing to name, nothing to offer.
        can_activate: Some(condition_hook(|c| {
            PLAYER_IDS
                .iter()
                .any(|&player| active_units_of(c.state, player).iter().any(|unit| unit.id != c.self_.id))
        })),
        has: None,
        run: hook(|ctx| {
            // A declaration with no legal pick is passed (R81): no Unit chosen, nothing struck.
            if ctx.targets.is_empty() {
                return Vec::new();
            }
            vec![chain_lightning(json_as(
                json!({ "to": { "of": "chosen" }, "amount": param(&*ctx, "damage2") }),
            ))]
        }),
    };
    Script {
        activations: vec![activation],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: arc(USES),
        radiant: arc(RADIANT_USES),
    }
}

// S+ #38 Arc — SPEC §8.12 row 38 (R3554006, R3559001; R81, R384, R430, R1857003).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{next_turn_of, weapon_board};
    use jackioh_engine::subsystems::activate::activate_actions_for;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// An enemy no Activate brings down.
    const TOUGH: &str = "stand-in (1) 0/40";
    /// A Flying Unit, which stands in the backrow as well as in a unit row.
    const FLYING: &str = "stand-in (1) 0/40 Flying";

    /// What each hit of the chain deals: the catalog's `damage2`.
    const DAMAGE: i32 = 2;
    const RADIANT_HEALTH: i32 = 10;
    const SEEDS: usize = 20;

    fn at(target: &str) -> Value {
        json!({ "ability": "arc", "targets": pick::id(target) })
    }

    /// Arc in p1's lane `lane`, p1's other cards and p2's as given.
    fn board(radiant: bool, seed: &str, lane: i32, p1: Value, p2: Value) -> Scenario {
        let mut field = vec![json!({ "def": ID, "radiant": radiant, "lane": lane })];
        field.extend(p1.as_array().cloned().unwrap_or_default());
        weapon_board(seed, json!({ "field": field }), json!({ "field": p2 }))
    }

    fn offers(s: &Scenario) -> usize {
        activate_actions_for(s.state(), P1, s.card(ID)).len()
    }

    fn damage(s: &Scenario, id: &str) -> i32 {
        s.card(id).damage
    }

    mod base {
        use super::*;

        #[test]
        fn r3554006_the_chain_hits_the_whole_row_once_each() {
            for seed in 0..SEEDS {
                let mut s = board(
                    false,
                    &format!("s038-row-{seed}"),
                    1,
                    json!([]),
                    json!([
                        { "def": TOUGH, "lane": 1 },
                        { "def": TOUGH, "lane": 2 },
                        { "def": TOUGH, "lane": 3 },
                        { "def": TOUGH, "lane": 5 },
                    ]),
                );
                let lanes: Vec<String> = [1, 2, 3, 5].map(|lane| s.unit_at(P2, lane).id).to_vec();
                s.activate(ID, at(&lanes[1]));
                for id in &lanes[..3] {
                    assert_eq!(damage(&s, id), DAMAGE, "lanes 1 to 3 are hit once each");
                }
                assert_eq!(damage(&s, &lanes[3]), 0, "lane 5 is cut off by the empty lane 4");
            }
        }

        #[test]
        fn r3554006_the_chain_jumps_to_the_backrow_behind() {
            let mut s = weapon_board(
                "s038-backrow",
                json!({ "field": [{ "def": ID, "lane": 1 }] }),
                json!({ "field": [
                    { "def": TOUGH, "lane": 1 },
                    { "def": FLYING, "row": "backrow", "lane": 1 },
                    { "def": FLYING, "row": "backrow", "lane": 4 },
                ] }),
            );
            let (front, back, far) = (s.unit_at(P2, 1).id, s.backrow(P2, 1).expect("the backrow Unit").id, s.backrow(P2, 4).expect("the far one").id);
            s.activate(ID, at(&front));
            assert_eq!(damage(&s, &front), DAMAGE);
            assert_eq!(damage(&s, &back), DAMAGE, "the backrow behind is the next jump");
            assert_eq!(damage(&s, &far), 0, "a backrow Unit in a lane with no neighbour in reach is not");
        }

        #[test]
        fn r3559001_it_never_targets_itself_nor_is_offered_alone() {
            let s = board(false, "s038-alone", 1, json!([]), json!([]));
            assert_eq!(offers(&s), 0, "no Unit but Arc itself: nothing to name");
            let s = board(false, "s038-one", 1, json!([]), json!([TOUGH]));
            assert_eq!(offers(&s), 1, "one enemy Unit, one offer");
            let own = s.card(ID).id.clone();
            let named: Vec<Selection> = activate_actions_for(s.state(), P1, s.card(ID))
                .into_iter()
                .flat_map(|action| action.targets.unwrap_or_default())
                .collect();
            assert!(!named.contains(&Selection::Instance { instance_id: own }));
        }

        #[test]
        fn r3559001_arc_cannot_be_its_own_target() {
            let mut s = board(false, "s038-self", 1, json!([]), json!([TOUGH]));
            let own = s.card(ID).id.clone();
            s.expect_refused(|s| s.activate(ID, at(own.as_str())));
            assert_eq!(damage(&s, own.as_str()), 0);
        }

        #[test]
        fn r3559001_it_may_name_a_unit_of_its_own_side_and_no_hero() {
            let mut s = board(false, "s038-ally", 1, json!([{ "def": TOUGH, "lane": 3 }]), json!([TOUGH]));
            let ally = s.unit_at(P1, 3).id;
            s.expect_refused(|s| {
                s.activate(ID, json!({ "ability": "arc", "targets": [{ "pick": "hero", "player": "p2" }] }))
            });
            s.activate(ID, at(&ally));
            assert_eq!(damage(&s, &ally), DAMAGE, "an ally takes it");
            assert_eq!(damage(&s, ID), 0, "lane 3 is not beside lane 1");
        }

        #[test]
        fn r3554006_the_chain_may_reach_arc_itself() {
            let mut s = board(false, "s038-reach", 1, json!([{ "def": TOUGH, "lane": 2 }]), json!([]));
            let ally = s.unit_at(P1, 2).id;
            s.activate(ID, at(&ally));
            assert_eq!(damage(&s, &ally), DAMAGE);
            assert_eq!(damage(&s, ID), DAMAGE, "Arc stands beside it, so the chain jumps onto Arc");
        }

        #[test]
        fn r384_it_activates_twice_a_turn_and_no_third_time() {
            let mut s = board(false, "s038-twice", 1, json!([]), json!([TOUGH]));
            let enemy = s.unit_at(P2, 1).id;
            s.activate(ID, at(&enemy));
            assert_eq!(offers(&s), 1, "one use left");
            s.activate(ID, at(&enemy));
            assert_eq!(offers(&s), 0);
            s.expect_refused(|s| s.activate(ID, at(&enemy)));
            assert_eq!(damage(&s, &enemy), 2 * DAMAGE);
            next_turn_of(&mut s, P1);
            assert_eq!(offers(&s), 1, "the uses come back");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r384_it_activates_five_times_a_turn_and_no_sixth() {
            let mut s = board(true, "s038-five", 1, json!([]), json!([TOUGH]));
            let enemy = s.unit_at(P2, 1).id;
            for done in 1..=RADIANT_USES {
                assert_eq!(offers(&s), 1, "use {done} is offered");
                s.activate(ID, at(&enemy));
            }
            assert_eq!(offers(&s), 0);
            s.expect_refused(|s| s.activate(ID, at(&enemy)));
            assert_eq!(damage(&s, &enemy), RADIANT_USES * DAMAGE);
        }

        #[test]
        fn r3554006_it_chains_as_the_base_face_does() {
            let mut s = board(
                true,
                "s038-radiant-row",
                1,
                json!([]),
                json!([TOUGH, TOUGH, TOUGH]),
            );
            let lanes: Vec<String> = (1..=3).map(|lane| s.unit_at(P2, lane).id).collect();
            s.activate(ID, at(&lanes[0]));
            for id in &lanes {
                assert_eq!(damage(&s, id), DAMAGE);
            }
        }

        #[test]
        fn its_health_is_doubled() {
            let s = board(true, "s038-health", 1, json!([]), json!([]));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (0, RADIANT_HEALTH));
        }
    }
}
