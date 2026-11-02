//! S #118 Secure System (SPEC §8.10 row S 118; §10.4, R1748003, R1848004, R1868001): (3) Spell, Epic.
//!   Base:    "Set all Units' Attack to 0. Lose {loss} mana next turn." — 1.
//!   Radiant: "Set all enemy Units' Attack to 0. Lose {loss} mana next turn." — 1.
//!
//! Engine: existing primitives (ST-S14-7): each Unit on the field of both sides is given a `buff` of
//! minus its Attack as it stands now through the layers, before layer 5's floor (`unclamped_attack`,
//! the reading KY's Constant sets a number with), so it shows exactly 0, a Unit Nerfed below 0
//! included, and a later Buff adds from 0 (R1868001); a Unit Immune to Spells is passed by. Nothing
//! here opens a prompt, so the Units read now are the ones the list reaches.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-118";

fn secure_system(side: &'static str) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let units = cards_in_scope(ctx, &json_as(json!({ "side": side })));
            let mut effects: Vec<Effect> = units
                .iter()
                .filter_map(|unit| {
                    let attack = unclamped_attack(&*ctx.state, unit);
                    (attack != 0).then(|| {
                        buff(json_as(json!({
                            "target": { "of": "instance", "instanceId": unit.id },
                            "attack": -attack,
                        })))
                    })
                })
                .collect();
            effects.push(next_turn_mana(json_as(json!({ "amount": -param(&*ctx, "loss") }))));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: secure_system("any"),
        radiant: secure_system("enemy"),
    }
}

// S #118 Secure System — SPEC §8.10 row S 118, BUILD M12 row S 118 (#119, R1868001).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{degrade_number, filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SECURE: &str = "stinkyscape-118";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const GREASER: &str = "meditative-100"; // (2) Unit 7/7.
    const BOOK_OF_STATS: &str = "classicplus-057"; // (1) Spell: Give a Unit +5/+5.
    const FILLER: &str = "core-005";

    fn refresh_label(s: &Scenario) -> Option<String> {
        s.view(P1)
            .you
            .modifiers
            .iter()
            .find(|modifier| modifier.id == "nextTurnMana")
            .map(|modifier| modifier.label.clone())
    }

    fn secure_board(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": {
                "hand": [{ "def": SECURE, "radiant": radiant }, BOOK_OF_STATS, FILLER],
                "field": mine,
                "mana": 10,
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "field": theirs, "library": filler(4) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r1868001_every_unit_shows_0_and_a_buff_adds_from_0() {
            let mut s = secure_board(
                false,
                json!([GREASER]),
                json!([{ "def": GREASER, "statsOverride": { "attack": 7, "health": 3 } }]),
            );
            let mine = s.unit_at(P1, 1);
            let theirs = s.unit_at(P2, 1);

            s.play(SECURE, json!({}));

            s.expect_stats(&mine, json!({ "attack": 0, "health": 7 }));
            s.expect_stats(&theirs, json!({ "attack": 0, "health": 3 }));
            s.play(BOOK_OF_STATS, json!({ "targets": pick::instance(&mine) }));
            s.expect_stats(&mine, json!({ "attack": 5, "health": 12 }));
        }

        #[test]
        fn r1868001_a_unit_nerfed_below_0_stands_at_0() {
            let mut s = secure_board(false, json!([VANILLA]), json!([]));
            let nerfed = s.unit_at(P1, 1);
            s.card_mut(&nerfed).buffs.attack = -5;
            assert_eq!(unclamped_attack(s.state(), s.card(&nerfed)), -1);

            s.play(SECURE, json!({}));

            assert_eq!(unclamped_attack(s.state(), s.card(&nerfed)), 0);
            s.play(BOOK_OF_STATS, json!({ "targets": pick::instance(&nerfed) }));
            s.expect_stats(&nerfed, json!({ "attack": 5 }));
        }

        #[test]
        fn a_unit_immune_to_spells_keeps_its_attack() {
            let mut s = secure_board(false, json!([]), json!([VANILLA]));
            let immune = s.unit_at(P2, 1);
            s.card_mut(&immune).granted_keywords.push(Keyword::ImmuneToSpells);

            s.play(SECURE, json!({}));

            s.expect_stats(&immune, json!({ "attack": 4 }));
        }

        #[test]
        fn r1848004_next_refresh_is_1_less() {
            let mut s = secure_board(false, json!([]), json!([VANILLA]));

            s.play(SECURE, json!({}));

            assert_eq!(refresh_label(&s).as_deref(), Some("Next refresh −1 mana"));
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().active, P1);
            let mana = &s.state().players[P1].mana;
            assert_eq!(mana.current, mana.max - 1);
        }

        #[test]
        fn loss_reads_through_param() {
            let mut s = secure_board(false, json!([]), json!([VANILLA]));
            assert_eq!(degrade_number(&mut s, SECURE, "loss"), 2);

            s.play(SECURE, json!({}));

            assert_eq!(refresh_label(&s).as_deref(), Some("Next refresh −2 mana"));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1868001_only_enemy_units() {
            let mut s = secure_board(true, json!([GREASER]), json!([GREASER, VANILLA]));
            let mine = s.unit_at(P1, 1);

            s.play(SECURE, json!({}));

            s.expect_stats(&mine, json!({ "attack": 7 }));
            for lane in 1..=2 {
                s.expect_stats(s.unit_at(P2, lane), json!({ "attack": 0 }));
            }
            assert_eq!(refresh_label(&s).as_deref(), Some("Next refresh −1 mana"));
        }
    }
}
