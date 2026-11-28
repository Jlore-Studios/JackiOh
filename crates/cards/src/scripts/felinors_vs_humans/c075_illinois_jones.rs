//! SPEC §8.14 FvH #75 Illinois Jones ((2) Unit, Human, Legendary, 5/4 → 10/8), a card of the held
//! Felinors vs Humans set. Built by S10 (#5260).
//!   Base:    "Cry: Destroy an enemy Field Spell. Draw cards equal to its cost."
//!   Radiant: "Cry: Steal an enemy Field Spell. Draw cards equal to its cost."
//!
//! Cry (R1): one declared enemy Field Spell in the backrow (R81), so a face-down card, which is no Field
//! Spell to its enemy, is never offered. Its cost is read as the Cry resolves, before it moves (R65, an X
//! card's X as 0; R66); then `destroy` it (Radiant: `steal` it into the stealer's backrow, destroyed with
//! no room, R15, R4205000) and `draw` that many, the hand cap burning the rest (R4). A pick gone from the
//! field by the time the Cry resolves takes nothing and draws nothing. No legal target: it enters and the
//! Cry does nothing (R90). The voice line is the card's text data. Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-075";

/// R81: one enemy Field Spell, picked with the play.
fn enemy_field_spell() -> Vec<TargetDecl> {
    vec![TargetDecl::target(
        1,
        1,
        json!({ "side": "enemy", "of": ["backrow"], "type": "Field Spell" }),
    )]
}

/// The chosen Field Spell, still on the field, and its cost now (R65, R66).
fn the_pick(ctx: &EffectContext<'_>) -> Option<i32> {
    let card = instance_of(ctx, &TargetSpec::Chosen { index: None })?;
    if card.zone.z() != ZoneName::Field {
        return None;
    }
    Some(effective_cost(ctx.state, &card, Default::default()).max(0))
}

/// One face: the pick destroyed (Radiant: stolen), then the draws its cost names.
fn jones(steals: bool) -> Script {
    Script {
        targets: enemy_field_spell(),
        cry: Some(hook(move |ctx| {
            let Some(cost) = the_pick(ctx) else {
                return vec![];
            };
            let chosen = json!({ "target": { "of": "chosen" } });
            let mut effects = vec![if steals { steal(json_as(chosen)) } else { destroy(json_as(chosen)) }];
            if cost > 0 {
                effects.push(draw(json_as(json!({ "count": cost }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: jones(false),
        radiant: jones(true),
    }
}

// FvH #75 Illinois Jones — SPEC §8.14 row 75 (R1, R4, R15, R65, R81, R90, R4205000).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::{filler, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// The enemy's Field Spell, and the cards its cost draws.
    const FIELD: &str = "stand-in (3) Field Spell";
    const ITS_COST: usize = 3;
    /// A face-down card in the enemy's backrow.
    const TRAP: &str = "stand-in (2) Trap";

    /// p1 with Illinois in hand; p2's backrow `backrow`.
    fn jones_board(radiant: bool, backrow: Value) -> Scenario {
        weapon_board(
            "fvh075",
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "library": filler(8), "mana": 10 }),
            json!({ "backrow": backrow }),
        )
    }

    fn drawn_by(s: &Scenario, player: PlayerId) -> usize {
        s.last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Drawn { player: drawer, .. } if *drawer == player))
            .count()
    }

    #[test]
    fn r81_only_an_enemy_field_spell_is_offered_never_a_face_down_card() {
        let s = jones_board(false, json!([FIELD, TRAP]));
        let field = must(s.backrow(P2, 1), "p2's Field Spell");
        for offered in nonempty(pick::instance_ids(&s.play_targets(ID)), "the offered targets") {
            assert_eq!(offered, field.id);
        }
    }

    mod base {
        use super::*;

        #[test]
        fn destroys_the_enemy_field_spell_and_draws_cards_equal_to_its_cost() {
            let mut s = jones_board(false, json!([FIELD]));
            let field = must(s.backrow(P2, 1), "p2's Field Spell");
            s.play(ID, pick::targets(pick::instance(&field)));
            s.expect_in_zone(&field, "graveyard");
            assert_eq!(drawn_by(&s, P1), ITS_COST);
        }

        #[test]
        fn r90_with_no_enemy_field_spell_it_enters_and_draws_nothing() {
            let mut s = jones_board(false, json!([]));
            s.play(ID, json!({}));
            s.expect_in_zone(ID, "field");
            assert_eq!(drawn_by(&s, P1), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r15_steals_the_enemy_field_spell_and_draws_cards_equal_to_its_cost() {
            let mut s = jones_board(true, json!([FIELD]));
            let field = must(s.backrow(P2, 1), "p2's Field Spell");
            s.play(ID, pick::targets(pick::instance(&field)));
            assert_eq!(s.card(&field).controller, P1);
            s.expect_in_zone(&field, "field");
            assert!(s.backrow(P2, 1).is_none());
            assert_eq!(drawn_by(&s, P1), ITS_COST);
        }

        #[test]
        fn r90_with_no_enemy_field_spell_it_enters_and_draws_nothing() {
            let mut s = jones_board(true, json!([]));
            s.play(ID, json!({}));
            s.expect_in_zone(ID, "field");
            assert_eq!(drawn_by(&s, P1), 0);
        }
    }
}
