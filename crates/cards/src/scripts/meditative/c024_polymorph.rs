//! M #24 Polymorph (SPEC §8.8 row 24, §3.1, §7, R11, R23, R81, R691): (3) Spell, Common.
//!
//! Base:    "Transform a Unit into a Sheep Token."
//! Radiant: "Transform a Unit and the Units next to it into Sheep Tokens."
//! Engine: a declared target, any Unit on either side (R81), then `transform` of it into a Sheep Token
//! (Core #41 Sheepish's verb): in place, no Cry (R691), an Immutable target left alone (R23). The Radiant
//! face first lists the Units next to the target, on the target's side and row (§3.1), and transforms
//! each into a base Sheep, as Core #34 Collateral Damage reaches its neighbours before its target. The
//! neighbours change whether or not the target does. A Spell's unlabelled one-time text is its Cry.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-024";

/// §7: the Sheep Token.
const SHEEP_TOKEN: &str = "core-t-sheep";

/// R81: "a Unit", either side.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

fn polymorph(neighbours: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |_ctx| {
            let mut effects: Vec<Effect> = Vec::new();
            if neighbours {
                effects.push(for_each_card(ForEachCardArgs {
                    cards: Arc::new(|at: &mut EffectContext<'_>| {
                        adjacent_to(at, &json_as(json!({ "of": "chosen" })), &BoardScope::default())
                            .into_iter()
                            .map(|card| card.id)
                            .collect()
                    }),
                    each: Arc::new(|instance_id: &str| {
                        transform(json_as(json!({ "instanceId": instance_id, "defId": SHEEP_TOKEN })))
                    }),
                }));
            }
            effects.push(transform(json_as(json!({ "target": { "of": "chosen" }, "defId": SHEEP_TOKEN }))));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: polymorph(false),
        radiant: polymorph(true),
    }
}

// M #24 Polymorph — SPEC §8.8 row 24, BUILD M10 row M 24: "Declares one Unit on either side (R81) and
// transforms it into a base Sheep Token in place (R691); an Immutable target is left alone (R23); the
// neighbours are untouched; the Radiant face also transforms the Units next to the target, on its side and
// row, each into a base Sheep, whether or not the target changes".
//
// Fixtures: #8 Mr. Vanilla (4/4) fills the lanes, #66 The Rock (Radiant, Immutable) is the target that
// stays, and #10 Rapid Replenish is the spare card that keeps the turn open (R82).
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::js;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008";
    const ROCK: &str = "core-066";
    const FILLER: &str = "core-010";
    const SHEEP: &str = "core-t-sheep";

    /// p1 holds Polymorph (Radiant or not) with mana for it; each side's lanes 1 to 3 hold `field`.
    fn board(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        crate::scenario(json!({
            "p1": {
                "field": mine,
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": [VANILLA, VANILLA],
                "mana": 8,
            },
            "p2": { "field": theirs, "hand": [FILLER], "library": [VANILLA, VANILLA] },
        }))
    }

    fn three_vanillas() -> Value {
        json!([VANILLA, VANILLA, VANILLA])
    }

    fn at(unit: &CardInstance) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": unit.id }] })
    }

    fn lane_def(s: &Scenario, player: PlayerId, lane: i32) -> Option<String> {
        s.unit(player, lane).map(|unit| unit.def_id)
    }

    fn assert_base_sheep(s: &mut Scenario, player: PlayerId, lane: i32) {
        let Some(sheep) = s.unit(player, lane) else {
            panic!("{player:?} lane {lane} is empty");
        };
        assert_eq!(sheep.def_id, SHEEP, "{player:?} lane {lane}");
        assert!(!sheep.radiant, "{player:?} lane {lane}");
        s.expect_stats(&sheep, json!({ "attack": 1, "health": 1, "maxHealth": 1 }));
    }

    #[test]
    fn declares_one_unit_on_either_side() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.cost, CardCost::Fixed(3));
        assert_eq!(def.type_, CardType::Spell);
        assert_eq!(def.rarity, Rarity::Common);
        assert!(def.tags.is_empty());
        assert!(def.params.is_none());
        assert_eq!(js(&def.refs), json!([SHEEP]));
        let scripts = super::script();
        for face in [&scripts.base, &scripts.radiant] {
            assert_eq!(face.targets.len(), 1);
            assert_eq!(js(&face.targets[0].filter)["side"], json!("any"));
            assert_eq!(js(&face.targets[0].filter)["of"], json!(["unit"]));
            assert!(face.cry.is_some());
        }
    }

    mod base {
        use super::*;

        #[test]
        fn an_enemy_unit_becomes_a_base_sheep_in_place() {
            let mut s = board(false, json!([]), json!([VANILLA, { "def": VANILLA, "damage": 1 }]));
            let victim = s.unit(P2, 2).map(|unit| unit.id).unwrap_or_default();
            let victim = s.card(victim.as_str()).clone();
            s.play(ID, at(&victim));
            assert_base_sheep(&mut s, P2, 2);
            s.expect_in_zone(&victim, "gone");
            assert!(s.pile(P2, "graveyard").is_empty());
            assert_eq!(lane_def(&s, P2, 1), Some(VANILLA.to_string()));
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn your_own_unit_too() {
            let mut s = board(false, json!([VANILLA]), json!([VANILLA]));
            let mine = s.unit(P1, 1).map(|unit| unit.id).unwrap_or_default();
            let mine = s.card(mine.as_str()).clone();
            s.play(ID, at(&mine));
            assert_base_sheep(&mut s, P1, 1);
            assert_eq!(lane_def(&s, P2, 1), Some(VANILLA.to_string()));
        }

        #[test]
        fn r23_an_immutable_target_is_left_alone() {
            let mut s = board(false, json!([]), json!([{ "def": ROCK, "radiant": true }]));
            let rock = s.card(ROCK).clone();
            s.play(ID, at(&rock));
            assert_eq!(s.unit(P2, 1).map(|unit| unit.id), Some(rock.id));
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn the_neighbours_are_untouched() {
            let mut s = board(false, json!([]), three_vanillas());
            let middle = s.unit(P2, 2).map(|unit| unit.id).unwrap_or_default();
            let middle = s.card(middle.as_str()).clone();
            s.play(ID, at(&middle));
            assert_base_sheep(&mut s, P2, 2);
            assert_eq!(lane_def(&s, P2, 1), Some(VANILLA.to_string()));
            assert_eq!(lane_def(&s, P2, 3), Some(VANILLA.to_string()));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn also_the_units_next_to_the_target_on_its_side_each_a_base_sheep() {
            let mut s = board(true, three_vanillas(), three_vanillas());
            let middle = s.unit(P2, 2).map(|unit| unit.id).unwrap_or_default();
            let middle = s.card(middle.as_str()).clone();
            s.play(ID, at(&middle));
            for lane in [1, 2, 3] {
                assert_base_sheep(&mut s, P2, lane);
            }
            // Its own side is another side: nothing there is next to the target.
            for lane in [1, 2, 3] {
                assert_eq!(lane_def(&s, P1, lane), Some(VANILLA.to_string()));
            }
        }

        #[test]
        fn a_target_at_the_edge_has_one_neighbour() {
            let mut s = board(true, json!([]), three_vanillas());
            let first = s.unit(P2, 1).map(|unit| unit.id).unwrap_or_default();
            let first = s.card(first.as_str()).clone();
            s.play(ID, at(&first));
            assert_base_sheep(&mut s, P2, 1);
            assert_base_sheep(&mut s, P2, 2);
            assert_eq!(lane_def(&s, P2, 3), Some(VANILLA.to_string()));
        }

        #[test]
        fn an_immutable_target_stays_and_its_neighbours_still_change() {
            let theirs = json!([VANILLA, { "def": ROCK, "radiant": true }, VANILLA]);
            let mut s = board(true, json!([]), theirs);
            let rock = s.card(ROCK).clone();
            s.play(ID, at(&rock));
            assert_eq!(s.unit(P2, 2).map(|unit| unit.id), Some(rock.id));
            assert_base_sheep(&mut s, P2, 1);
            assert_base_sheep(&mut s, P2, 3);
        }
    }
}
