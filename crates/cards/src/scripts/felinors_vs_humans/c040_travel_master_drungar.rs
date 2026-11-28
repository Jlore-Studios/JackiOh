//! SPEC §8.14 FvH #40 Travel Master Drungar ((4) Unit, Human, Legendary, 3/3 → 6/6), a card of the held
//! Felinors vs Humans set (R5235000). Built by S09 (#5259).
//!   Base:    "Tribute 1. Cry: Recruit a Unit from each set. Overload ({mana})." (4)
//!   Radiant: "Tribute 1. Cry: Recruit a Unit from each set and make them Radiant. Overload ({mana})." (2)
//!
//! Engine: Tribute 1 (`StaticFlags.tribute`, R101, R2261000). The Cry (R1) runs one `recruit` per set that
//! ships (`SHIPPED_SETS`, Core included, in that order), each with `RecruitFilter { type: Unit, set }`
//! (R5238005), each scan reading the board the last left; a set with no Unit in the deck, or a full row,
//! recruits nothing (R129, R4238000). The Radiant face makes each one Radiant as it lands. Then the
//! designer's standalone "Overload" line, folded into the Cry as Core #2's (FV2, R4353021, R4751000):
//! `next_turn_mana` of minus the declared `mana`, its controller's next refresh, which a summon (no Cry)
//! never pays. Tunes: mana 4 ↓ (Radiant 2), read by its Cry (R1748000).

use jackioh_engine::effects::{next_turn_mana, recruit};
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-040";

/// §6.3 "Tribute 1": one of your Units.
const TRIBUTE_COST: i32 = 1;

/// R5238005: one Recruit of a Unit of `set`, Radiant on the Radiant face.
fn recruit_from(set: SetName, radiant: bool) -> Effect {
    let mut filter: RecruitFilter = json_as(json!({ "type": "Unit" }));
    filter.set = Some(OneOrMany::One(set));
    let mut args: RecruitArgs = json_as(json!({}));
    args.filter = Some(filter);
    if radiant {
        args.radiant = Some(true);
    }
    recruit(args)
}

fn drungar(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE_COST),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |ctx| {
            let mut effects: Vec<Effect> = SHIPPED_SETS.iter().map(|set| recruit_from(*set, radiant)).collect();
            // R4353021: the Overload is the Cry's, paid whatever the Recruits find.
            effects.push(next_turn_mana(json_as(json!({ "amount": -param(ctx, "mana") }))));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: drungar(false),
        radiant: drungar(true),
    }
}

// FvH #40 Travel Master Drungar — SPEC §8.14 row 40 (R101, R5238005, R129, R4353021, R1748000).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005"; // Stockpile: a Spell kept in hand and the deck's filler
    const BODY: &str = "stand-in (1) 1/1"; // the Tribute
    const CORE_UNIT: &str = "core-008"; // Mr. Vanilla: a Core Unit with no text
    const CORE_OTHER: &str = "stand-in (1) 2/2"; // a second Core Unit
    const TG_UNIT: &str = "traininggrounds-001"; // Spikey Steven: a Training Grounds Unit with no text

    /// The declared `mana` of each face.
    const LOSS: i32 = 4;
    const RADIANT_LOSS: i32 = 2;

    fn board(radiant: bool, library: Value) -> Scenario {
        scenario(json!({
            "seed": "travel-master-drungar",
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": [BODY],
                "library": library,
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(3) },
        }))
    }

    /// Plays Drungar, tributing the Unit on the field.
    fn play(s: &mut Scenario) {
        let tribute = s.unit_at(P1, 1).id;
        s.play(ID, json!({ "tributes": [tribute] }));
    }

    fn on_field(s: &Scenario, def_id: &str) -> Vec<CardInstance> {
        (1..=UNIT_ZONES).filter_map(|lane| s.unit(P1, lane)).filter(|unit| unit.def_id == def_id).collect()
    }

    fn in_library(s: &Scenario, def_id: &str) -> usize {
        s.pile(P1, "library").iter().filter(|card| card.def_id == def_id).count()
    }

    #[test]
    fn r101_it_asks_a_tribute_of_one() {
        let mut s = board(false, json!([CORE_UNIT, FILLER]));
        s.expect_refused(|s| s.play(ID, json!({})));
        play(&mut s);
        s.expect_in_zone(ID, "field");
    }

    #[test]
    fn r4353021_a_summoned_drungar_runs_no_cry_and_loses_nothing() {
        let mut s = board(false, json!([CORE_UNIT, TG_UNIT, FILLER]));
        crate::run_as(&mut s, jackioh_engine::effects::summon(json_as(json!({ "defId": ID }))), P1);
        assert_eq!(s.state().players.p1.mana.next_turn_mod, 0);
        assert_eq!(in_library(&s, CORE_UNIT), 1, "no Recruit");
    }

    mod base {
        use super::*;

        #[test]
        fn r5238005_recruits_one_unit_of_each_set_the_deck_holds() {
            let mut s = board(false, json!([FILLER, CORE_UNIT, CORE_OTHER, TG_UNIT, FILLER]));
            play(&mut s);
            assert_eq!(on_field(&s, CORE_UNIT).len(), 1, "Core's first Unit, top down");
            assert_eq!(in_library(&s, CORE_OTHER), 1, "one Unit a set");
            let steven = on_field(&s, TG_UNIT);
            assert_eq!(steven.len(), 1, "Training Grounds' Unit");
            assert!(!steven[0].radiant);
            assert_eq!(in_library(&s, FILLER), 2, "no Spell is recruited");
        }

        #[test]
        fn r129_a_deck_with_no_unit_recruits_nothing() {
            let mut s = board(false, json!([FILLER, FILLER]));
            play(&mut s);
            assert!(s.unit(P1, 2).is_none());
            assert_eq!(in_library(&s, FILLER), 2);
        }

        #[test]
        fn r4353021_the_cry_overloads_4_and_the_turn_after_is_whole() {
            let mut s = board(false, json!([CORE_UNIT, FILLER, FILLER, FILLER]));
            play(&mut s);
            assert_eq!(s.state().players.p1.mana.next_turn_mod, -LOSS);
            assert_eq!(s.state().players.p2.mana.next_turn_mod, 0, "its own side");
            crate::next_turn_of(&mut s, P1);
            let side = &s.state().players.p1;
            assert_eq!(side.mana.current, (side.mana.max - LOSS).max(0));
        }

        #[test]
        fn r1748000_the_overload_reads_through_param() {
            let mut s = board(false, json!([FILLER]));
            let loss = crate::upgrade_number(&mut s, ID, "mana");
            assert_eq!(loss, LOSS - 1);
            play(&mut s);
            assert_eq!(s.state().players.p1.mana.next_turn_mod, -loss);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5238005_recruits_one_unit_of_each_set_and_makes_them_radiant() {
            let mut s = board(true, json!([CORE_UNIT, TG_UNIT, FILLER]));
            play(&mut s);
            for def_id in [CORE_UNIT, TG_UNIT] {
                let recruited = on_field(&s, def_id);
                assert_eq!(recruited.len(), 1, "{def_id}");
                assert!(recruited[0].radiant, "{def_id} is Radiant");
            }
        }

        #[test]
        fn r4353021_the_cry_overloads_2() {
            let mut s = board(true, json!([CORE_UNIT, FILLER]));
            play(&mut s);
            assert_eq!(s.state().players.p1.mana.next_turn_mod, -RADIANT_LOSS);
        }
    }
}
