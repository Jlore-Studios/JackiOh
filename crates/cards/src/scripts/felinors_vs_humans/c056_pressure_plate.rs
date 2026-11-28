//! FvH #56 Pressure Plate (SPEC §8.14 row 56): (1) Trap, Common, of the Felinors vs Humans set.
//!   Base:    "Reveals when your opponent plays a Spell while they control a Unit: Destroy a random enemy
//!            Unit."
//!   Radiant: "… Destroy a random enemy Unit and the Units adjacent to it."
//!
//! A trigger on the opponent's `cardAnnounced` of a Spell, in §10.5's window, as IA #37 Bluff Called's
//! (R70); its `when` holds only while the opponent controls a Unit on the field (a Dormant card is none,
//! R1839000), so it stays set otherwise (R99). Then one random enemy Unit (R60) is `destroy`ed; the
//! Radiant face adds `destroy_adjacent_to`, the designer's Cleave read as Core #16 Hit Job's Radiant face,
//! the marks collected together (R59). Nothing in that list can open a prompt, so the pick is drawn as the
//! list is built. A Trap's destroy is no Spell's, so Immune to Spells does not stop it. The Spell played
//! still resolves. Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-056";

/// The enemy Units that may be picked: the opponent's Units on the field, lane by lane.
fn enemy_units(ctx: &EffectContext<'_>) -> Vec<String> {
    active_units_of(&*ctx.state, opponent_of(ctx.controller))
        .into_iter()
        .map(|unit| unit.id.clone())
        .collect()
}

/// R99: what springs it — the opponent's Spell announced while they control a Unit.
fn springs(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::CardAnnounced { player, card_type, .. } = event else {
        return false;
    };
    *player != ctx.controller && *card_type == CardType::Spell && !enemy_units(ctx).is_empty()
}

fn pressure_plate(adjacent_too: bool) -> Script {
    Script {
        triggers: vec![TriggerDef::new(
            "pressure-plate",
            &[GameEventType::CardAnnounced],
            move |ctx, event| {
                if !springs(ctx, event) {
                    return vec![];
                }
                let units = enemy_units(ctx);
                let Some(picked) = ctx.rng.pick(&units).cloned() else {
                    return vec![];
                };
                let aim = json!({ "target": { "of": "instance", "instanceId": picked } });
                let mut effects = vec![destroy(json_as(aim.clone()))];
                if adjacent_too {
                    effects.push(destroy_adjacent_to(json_as(aim)));
                }
                effects
            },
        )
        .with_when(|ctx, event| springs(ctx, event))],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: pressure_plate(false),
        radiant: pressure_plate(true),
    }
}

// FvH #56 Pressure Plate — SPEC §8.14 row 56 (R60, R70, R99).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    /// Stockpile: the opponent's Spell, "Draw 2. Heal your hero 2."
    const STOCKPILE: &str = "core-005";
    /// A sturdy vanilla Unit.
    const WALL: &str = "stand-in (1) 1/5";

    /// p1's Pressure Plate is set; p2, active, holds Stockpile and a Wall, with `field` on its side.
    fn armed(radiant: bool, field: Value) -> Scenario {
        scenario(json!({
            "seed": "pressure-plate",
            "active": "p2",
            "p1": { "backrow": [{ "def": ID, "radiant": radiant }], "hand": [STOCKPILE], "library": filler(4) },
            "p2": { "hand": [STOCKPILE, WALL], "field": field, "library": filler(4) },
        }))
    }

    /// p2's Units on the field.
    fn enemy_count(s: &Scenario) -> usize {
        (1..=UNIT_ZONES).filter(|lane| s.unit(P2, *lane).is_some()).count()
    }

    /// p2 plays its Stockpile.
    fn cast_stockpile(s: &mut Scenario) {
        let spell = must(
            s.hand(P2).into_iter().find(|card| card.def_id == STOCKPILE),
            "p2's Stockpile",
        );
        s.play(&spell, json!({}));
        s.expect_in_zone(&spell, "graveyard");
    }

    mod base {
        use super::*;

        #[test]
        fn the_opponents_spell_springs_it_and_one_random_enemy_unit_is_destroyed() {
            let mut s = armed(false, json!([WALL, WALL, WALL]));
            let hand_before = s.hand(P2).len();
            cast_stockpile(&mut s);
            assert_eq!(s.count("trapFired"), 1);
            s.expect_in_zone(ID, "graveyard");
            assert_eq!(enemy_count(&s), 2, "one of three destroyed");
            assert_eq!(s.hand(P2).len(), hand_before - 1 + 2, "the Spell still resolves: Draw 2");
        }

        #[test]
        fn with_no_enemy_unit_it_stays_set() {
            let mut s = armed(false, json!([]));
            cast_stockpile(&mut s);
            assert_eq!(s.count("trapFired"), 0);
            s.expect_in_zone(ID, "field");
        }

        #[test]
        fn a_unit_play_does_not_spring_it() {
            let mut s = armed(false, json!([WALL]));
            let wall = must(s.hand(P2).into_iter().find(|card| card.def_id == WALL), "p2's Wall");
            s.play(&wall, json!({ "zone": 2 }));
            assert_eq!(s.count("trapFired"), 0);
            s.expect_in_zone(ID, "field");
            assert_eq!(enemy_count(&s), 2);
        }

        #[test]
        fn its_own_controllers_spell_does_not_spring_it() {
            let mut s = armed(false, json!([WALL]));
            s.end_turn();
            assert_eq!(s.state().active, P1);
            let spell = must(
                s.hand(P1).into_iter().find(|card| card.def_id == STOCKPILE),
                "p1's Stockpile",
            );
            s.play(&spell, json!({}));
            assert_eq!(s.count("trapFired"), 0);
            s.expect_in_zone(ID, "field");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_random_enemy_unit_dies_with_the_units_adjacent_to_it() {
            // Two Units side by side: whichever is picked, the other is adjacent to it.
            let mut s = armed(true, json!([WALL, WALL]));
            cast_stockpile(&mut s);
            assert_eq!(s.count("trapFired"), 1);
            assert_eq!(enemy_count(&s), 0, "the pick and its neighbour die together");
        }
    }
}
