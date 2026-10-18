//! Meditative+ #1.2 Spa Day (SPEC §8.9 row 1.2, §7, R1178002, R311, R80). (2) Spell, AI, Token: an AI
//! generated card M+ #1 Bob's Plan adds after M #20 Aestheticize the Game.
//!   Base:    "Heal your hero {heal}. Shuffle {bombs|Love Bomb|Love Bombs} into your deck."
//!   Radiant: "Heal your hero {heal}. Shuffle {bombs|Radiant Love Bomb|Radiant Love Bombs} into your deck."
//!
//! M #20's easiest line is the hero's health (100, Radiant 90), and a heal of a hero has no cap (§6.3
//! Heal), so the heal climbs toward it. Then M #30.1 Love Bombs, each at a random position (R311), a
//! full deck refusing them (R80); each heals on draw (M #30.1's Cast on draw), Radiant ones twice as
//! much.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-001-2";

/// M #30.1 Love Bomb.
const LOVE_BOMB: &str = "meditative-030-1";

fn spa_day(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![
                heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": param(&*ctx, "heal") }))),
                shuffle_into(json_as(json!({
                    "defId": LOVE_BOMB,
                    "count": param(&*ctx, "bombs"),
                    "radiant": radiant,
                }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: spa_day(false),
        radiant: spa_day(true),
    }
}

// Meditative+ #1.2 Spa Day — SPEC §8.9 row 1.2, BUILD M11 row M+ 1.2: "heals its caster's hero 25 past
// 30 (no cap); shuffles 2 base Love Bombs into its caster's deck, each healing 7 when drawn; a full
// deck refuses them (R80); heal and bombs read through `param()`; radiant heal 50 and 2 Radiant Love
// Bombs (14 each)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";

    fn spa(seed: &str, radiant: bool, library: Value) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": library },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
        }))
    }

    fn bombs(s: &Scenario) -> Vec<CardInstance> {
        s.pile(P1, "library").into_iter().filter(|card| card.def_id == LOVE_BOMB).collect()
    }

    fn health(s: &Scenario) -> i32 {
        s.state().players.p1.hero.health
    }

    /// M #30.1 Love Bomb's heal on the face given, read through its declared number.
    fn bomb_heal(s: &Scenario, radiant: bool) -> i32 {
        param_value(
            s.state(),
            None,
            "heal",
            ParamValueOptions {
                def_id: Some(LOVE_BOMB.into()),
                radiant: Some(radiant),
            },
        )
    }

    /// The turn's draw takes a Love Bomb, cast on draw (M #30.1), and its chain the other (R58), then
    /// finds the deck empty: two heals and the first fatigue (§2.4).
    fn after_the_bombs_are_drawn(s: &Scenario, before: i32, radiant: bool) -> i32 {
        before + 2 * bomb_heal(s, radiant) - FATIGUE_DAMAGE(1)
    }

    mod base {
        use super::*;

        #[test]
        fn heals_your_hero_25_past_30_with_no_cap() {
            let mut s = spa("spa-day", false, json!([FILLER, FILLER]));
            let before = health(&s);
            assert_eq!(before, HERO_HEALTH, "the hero starts full");
            s.play(ID, json!({}));
            assert_eq!(health(&s), before + 25, "§6.3: a hero's heal has no cap");
        }

        #[test]
        fn shuffles_2_love_bombs_into_your_deck_each_healing_7_when_drawn() {
            let mut s = spa("spa-day-bombs", false, json!([]));
            s.play(ID, json!({}));
            let made = bombs(&s);
            assert_eq!(made.len(), 2);
            assert!(made.iter().all(|bomb| !bomb.radiant));
            let before = health(&s);
            s.end_turn();
            s.end_turn();
            assert_eq!(health(&s), after_the_bombs_are_drawn(&s, before, false));
        }

        #[test]
        fn r80_a_full_deck_refuses_them() {
            let full = vec![json!(FILLER); LIBRARY_CAP as usize];
            let mut s = spa("spa-day-full", false, json!(full));
            s.play(ID, json!({}));
            assert!(bombs(&s).is_empty());
        }

        #[test]
        fn heal_and_bombs_read_through_param() {
            let mut s = spa("spa-day-param", false, json!([FILLER]));
            let heal = crate::upgrade_number(&mut s, ID, "heal");
            let count = crate::upgrade_number(&mut s, ID, "bombs");
            s.play(ID, json!({}));
            assert_eq!(health(&s), HERO_HEALTH + heal);
            assert_eq!(bombs(&s).len() as i32, count);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn heals_50_and_shuffles_2_radiant_love_bombs_healing_14_each() {
            let mut s = spa("spa-day-radiant", true, json!([]));
            s.play(ID, json!({}));
            assert_eq!(health(&s), HERO_HEALTH + 50);
            let made = bombs(&s);
            assert_eq!(made.len(), 2);
            assert!(made.iter().all(|bomb| bomb.radiant));
            let before = health(&s);
            s.end_turn();
            s.end_turn();
            assert_eq!(health(&s), after_the_bombs_are_drawn(&s, before, true));
        }
    }
}
