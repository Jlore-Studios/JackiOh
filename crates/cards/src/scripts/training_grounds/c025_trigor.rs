//! TG #25 Trigor (SPEC §8.11 row 25; R2197002, R2197004): (3) Unit, Human, Rare, 3/3 → 6/6, a card of the held
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
//! - **The end** (REVIEW A3.6, R2197004): a Trigor that a Trigor's Death summoned this turn summons
//!   nothing when it dies this turn, so each lineage adds at most one body a turn. Growth alone (R2197002)
//!   was no terminator: a Poisonous hit, a destroy or two Trigors forced into each other (S #24 Warp
//!   Disruptor II) kill a body whatever its size. The Death marks the Trigor it summons in that
//!   Trigor's memory (`CHAIN_MARK`, R2206000), and the hook reads the card as it died (R78): a stat
//!   override is no mark, since TG #34 Dino-size's Set stats and a Seance copy wear one too (R2198000,
//!   R2198001), and a played Trigor they touch still summons. Leaving the field clears the mark (R78).

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-025";

/// §6.3 "Tribute 3": three of your Units.
const TRIBUTE_COST: i32 = 3;

/// R2206000: the memory key the Death's summon sets on the Trigor it makes, holding the turn.
const CHAIN_MARK: &str = "trigorChain";

/// R2197002: the base stats the Death summons a Trigor with: this one's base stats plus `grow`.
fn next_stats(state: &GameState, this: &CardInstance, grow: i32) -> AttackHealth {
    let base = base_stats_of(state, this);
    AttackHealth { attack: base.attack + grow, health: base.health + grow }
}

/// R2206000: whether this Trigor's Death summons nothing: a Trigor's Death made it this turn.
fn chained(state: &GameState, this: &CardInstance) -> bool {
    this.memory.get(CHAIN_MARK).and_then(Value::as_i64) == Some(i64::from(state.turn))
}

/// R2476000: after "Summon a Trigor", the base stats its Death would summon one with ("6/6"), its own
/// base stats down the lineage plus `grow`; the dash while its chain mark holds this turn, as it
/// summons nothing then. Its value is that Health, 0 for none. Base stats are public.
fn preview() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        if chained(ctx.state, ctx.self_) {
            return vec![PreviewValue {
                label: "Summon a Trigor".to_string(),
                value: 0,
                display: Some(NOTHING_REMEMBERED.to_string()),
                ids: None,
            }];
        }
        let next = next_stats(ctx.state, ctx.self_, param(&ctx, "grow"));
        vec![PreviewValue {
            label: "Summon a Trigor".to_string(),
            value: next.health,
            display: Some(format!("{}/{}", next.attack, next.health)),
            ids: None,
        }]
    })
}

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
            // R2197004, R2206000: a Trigor a Trigor's Death summoned this turn (its mark holds this
            // turn) summons nothing when it dies this turn, so each lineage adds at most one body a
            // turn whatever kills them.
            let turn = ctx.state.turn;
            if chained(ctx.state, &this) {
                return vec![];
            }
            // R2197002: cumulative, so each body is larger than the last.
            let next = next_stats(ctx.state, &this, param(&*ctx, "grow"));
            vec![summon(json_as(json!({
                "defId": ID,
                "player": "self",
                "radiant": false,
                "statsOverride": { "attack": next.attack, "health": next.health },
                "memory": { CHAIN_MARK: turn },
            })))]
        })),
        preview: Some(preview()),
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
// destroyed at 3/3: a base Trigor 6/6 arrives in the leftmost open zone; that one destroyed on a later turn:
// a 9/9; buffs on the dying one are not carried (base stats only); the new one fires no Cry and pays no
// Tribute; a full row: none summoned; a Trigor its Death summoned this turn summons nothing when it dies this
// turn (R2197004): under an aura that takes 7 Health the 6/6 dies and none follows, and seeds 1543 and 9463's
// endless chains (a Poisonous forced attacker; two Trigors forced into each other) end; exiled or bounced: no
// Death, no summon; the growth reads through `param()`. radiant 6/6; its Death summons a base 12/12, whose
// Death on a later turn summons 15/15".
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const VANILLA: &str = "core-008"; // (1) Mr. Vanilla, 4/4.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const FLOOD: &str = "core-017"; // (4) Spell: Bounce all Units.
    const COLLATERAL: &str = "core-034"; // (4) Spell: Exile a permanent and a random card from the enemy deck.
    const SUPPRESSIVE: &str = "core-046"; // (2) Field Spell: Aura: All Units −1/−1; Radiant: enemy Units −2/−2.
    const FILLER: &str = "core-005"; // (1) Spell Stockpile: a card in hand, so no turn ends by itself.
    const MOTHS: &str = "core-009"; // (2) 1/14.
    /// TG #34 Dino-size: "Set a Unit's Attack and Health to 10/10."
    const DINO: &str = "traininggrounds-034";
    /// S #24 Warp Disruptor II: "After your opponent summons a Unit, each of your Units attacks it."
    const DISRUPTOR_II: &str = "stinkyscape-024";

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

    /// R2476000: after "Summon a Trigor", the stats its Death would summon, to both seats: 6/6 on a 3/3;
    /// once it dies, the 6/6 its Death made shows the dash this turn, for it summons nothing then.
    #[test]
    fn r2476000_it_previews_the_stats_its_death_would_summon_and_the_dash_while_chained() {
        let mut s = game(json!([VANILLA, VANILLA, { "def": ID, "lane": 3 }]), json!([HIT_JOB, FILLER]));
        let first = trigor_id(&s);
        let next = vec![("Summon a Trigor".to_string(), 6, Some("6/6".to_string()))];
        assert_eq!(crate::preview_seen(&s, P1, &first), next);
        assert_eq!(crate::preview_seen(&s, P2, &first), next);
        hit(&mut s, &first);
        let second = trigor_id(&s);
        assert_ne!(second, first);
        assert_eq!(
            crate::preview_seen(&s, P1, &second),
            vec![("Summon a Trigor".to_string(), 0, Some("—".to_string()))]
        );
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
        fn r2197002_destroyed_at_3_3_a_base_6_6_arrives_in_the_leftmost_open_zone_then_on_a_later_turn_a_9_9() {
            let mut s = game(json!([{ "def": ID, "lane": 3 }]), json!([HIT_JOB, HIT_JOB, FILLER]));
            let first = trigor_id(&s);
            hit(&mut s, &first);
            s.expect_in_zone(first.as_str(), "graveyard");
            assert_eq!(trigors(&s), vec![(1, 6, 6, false)]);
            crate::next_turn_of(&mut s, P1);
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
        fn r2197004_under_an_aura_taking_7_health_the_6_6_dies_and_none_follows() {
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
            // R2197004: the 3/3 dies and summons a 6/6, which the aura kills in the same check; it arrived
            // this turn from a Trigor's Death, so it summons nothing.
            let summoned = s
                .last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == ID))
                .count();
            assert_eq!(summoned, 1, "one body, then it stops");
            assert!(trigors(&s).is_empty());
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

    /// R2197004: the chains the handicap fuzz found endless on R2197002's growth alone, each now ending.
    mod r2197004_the_chain_ends {
        use super::*;

        fn trigors_summoned(s: &Scenario) -> usize {
            s.last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == ID))
                .count()
        }

        /// Fuzz seed 1543 (`--handicap`): a Poisonous Unit forced into each new Trigor kills it whatever
        /// its size.
        #[test]
        fn r2197004_a_poisonous_unit_forced_into_each_new_trigor_ends_the_chain_after_one_body() {
            let mut s = crate::scenario(json!({
                "seed": "trigor-poison",
                "p1": { "field": [ID], "hand": [HIT_JOB, FILLER], "library": crate::filler(4), "mana": 30 },
                "p2": {
                    "field": [MOTHS],
                    "backrow": [DISRUPTOR_II],
                    "hand": [FILLER],
                    "library": crate::filler(4),
                },
            }));
            let moths = s.unit_at(P2, 1).id;
            s.card_mut(moths.as_str()).granted_keywords = vec![Keyword::Poisonous];
            let first = trigor_id(&s);
            hit(&mut s, &first);
            // The 6/6 its Death summoned is forced into, poisoned and summons nothing (R2197004).
            assert_eq!(trigors_summoned(&s), 1);
            assert!(trigors(&s).is_empty());
            assert!(s.state().result.is_none());
        }

        /// Fuzz seed 9463 (`--handicap`): Trigors and Warp Disruptor IIs on both sides; each new Trigor is
        /// forced into the other side's, both die and both return.
        #[test]
        fn r2197004_two_trigors_forced_into_each_other_end_after_one_body_each() {
            let mut s = crate::scenario(json!({
                "seed": "trigor-mutual",
                "p1": {
                    "field": [ID],
                    "backrow": [DISRUPTOR_II],
                    "hand": [HIT_JOB, FILLER],
                    "library": crate::filler(4),
                    "mana": 30,
                },
                "p2": { "field": [ID], "backrow": [DISRUPTOR_II], "hand": [FILLER], "library": crate::filler(4) },
            }));
            let first = trigor_id(&s);
            hit(&mut s, &first);
            assert!(trigors_summoned(&s) <= 3, "each lineage adds at most one body this turn");
            assert!(s.state().result.is_none());
        }

        #[test]
        fn r2197004_a_trigor_summoned_on_an_earlier_turn_still_summons() {
            let mut s = game(json!([ID]), json!([HIT_JOB, HIT_JOB, FILLER]));
            let first = trigor_id(&s);
            hit(&mut s, &first);
            let second = trigor_id(&s);
            hit(&mut s, &second);
            assert!(trigors(&s).is_empty(), "the second died the turn it arrived");
            let mut s = game(json!([{ "def": ID, "statsOverride": { "attack": 6, "health": 6 } }]), json!([HIT_JOB, FILLER]));
            let veteran = trigor_id(&s);
            hit(&mut s, &veteran);
            assert_eq!(trigors(&s), vec![(1, 9, 9, false)], "set up on an earlier turn, it is no chain body");
        }

        /// REVIEW B-1: a stat override is no chain mark. A Trigor played this turn and set to 10/10 by
        /// TG #34 Dino-size still summons its successor, a 13/13, when it dies this turn; that one, the
        /// chain's, carries the mark and summons nothing when it dies this turn too.
        #[test]
        fn r2206000_r2197004_a_played_trigor_set_to_10_10_this_turn_still_summons_a_13_13() {
            let mut s = game(json!([VANILLA, VANILLA, VANILLA]), json!([ID, DINO, HIT_JOB, HIT_JOB, FILLER]));
            let three: Vec<String> = (1..=3).map(|lane| s.unit_at(P1, lane).id).collect();
            s.play(ID, json!({ "tributes": three, "zone": 1 }));
            let played = trigor_id(&s);
            s.play(DINO, json!({ "targets": [{ "pick": "instance", "instanceId": played }] }));
            assert_eq!(trigors(&s), vec![(1, 10, 10, false)]);
            assert!(s.card(played.as_str()).stats_override.is_some());
            hit(&mut s, &played);
            assert_eq!(trigors(&s), vec![(1, 13, 13, false)], "a played Trigor summons its successor");
            let successor = trigor_id(&s);
            assert_eq!(s.card(successor.as_str()).memory.get(super::super::CHAIN_MARK), Some(&json!(s.state().turn)));
            hit(&mut s, &successor);
            assert!(trigors(&s).is_empty(), "the chain's body summons nothing this turn");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2197002_r1828003_it_is_6_6_its_death_summons_a_base_12_12_whose_death_on_a_later_turn_summons_15_15() {
            let mut s = game(json!([{ "def": ID, "radiant": true }]), json!([HIT_JOB, HIT_JOB, FILLER]));
            assert_eq!(trigors(&s), vec![(1, 6, 6, true)]);
            let first = trigor_id(&s);
            hit(&mut s, &first);
            assert_eq!(trigors(&s), vec![(1, 12, 12, false)]);
            crate::next_turn_of(&mut s, P1);
            let second = trigor_id(&s);
            hit(&mut s, &second);
            assert_eq!(trigors(&s), vec![(1, 15, 15, false)]);
        }
    }
}
