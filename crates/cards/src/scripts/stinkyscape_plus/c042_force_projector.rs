//! SPEC §8.12 S+ #42 Force Projector (R3550004, R3556004; R1420):
//! (3) Field Spell, Mindustrial, Rare, a card of the Stinkyscape+ set.
//!   Base:    "The first {pool} damage allies would take per turn is nullified."
//!   Radiant: "Armor 2
//!            The first {pool} damage allies would take per turn is nullified."
//!
//! The pool is the E6 per-turn damage pool (`StaticFlags.damagePool`, R3550004): the first N damage
//! its side would take each turn is nullified after Armor, refilling at the start of its
//! controller's turn, with a preview of what is left. The Radiant face's Armor 2 is an aura on its
//! controller's Units (R3556004), read off its own printed Armor — inert on the Field Spell itself
//! — so a hit soaks Armor first, then the pool.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-042";

/// "The first 5 damage allies would take per turn".
const POOL: i32 = 5;
/// "The first 10 damage allies would take per turn".
const RADIANT_POOL: i32 = 10;

/// What the pool has left this turn, as the E6 fixture's (R3550004).
fn pool_preview() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        if ctx.zone != ConditionZone::Field {
            return vec![];
        }
        let Some(left) = damage_pool_left(ctx.state, ctx.self_) else {
            return vec![];
        };
        vec![count_preview(
            &format!("up to {} each turn", param_or(&ctx, "pool", POOL)),
            left,
        )]
    })
}

/// R3556004: Armor 2 on its controller's Units, read off its own printed Armor.
fn armor_aura() -> AuraHook {
    aura_hook(|args: HookArgs<'_>| {
        let n = numbered_keywords_on(args.state, args.self_)
            .into_iter()
            .find(|keyword| keyword.key == NumberedKey::Armor)
            .map(|keyword| keyword.value)
            .unwrap_or(0);
        if n <= 0 {
            return vec![];
        }
        let controller = args.self_.controller;
        let state = args.state;
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                unit.controller == controller && in_units_row_or_standing(state, unit)
            }),
            mod_: StatMod {
                keywords: Some(vec![Keyword::Armor { n }]),
                ..StatMod::default()
            },
        }]
    })
}

fn force_projector(pool: i32, radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            damage_pool: Some(pool),
            ..StaticFlags::default()
        }),
        preview: Some(pool_preview()),
        aura: radiant.then(armor_aura),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: force_projector(POOL, false),
        radiant: force_projector(RADIANT_POOL, true),
    }
}

// S+ #42 Force Projector — SPEC §8.12 row S+ 42 (R3550004, R3556004).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, next_turn_of, run_as, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A 3-attack Ranged Unit (Stinkyscape #68 Stratos).
    const RANGER: &str = "stinkyscape-068";
    /// A durable body to be hit.
    const BODY: &str = "stand-in (1) 4/4 R12/12";

    /// p1 holds the Projector; `"mana": 10` plays it from hand.
    fn projected(radiant: bool, seed: &str) -> Scenario {
        let mut s = weapon_board(
            seed,
            json!({
                "mana": 10,
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": [BODY],
                "library": filler(6),
            }),
            json!({
                "field": [RANGER, RANGER, RANGER],
                "library": filler(6),
            }),
        );
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    fn pool_left(s: &Scenario) -> Option<i32> {
        damage_pool_left(s.state(), s.card(ID))
    }

    mod base {
        use super::*;

        #[test]
        fn r3550004_it_soaks_the_first_five_damage_its_side_takes_each_turn() {
            let mut s = projected(false, "s42-pool");
            assert_eq!(pool_left(&s), Some(5));
            next_turn_of(&mut s, P2);
            // Three 3-attack hits deal 0, then 1, then 3: the pool soaks the first 5.
            let ranger = |s: &Scenario, lane: i32| s.unit(P2, lane).expect("a ranger there").id;
            s.attack(ranger(&s, 1).as_str(), "hero");
            s.expect_health(P1, HERO_HEALTH);
            assert_eq!(pool_left(&s), Some(2));
            s.attack(ranger(&s, 2).as_str(), "hero");
            s.expect_health(P1, HERO_HEALTH - 1);
            assert_eq!(pool_left(&s), Some(0));
            s.attack(ranger(&s, 3).as_str(), "hero");
            s.expect_health(P1, HERO_HEALTH - 4);
            // It refills at the start of its controller's next turn.
            next_turn_of(&mut s, P1);
            next_turn_of(&mut s, P2);
            assert_eq!(pool_left(&s), Some(5));
        }

        #[test]
        fn r3550004_the_flag_matches_the_declared_pool_on_both_faces() {
            for (radiant, pool) in [(false, POOL), (true, RADIANT_POOL)] {
                let s = projected(radiant, &format!("s42-flag-{radiant}"));
                assert_eq!(
                    jackioh_engine::scripts::flags_of(s.state(), s.card(ID)).damage_pool,
                    Some(pool),
                    "the flag matches the declared pool",
                );
                assert_eq!(pool_left(&s), Some(pool));
            }
        }

        #[test]
        fn r3550004_the_preview_shows_what_is_left() {
            let mut s = projected(false, "s42-preview");
            next_turn_of(&mut s, P2);
            let ranger = s.unit(P2, 1).expect("a ranger there").id;
            s.attack(ranger.as_str(), "hero");
            let shown = s
                .view(P1)
                .you
                .backrow
                .into_iter()
                .flatten()
                .find_map(|entry| match entry {
                    BackrowView::Public(public) if public.def_id == ID => public.preview,
                    _ => None,
                })
                .expect("the projector shows a preview");
            assert!(!shown.is_empty(), "a preview while 2 of 5 are left");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3556004_radiant_gives_its_units_armor_2_and_the_pool_soaks_after_it() {
            let mut s = projected(true, "s42-radiant");
            assert_eq!(pool_left(&s), Some(10));
            next_turn_of(&mut s, P2);
            // A 3-attack hit past Armor 2 leaves 1 for the pool, and the Unit takes nothing.
            let ranger = s.unit(P2, 1).expect("a ranger there").id;
            let body = s.unit(P1, 1).expect("a body there").id;
            s.attack(ranger.as_str(), body.as_str());
            s.expect_stats(body.as_str(), json!({ "health": 4, "maxHealth": 4 }));
            assert_eq!(pool_left(&s), Some(9));
            // A fresh pool soaks after Armor too: a 13 hit past Armor 2 soaks 10 and deals 1.
            next_turn_of(&mut s, P1);
            next_turn_of(&mut s, P2);
            assert_eq!(pool_left(&s), Some(10));
            run_as(
                &mut s,
                jackioh_engine::effects::damage(json_as(json!({
                    "to": { "of": "instance", "instanceId": body },
                    "amount": 13,
                }))),
                P2,
            );
            s.expect_stats(body.as_str(), json!({ "health": 3, "maxHealth": 4 }));
            assert_eq!(pool_left(&s), Some(0));
        }
    }
}
