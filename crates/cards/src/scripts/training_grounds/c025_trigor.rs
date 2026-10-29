//! TG #25 Trigor (SPEC §8.11 row 25; R2197002): (3) Unit, Human, Rare, 3/3 → 6/6, a card of the held
//! Training Grounds set (R1420, R2194000).
//!   Base and Radiant: "Tribute 3 / Death: Summon a Trigor with this one's base stats +{grow}/+{grow}." —
//!   grow 3, Radiant 6
//! Engine:
//! - **Tribute 3** is the play's price (§6.3, R101), paid at §10.5 step 2, as C #61 Plague Bringer
//!   Goliath's; a zone its Tribute empties counts (R391).
//! - **The Death** is M #94 Shrinking Felinor's mirror: `ctx.self_` is the card as it died, its
//!   `statsOverride` still on it (R78), and `base_stats_of` reads its §10.4 layer-1 stats off that (R1181).
//!   It summons a fresh base Trigor, not a copy (R57), whose base stats are those +{grow}/+{grow}, into
//!   the leftmost open unit zone (R64): a summon, so no Cry and no Tribute (§6.3). A full row summons none.
//!   Both faces summon a base body (R1828003's pattern), so the Radiant one's chain goes on at +3/+3.
//! - **The end** (REVIEW A3.6, R2197002): every Trigor is strictly larger than the one that died, so no
//!   fixed aura or damage kills each new one as it arrives; under an aura taking N Health the chain stops
//!   after at most ⌈N/grow⌉ bodies. `grow` never tunes below 1.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-025";

/// §6.3 "Tribute 3": three of your Units.
const TRIBUTE_COST: i32 = 3;

fn trigor() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE_COST),
            ..StaticFlags::default()
        }),
        death: Some(hook(|ctx| {
            let Some(this) = ctx.self_.clone() else {
                return vec![];
            };
            let base = base_stats_of(ctx.state, &this);
            let grow = param(&*ctx, "grow");
            // R2197002: cumulative, so each body is larger than the last and the chain ends.
            vec![summon(json_as(json!({
                "defId": ID,
                "player": "self",
                "radiant": false,
                "statsOverride": { "attack": base.attack + grow, "health": base.health + grow },
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: trigor(),
        radiant: trigor(),
    }
}

// TG #25 Trigor — SPEC §8.11 row 25, BUILD M13 row TG 25: "can't be played without 3 Units to Tribute;
// destroyed at 3/3: a base Trigor 6/6 arrives in the leftmost open zone; that one destroyed: a 9/9; buffs
// on the dying one are not carried (base stats only); the new one fires no Cry and pays no Tribute; a full
// row: none summoned; under an aura that takes 7 Health, the chain stops by itself (6/6 dies, 9/9 survives
// as 9/2); exiled or bounced: no Death, no summon; the growth reads through `param()`. radiant 6/6; its
// Death summons a base 12/12, whose Death summons 15/15".
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    const VANILLA: &str = "core-008"; // (1) Mr. Vanilla, 4/4.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const FLOOD: &str = "core-017"; // (4) Spell: Bounce all Units.
    const COLLATERAL: &str = "core-034"; // (4) Spell: Exile a permanent and a random card from the enemy deck.
    const SUPPRESSIVE: &str = "core-046"; // (2) Field Spell: Aura: All Units −1/−1; Radiant: enemy Units −2/−2.
    const FILLER: &str = "core-005"; // (1) Spell Stockpile: a card in hand, so no turn ends by itself.

    /// p1, active with 30 mana, has `field` and `hand`; p2 has a library for Collateral Damage.
    fn game(field: Value, hand: Value) -> Scenario {
        crate::scenario(json!({
            "seed": "trigor",
            "p1": { "field": field, "hand": hand, "library": crate::filler(4), "mana": 30 },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }))
    }

    fn hit(s: &mut Scenario, id: &str) {
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": id }] }));
    }

    /// The Trigors on p1's unit zones, lane by lane: (lane, attack, max health, radiant).
    fn trigors(s: &Scenario) -> Vec<(i32, i32, i32, bool)> {
        (1..=5)
            .filter_map(|lane| s.unit(P1, lane).filter(|unit| unit.def_id == ID).map(|unit| (lane, unit)))
            .map(|(lane, unit)| {
                let view = s.stats(&unit);
                (lane, view.attack, view.max_health, unit.radiant)
            })
            .collect()
    }

    fn trigor_id(s: &Scenario) -> String {
        (1..=5)
            .filter_map(|lane| s.unit(P1, lane))
            .find(|unit| unit.def_id == ID)
            .map(|unit| unit.id)
            .unwrap_or_else(|| panic!("no Trigor on p1's side"))
    }

    mod base {
        use super::*;

        #[test]
        fn r101_it_cant_be_played_without_3_units_to_tribute() {
            let mut s = game(json!([VANILLA, VANILLA]), json!([ID, FILLER]));
            let trigor = s.card(ID).clone();
            let two: Vec<String> = (1..=2).map(|lane| s.unit_at(P1, lane).id).collect();
            s.expect_refused_with(|s| s.play(&trigor, json!({ "tributes": two })), "Tribute");
            s.expect_in_zone(ID, "hand");
        }

        #[test]
        fn s6_3_with_3_units_tributed_it_enters_3_3() {
            let mut s = game(json!([VANILLA, VANILLA, VANILLA]), json!([ID, FILLER]));
            let three: Vec<String> = (1..=3).map(|lane| s.unit_at(P1, lane).id).collect();
            s.play(ID, json!({ "tributes": three, "zone": 1 }));
            assert_eq!(s.state().players[P1].graveyard.len(), 3, "the three Tributes");
            assert_eq!(trigors(&s), vec![(1, 3, 3, false)]);
        }

        #[test]
        fn r2197002_destroyed_at_3_3_a_base_6_6_arrives_in_the_leftmost_open_zone_then_a_9_9() {
            let mut s = game(json!([{ "def": ID, "lane": 3 }]), json!([HIT_JOB, HIT_JOB, FILLER]));
            let first = trigor_id(&s);
            hit(&mut s, &first);
            s.expect_in_zone(first.as_str(), "graveyard");
            assert_eq!(trigors(&s), vec![(1, 6, 6, false)]);
            let second = trigor_id(&s);
            hit(&mut s, &second);
            assert_eq!(trigors(&s), vec![(1, 9, 9, false)]);
        }

        #[test]
        fn r2197002_r57_buffs_and_granted_keywords_on_the_dying_one_are_not_carried() {
            let mut s = game(json!([ID]), json!([HIT_JOB, FILLER]));
            let first = trigor_id(&s);
            s.card_mut(first.as_str()).buffs = AttackHealth { attack: 5, health: 5 };
            s.card_mut(first.as_str()).granted_keywords = vec![Keyword::Taunt];
            hit(&mut s, &first);
            assert_eq!(trigors(&s), vec![(1, 6, 6, false)]);
            let next = s.unit_at(P1, 1);
            assert_eq!(next.buffs, AttackHealth::default());
            assert!(keywords_of(s.state(), &next).is_empty());
        }

        #[test]
        fn s6_3_the_new_one_is_summoned_so_it_fires_no_cry_and_pays_no_tribute() {
            let mut s = game(json!([VANILLA, VANILLA, VANILLA, ID]), json!([HIT_JOB, FILLER]));
            let first = trigor_id(&s);
            hit(&mut s, &first);
            assert_eq!(trigors(&s), vec![(4, 6, 6, false)]);
            assert!((1..=3).all(|lane| s.unit_at(P1, lane).def_id == VANILLA), "no Tribute taken");
            let played = s
                .last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::CardPlayed { .. }))
                .count();
            assert_eq!(played, 1, "no play but the Hit Job");
        }

        #[test]
        fn r64_a_full_row_summons_none() {
            // The Trigor tops a pile in lane 1; when it dies the Mr. Vanilla under it resumes there (§3.2).
            let mut s = game(
                json!([VANILLA, { "def": ID, "stack": true }, VANILLA, VANILLA, VANILLA, VANILLA]),
                json!([HIT_JOB, FILLER]),
            );
            let first = trigor_id(&s);
            hit(&mut s, &first);
            s.expect_in_zone(first.as_str(), "graveyard");
            assert!(trigors(&s).is_empty());
        }

        #[test]
        fn r2197002_under_an_aura_taking_7_health_the_chain_stops_by_itself() {
            // p2's Suppressive Auras, three Radiant (−2/−2 on p1's Units) and one base (−1/−1 on all),
            // take 7 from each of p1's Units; the first Trigor is buffed to live through them.
            let mut s = crate::scenario(json!({
                "seed": "trigor-aura",
                "active": "p2",
                "p1": { "field": [ID], "hand": [FILLER], "library": crate::filler(4) },
                "p2": {
                    "hand": [
                        { "def": SUPPRESSIVE, "radiant": true },
                        { "def": SUPPRESSIVE, "radiant": true },
                        { "def": SUPPRESSIVE, "radiant": true },
                        SUPPRESSIVE,
                        HIT_JOB,
                        FILLER,
                    ],
                    "library": crate::filler(4),
                    "mana": 30,
                },
            }));
            let first = trigor_id(&s);
            s.card_mut(first.as_str()).buffs = AttackHealth { attack: 0, health: 10 };
            for lane in 1..=4 {
                s.play(SUPPRESSIVE, json!({ "zone": lane }));
            }
            assert_eq!(s.stats(first.as_str()).max_health, 13 - 7);
            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": first }] }));
            // 3/3 dies; the 6/6 is −1/−1 and dies in the same check; the 9/9 stands as 2/2.
            let summoned: Vec<i32> = s
                .last_events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Summoned { def_id, .. } if def_id == ID => Some(1),
                    _ => None,
                })
                .collect();
            assert_eq!(summoned.len(), 2, "two bodies, then it stops");
            assert_eq!(trigors(&s), vec![(1, 2, 2, false)]);
            let last = s.card(trigor_id(&s).as_str()).clone();
            let base = base_stats_of(s.state(), &last);
            assert_eq!((base.attack, base.health), (9, 9), "a 9/9 under the aura");
        }

        #[test]
        fn s6_3_bounced_it_has_no_death_and_summons_none() {
            let mut s = game(json!([ID]), json!([FLOOD, FILLER]));
            s.play(FLOOD, json!({}));
            s.expect_in_zone(ID, "hand");
            assert!(trigors(&s).is_empty());
        }

        #[test]
        fn s6_3_exiled_it_has_no_death_and_summons_none() {
            let mut s = game(json!([ID]), json!([COLLATERAL, FILLER]));
            let first = trigor_id(&s);
            s.play(COLLATERAL, json!({ "targets": [{ "pick": "instance", "instanceId": first }] }));
            s.expect_in_zone(first.as_str(), "exile");
            assert!(trigors(&s).is_empty());
        }

        #[test]
        fn grow_reads_through_param() {
            let mut s = game(json!([ID]), json!([HIT_JOB, FILLER]));
            let first = trigor_id(&s);
            assert_eq!(crate::upgrade_number(&mut s, &first, "grow"), 4);
            hit(&mut s, &first);
            assert_eq!(trigors(&s), vec![(1, 7, 7, false)]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2197002_r1828003_it_is_6_6_its_death_summons_a_base_12_12_whose_death_summons_15_15() {
            let mut s = game(json!([{ "def": ID, "radiant": true }]), json!([HIT_JOB, HIT_JOB, FILLER]));
            assert_eq!(trigors(&s), vec![(1, 6, 6, true)]);
            let first = trigor_id(&s);
            hit(&mut s, &first);
            assert_eq!(trigors(&s), vec![(1, 12, 12, false)]);
            let second = trigor_id(&s);
            hit(&mut s, &second);
            assert_eq!(trigors(&s), vec![(1, 15, 15, false)]);
        }
    }
}
