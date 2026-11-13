//! SPEC §8.12 S+ #122 Quad, a card of the Stinkyscape+ set (R1420, R3535000). Built by S13 (#3560).
//! 3/12
//! **Tribute 2**
//! **Can’t attack**
//! **Can’t defend**
//! **Flying**
//! **End of turn:** Heal all allies 6 hp and deal 3 damage to all enemies
//! \~\~\~
//! 6/24
//! **Tribute 2**
//! **Can’t attack**
//! **Can’t defend**
//! **Flying**
//! **Armor 2**
//! **End of turn:** Heal all allies 12 hp and deal 6 damage to all enemies
//!
//! Tribute 2 is the script's `tribute` (R101, R366); the other keywords and Armor 2 are catalog data.
//! End of turn (R62) heals first (R1860003), then damages (R1862001).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-122";

/// "Tribute 2" (R101).
const TRIBUTE: i32 = 2;

fn face() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE),
            ..StaticFlags::default()
        }),
        end_of_turn: Some(hook(|ctx| {
            let heal_amount = param(ctx, "heal");
            let damage = param(ctx, "damage");
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
                damage_all(json_as(json!({
                    "amount": damage,
                    "side": "enemy",
                    "heroes": true,
                }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(),
        radiant: face(),
    }
}

// S+ #122 Quad — SPEC §8.12 row 122.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::play_choices::tribute_cost_of;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{filler, weapon_board};

    const FODDER: &str = "stand-in (0) 1/1";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    /// A Unit that outlasts the end of turn's heal and damage (the Radiant face's 6 would kill a 4/4).
    const SOLID: &str = "stand-in (1) 10/10 R20/20";

    fn on_field(radiant: bool, p1_extra: Value, p2_extra: Value) -> Scenario {
        let mut p1 = json!({ "field": [{ "def": ID, "radiant": radiant }] });
        for (key, value) in p1_extra.as_object().cloned().unwrap_or_default() {
            p1[key] = value;
        }
        let mut p2 = json!({});
        for (key, value) in p2_extra.as_object().cloned().unwrap_or_default() {
            p2[key] = value;
        }
        weapon_board("s122", p1, p2)
    }

    fn in_hand(radiant: bool, fodder: usize) -> Scenario {
        weapon_board(
            "s122-tribute",
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "field": vec![FODDER; fodder], "library": filler(4) }),
            json!({}),
        )
    }

    #[test]
    fn tribute_2_is_paid() {
        for radiant in [false, true] {
            let mut s = in_hand(radiant, TRIBUTE as usize);
            assert_eq!(
                tribute_cost_of(s.state(), s.card(ID)),
                TRIBUTE,
                "radiant {radiant}"
            );
            s.expect_refused(|s| s.play(ID, json!({ "zone": 1 })));
            let fodder: Vec<String> = (1..=TRIBUTE).map(|lane| s.unit_at(P1, lane).id).collect();
            s.play(ID, json!({ "zone": 1, "tributes": fodder }));
            for id in &fodder {
                s.expect_in_zone(id.as_str(), "graveyard");
            }
            assert_eq!(s.unit_at(P1, 1).def_id, ID, "radiant {radiant}");
        }
    }

    #[test]
    fn r1860003_end_of_turn_heals_allies_then_r1862001_damages_all_enemies() {
        // Base: heal 6, damage 3. Radiant: heal 12, damage 6.
        for radiant in [false, true] {
            let (heal, damage) = if radiant { (12, 6) } else { (6, 3) };
            let mut s = on_field(
                radiant,
                json!({ "health": 20, "field": [{ "def": ID, "radiant": radiant }, { "def": SOLID, "lane": 2, "damage": 4 }] }),
                json!({ "health": 30, "field": [{ "def": SOLID, "lane": 1, "damage": 0 }] }),
            );
            // Quad is in lane 1 with damage 0; the ally in lane 2 has damage 4.
            s.end_turn();
            let enemy = s.unit_at(P2, 1).id;
            let ally = s.unit_at(P1, 2).id;
            assert_eq!(
                find_instance(s.state(), &ally)
                    .map(|card| card.damage)
                    .unwrap_or(-1),
                (4 - heal).max(0),
                "radiant {radiant}"
            );
            assert_eq!(
                find_instance(s.state(), &enemy)
                    .map(|card| card.damage)
                    .unwrap_or(-1),
                damage,
                "radiant {radiant}"
            );
            s.expect_health(P1, 20 + heal);
            s.expect_health(P2, 30 - damage);
        }
    }

    #[test]
    fn r1862001_enemy_hero_takes_the_damage_too() {
        let mut s = on_field(false, json!({}), json!({}));
        s.end_turn();
        s.expect_health(P2, 30 - 3);
        // Its own hero is an ally it heals (R1860003), on top of the 30 a hero starts with: a hero's
        // health has no cap.
        s.expect_health(P1, 30 + 6);
    }

    mod base {
        use super::*;

        #[test]
        fn cannot_attack_or_defend() {
            let mut s = on_field(false, json!({}), json!({ "field": [VANILLA] }));
            let quad = s.card(ID).id.clone();
            let enemy = s.unit_at(P2, 1).id;
            s.expect_refused(|s| s.attack(quad.as_str(), enemy.as_str()));
            s.expect_refused(|s| s.attack(quad.as_str(), "hero"));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn armor_2_is_printed() {
            let s = on_field(true, json!({}), json!({}));
            assert_eq!(s.stats(ID).armor, 2);
        }
    }
}
