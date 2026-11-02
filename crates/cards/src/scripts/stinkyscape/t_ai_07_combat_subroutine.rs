//! S T-AI-7 Combat Subroutine (SPEC §7, §8.10 row T-AI-7; R1, R21, R78, R347, R482, R1839000, R1881004):
//! (2) Unit, AI, Token (no printed rarity), 2/2 → 4/4, an AI generated token of the Stinkyscape
//! set, built by S21.
//!   Base:    "Cry: Give each of your other Units a random combat keyword."
//!   Radiant: "Cry: Give each of your other Units 2 random combat keywords."
//! Engine: Cry (R1): for each other Unit acting for its controller (`active_units_of`, so a Dormant card is
//!   passed by, R1839000), in their order, a `grant_random_keywords` of its own (R1881004): R21's draws from
//!   the combat keyword pool, never one it has, no Taunt beside Indestructible (R347), granted, so gone with
//!   R78's reset. Radiant: two each. Declares no numbers (R482).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-ai-07";

/// "a random combat keyword", "2 random combat keywords".
const KEYWORDS: i32 = 1;
const RADIANT_KEYWORDS: i32 = 2;

fn face(count: i32) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let me = ctx.self_.as_ref().map(|card| card.id.clone());
            active_units_of(&*ctx.state, ctx.controller)
                .into_iter()
                .filter(|unit| Some(&unit.id) != me.as_ref())
                .map(|unit| {
                    grant_random_keywords(json_as(json!({
                        "target": { "of": "instance", "instanceId": unit.id },
                        "count": count,
                    })))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(KEYWORDS),
        radiant: face(RADIANT_KEYWORDS),
    }
}

// S T-AI-7 Combat Subroutine — SPEC §8.10 row T-AI-7, BUILD M12 row S T-AI-7 (R1881004).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SUBROUTINE: &str = "stinkyscape-t-ai-07";
    const VANILLA: &str = "core-008"; // (1) 4/4, no keywords.
    const FILLER: &str = "core-005";

    fn played(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": SUBROUTINE, "radiant": radiant }, FILLER],
                "field": [{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 2 }],
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "field": [VANILLA], "library": filler(4) },
        }));
        s.play(SUBROUTINE, json!({ "zone": 3 }));
        s
    }

    fn keywords_at(s: &Scenario, player: PlayerId, lane: i32) -> Vec<Keyword> {
        let unit = s.unit(player, lane).expect("a Unit");
        keywords_of(s.state(), s.card(unit.id.as_str()))
    }

    #[test]
    fn r1881004_each_other_unit_of_yours_gains_one_random_combat_keyword() {
        let mut pairs = 0;
        let mut differed = false;
        for i in 0..12 {
            let s = played(false, &format!("subroutine-{i}"));
            let first = keywords_at(&s, P1, 1);
            let second = keywords_at(&s, P1, 2);
            assert_eq!(first.len(), 1, "{first:?}");
            assert_eq!(second.len(), 1, "{second:?}");
            assert!(keywords_at(&s, P1, 3).is_empty(), "not itself");
            assert!(keywords_at(&s, P2, 1).is_empty(), "not the enemy's");
            pairs += 1;
            differed |= first != second;
        }
        assert!(pairs > 0 && differed, "each Unit draws its own");
    }

    #[test]
    fn radiant_4_4_and_two_different_keywords_each() {
        let mut s = played(true, "subroutine-radiant");
        s.expect_stats(SUBROUTINE, json!({ "attack": 4, "health": 4 }));
        for lane in 1..=2 {
            let keywords = keywords_at(&s, P1, lane);
            assert_eq!(keywords.len(), 2, "{keywords:?}");
            assert_ne!(keywords[0].kind(), keywords[1].kind());
        }
    }

    #[test]
    fn r1881004_a_dormant_card_is_passed_by() {
        const AP_ROUNDS: &str = "stinkyscape-100"; // (1) 3/1 Starship Piece, Equip, Pierce.
        let (mut s, ship) = crate::built_starship(
            "subroutine-dormant",
            json!([{ "def": AP_ROUNDS, "lane": 2 }]),
            json!({ "hand": [SUBROUTINE, FILLER], "field": [{ "def": VANILLA, "lane": 1 }] }),
            json!({}),
        );
        let ship_keywords = keywords_of(s.state(), s.card(ship.as_str()));
        s.play(SUBROUTINE, json!({ "zone": 3 }));
        assert_eq!(keywords_at(&s, P1, 1).len(), 1, "the Vanilla gains one");
        assert!(
            s.card(ship.as_str()).granted_keywords.is_empty(),
            "the Dormant Starship none"
        );
        assert_eq!(keywords_of(s.state(), s.card(ship.as_str())), ship_keywords);
    }
}
