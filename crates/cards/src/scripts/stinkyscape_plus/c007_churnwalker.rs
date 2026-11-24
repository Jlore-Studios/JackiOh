//! SPEC §8.12 S+ #7 Churnwalker, a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    3/15, Armor 4, Tribute 1 (since issue #4665); "Cry: Choose one — Chain all enemy Units together, when one takes
//!            damage all other chained Units also take the damage; Chain all friendly Units
//!            together, when one takes damage it is split between all allies (rounded down)."
//!   Radiant: 6/30, Armor 8, Tribute 1; "Cry:" the same choice.
//!
//! Engine: the Cry's two modes are declared at play (R81), in the order the card prints them,
//! and the Cry chains every Unit of the chosen side: `chain_units` shared for the enemy,
//! split for its controller's side (R3554007). Both faces choose (SP5); Armor is catalog data. Tribute 1
//! (§6.3) is the static flag `play_choices.rs` validates, as #66 The Rock's: one of your Units, which
//! the play action's `tributes` names (R81, R90); `script_v0_6_12c` keeps the free play before issue
//! #4665 (R767000).
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-007";

/// The Cry's two modes, in the order the card prints them (R81).
const ENEMY: &str = "enemy";
const FRIENDLY: &str = "friendly";

/// §6.3 "Tribute 1", since issue #4665.
const TRIBUTE_COST: i32 = 1;

/// Both faces; `tribute`: whether the play costs `TRIBUTE_COST`, as since issue #4665.
fn churnwalker(tribute: bool) -> Script {
    Script {
        static_flags: tribute.then(|| StaticFlags {
            tribute: Some(TRIBUTE_COST),
            ..StaticFlags::default()
        }),
        modes: vec![ModeDecl {
            kind: PromptKind::Mode,
            options: vec![ENEMY.to_string(), FRIENDLY.to_string()],
        }],
        cry: Some(hook(|ctx| match ctx.modes.first().map(String::as_str) {
            Some(ENEMY) => vec![chain_units(json_as(json!({ "side": "enemy", "kind": "shared" })))],
            Some(FRIENDLY) => vec![chain_units(json_as(json!({ "side": "self", "kind": "split" })))],
            _ => Vec::new(),
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: churnwalker(true),
        radiant: churnwalker(true),
    }
}

/// R767000: through v0.6.12c it had no Tribute (issue #4665); `KEPT_SCRIPTS` folds those replays
/// under it.
pub fn script_v0_6_12c() -> CardScripts {
    CardScripts {
        base: churnwalker(false),
        radiant: churnwalker(false),
    }
}

// S+ #7 Churnwalker — SPEC §8.12 row 7 (R81, R3554007).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::subsystems::ai_policy::policy_actions;
    use jackioh_engine::testkit::*;

    use crate::{filler, run_as, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A durable Unit: it survives the test's hits.
    const TOUGH: &str = "stand-in (1) 2/30";
    /// The Unit its Tribute 1 takes.
    const FODDER: &str = "stand-in (1) 1/1";
    /// The Rock's refusal word for a play that names no Tribute.
    const TRIBUTE_TEXT: &str = "ribute";

    fn hit(s: &mut Scenario, id: &str, amount: i32) {
        run_as(
            s,
            damage(json_as(json!({
                "to": { "of": "instance", "instanceId": id },
                "amount": amount,
            }))),
            P1,
        );
    }

    fn board(seed: &str, radiant: bool, p1: Value, p2: Value) -> Scenario {
        let mut p1 = p1;
        if p1.get("hand").is_none() {
            p1["hand"] = json!([{ "def": ID, "radiant": radiant }]);
        }
        weapon_board(seed, p1, p2)
    }

    #[test]
    fn the_enemy_choice_echoes_damage_to_every_chained_enemy() {
        for radiant in [false, true] {
            let mut s = board(
                "s007-enemy",
                radiant,
                json!({ "mana": 10, "field": [FODDER], "library": filler(4) }),
                json!({ "field": [TOUGH, TOUGH], "library": filler(4) }),
            );
            let fodder = s.unit_at(P1, 1).id;
            s.play(ID, json!({ "zone": 2, "modes": [ENEMY], "tributes": [fodder] }));
            let (first, second) = (s.unit_at(P2, 1).id, s.unit_at(P2, 2).id);
            hit(&mut s, &first, 5);
            assert_eq!(s.card(&first).damage, 5);
            assert_eq!(s.card(&second).damage, 5, "the chained enemy takes it too");
        }
    }

    #[test]
    fn the_friendly_choice_splits_damage_rounded_down() {
        for radiant in [false, true] {
            let mut s = board(
                "s007-friendly",
                radiant,
                json!({ "mana": 10, "field": [TOUGH, TOUGH, FODDER], "library": filler(4) }),
                json!({ "library": filler(4) }),
            );
            let fodder = s.unit_at(P1, 3).id;
            s.play(ID, json!({ "zone": 4, "modes": [FRIENDLY], "tributes": [fodder] }));
            let tough = [s.unit_at(P1, 1).id, s.unit_at(P1, 2).id];
            let churnwalker = s.unit_at(P1, 4).id;
            hit(&mut s, &tough[0], 7);
            // 7 among the three chained friendlies is 2 each, rounded down: 2 — not 3 — on
            // the other Tough proves Churnwalker shares the chain.
            for id in &tough {
                assert_eq!(s.card(id).damage, 2, "{id} took its share");
            }
            assert_eq!(s.card(&churnwalker).damage, 0, "its Armor absorbs its own share");
        }
    }

    #[test]
    fn both_faces_offer_both_choices_to_the_ai() {
        for radiant in [false, true] {
            let s = weapon_board(
                "s007-ai",
                json!({
                    "mana": 10,
                    "hand": [{ "def": ID, "radiant": radiant }],
                    "field": [FODDER],
                    "library": filler(4),
                }),
                json!({}),
            );
            let offered: Vec<Vec<String>> = legal_actions(s.state(), P1)
                .into_iter()
                .filter_map(|action| match action {
                    ActionBody::Play {
                        modes: Some(modes), ..
                    } => Some(modes),
                    _ => None,
                })
                .collect();
            assert!(
                offered.iter().any(|modes| modes == &[ENEMY]),
                "the enemy choice is offered"
            );
            assert!(
                offered.iter().any(|modes| modes == &[FRIENDLY]),
                "the friendly choice is offered"
            );
            assert!(
                !policy_actions(s.state(), P1, Default::default()).is_empty(),
                "the AI answers"
            );
        }
    }

    /// §6.3: Tribute 1 since issue #4665: a play that names none is refused, and the one it names dies.
    #[test]
    fn s6_3_tribute_1_a_play_must_name_a_unit_of_yours_which_dies() {
        for radiant in [false, true] {
            let mut s = board(
                "s007-tribute",
                radiant,
                json!({ "mana": 10, "field": [FODDER], "library": filler(4) }),
                json!({ "library": filler(4) }),
            );
            let fodder = s.unit_at(P1, 1).id;
            s.expect_refused_with(|s| s.play(ID, json!({ "zone": 2, "modes": [ENEMY] })), TRIBUTE_TEXT);
            s.play(ID, json!({ "zone": 2, "modes": [ENEMY], "tributes": [fodder.clone()] }));
            s.expect_in_zone(fodder.as_str(), "graveyard");
            assert_eq!(s.unit_at(P1, 2).def_id, ID);
        }
    }

    /// R767000: through v0.6.12c it was played with no Tribute.
    #[test]
    fn r767000_the_script_kept_for_v0_6_12c_plays_with_no_tribute() {
        crate::under_kept(ID, script_v0_6_12c(), || {
            let mut s = board(
                "s007-kept",
                false,
                json!({ "mana": 10, "field": [FODDER], "library": filler(4) }),
                json!({ "library": filler(4) }),
            );
            s.play(ID, json!({ "zone": 2, "modes": [ENEMY] }));
            assert_eq!(s.unit_at(P1, 2).def_id, ID);
            assert!(s.unit(P1, 1).is_some(), "no Tribute taken");
        });
    }
}
