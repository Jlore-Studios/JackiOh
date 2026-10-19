//! M+ #13 Living Health Potion (SPEC §8.9 row 13, BUILD M11 row M+ 13): (1) Unit, Potion,
//! Rare, 3/3 → 6/6.
//!   Base:    "Activate: Tribute this. Heal your hero and each of your Units {heal}." (5)
//!   Radiant: "Activate: Heal your hero and each of your Units {heal}." (5)
//!
//! An Activate (§6.2, R384): in its controller's main phase, the turn it lands included (no
//! summoning sickness), once a turn. The base face's cost is the card's own Tribute
//! (`tribute_self`, C #84), paid first, so it leaves before the heal and does not heal itself;
//! the Radiant face's has none, so it stays and heals itself with the rest. The heal is `heal`
//! on the controller's hero (no cap, §6.3) and a `for_each_card` over each of their Units (M
//! #7's shape), each healed up to its max health; enemies are untouched (R1179002). The number
//! is the declared `heal`, read as the ability runs (R386). Tunes: heal 5 ↑.

use jackioh_engine::effects::{ForEachCardArgs, for_each_card, heal};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-013";

fn heal_allies() -> Hook {
    hook(|ctx| {
        let amount = param(&*ctx, "heal");
        vec![
            heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": amount }))),
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
                        "amount": amount,
                    })))
                }),
            }),
        ]
    })
}

fn potion(radiant: bool) -> Script {
    Script {
        activations: vec![ActivationDecl {
            id: "heal".to_string(),
            label: "Heal your hero and Units".to_string(),
            uses: ActivationUses::Count(1),
            cost: (!radiant).then(|| ActivationCost {
                tribute_self: Some(true),
                ..ActivationCost::default()
            }),
            targets: Vec::new(),
            modes: Vec::new(),
            can_activate: None,
            has: None,
            run: heal_allies(),
        }],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: potion(false),
        radiant: potion(true),
    }
}

// M+ #13 Living Health Potion — SPEC §8.9 row 13, BUILD M11 row M+ 13: "Activate: tributed
// first, then heals your hero 5 (no cap) and each of your Units 5 (up to its max health); enemy
// characters untouched; heal reads through `param()`; radiant 6/6, kept on the field and healed
// with the rest, once a turn".
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// (1) Spell. A spare that keeps the turn alive (§2.5).
    const FILLER: &str = "core-010";
    /// (3) 9/9 Taunt.
    const BODY: &str = "core-019";
    /// Library filler.
    const X: &str = "core-020";

    fn game(seed: &str, p1: Value, p2: Value) -> Scenario {
        let mut sides = json!({
            "seed": seed,
            "p1": { "library": [X, X, X] },
            "p2": { "hand": [FILLER], "library": [X, X, X] },
        });
        for (seat, extra) in [("p1", p1), ("p2", p2)] {
            if let Value::Object(entries) = extra {
                for (key, value) in entries {
                    sides[seat][key.as_str()] = value;
                }
            }
        }
        crate::scenario(sides)
    }

    fn activate(id: &str, s: &mut Scenario) {
        let potion = s.card(id).id.clone();
        s.activate(potion.as_str(), json!({}));
    }

    fn damage_of(s: &Scenario, def: &str) -> i32 {
        s.card(def).damage
    }

    #[test]
    fn r1179002_tributed_first_then_heals_your_hero_and_each_of_your_units_5() {
        let mut s = game(
            "m+013-heals",
            json!({
                "health": 20,
                "field": [ID, { "def": BODY, "lane": 2, "damage": 6 }, { "def": BODY, "lane": 3, "damage": 2 }],
                "hand": [FILLER],
            }),
            json!({}),
        );
        activate(ID, &mut s);
        // The Tribute went first: the potion is gone and healed nothing of itself.
        s.expect_in_zone(ID, "graveyard");
        s.expect_health(P1, 25);
        // A 9/9 at 3 heals to 8; one at 7 heals to its max of 9.
        let first = s.unit(P1, 2).expect("the first body");
        let second = s.unit(P1, 3).expect("the second body");
        assert_eq!(9 - first.damage, 8);
        assert_eq!(9 - second.damage, 9);
    }

    #[test]
    fn s6_3_heals_your_hero_5_with_no_cap() {
        let mut s = game("m+013-no-cap", json!({ "health": 28, "field": [ID], "hand": [FILLER] }), json!({}));
        activate(ID, &mut s);
        s.expect_health(P1, 33);
    }

    #[test]
    fn s6_3_radiant_heals_your_hero_5_with_no_cap() {
        let mut s = game(
            "m+013-radiant-no-cap",
            json!({ "health": 30, "field": [{ "def": ID, "radiant": true }], "hand": [FILLER] }),
            json!({}),
        );
        activate(ID, &mut s);
        s.expect_health(P1, 35);
    }

    #[test]
    fn enemy_characters_are_untouched() {
        let mut s = game(
            "m+013-theirs",
            json!({ "field": [ID], "hand": [FILLER] }),
            json!({ "health": 20, "field": [{ "def": BODY, "damage": 4 }] }),
        );
        activate(ID, &mut s);
        s.expect_health(P2, 20);
        let enemy = s.unit(P2, 1).expect("their body");
        assert_eq!(enemy.damage, 4);
    }

    #[test]
    fn heal_reads_through_param() {
        let mut s = game("m+013-param", json!({ "health": 20, "field": [ID], "hand": [FILLER] }), json!({}));
        let potion = s.unit(P1, 1).expect("the potion").id.clone();
        set_param(s.card_mut(&potion), "heal", 3);
        s.activate(potion.as_str(), json!({}));
        s.expect_health(P1, 23);
    }

    #[test]
    fn r1179002_radiant_stays_heals_itself_and_is_once_a_turn() {
        let mut s = game(
            "m+013-radiant",
            json!({
                "health": 20,
                "field": [{ "def": ID, "radiant": true, "damage": 2 }],
                "hand": [FILLER],
            }),
            json!({}),
        );
        activate(ID, &mut s);
        s.expect_in_zone(ID, "field");
        // It stayed, so it healed itself with the rest: its 2 damage is gone.
        assert_eq!(damage_of(&s, ID), 0);
        s.expect_health(P1, 25);
        s.expect_refused(|s| s.activate(ID, json!({})));
    }
}
