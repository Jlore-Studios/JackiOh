//! TG #21 Flame Tongue Totem (SPEC §8.11 row 21; R2197001): (1) Unit, Rare, 0/4 → 0/8, a card of the held
//! Training Grounds set (R1420, R2194000).
//!   Base and Radiant: "Aura: Adjacent Units have +{attack} Attack." — attack 3, Radiant 6
//! Engine: an aura (§10.4 layer 5) over §3.1's adjacency of the Totem's own zone (R2197001): lanes N−1
//!   and N+1 of its side and row, never itself; in the backrow (R1081000), the Units standing beside it.
//!   Only a pile's top projects or gets an aura (`layers::aura_sources`, R13). `applies` reads zones only,
//!   so the aura follows the Totem as it stands. Tunes: attack 3 ↑.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-021";

fn totem_aura() -> AuraHook {
    aura_hook(|a| {
        let Some(slot) = slot_of(a.state, a.self_) else {
            return Vec::new();
        };
        let beside = adjacent(slot);
        let totem = a.self_;
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                unit.id != totem.id
                    && slot_of(a.state, unit).is_some_and(|at| beside.contains(&at))
                    && in_units_row_or_standing(a.state, unit)
                    && is_adjacent_card(a.state, totem, unit)
            }),
            mod_: StatMod {
                attack: Some(param(&a, "attack")),
                ..StatMod::default()
            },
        }]
    })
}

pub fn script() -> CardScripts {
    // The Radiant face is the same text at +6, a catalog value.
    let base = Script {
        aura: Some(totem_aura()),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// TG #21 Flame Tongue Totem — SPEC §8.11 row 21, BUILD M13 row TG 21.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const VANILLA: &str = "core-008"; // (1) Mr. Vanilla, 4/4.
    const FIENDER: &str = "core-092"; // (2) Felinor Fiender, 5/7 Stack.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const FARM: &str = "core-058"; // (2) Field Spell Rush Token Farm.
    const FILLER: &str = "core-005"; // (1) Spell Stockpile: a card in hand, so no turn ends by itself.
    const PRINTED: i32 = 4;
    const AURA: i32 = 3;

    fn board(radiant: bool) -> Scenario {
        crate::scenario(json!({
            "seed": "flame-tongue-totem",
            "p1": {
                "field": [
                    { "def": ID, "radiant": radiant, "lane": 3 },
                    { "def": VANILLA, "lane": 2 },
                    { "def": VANILLA, "lane": 4 },
                    { "def": VANILLA, "lane": 5 },
                ],
                "backrow": [{ "def": FARM, "lane": 3 }],
                "hand": [HIT_JOB, FILLER],
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": { "field": [{ "def": VANILLA, "lane": 3 }], "hand": [FILLER], "library": crate::filler(4) },
        }))
    }

    fn attack_at(s: &Scenario, player: PlayerId, lane: i32) -> i32 {
        s.stats(s.unit_at(player, lane).id.as_str()).attack
    }

    fn hit(s: &mut Scenario, id: &str) {
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": id }] }));
    }

    mod base {
        use super::*;

        #[test]
        fn r2197001_the_units_in_lanes_n_minus_1_and_n_plus_1_have_3_more_attack_and_the_same_health() {
            let s = board(false);
            for lane in [2, 4] {
                let view = s.stats(s.unit_at(P1, lane).id.as_str());
                assert_eq!((view.attack, view.max_health), (PRINTED + AURA, PRINTED), "lane {lane}");
            }
        }

        #[test]
        fn r2197001_the_totem_itself_reads_0_attack() {
            let s = board(false);
            let view = s.stats(s.unit_at(P1, 3).id.as_str());
            assert_eq!((view.attack, view.max_health), (0, 4));
        }

        #[test]
        fn r2197001_a_unit_two_lanes_away_an_enemy_unit_across_and_a_backrow_card_read_nothing() {
            let s = board(false);
            assert_eq!(attack_at(&s, P1, 5), PRINTED, "two lanes away");
            assert_eq!(attack_at(&s, P2, 3), PRINTED, "across, on the other side");
            let farm = s.backrow(P1, 3).expect("the Field Spell");
            assert_eq!(s.stats(farm.id.as_str()).attack, 0, "a backrow card, no Unit");
        }

        #[test]
        fn r2197001_r13_a_unit_beneath_a_stack_pile_beside_it_gets_nothing_and_the_top_does() {
            let mut s = crate::scenario(json!({
                "seed": "flame-tongue-totem-pile",
                "p1": {
                    "field": [
                        { "def": VANILLA, "lane": 2 },
                        { "def": FIENDER, "lane": 2, "stack": true },
                        { "def": ID, "lane": 3 },
                    ],
                    "hand": [HIT_JOB, FILLER],
                    "library": crate::filler(4),
                    "mana": 10,
                },
                "p2": { "hand": [FILLER], "library": crate::filler(4) },
            }));
            let top = s.unit_at(P1, 2).id;
            assert_eq!(s.card(top.as_str()).def_id, FIENDER);
            let under = s.card(VANILLA).id.clone();
            let with_totem = s.stats(top.as_str()).attack;
            assert_eq!(s.stats(under.as_str()).attack, PRINTED, "a dormant card gets no aura");
            let totem = s.unit_at(P1, 3).id;
            hit(&mut s, &totem);
            assert_eq!(s.stats(top.as_str()).attack, with_totem - AURA, "the top had it");
        }

        #[test]
        fn r2197001_r1081000_standing_in_the_backrow_it_reaches_only_the_units_standing_beside_it() {
            let mut s = board(false);
            let totem = s.unit_at(P1, 3).id;
            let left = s.unit_at(P1, 2).id;
            crate::stand_in_backrow(&mut s, &totem, 2);
            crate::stand_in_backrow(&mut s, &left, 1);
            assert_eq!(s.stats(left.as_str()).attack, PRINTED + AURA, "standing beside it");
            assert_eq!(attack_at(&s, P1, 4), PRINTED, "another row");
            assert_eq!(s.stats(s.backrow(P1, 3).expect("the Field Spell").id.as_str()).attack, 0);
        }

        #[test]
        fn r2197001_the_bonus_leaves_with_the_totem() {
            let mut s = board(false);
            let totem = s.unit_at(P1, 3).id;
            hit(&mut s, &totem);
            s.expect_in_zone(totem.as_str(), "graveyard");
            assert_eq!(attack_at(&s, P1, 2), PRINTED);
            assert_eq!(attack_at(&s, P1, 4), PRINTED);
        }

        #[test]
        fn r2197001_the_bonus_leaves_a_neighbour_that_moves_away() {
            let mut s = board(false);
            let left = s.unit_at(P1, 2).id;
            crate::stand_in_backrow(&mut s, &left, 1);
            assert_eq!(s.stats(left.as_str()).attack, PRINTED);
            assert_eq!(s.card(left.as_str()).buffs, AttackHealth::default(), "nothing was stored on it");
        }

        #[test]
        fn s6_3_a_vanilla_totem_gives_nothing() {
            let mut s = board(false);
            let totem = s.unit_at(P1, 3).id;
            s.card_mut(totem.as_str()).vanilla = true;
            assert_eq!(attack_at(&s, P1, 2), PRINTED);
            assert_eq!(attack_at(&s, P1, 4), PRINTED);
        }

        #[test]
        fn attack_reads_through_param() {
            let mut s = board(false);
            let totem = s.unit_at(P1, 3).id;
            assert_eq!(crate::upgrade_number(&mut s, &totem, "attack"), AURA + 1);
            assert_eq!(attack_at(&s, P1, 2), PRINTED + AURA + 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2197001_it_is_0_8_and_each_neighbour_has_6_more_attack() {
            let s = board(true);
            let view = s.stats(s.unit_at(P1, 3).id.as_str());
            assert_eq!((view.attack, view.max_health), (0, 8));
            assert_eq!(attack_at(&s, P1, 2), PRINTED + 6);
            assert_eq!(attack_at(&s, P1, 4), PRINTED + 6);
            assert_eq!(attack_at(&s, P1, 5), PRINTED);
        }
    }
}
