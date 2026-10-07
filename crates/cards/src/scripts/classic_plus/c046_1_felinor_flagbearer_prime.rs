//! C+ #46.1 Felinor Flagbearer Prime (SPEC §8.7 row 46.1). (2) Unit, Felinor, Token (printed
//! Legendary), 5/5 → 10/10; the card C+ #46's Death shuffles in.
//!   Base:    "Rush. Cry: Fill your board with copies of this. Aura: Your other Felinors have
//!            +{aura}/+{aura}." — aura 1
//!   Radiant: the same text, aura 2.
//!   Engine:  "A unit-token card that lives in the deck and hand until it is played (R11). Its Cry fills
//!            every empty, unlocked unit zone left to right (R64) with copies per R57 (Radiant flag,
//!            buffs and granted keywords kept), which do not fire their Cry (R1), so nothing loops;
//!            copies are not generation (R387). Every copy's aura lifts the others. Tunes: aura 1 ↑."
//!
//! "Fill your board" is one `summonCopy` per unit zone: each takes the leftmost empty, unlocked,
//! unreserved zone (R64) and fizzles once none is left (§3.2), so a full board makes none. A copy is a
//! summon, which fires no Cry (R1). The aura is #46's base aura ("your other Felinors").

use jackioh_engine::effects::summon_copy;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-046-1";

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
                        && matches!(unit.zone, Zone::Field { row: Row::Units, .. })
                        && unit.id != ctx.self_.id
                        && def_of(Some(ctx.state), &unit.def_id).tags.contains(&Tag::Felinor)
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

    // The same script: the Radiant face's 10/10 is catalog data and its +2/+2 its declared `aura`.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C+ #46.1 Felinor Flagbearer Prime — SPEC §8.7 row 46.1, BUILD M9 Classic+ row C+ 46.1: "Rush; Cry
// fills every empty, unreserved unit zone of yours with copies of itself (R688: a Locked zone takes
// one; R57: face, buffs and
// granted keywords, not damage), none firing a Cry (R1), so nothing loops; each copy's aura gives your
// other Felinors +1/+1, so each of n Primes has +(n − 1)/+(n − 1) from the rest; a full board makes
// none; the copies are Tokens and cease to exist when they leave (R11); the aura reads through
// `param()`; radiant +2/+2 and the copies are Radiant".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PRIME: &str = "classicplus-046-1";
    const VANILLA: &str = "core-008"; // Mr. Vanilla 4/4, no tag
    const DUPE: &str = "core-012"; // Duplicating Felinors 3/4, a Felinor
    const FLOOD: &str = "core-017"; // (4) Spell: bounce all Units
    const FILLER: &str = "core-005";

    fn primes(s: &Scenario) -> Vec<String> {
        let mut out = Vec::new();
        for lane in 1..=5 {
            if let Some(unit) = s.unit(P1, lane)
                && unit.def_id == PRIME
            {
                out.push(unit.id);
            }
        }
        out
    }

    /// TS `muster({ radiant?, field? })`: the Prime, a Flood and a filler in p1's hand with 10 mana,
    /// a Felinor on p2's field.
    fn muster(radiant: bool, field: Value) -> Scenario {
        let mut prime = json!({ "def": PRIME });
        if radiant {
            prime["radiant"] = json!(true);
        }
        scenario(json!({
            "p1": { "hand": [prime, FLOOD, FILLER], "field": field, "mana": 10 },
            "p2": { "hand": [FILLER], "field": [DUPE] },
        }))
    }

    fn lanes(s: &Scenario) -> Vec<Option<String>> {
        (1..=5).map(|lane| s.unit(P1, lane).map(|unit| unit.def_id)).collect()
    }

    use crate::js;

    #[test]
    fn is_a_2_felinor_prime_unit_token_card_printed_legendary_5_5_rush_both_faces_run_one_script() {
        crate::register_all();
        let def = crate::card_def(super::ID);
        assert!(def.token);
        assert_eq!(js(&def.tags), json!(["Felinor", "Prime", "Token"]));
        assert_eq!(def.printed_rarity, Some(PrintedRarity::Legendary));
        // TS `expect(radiant).toBe(base)`: both faces hold the very same hooks.
        let scripts = script();
        assert!(Arc::ptr_eq(scripts.radiant.cry.as_ref().unwrap(), scripts.base.cry.as_ref().unwrap()));
        assert!(Arc::ptr_eq(scripts.radiant.aura.as_ref().unwrap(), scripts.base.aura.as_ref().unwrap()));
    }

    mod base {
        use super::*;

        #[test]
        fn r64_its_cry_fills_every_empty_unit_zone_of_yours_left_to_right_with_copies_of_itself() {
            crate::register_all();
            let mut s = muster(false, json!([{ "def": VANILLA, "lane": 2 }]));
            s.play(PRIME, json!({ "zone": 4 }));
            assert_eq!(
                lanes(&s),
                [PRIME, VANILLA, PRIME, PRIME, PRIME].map(|id| Some(id.to_string()))
            );
            let summons: Vec<i32> = s
                .last_events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Summoned { lane, .. } => Some(*lane),
                    _ => None,
                })
                .collect();
            assert_eq!(summons, [4, 1, 3, 5]); // the played Prime, then its copies
        }

        #[test]
        fn r1_no_copy_fires_a_cry_so_nothing_loops_one_cry_four_copies() {
            crate::register_all();
            let mut s = muster(false, json!([]));
            s.play(PRIME, json!({}));
            assert_eq!(primes(&s).len(), 5);
            assert_eq!(
                s.last_events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::CardResolved { .. }))
                    .count(),
                1
            );
        }

        #[test]
        fn s10_4_each_of_n_primes_has_n_1_n_1_from_the_rest_other_felinors_get_n() {
            crate::register_all();
            let mut s = muster(false, json!([{ "def": DUPE, "lane": 5 }]));
            s.play(PRIME, json!({}));
            let ids = primes(&s);
            assert_eq!(ids.len(), 4);
            for id in &ids {
                s.expect_stats(id, json!({ "attack": 5 + 3, "health": 5 + 3 }));
            }
            let dupe = s.unit(P1, 5).expect("a unit in lane 5");
            s.expect_stats(&dupe, json!({ "attack": 3 + 4, "health": 4 + 4 }));
            let theirs = s.unit(P2, 1).expect("a unit in p2's lane 1");
            s.expect_stats(&theirs, json!({ "attack": 3, "health": 4 }));
        }

        #[test]
        fn r688_a_locked_zone_takes_a_copy_a_reserved_zone_is_still_skipped() {
            crate::register_all();
            let mut s = muster(false, json!([]));
            lock_zone(
                s.state_mut(),
                ZoneRef {
                    player: P1,
                    row: Row::Units,
                    lane: 3,
                },
            );
            s.state_mut().reserved.push(ZoneRef {
                player: P1,
                row: Row::Units,
                lane: 4,
            });
            s.play(PRIME, json!({}));
            let prime = || Some(PRIME.to_string());
            assert_eq!(lanes(&s), vec![prime(), prime(), prime(), None, prime()]);
        }

        #[test]
        fn s3_2_a_full_board_makes_no_copy() {
            crate::register_all();
            let mut s = muster(false, json!([VANILLA, VANILLA, VANILLA, VANILLA]));
            s.play(PRIME, json!({}));
            assert_eq!(primes(&s).len(), 1);
            assert_eq!(
                s.last_events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::Summoned { .. }))
                    .count(),
                1
            );
        }

        #[test]
        fn r57_the_copies_keep_its_face_buffs_and_granted_keywords_and_are_summoning_sick_with_rush() {
            crate::register_all();
            let mut s = muster(false, json!([]));
            let id = s.card(PRIME).id.clone();
            {
                let card = find_instance_mut(s.state_mut(), &id).expect("the Prime in hand");
                card.buffs = AttackHealth { attack: 2, health: 1 };
                card.granted_keywords.push(Keyword::Taunt);
            }
            s.play(PRIME, json!({}));
            let ids = primes(&s);
            assert_eq!(ids.len(), 5);
            for id in &ids {
                assert_eq!(js(&s.card(id).buffs), json!({ "attack": 2, "health": 1 }));
                let kinds: Vec<KeywordKind> = s.stats(id).keywords.iter().map(|keyword| keyword.kind()).collect();
                assert!(kinds.contains(&KeywordKind::Rush) && kinds.contains(&KeywordKind::Taunt));
                s.expect_stats(id, json!({ "attack": 5 + 2 + 4, "health": 5 + 1 + 4 }));
            }
        }

        #[test]
        fn r11_the_copies_are_tokens_bounced_they_cease_to_exist() {
            crate::register_all();
            let mut s = muster(false, json!([]));
            s.play(PRIME, json!({}));
            let ids = primes(&s);
            assert_eq!(ids.len(), 5);
            s.play(FLOOD, json!({}));
            for id in &ids {
                s.expect_in_zone(id, "gone");
            }
            assert!(!s.hand(P1).iter().any(|card| card.def_id == PRIME));
        }

        #[test]
        fn r387_copies_are_not_generation_every_copy_is_a_prime() {
            crate::register_all();
            let mut s = muster(false, json!([]));
            s.play(PRIME, json!({}));
            assert_eq!(primes(&s).len(), 5);
        }

        #[test]
        fn r386_an_upgrade_makes_each_aura_2_2_and_the_copies_keep_it() {
            crate::register_all();
            let mut s = muster(false, json!([]));
            let id = s.card(PRIME).id.clone();
            step_param(find_instance_mut(s.state_mut(), &id).expect("the Prime in hand"), "aura", 1);
            s.play(PRIME, json!({ "zone": 1 }));
            assert_eq!(primes(&s).len(), 5);
            for id in primes(&s) {
                s.expect_stats(&id, json!({ "attack": 5 + 8, "health": 5 + 8 }));
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn s5_2_10_10_rush_the_copies_are_radiant_and_each_aura_gives_2_2() {
            crate::register_all();
            let mut s = muster(true, json!([VANILLA, VANILLA, VANILLA]));
            s.play(PRIME, json!({}));
            let ids = primes(&s);
            assert_eq!(ids.len(), 2);
            for id in &ids {
                assert!(s.card(id).radiant);
                s.expect_stats(id, json!({ "attack": 12, "health": 12 }));
            }
        }

        #[test]
        fn s10_4_a_full_radiant_board_each_of_five_has_8_8() {
            crate::register_all();
            let mut s = muster(true, json!([]));
            s.play(PRIME, json!({}));
            assert_eq!(primes(&s).len(), 5);
            for id in primes(&s) {
                s.expect_stats(&id, json!({ "attack": 18, "health": 18 }));
            }
        }
    }
}
