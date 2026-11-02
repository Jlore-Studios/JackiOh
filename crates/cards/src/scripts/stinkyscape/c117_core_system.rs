//! S #117 Core System (SPEC §8.10 row S 117; §2.4, §6.3 Bounce, R13, R78, R1839000, R1848004): (3)
//! Spell, Epic.
//!   Base:    "Bounce all permanents. Lose {loss} mana next turn." — 2.
//!   Radiant: "Bounce all enemy permanents. Lose {loss} mana next turn." — 2.
//!
//! Engine: `bounce_all` over both rows of both sides (ST-S14-6): every permanent on the field, each
//! pile's top and every backrow card, face-down ones included (R13), back to its controller's hand,
//! reset (R78), a token vanishing and a full hand burning the rest (§2.4); a Dormant card is out of
//! reach (R1839000), and so is one Immune to Spells. Then `next_turn_mana` of minus `loss` on its
//! caster (R1848004): their next refresh gives their max mana less it, never below 0, a badge both
//! seats read.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-117";

fn core_system(side: &'static str) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![
                bounce_all(json_as(json!({ "side": side, "rows": ["units", "backrow"] }))),
                next_turn_mana(json_as(json!({ "amount": -param(&*ctx, "loss") }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: core_system("any"),
        radiant: core_system("enemy"),
    }
}

// S #117 Core System — SPEC §8.10 row S 117, BUILD M12 row S 117.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CORE_SYSTEM: &str = "stinkyscape-117";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const WEAPONS: &str = "core-014"; // (4) Field Spell.
    const SHEEPISH: &str = "core-041"; // (1) Trap.
    const RUSH_TOKEN: &str = "core-t-rush"; // A Unit token.
    const FILLER: &str = "core-005";

    fn refresh_label(s: &Scenario, viewer: PlayerId, of: PlayerId) -> Option<String> {
        let view = s.view(viewer);
        let side = if of == viewer { &view.you } else { &view.opponent };
        side.modifiers
            .iter()
            .find(|modifier| modifier.id == "nextTurnMana")
            .map(|modifier| modifier.label.clone())
    }

    fn full_board(radiant: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": {
                "hand": [{ "def": CORE_SYSTEM, "radiant": radiant }, FILLER],
                "field": [VANILLA, { "def": WEAPONS, "lane": 2 }],
                "library": filler(4),
            },
            "p2": {
                "hand": [FILLER],
                "field": [VANILLA, RUSH_TOKEN, { "def": SHEEPISH, "lane": 3 }],
                "library": filler(4),
            },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn every_unit_and_backrow_card_of_both_sides_returns_and_a_token_vanishes() {
            let mut s = full_board(false);
            let mine = s.unit_at(P1, 1);
            let field_spell = s.backrow(P1, 2).expect("the Field Spell");
            let theirs = s.unit_at(P2, 1);
            let token = s.unit_at(P2, 2);
            let trap = s.backrow(P2, 3).expect("the face-down Trap");

            s.play(CORE_SYSTEM, json!({}));

            for card in [&mine, &field_spell, &theirs, &trap] {
                s.expect_in_zone(card, "hand");
            }
            assert_eq!(s.card(&trap).controller, P2, "to its controller's hand");
            s.expect_in_zone(&token, "gone");
            for lane in 1..=5 {
                assert!(s.unit(P1, lane).is_none() && s.unit(P2, lane).is_none(), "lane {lane}");
                assert!(s.backrow(P1, lane).is_none() && s.backrow(P2, lane).is_none(), "lane {lane}");
            }
        }

        #[test]
        fn a_full_hand_burns_the_rest() {
            crate::register_all();
            let mut hand = vec![json!(CORE_SYSTEM)];
            hand.extend(filler(HAND_CAP as usize - 2).as_array().cloned().unwrap_or_default());
            let mut s = scenario(json!({
                "p1": { "hand": hand, "field": [VANILLA, VANILLA, VANILLA] },
                "p2": { "hand": [FILLER] },
            }));
            let units: Vec<CardInstance> = (1..=3).map(|lane| s.unit_at(P1, lane)).collect();

            s.play(CORE_SYSTEM, json!({}));

            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            s.expect_in_zone(&units[0], "hand");
            s.expect_in_zone(&units[1], "hand");
            s.expect_in_zone(&units[2], "graveyard");
        }

        #[test]
        fn r1848004_its_casters_next_turn_starts_2_less_and_both_see_the_badge() {
            let mut s = full_board(false);

            s.play(CORE_SYSTEM, json!({}));

            let badge = Some("Next refresh −2 mana".to_string());
            assert_eq!(refresh_label(&s, P1, P1), badge);
            assert_eq!(refresh_label(&s, P2, P1), badge);
            assert_eq!(refresh_label(&s, P2, P2), None, "its caster's alone");
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().active, P1);
            let mana = &s.state().players[P1].mana;
            assert_eq!(mana.current, mana.max - 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn only_the_enemys_return() {
            let mut s = full_board(true);
            let mine = s.unit_at(P1, 1);
            let field_spell = s.backrow(P1, 2).expect("the Field Spell");
            let theirs = s.unit_at(P2, 1);
            let trap = s.backrow(P2, 3).expect("the face-down Trap");

            s.play(CORE_SYSTEM, json!({}));

            s.expect_in_zone(&mine, "field");
            s.expect_in_zone(&field_spell, "field");
            s.expect_in_zone(&theirs, "hand");
            s.expect_in_zone(&trap, "hand");
            assert_eq!(refresh_label(&s, P1, P1).as_deref(), Some("Next refresh −2 mana"));
        }
    }
}
