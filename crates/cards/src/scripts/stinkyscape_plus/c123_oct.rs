//! SPEC §8.12 S+ #123 Oct, a card of the Stinkyscape+ set (R1420, R3535000). Built by S13 (#3560).
//! 4/16
//! **Tribute 3**
//! **Can’t attack**
//! **Can’t defend**
//! **Flying**
//! **End of turn:** Heal all allies 8 hp
//! **Aura:** The first 5 damage allies would take per turn is nullified *{5 left}*
//! \~\~\~
//! 8/32
//! **Tribute 3**
//! **Can’t attack**
//! **Can’t defend**
//! **Flying**
//! **End of turn:** Heal all allies 16 hp
//! **Armor 2**
//! The first 10 damage allies would take per turn is nullified *{10 left}*
//!
//! Tribute 3 is the script's `tribute` (R101); the pool is E6's per-turn damage pool
//! (`StaticFlags.damage_pool`, R3550004), read through the declared `pool` (R386), after Armor.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-123";

/// "Tribute 3" (R101).
const TRIBUTE: i32 = 3;

/// "The first 5 damage allies would take per turn".
const POOL: i32 = 5;
/// "The first 10 damage allies would take per turn".
const RADIANT_POOL: i32 = 10;

/// What the pool has left this turn (R3550004), shown as "{left} left" (R3572002).
fn pool_preview() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        if ctx.zone != ConditionZone::Field {
            return vec![];
        }
        let Some(left) = damage_pool_left(ctx.state, ctx.self_) else {
            return vec![];
        };
        let counted = count_preview(
            &format!("up to {} each turn", param_or(&ctx, "pool", POOL)),
            left,
        );
        // R3572002: the text's `{5 left}` reads what the pool has left, not a bare number.
        vec![PreviewValue {
            display: Some(format!("{left} left")),
            ..counted
        }]
    })
}

fn face(pool: i32) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE),
            damage_pool: Some(pool),
            ..StaticFlags::default()
        }),
        preview: Some(pool_preview()),
        end_of_turn: Some(hook(|ctx| {
            let heal_amount = param(ctx, "heal");
            vec![
                heal(json_as(
                    json!({ "target": { "of": "selfHero" }, "amount": heal_amount }),
                )),
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(|each: &mut EffectContext<'_>| {
                        cards_in_scope(each, &json_as(json!({ "side": "self" })))
                            .into_iter()
                            .map(|card| card.id)
                            .collect()
                    }),
                    each: Arc::new(move |instance_id: &str| {
                        heal(json_as(json!({
                            "target": { "of": "instance", "instanceId": instance_id },
                            "amount": heal_amount,
                        })))
                    }),
                }),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(POOL),
        radiant: face(RADIANT_POOL),
    }
}

// S+ #123 Oct — SPEC §8.12 row 123.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::play_choices::tribute_cost_of;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{filler, weapon_board};

    const FODDER: &str = "stand-in (0) 1/1";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const RANGER: &str = "stinkyscape-068"; // Stratos: a 3-attack Ranged Unit

    fn on_field(radiant: bool) -> Scenario {
        weapon_board(
            "s123",
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            json!({ "field": [RANGER, RANGER] }),
        )
    }

    fn in_hand(radiant: bool) -> Scenario {
        weapon_board(
            "s123-tribute",
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "field": [FODDER, FODDER, FODDER], "library": filler(4) }),
            json!({}),
        )
    }

    #[test]
    fn tribute_3_is_paid() {
        for radiant in [false, true] {
            let mut s = in_hand(radiant);
            assert_eq!(
                tribute_cost_of(s.state(), s.card(ID)),
                TRIBUTE,
                "radiant {radiant}"
            );
            let fodder: Vec<String> = (1..=3).map(|lane| s.unit_at(P1, lane).id).collect();
            s.play(ID, json!({ "zone": 1, "tributes": fodder }));
            assert_eq!(s.unit_at(P1, 1).def_id, ID, "radiant {radiant}");
        }
    }

    #[test]
    fn r3550004_the_first_5_damage_a_turn_is_nullified_after_armor() {
        for radiant in [false, true] {
            let pool = if radiant { 10 } else { 5 };
            let mut s = on_field(radiant);
            let oct = s.unit_at(P1, 1).id;
            assert_eq!(
                damage_pool_left(s.state(), s.card(ID)),
                Some(pool),
                "radiant {radiant}"
            );
            // Two Rangers hit the Oct for 3 each: the first 5 (10) is nullified.
            crate::next_turn_of(&mut s, P2);
            let first = s.unit_at(P2, 1).id;
            let second = s.unit_at(P2, 2).id;
            s.attack(first.as_str(), oct.as_str());
            let after_first = find_instance(s.state(), &oct)
                .map(|card| card.damage)
                .unwrap_or(-1);
            s.attack(second.as_str(), oct.as_str());
            let after_second = find_instance(s.state(), &oct)
                .map(|card| card.damage)
                .unwrap_or(-1);
            if radiant {
                assert_eq!(
                    (after_first, after_second),
                    (0, 0),
                    "radiant {radiant}: 3 + 3 within 10"
                );
            } else {
                assert_eq!(after_first, 0, "radiant {radiant}: first 3 within 5");
                assert_eq!(after_second, 1, "radiant {radiant}: 3 + 3 is 1 past 5");
            }
        }
    }

    #[test]
    fn r3550004_the_pool_refills_at_its_controllers_turn_start() {
        let mut s = on_field(false);
        let oct = s.unit_at(P1, 1).id;
        crate::next_turn_of(&mut s, P2);
        let first = s.unit_at(P2, 1).id;
        s.attack(first.as_str(), oct.as_str());
        assert_eq!(damage_pool_left(s.state(), s.card(ID)), Some(2));
        crate::next_turn_of(&mut s, P1);
        assert_eq!(damage_pool_left(s.state(), s.card(ID)), Some(5));
    }

    #[test]
    fn r1860003_end_of_turn_heals_all_allies() {
        for radiant in [false, true] {
            let heal = if radiant { 16 } else { 8 };
            let mut s = weapon_board(
                "s123-heal",
                json!({ "health": 20, "field": [{ "def": ID, "radiant": radiant }, { "def": VANILLA, "lane": 2, "damage": 3 }] }),
                json!({}),
            );
            s.end_turn();
            let ally = s.unit_at(P1, 2).id;
            assert_eq!(
                find_instance(s.state(), &ally)
                    .map(|card| card.damage)
                    .unwrap_or(-1),
                (3 - heal).max(0),
                "radiant {radiant}"
            );
            s.expect_health(P1, 20 + heal);
        }
    }

    /// What the Oct's `{5 left}` reads for its controller on the field.
    fn left_read(s: &Scenario) -> Vec<PreviewValue> {
        let oct = s.card(ID).clone();
        preview_of(s.state(), &oct, P1, ConditionZone::Field).expect("a previewed field card")
    }

    #[test]
    fn r3572002_oct_reads_what_its_pool_has_left() {
        for radiant in [false, true] {
            let pool = if radiant { 10 } else { 5 };
            let mut s = on_field(radiant);
            let oct = s.unit_at(P1, 1).id;
            let read = left_read(&s);
            assert_eq!(read.len(), 1, "radiant {radiant}");
            assert_eq!(read[0].label, format!("up to {pool} each turn"), "radiant {radiant}");
            assert_eq!(read[0].value, pool, "radiant {radiant}");
            assert_eq!(read[0].display, Some(format!("{pool} left")), "radiant {radiant}");
            // A Ranger's 3-attack hit is soaked by the pool, after armor, which then has that much less left.
            crate::next_turn_of(&mut s, P2);
            let first = s.unit_at(P2, 1).id;
            s.attack(first.as_str(), oct.as_str());
            // The Radiant face's 2 armor takes 2 of the 3 first, so the pool soaks only the 1 left.
            let left = pool - if radiant { 1 } else { 3 };
            let read = left_read(&s);
            assert_eq!(read[0].value, left, "radiant {radiant}");
            assert_eq!(read[0].display, Some(format!("{left} left")), "radiant {radiant}");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn armor_2_is_printed() {
            let s = on_field(true);
            assert_eq!(s.stats(ID).armor, 2);
        }
    }
}
