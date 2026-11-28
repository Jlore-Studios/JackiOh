//! SPEC §8.14 FvH #110 Lord Jakrov ((1) Unit, Human, Legendary, 3/2 → 6/4), a card of the Felinors vs
//! Humans set (R5235000).
//!   Base:    "Cry: Give every other Unit Fragile. / Death: Cast Whirlwind."
//!   Radiant: "Cry: Give every enemy Unit Fragile. / Death: Cast a Radiant Whirlwind."
//!
//! Cry (R1): `grant_keyword_cards` of Fragile (R5236010) on every other Unit on the field, both sides
//! (Radiant: the enemy's), for good, as C+ #77 Anti-Softlock gives its keywords; a Unit that leaves the
//! field loses it (R78). Death: `cast_new` of C+ #21 Whirlwind (R70: made, cast free and landing in its
//! caster's graveyard), Radiant on the Radiant face, whose 1 damage then destroys every Fragile Unit it
//! hurts at the state check (R5236010). Declares no numbers. Built by S11 (#5261).

use jackioh_engine::effects::{CastDef, CastHow, CastNewArgs, CastNewDef, cast_new, grant_keyword_cards};
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-110";

/// C+ #21 Whirlwind.
const WHIRLWIND: &str = "classicplus-021";

/// "Give every other (Radiant: every enemy) Unit Fragile": `side` is the scope's.
fn fragile_cry(side: &'static str) -> Hook {
    hook(move |_ctx| {
        vec![grant_keyword_cards(json_as(json!({
            "scope": {
                "side": side,
                "zones": ["field"],
                "rows": ["units"],
                "types": ["Unit"],
                "excludeSelf": true,
            },
            "keyword": { "kind": "Fragile" },
        })))]
    })
}

/// "Cast (a Radiant) Whirlwind".
fn whirlwind(radiant: bool) -> Hook {
    hook(move |_ctx| {
        vec![cast_new(CastNewArgs {
            def: CastNewDef::Def(CastDef {
                def_id: WHIRLWIND.to_string(),
                radiant: Some(radiant),
            }),
            radiant: None,
            how: CastHow::default(),
        })]
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(fragile_cry("any")),
            death: Some(whirlwind(false)),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(fragile_cry("enemy")),
            death: Some(whirlwind(true)),
            ..Script::default()
        },
    }
}

// FvH #110 Lord Jakrov — SPEC §8.14 row 110 (R5236010).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const KEEPER: &str = "stand-in (0) 1/1"; // a play always open, so no turn ends by itself
    /// A Unit Whirlwind's 1 damage never kills unless it is Fragile.
    const TOUGH: &str = "stand-in (1) 1/10";
    const HIT_JOB: &str = "core-016"; // Hit Job: the removal

    /// Jakrov in p1's hand, a tough Unit on each side, and Hit Job to kill Jakrov with.
    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, HIT_JOB, KEEPER],
                "field": [{ "def": TOUGH, "lane": 2 }],
                "mana": 10,
                "library": filler(4),
            },
            "p2": { "hand": [KEEPER], "field": [{ "def": TOUGH, "lane": 1 }], "library": filler(4) },
        }))
    }

    fn fragile(s: &Scenario, card: &CardInstance) -> bool {
        has_keyword(&s.stats(card).keywords, KeywordKind::Fragile)
    }

    /// Plays Jakrov into lane 1: Jakrov, p1's tough Unit and p2's.
    fn played(radiant: bool) -> (Scenario, CardInstance, CardInstance, CardInstance) {
        let mut s = board(radiant);
        s.play(ID, json!({ "zone": 1 }));
        let jakrov = s.unit_at(P1, 1);
        let ours = s.unit_at(P1, 2);
        let theirs = s.unit_at(P2, 1);
        (s, jakrov, ours, theirs)
    }

    /// Hit Job on Jakrov.
    fn kill(s: &mut Scenario, jakrov: &CardInstance) {
        s.play(HIT_JOB, pick::targets(pick::instance(jakrov)));
        s.expect_in_zone(jakrov, "graveyard");
    }

    fn whirlwinds_cast(s: &Scenario) -> Vec<CardInstance> {
        s.pile(P1, "graveyard").into_iter().filter(|card| card.def_id == WHIRLWIND).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn declares_no_tuned_numbers() {
            assert!(crate::card_def(ID).params.is_none());
        }

        #[test]
        fn r5236010_its_cry_gives_every_other_unit_on_both_sides_fragile() {
            let (s, jakrov, ours, theirs) = played(false);
            assert!(fragile(&s, &ours), "its own side's other Unit");
            assert!(fragile(&s, &theirs), "the enemy's Unit");
            assert!(!fragile(&s, &jakrov), "not itself");
        }

        #[test]
        fn r5236010_its_death_casts_whirlwind_which_destroys_every_fragile_unit() {
            let (mut s, jakrov, ours, theirs) = played(false);
            kill(&mut s, &jakrov);
            let cast = whirlwinds_cast(&s);
            assert_eq!(cast.len(), 1, "one Whirlwind cast, in p1's graveyard");
            assert!(!cast[0].radiant);
            s.expect_in_zone(&ours, "graveyard");
            s.expect_in_zone(&theirs, "graveyard");
        }

        #[test]
        fn its_death_casts_whirlwind_with_no_cry_before_it() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [HIT_JOB, KEEPER],
                    "field": [{ "def": ID, "lane": 1 }],
                    "mana": 10,
                    "library": filler(4),
                },
                "p2": { "hand": [KEEPER], "field": [{ "def": TOUGH, "lane": 1 }], "library": filler(4) },
            }));
            let jakrov = s.unit_at(P1, 1);
            let theirs = s.unit_at(P2, 1);
            kill(&mut s, &jakrov);
            assert_eq!(whirlwinds_cast(&s).len(), 1);
            s.expect_in_zone(&theirs, "field");
            let hurt = s.card(&theirs).damage;
            assert_eq!(hurt, 1, "Whirlwind's 1 damage, which a Unit lacking Fragile outlives");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5236010_its_cry_gives_every_enemy_unit_fragile_and_none_of_yours() {
            let (s, jakrov, ours, theirs) = played(true);
            assert!(fragile(&s, &theirs), "the enemy's Unit");
            assert!(!fragile(&s, &ours), "not its own side's");
            assert!(!fragile(&s, &jakrov));
        }

        #[test]
        fn r5236010_its_death_casts_a_radiant_whirlwind_which_destroys_the_fragile_enemy() {
            let (mut s, jakrov, ours, theirs) = played(true);
            kill(&mut s, &jakrov);
            let cast = whirlwinds_cast(&s);
            assert_eq!(cast.len(), 1, "one Whirlwind cast, in p1's graveyard");
            assert!(cast[0].radiant, "a Radiant one");
            s.expect_in_zone(&theirs, "graveyard");
            s.expect_in_zone(&ours, "field");
        }
    }
}
