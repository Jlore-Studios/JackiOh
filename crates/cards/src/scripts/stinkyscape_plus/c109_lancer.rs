//! SPEC §8.12 S+ #109 Lancer (Unit 2/4 → 4/8, Mindustrial, Rare, cost 2), a card of the Stinkyscape+ set (R1420).
//!   Base:    "Armor 2, Can't attack, Can't Defend, Flexible
//!            Activate: Discover an enemy tile, then deal {damage} Pierce damage to every Unit between
//!            that tile and this." — damage 5
//!   Radiant: "Armor 4, Can't attack, Can't Defend, Flexible
//!            Activate 2: …the same…" — damage 8
//!
//! Engine: Flexible is a catalog keyword (R3543000): played into the backrow Lancer is a Unit there and still
//! activates, drawing its line from there. The Activate is `discover` uses a turn (R384, R1857003: 1 on the
//! base face, 2 on the Radiant face). R3554005: it opens a Discover of `DISCOVER_OPTIONS` enemy tiles (a lane's
//! Units cell or its backrow cell, drawn from the game's Rng as the prompt opens), a `mode` prompt that
//! `legal_actions`, the AI's policy, the random policy and the timeout all answer; the answer draws the line
//! between Lancer and the tile as KY's Papaya draws it (R422) and deals `damage` to every Unit on it — friend
//! or foe — piercing Armor, one damage instance each. Both faces share the script; the number is the declared
//! `damage` (R386), read through `param`.

use jackioh_engine::effects::{LINE_STEP, LineRules, line_damage_answered, line_damage_begin};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-109";

/// "Activate": 1 use a turn on the base face, a label and no param (R384, R482, R1857003).
const USES: i32 = 1;
/// "Activate 2": 2 uses a turn on the Radiant face.
const RADIANT_USES: i32 = 2;

/// The activation's id.
const ABILITY: &str = "lance";

/// R3554005: Lancer's line deals `amount`, Discovered from `DISCOVER_OPTIONS` tiles, no Cleave, no laser.
fn rules(amount: i32) -> LineRules {
    LineRules {
        amount,
        offers: Some(DISCOVER_OPTIONS),
        cleave: false,
        laser: false,
        // R3571000: Lancer prints Pierce on its line.
        pierce: true,
    }
}

fn lancer(uses: i32) -> Script {
    let activation = ActivationDecl {
        id: ABILITY.into(),
        label: "Discover an enemy tile and strike the line to it".into(),
        uses: ActivationUses::Count(uses),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(|ctx| line_damage_begin(rules(param(&*ctx, "damage")))),
    };
    Script {
        activations: vec![activation],
        resume: IndexMap::from([(
            LINE_STEP,
            hook(|ctx| {
                let line = rules(param(&*ctx, "damage"));
                line_damage_answered(line, ctx)
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: lancer(USES),
        radiant: lancer(RADIANT_USES),
    }
}

// S+ #109 Lancer — SPEC §8.12 row 109 (R3554005; R384, R386, R422, R1857003, R3543000).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{next_turn_of, weapon_board};
    use jackioh_engine::subsystems::activate::activate_actions_for;
    use jackioh_engine::subsystems::ai_policy::choose_action;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A target no line brings down, and one whose Armor a plain hit would not get through.
    const TOUGH: &str = "stand-in (1) 0/60";
    const ARMORED: &str = "stand-in (1) 0/60 Armor 3";

    /// What the line deals, base and Radiant (the catalog's `damage`), and the Radiant face's health.
    const DAMAGE: i32 = 5;
    const RADIANT_DAMAGE: i32 = 8;
    const RADIANT_HEALTH: i32 = 8;
    /// The tile straight ahead of lane 3: x is the lane less one, y 2 the enemy Units' row (R3554005).
    const AHEAD: &str = "2,2";

    /// Lancer for p1 as `p1` places it, p2's Units as given.
    fn board(radiant: bool, seed: &str, p1: Value, p2: Value) -> Scenario {
        let mut mine = vec![json!({ "def": ID, "radiant": radiant, "lane": 3 })];
        mine.extend(p1.as_array().cloned().unwrap_or_default());
        weapon_board(seed, json!({ "field": mine }), json!({ "field": p2 }))
    }

    /// Whether the Discover Lancer's activation opened on `s` offers `tile`.
    fn offers_tile(s: &Scenario, tile: &str) -> bool {
        s.state().pending.as_ref().is_some_and(|pending| {
            pending
                .options
                .iter()
                .any(|option| matches!(&option.selection, Selection::Mode { option } if option == tile))
        })
    }

    /// `build`'s board on the first seed whose Discover, opened by Lancer's Activate, offers `tile`
    /// (the offer is the match rng's, so no seed is pinned), with that Discover open.
    fn offering(prefix: &str, tile: &str, build: impl Fn(&str) -> Scenario) -> Scenario {
        let seed = seed_where(prefix, |seed| {
            let mut s = build(seed);
            s.activate(ID, json!({}));
            offers_tile(&s, tile)
        });
        let mut s = build(&seed);
        s.activate(ID, json!({}));
        s
    }

    fn damage(s: &Scenario, id: &str) -> i32 {
        s.card(id).damage
    }

    fn offers(s: &Scenario) -> usize {
        activate_actions_for(s.state(), P1, s.card(ID)).len()
    }

    mod base {
        use super::*;

        #[test]
        fn r3554005_it_discovers_three_tiles() {
            let mut s = board(false, "s109-three", json!([]), json!([TOUGH]));
            s.activate(ID, json!({}));
            let pending = s.state().pending.clone().expect("the tile Discover");
            assert_eq!(pending.options.len(), DISCOVER_OPTIONS as usize);
            let keys: IndexSet<&str> = pending.options.iter().map(|option| option.key.as_str()).collect();
            assert_eq!(keys.len(), DISCOVER_OPTIONS as usize, "three different tiles");
        }

        #[test]
        fn r3554005_straight_ahead_hits_the_unit_across_through_armor() {
            let build = |seed: &str| {
                board(
                    false,
                    seed,
                    json!([]),
                    json!([{ "def": TOUGH, "lane": 2 }, { "def": ARMORED, "lane": 3 }]),
                )
            };
            let mut s = offering("s109-ahead", AHEAD, build);
            s.answer(json!([AHEAD]));
            assert_eq!(damage(&s, s.unit_at(P2, 3).id.as_str()), DAMAGE, "Pierce went through Armor 3");
            assert_eq!(damage(&s, s.unit_at(P2, 2).id.as_str()), 0, "lane 2 is off the line");
            assert!(s.state().pending.is_none());
        }

        #[test]
        fn r3554005_from_the_backrow_the_line_hits_a_friend_in_front() {
            let build = |seed: &str| {
                weapon_board(
                    seed,
                    json!({ "field": [
                        { "def": ID, "row": "backrow", "lane": 3 },
                        { "def": TOUGH, "lane": 3 },
                    ] }),
                    json!({ "field": [{ "def": TOUGH, "lane": 3 }] }),
                )
            };
            let mut s = offering("s109-backrow", AHEAD, build);
            assert!(s.backrow(P1, 3).is_some_and(|card| card.def_id == ID), "it stands in the backrow");
            s.answer(json!([AHEAD]));
            let (friend, foe) = (s.unit_at(P1, 3).id, s.unit_at(P2, 3).id);
            assert_eq!(damage(&s, friend.as_str()), DAMAGE, "a friend on the line is hit");
            assert_eq!(damage(&s, foe.as_str()), DAMAGE);
            assert_eq!(damage(&s, ID), 0, "the source is not on its own line");
        }

        #[test]
        fn r384_it_activates_once_a_turn() {
            let build = |seed: &str| board(false, seed, json!([]), json!([TOUGH, TOUGH, TOUGH]));
            let mut s = offering("s109-once", AHEAD, build);
            s.answer(json!([AHEAD]));
            assert_eq!(offers(&s), 0);
            s.expect_refused(|s| s.activate(ID, json!({})));
            next_turn_of(&mut s, P1);
            assert_eq!(offers(&s), 1, "the use comes back");
        }

        #[test]
        fn r3554005_the_random_policy_answers_the_discover() {
            let mut s = board(false, "s109-policy", json!([]), json!([TOUGH, TOUGH, TOUGH]));
            s.activate(ID, json!({}));
            let state = s.state().clone();
            let mut rng = Rng::new("s109-policy", 0);
            let body = choose_action(&state, P1, &mut rng).expect("the policy answers");
            assert!(matches!(body, ActionBody::Answer { .. }), "{body:?}");
            let next = reduce(&state, &Action::new(body, P1, "s109-policy-answer".to_string()));
            assert!(next.error.is_none(), "{:?}", next.error);
            assert!(next.state.pending.is_none(), "the Discover is answered");
        }

        #[test]
        fn r3554005_a_timeout_answers_the_discover() {
            let mut s = board(false, "s109-timeout", json!([]), json!([TOUGH, TOUGH, TOUGH]));
            s.activate(ID, json!({}));
            assert!(s.state().pending.is_some());
            s.act(ActionBody::Timeout, P1, "clock");
            assert!(s.state().pending.is_none(), "the timeout picked a tile");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3554005_it_deals_eight_through_armor() {
            let build = |seed: &str| {
                board(true, seed, json!([]), json!([{ "def": ARMORED, "lane": 3 }]))
            };
            let mut s = offering("s109-radiant", AHEAD, build);
            s.answer(json!([AHEAD]));
            assert_eq!(damage(&s, s.unit_at(P2, 3).id.as_str()), RADIANT_DAMAGE);
        }

        #[test]
        fn r384_it_activates_twice_a_turn_and_no_third_time() {
            let mut s = board(true, "s109-twice", json!([]), json!([TOUGH, TOUGH, TOUGH, TOUGH, TOUGH]));
            for use_ in 1..=2 {
                assert_eq!(offers(&s), 1, "use {use_} is offered");
                s.activate(ID, json!({}));
                let pending = s.state().pending.clone().expect("the tile Discover");
                s.answer(json!([pending.options[0].key.clone()]));
                assert!(s.state().pending.is_none());
            }
            assert_eq!(offers(&s), 0);
            s.expect_refused(|s| s.activate(ID, json!({})));
        }

        #[test]
        fn its_health_is_doubled() {
            let s = board(true, "s109-health", json!([]), json!([]));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (4, RADIANT_HEALTH));
        }
    }
}
