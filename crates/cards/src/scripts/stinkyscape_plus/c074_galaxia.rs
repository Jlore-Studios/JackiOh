//! SPEC §8.12 S+ #74 Galaxia (§6.2, §10.5 step 7; R119, R1420, R2179002, R3568000): (2) Unit, Epic,
//! 4/6 → 8/12, a card of the Stinkyscape+ set. Built by S20 (#3568).
//!   Base:    "Aura: The first Unit you play each turn gets +{attack} Attack." (attack 3)
//!   Radiant: the same at +6 attack.
//! The designer's note binds: the first Unit played after Galaxia on the turn she is summoned counts as that
//! turn's first.
//!
//! Her "Aura" is a trigger on her controller's resolved plays (§10.5 step 7, as Meditative #11 Double Header
//! answers them): the first Unit its controller plays while she stands on the field each turn gets a
//! permanent buff of attack (§10.4 layer 4) once its play resolves, its Cry included (R3568000). "First" is
//! the first play she hears: a Unit played before she arrived this turn was never heard, so the first Unit
//! played after her counts, whether she was played or summoned (the designer's note). She does not answer
//! her own play (R119), nor a play that resolved as she arrived, so she never buffs herself. She remembers
//! the turn she last gave the attack, which ends that turn's offer; a new turn, or a new arrival after she
//! left, is a new offer. A card set face-down is no Unit (R783004), a cast on draw is no play (R2179002),
//! and only her controller's plays count. Both faces are one script: the Radiant face's 6 is her declared
//! `attack`.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-074";

/// The memory key of the turn she last gave her attack: once a turn (R3568000).
const GRANTED_TURN: &str = "grantedTurn";

/// The Unit this event is her controller's play of, standing face-up on the field as a Unit, or `None`.
fn played_unit(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<String> {
    let GameEvent::CardResolved {
        player,
        instance_id,
        permanent,
        ..
    } = event
    else {
        return None;
    };
    // R2179002: a card cast on draw is no play. R119: she does not answer her own play.
    let this = ctx.self_.as_ref()?;
    if *player != ctx.controller || !*permanent || cast_on_draw_event(event) || *instance_id == this.id {
        return None;
    }
    let state: &GameState = &*ctx.state;
    let unit = find_instance(state, instance_id)?;
    // R383, R783004: what stands in a unit zone is a Unit, a card set face-down is a trap.
    (unit.zone.z() == ZoneName::Field
        && unit.controller == ctx.controller
        && card_type_of(state, unit) == CardType::Unit)
        .then(|| instance_id.clone())
}

fn first_unit() -> TriggerDef {
    TriggerDef::new("g74-first-unit", &[GameEventType::CardResolved], |ctx, event| {
        let Some(unit) = played_unit(&*ctx, event) else {
            return vec![];
        };
        let turn = ctx.state.turn;
        if recalled(&*ctx, GRANTED_TURN) == Some(json!(turn)) {
            return vec![];
        }
        vec![
            remember(json_as(json!({ "key": GRANTED_TURN, "value": turn }))),
            buff(json_as(json!({
                "target": { "of": "instance", "instanceId": unit },
                "attack": param(&*ctx, "attack"),
            }))),
        ]
    })
}

fn galaxia() -> Script {
    Script {
        triggers: vec![first_unit()],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = galaxia();
    // The same script: the Radiant face's +6 is its declared `attack`, which `param` reads.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #74 Galaxia — SPEC §8.12 row 74 (R119, R2179002, R3568000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const FIRST: &str = "stand-in (1) 2/2";
    const SECOND: &str = "stand-in (1) 3/3";
    const THIRD: &str = "stand-in (1) 4/4";
    const FOURTH: &str = "stand-in (1) 5/5";
    const ENEMY: &str = "stand-in (1) 6/6"; // p2's own Unit
    const SPELL: &str = "stand-in (0) Spell";
    const FILLER: &str = "core-005"; // Stockpile: a card in hand, so no turn ends by itself

    /// The attack a face adds: `attack` 3, Radiant 6.
    fn bonus(radiant: bool) -> i32 {
        if radiant { 6 } else { 3 }
    }

    /// p1 stands Galaxia of `radiant`'s face and holds `hand`; p2 holds a Unit and a card.
    fn board(radiant: bool, hand: Value) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": hand,
                "field": [{ "def": ID, "radiant": radiant }],
                "mana": 10,
                "library": [FILLER, FILLER, FILLER],
            },
            "p2": { "hand": [ENEMY, FILLER], "mana": 10, "library": [FILLER, FILLER, FILLER] },
        }))
    }

    #[test]
    fn r3568000_first_unit_each_turn_gets_the_buff() {
        crate::register_all();
        for radiant in [false, true] {
            let mut s = board(radiant, json!([FIRST, SECOND, FILLER]));
            s.play(FIRST, json!({}));
            s.expect_stats(FIRST, json!({ "attack": 2 + bonus(radiant), "health": 2 }));
            // The second Unit of the turn is no first.
            s.play(SECOND, json!({}));
            s.expect_stats(SECOND, json!({ "attack": 3, "health": 3 }));
        }
    }

    #[test]
    fn r3568000_the_buff_is_permanent() {
        crate::register_all();
        let mut s = board(false, json!([FIRST, SECOND, FILLER]));
        s.play(FIRST, json!({}));
        s.end_turn().end_turn();
        s.expect_stats(FIRST, json!({ "attack": 2 + bonus(false) }));
    }

    #[test]
    fn r3568000_a_spell_is_not_a_unit() {
        crate::register_all();
        let mut s = board(false, json!([SPELL, FIRST, FILLER]));
        s.play(SPELL, json!({}));
        s.play(FIRST, json!({}));
        s.expect_stats(FIRST, json!({ "attack": 2 + bonus(false) }));
    }

    #[test]
    fn r3568000_a_unit_before_a_mid_turn_arrival_does_not_consume_it() {
        crate::register_all();
        for radiant in [false, true] {
            // Galaxia is played from the hand, after a Unit and before another.
            let mut s = scenario(json!({
                "p1": {
                    "hand": [FIRST, { "def": ID, "radiant": radiant }, SECOND, FILLER],
                    "mana": 10,
                    "library": [FILLER, FILLER, FILLER],
                },
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
            }));
            s.play(FIRST, json!({}));
            s.play(ID, json!({}));
            // Neither the Unit before her nor she herself (R119) took the aura, base attack 4, Radiant 8.
            s.expect_stats(FIRST, json!({ "attack": 2 }));
            s.expect_stats(ID, json!({ "attack": if radiant { 8 } else { 4 } }));
            // The first Unit played after her counts as the turn's first.
            s.play(SECOND, json!({}));
            s.expect_stats(SECOND, json!({ "attack": 3 + bonus(radiant) }));
            s.expect_stats(FIRST, json!({ "attack": 2 }));
        }
    }

    #[test]
    fn r3568000_the_buff_returns_next_turn() {
        crate::register_all();
        let mut s = board(false, json!([FIRST, SECOND, THIRD, FOURTH, FILLER]));
        s.play(FIRST, json!({})).play(SECOND, json!({}));
        s.expect_stats(FIRST, json!({ "attack": 2 + bonus(false) }));
        s.expect_stats(SECOND, json!({ "attack": 3 }));
        s.end_turn().end_turn();
        // A new turn of p1's: the first Unit takes it again, the next does not.
        s.play(THIRD, json!({})).play(FOURTH, json!({}));
        s.expect_stats(THIRD, json!({ "attack": 4 + bonus(false) }));
        s.expect_stats(FOURTH, json!({ "attack": 5 }));
        // And the turn's first of the last turn kept its own.
        s.expect_stats(FIRST, json!({ "attack": 2 + bonus(false) }));
    }

    #[test]
    fn r3568000_enemy_units_are_untouched() {
        crate::register_all();
        let mut s = board(false, json!([FILLER]));
        s.end_turn();
        // p2's turn: their first Unit is not her controller's play.
        s.play(ENEMY, json!({}));
        s.expect_stats(ENEMY, json!({ "attack": 6 }));
    }
}
