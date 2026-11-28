//! FvH #21 Scandalous Top Hat (SPEC §8.14 row 21): (0) Spell, Rare, of the Felinors vs Humans set.
//!   Base:    "Give a Unit +{buff}/+{buff} and "Death: Add a Scandalous Top Hat to your hand."" (buff 1)
//!   Radiant: "Give a Unit +{buff}/+{buff} and "Death: Add a Radiant Scandalous Top Hat to your hand.""
//!            (buff 3)
//!
//! One declared Unit of either side (R81): a `buff` of the declared `buff` to Attack and Health, then a
//! `grant_ability` of the face's `hatDeath` grant (R1107, the quoted Death, R1445), the hook each face
//! registers beside it in `grants`: when the Unit dies, after its own Death, the controller it dies under
//! adds a Scandalous Top Hat to their hand, a Radiant one from a Radiant Hat. R5252000: each Hat is a grant
//! of its own, numbered one past the Hats the Unit already holds, since a card keeps one copy of a grant
//! it is given twice; so a Unit wearing two Hats adds two as it dies. The buff is read through `param`
//! (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-021";

/// The granted Death, as both faces quote it (`grants.hatDeath`) and register it (`Script.grants`).
pub const HAT_DEATH: &str = "felinorsvshumans-021#hatDeath";

/// The key each face registers its Death under, the second half of `HAT_DEATH`.
const HAT_KEY: &str = "hatDeath";

/// R5252000: how many Hats the Unit wears now.
fn hats_on(unit: &CardInstance) -> i32 {
    let hats = unit
        .grants
        .iter()
        .flatten()
        .filter(|grant| grant.grant == HAT_DEATH)
        .count();
    i32::try_from(hats).unwrap_or(i32::MAX)
}

fn top_hat(radiant: bool) -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))],
        cry: Some(hook(move |ctx| {
            let amount = param(&*ctx, "buff");
            let Some(unit) = instance_of(&*ctx, &TargetSpec::Chosen { index: None }) else {
                return vec![];
            };
            vec![
                buff(json_as(json!({
                    "target": { "of": "instance", "instanceId": unit.id },
                    "attack": amount,
                    "health": amount,
                }))),
                grant_ability(json_as(json!({
                    "instanceId": unit.id,
                    "grant": HAT_DEATH,
                    "radiant": radiant,
                    // R5252000: this Hat's own number, so it is not the grant of a Hat before it.
                    "params": { "hat": hats_on(&unit) + 1 },
                }))),
            ]
        })),
        grants: IndexMap::from([(
            HAT_KEY,
            hook(move |_ctx| vec![add_to_hand(json_as(json!({ "defId": ID, "radiant": radiant })))]),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: top_hat(false),
        radiant: top_hat(true),
    }
}

// FvH #21 Scandalous Top Hat — SPEC §8.14 row 21 (R1107, R5252000).
#[cfg(test)]
mod tests {
    use super::{HAT_DEATH, ID};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    /// A sturdy vanilla Unit for each side's lane 1.
    const WALL: &str = "stand-in (1) 1/5";
    /// Stockpile: a Spell each hand keeps, so no turn ends by itself.
    const FILLER: &str = "core-005";
    /// Hit Job: the removal.
    const KILL: &str = "core-016";

    /// p1 holds `hats` Top Hats on `radiant`'s face, Hit Job and a filler; each side has a Wall in lane 1.
    fn board(radiant: bool, hats: usize) -> Scenario {
        let mut hand: Vec<Value> = (0..hats).map(|_| json!({ "def": ID, "radiant": radiant })).collect();
        hand.extend([json!(KILL), json!(FILLER)]);
        scenario(json!({
            "seed": "top-hat",
            "p1": { "hand": hand, "field": [WALL], "library": filler(4) },
            "p2": { "hand": [FILLER], "field": [WALL], "library": filler(4) },
        }))
    }

    /// The Top Hats in `seat`'s hand.
    fn hats_in_hand(s: &Scenario, seat: PlayerId) -> Vec<CardInstance> {
        s.hand(seat).into_iter().filter(|card| card.def_id == ID).collect()
    }

    /// The Hat grants the card wears.
    fn hats_worn(s: &Scenario, id: &str) -> usize {
        s.card(id)
            .grants
            .iter()
            .flatten()
            .filter(|grant| grant.grant == HAT_DEATH)
            .count()
    }

    /// p1 casts the first Top Hat in hand at `id`.
    fn hat(s: &mut Scenario, id: &str) {
        let first = hats_in_hand(s, P1).remove(0);
        s.play(&first, pick::targets(pick::id(id)));
    }

    /// p1's Hit Job destroys `id`.
    fn kill(s: &mut Scenario, id: &str) {
        let job = must(s.hand(P1).into_iter().find(|card| card.def_id == KILL), "Hit Job in hand");
        s.play(&job, pick::targets(pick::id(id)));
        s.expect_in_zone(id, "graveyard");
    }

    mod base {
        use super::*;

        #[test]
        fn gives_1_1_and_its_death_adds_a_top_hat_to_its_controllers_hand() {
            let mut s = board(false, 1);
            let wall = s.unit_at(P1, 1).id;
            hat(&mut s, &wall);
            s.expect_stats(&wall, json!({ "attack": 2, "health": 6 }));
            assert_eq!(hats_worn(&s, &wall), 1);
            assert!(hats_in_hand(&s, P1).is_empty(), "the Hat was cast");
            kill(&mut s, &wall);
            let added = hats_in_hand(&s, P1);
            assert_eq!(added.len(), 1, "its Death adds a Hat");
            assert!(!added[0].radiant, "a plain one");
        }

        #[test]
        fn r1107_an_enemy_units_hat_goes_to_the_enemys_hand() {
            let mut s = board(false, 1);
            let theirs = s.unit_at(P2, 1).id;
            hat(&mut s, &theirs);
            s.expect_stats(&theirs, json!({ "attack": 2, "health": 6 }));
            kill(&mut s, &theirs);
            assert_eq!(hats_in_hand(&s, P2).len(), 1, "the controller it dies under gets the Hat");
            assert!(hats_in_hand(&s, P1).is_empty());
        }

        #[test]
        fn r5252000_two_hats_on_one_unit_are_two_grants_and_its_death_adds_two_hats() {
            let mut s = board(false, 2);
            let wall = s.unit_at(P1, 1).id;
            hat(&mut s, &wall);
            hat(&mut s, &wall);
            s.expect_stats(&wall, json!({ "attack": 3, "health": 7 }));
            assert_eq!(hats_worn(&s, &wall), 2, "each Hat is a grant of its own");
            kill(&mut s, &wall);
            assert_eq!(hats_in_hand(&s, P1).len(), 2, "one Hat back for each Hat worn");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn gives_3_3_and_its_death_adds_a_radiant_top_hat() {
            let mut s = board(true, 1);
            let wall = s.unit_at(P1, 1).id;
            hat(&mut s, &wall);
            s.expect_stats(&wall, json!({ "attack": 4, "health": 8 }));
            assert_eq!(hats_worn(&s, &wall), 1);
            kill(&mut s, &wall);
            let added = hats_in_hand(&s, P1);
            assert_eq!(added.len(), 1, "its Death adds a Hat");
            assert!(added[0].radiant, "a Radiant one");
        }
    }
}
