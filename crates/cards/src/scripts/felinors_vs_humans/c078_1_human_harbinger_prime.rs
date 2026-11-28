//! FvH #78.1 Human Harbinger Prime (SPEC §8.14 row 78.1): (3) Unit, Human, Prime, Token (printed
//! Legendary), 6/6 → 12/12; the card FvH #78's Death shuffles in. Built by S08 (#5258).
//!   Base:    "Taunt\nCry: Fill your board with copies of this.\nAura: Your other Humans have
//!            +{aura}/+{aura}." — aura 2
//!   Radiant: the same text, aura 4.
//!
//! C+ #46.1 Felinor Flagbearer Prime's shape with Human for Felinor. Taunt is catalog data. "Fill your
//! board" is one `summon_copy` of itself per unit zone: each takes the leftmost empty, unlocked,
//! unreserved zone (R64), else the leftmost empty Locked one (R688, R1136000), and fizzles once none is
//! left, so a full board makes none. The copies are made per R57 (the Radiant flag, buffs and granted
//! keywords kept) and run no Cry (R1), so nothing loops; copies are not generation (R387). The aura
//! (§10.4 layer 5) reaches its controller's other Human-tagged Units (tags now, R923), so every copy's
//! aura lifts the others. Tunes: aura 2 ↑ (Radiant 4).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-078-1";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| {
            (0..UNIT_ZONES)
                .map(|_| summon_copy(json_as(json!({ "of": { "of": "self" } }))))
                .collect()
        })),
        aura: Some(aura_hook(|ctx| {
            let amount = param(&ctx, "aura");
            vec![AuraEntry {
                applies: Box::new(move |unit: &CardInstance| {
                    unit.controller == ctx.self_.controller
                        && in_units_row_or_standing(ctx.state, unit)
                        && unit.id != ctx.self_.id
                        && tags_of(ctx.state, unit).contains(&Tag::Human)
                }),
                mod_: StatMod {
                    attack: Some(amount),
                    max_health: Some(amount),
                    ..StatMod::default()
                },
            }]
        })),
        ..Script::default()
    };

    // The same script: the Radiant face's 12/12 is catalog data and its +4/+4 its declared `aura`.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// FvH #78.1 Human Harbinger Prime — SPEC §8.14 row 78.1.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const HUMAN: &str = "core-008"; // Mr. Vanilla: a Human
    const OTHER: &str = "stand-in (1) 3/3";
    const FILLER: &str = "core-005"; // Stockpile: a card in hand

    fn muster(radiant: bool, field: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": field },
            "p2": { "hand": [FILLER], "field": [HUMAN] },
        }))
    }

    fn primes(s: &Scenario) -> Vec<String> {
        active_units_of(s.state(), P1)
            .into_iter()
            .filter(|unit| unit.def_id == ID)
            .map(|unit| unit.id.clone())
            .collect()
    }

    fn stat_line(s: &Scenario, card: &str) -> (i32, i32) {
        let view = s.stats(card);
        (view.attack, view.max_health)
    }

    mod base {
        use super::*;

        #[test]
        fn its_cry_fills_every_empty_unit_zone_with_copies_of_itself() {
            let mut s = muster(false, json!([{ "def": OTHER, "lane": 2 }]));
            s.play(ID, json!({}));
            assert_eq!(primes(&s).len(), UNIT_ZONES as usize - 1);
            assert_eq!(active_units_of(s.state(), P1).len(), UNIT_ZONES as usize);
        }

        #[test]
        fn no_copy_runs_a_cry_so_one_card_resolves() {
            let mut s = muster(false, json!([]));
            s.play(ID, json!({}));
            assert_eq!(primes(&s).len(), UNIT_ZONES as usize);
            assert_eq!(s.last_count("cardResolved"), 1);
        }

        #[test]
        fn a_full_board_makes_no_copy() {
            let full: Vec<&str> = vec![OTHER; UNIT_ZONES as usize - 1];
            let mut s = muster(false, json!(full));
            s.play(ID, json!({}));
            assert_eq!(primes(&s).len(), 1);
        }

        #[test]
        fn each_prime_lifts_the_others_and_your_other_humans_but_not_the_opponents() {
            let mut s = muster(false, json!([HUMAN]));
            let human = s.unit_at(P1, 1).id;
            let theirs = s.unit_at(P2, 1).id;
            let (human_before, theirs_before) = (stat_line(&s, &human), stat_line(&s, &theirs));
            let printed = stat_line(&s, ID);
            s.play(ID, json!({}));
            let ids = nonempty(primes(&s), "the Primes");
            let others = ids.len() as i32 - 1;
            for id in &ids {
                assert_eq!(stat_line(&s, id), (printed.0 + 2 * others, printed.1 + 2 * others));
            }
            let lift = 2 * ids.len() as i32;
            assert_eq!(stat_line(&s, &human), (human_before.0 + lift, human_before.1 + lift));
            assert_eq!(stat_line(&s, &theirs), theirs_before);
        }

        #[test]
        fn a_non_human_takes_nothing() {
            let mut s = muster(false, json!([OTHER]));
            let other = s.unit_at(P1, 1).id;
            let before = stat_line(&s, &other);
            s.play(ID, json!({}));
            assert_eq!(stat_line(&s, &other), before);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_copies_are_radiant_and_each_aura_gives_4_4() {
            let mut s = muster(true, json!([OTHER, OTHER, OTHER]));
            let printed = stat_line(&s, ID);
            s.play(ID, json!({}));
            let ids = nonempty(primes(&s), "the Primes");
            assert_eq!(ids.len(), 2);
            for id in &ids {
                assert!(s.card(id.as_str()).radiant);
                assert_eq!(stat_line(&s, id), (printed.0 + 4, printed.1 + 4));
            }
        }
    }
}
