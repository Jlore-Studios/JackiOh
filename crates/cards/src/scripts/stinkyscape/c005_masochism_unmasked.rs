//! S #5 Masochism Unmasked (SPEC §8.10 row 5; R1849004, R1875000): (1) Field Trap, Rare, of the held
//! Stinkyscape set (R1420).
//!   Base:    "Reveals when you take damage on your turn: Redirect all damage you take on your turn to
//!            your opponent, up to {cap} each turn."
//!   Radiant: the same, "and double it."
//! Engine: set face-down, it answers the `damage` of a hit on its controller's hero during their own
//! turn, an extra turn included (R1875000), after the hit, so the hit that reveals it is taken
//! (R1849004). The reveal does nothing of its own and happens once: face-up, the trap answers nothing
//! more and stays. What it does is its static flag `ownTurnRedirect` while it acts on the field: each
//! later hit on its controller's hero on their own turn moves at §4.4 step 0a, after Spell Damage and
//! before Divine Shield, the Armor and the caps; as much as the cap has left this turn is dealt to the
//! opponent's hero instead, from the same source, and the rest is taken. The cap counts what is taken
//! off, per turn, and is the declared `cap` (R386). Radiant: what moves is dealt twice over, on the
//! same cap (at most 40 a turn). Tunes: cap 20 ↑ (step 5).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-005";

/// R1849004: the Radiant face's "and double it": what moves is dealt twice over, on the same cap.
const RADIANT_MULTIPLIER: i32 = 2;

/// The catalog's declared cap, read off the face that is up; the engine reads the card's own `cap`
/// over it as it moves a hit (R386).
fn declared_cap(radiant: bool) -> i32 {
    match crate::card_def(ID)
        .params
        .as_ref()
        .and_then(|params| params.iter().find(|entry| entry.key == "cap"))
    {
        Some(param) if radiant => param.radiant,
        Some(param) => param.base,
        None => panic!("{ID} declares cap (catalog params)"),
    }
}

/// R1875000: "Reveals when you take damage on your turn": a hit on its controller's hero while they
/// are the active player, an extra turn of theirs included, and only while it is still face-down.
fn takes_damage_on_own_turn(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::Damage { target_id, .. } = event else {
        return false;
    };
    ctx.live_self().is_some_and(|this| this.face_up != Some(true))
        && ctx.state.active == ctx.controller
        && *target_id == format!("hero-{}", ctx.controller)
}

fn masochism(radiant: bool) -> Script {
    let reveal = TriggerDef::new(
        "masochism-unmasked",
        &[GameEventType::Damage],
        |_ctx, _event| Vec::new(),
    )
    .with_when(|ctx, event| takes_damage_on_own_turn(ctx, event));
    Script {
        static_flags: Some(StaticFlags {
            own_turn_redirect: Some(OwnTurnRedirect {
                cap: declared_cap(radiant),
                multiplier: radiant.then_some(RADIANT_MULTIPLIER),
            }),
            ..StaticFlags::default()
        }),
        triggers: vec![reveal],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: masochism(false),
        radiant: masochism(true),
    }
}

// S #5 Masochism Unmasked — SPEC §8.10 row 5, BUILD M12 row S 5: "set, it reveals on a hit on its
// controller's hero on their own turn, and that hit is taken; afterwards, on their turns, each hit on
// that hero is dealt to the opponent's hero instead, from the same source, until 20 have moved that
// turn (15 and then 10 move 20 and leave 5, R1849004), taken before the Armor; on the opponent's turn
// nothing moves; the cap starts again the next turn; cap reads through `param()`; radiant what moves is
// dealt twice over, at most 40 a turn".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;

    /// Core #68 Twisted Sorcerer, (2) 5/5: "Cry: Deal 4 damage" to any character, 8 on its Radiant face.
    const SORCERER: &str = "core-068";
    const SORCERER_HIT: i32 = 4;
    const RADIANT_SORCERER_HIT: i32 = 8;
    /// The cap a turn, the catalog's.
    const CAP: i32 = 20;
    /// p2's health, high enough for every doubled hit.
    const P2_HEALTH: i32 = 60;

    /// p1, active with 10 mana, holds `hand` and Masochism (`radiant` its face) in backrow zone 1,
    /// `face_up` once it has revealed; p2 has 60 health and a card to keep its turns open.
    fn board(radiant: bool, face_up: bool, hand: Value) -> Scenario {
        let mut s = crate::scenario(json!({
            "seed": "masochism",
            "p1": {
                "hand": hand,
                "backrow": [{ "def": ID, "radiant": radiant, "faceUp": face_up, "lane": 1 }],
                "library": crate::filler(6),
                "mana": 10,
            },
            "p2": { "hand": ["core-005"], "health": P2_HEALTH, "library": crate::filler(6) },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    fn sorcerers(radiant: bool, count: usize) -> Value {
        json!(vec![json!({ "def": SORCERER, "radiant": radiant }); count])
    }

    /// p1 plays the next Twisted Sorcerer in hand into `zone`, its Cry aimed at p1's own hero.
    fn hit_self(s: &mut Scenario, zone: i32) {
        let sorcerer = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == SORCERER)
            .expect("a Sorcerer in hand");
        s.play(
            sorcerer.id.as_str(),
            json!({ "zone": zone, "targets": [{ "pick": "hero", "player": "p1" }] }),
        );
    }

    fn redirected(s: &Scenario) -> usize {
        s.last_events()
            .iter()
            .filter(|event| event.event_type() == GameEventType::Redirected)
            .count()
    }

    mod base {
        use super::*;

        #[test]
        fn r1875000_set_it_reveals_on_a_hit_on_its_controllers_turn_and_that_hit_is_taken() {
            let mut s = board(false, false, sorcerers(false, 2));
            hit_self(&mut s, 1);
            s.expect_health(P1, HERO_HEALTH - SORCERER_HIT);
            s.expect_health(P2, P2_HEALTH);
            assert!(s.last_events().iter().any(|event| event.event_type() == GameEventType::TrapFired));
            let trap = s.backrow(P1, 1).expect("a Field Trap stays");
            assert_eq!((trap.def_id.as_str(), trap.face_up), (ID, Some(true)));
            // Face-up, the next hit moves.
            hit_self(&mut s, 2);
            s.expect_health(P1, HERO_HEALTH - SORCERER_HIT);
            s.expect_health(P2, P2_HEALTH - SORCERER_HIT);
            assert_eq!(redirected(&s), 1);
        }

        #[test]
        fn r1875000_face_up_it_answers_no_hit_again() {
            let mut s = board(false, true, sorcerers(false, 1));
            hit_self(&mut s, 1);
            assert!(!s.last_events().iter().any(|event| event.event_type() == GameEventType::TrapFired));
        }

        #[test]
        fn r1849004_hits_move_until_the_cap_and_the_rest_is_taken() {
            let mut s = board(false, true, sorcerers(true, 3));
            hit_self(&mut s, 1);
            hit_self(&mut s, 2);
            // 16 of the 20 have moved; of the next 8, 4 move and 4 are taken.
            s.expect_health(P2, P2_HEALTH - 2 * RADIANT_SORCERER_HIT);
            s.expect_health(P1, HERO_HEALTH);
            hit_self(&mut s, 3);
            s.expect_health(P2, P2_HEALTH - CAP);
            s.expect_health(P1, HERO_HEALTH - (3 * RADIANT_SORCERER_HIT - CAP));
        }

        #[test]
        fn r1849004_it_moves_the_hit_before_the_armor() {
            let mut s = board(false, true, sorcerers(false, 1));
            s.state_mut().players[P1].hero.armor = 3;
            hit_self(&mut s, 1);
            assert_eq!(s.state().players[P1].hero.armor, 3);
            s.expect_health(P2, P2_HEALTH - SORCERER_HIT);
        }

        #[test]
        fn r1849004_on_the_opponents_turn_nothing_moves() {
            let mut s = crate::scenario(json!({
                "seed": "masochism-theirs",
                "active": "p2",
                "p1": { "backrow": [{ "def": ID, "faceUp": true, "lane": 1 }], "library": crate::filler(6) },
                "p2": { "hand": [SORCERER, "core-005"], "mana": 10, "library": crate::filler(6) },
            }));
            s.play(
                SORCERER,
                json!({ "zone": 1, "targets": [{ "pick": "hero", "player": "p1" }] }),
            );
            s.expect_health(P1, HERO_HEALTH - SORCERER_HIT);
            s.expect_health(P2, HERO_HEALTH);
            assert_eq!(redirected(&s), 0);
        }

        #[test]
        fn r1849004_the_cap_starts_again_next_turn() {
            let mut s = board(false, true, sorcerers(true, 4));
            for zone in 1..=3 {
                hit_self(&mut s, zone);
            }
            s.expect_health(P1, HERO_HEALTH - (3 * RADIANT_SORCERER_HIT - CAP));
            crate::next_turn_of(&mut s, P1);
            s.state_mut().players[P1].mana.current = 10;
            hit_self(&mut s, 4);
            s.expect_health(P1, HERO_HEALTH - (3 * RADIANT_SORCERER_HIT - CAP));
            s.expect_health(P2, P2_HEALTH - CAP - RADIANT_SORCERER_HIT);
        }

        #[test]
        fn cap_reads_through_param() {
            let mut s = board(false, true, sorcerers(true, 3));
            let trap = s.backrow(P1, 1).expect("the trap").id;
            assert_eq!(crate::degrade_number(&mut s, &trap, "cap"), CAP - 5);
            hit_self(&mut s, 1);
            hit_self(&mut s, 2);
            // 15 move: 8, then 7 of the next 8.
            s.expect_health(P2, P2_HEALTH - (CAP - 5));
            s.expect_health(P1, HERO_HEALTH - (2 * RADIANT_SORCERER_HIT - (CAP - 5)));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1849004_what_moves_is_dealt_twice_over_at_most_40_a_turn() {
            let mut s = board(true, true, sorcerers(true, 3));
            hit_self(&mut s, 1);
            s.expect_health(P2, P2_HEALTH - 2 * RADIANT_SORCERER_HIT);
            hit_self(&mut s, 2);
            hit_self(&mut s, 3);
            s.expect_health(P2, P2_HEALTH - 2 * CAP);
            s.expect_health(P1, HERO_HEALTH - (3 * RADIANT_SORCERER_HIT - CAP));
        }

        #[test]
        fn r1875000_set_it_reveals_the_same_way() {
            let mut s = board(true, false, sorcerers(false, 2));
            hit_self(&mut s, 1);
            s.expect_health(P1, HERO_HEALTH - SORCERER_HIT);
            hit_self(&mut s, 2);
            s.expect_health(P2, P2_HEALTH - 2 * SORCERER_HIT);
        }
    }
}
