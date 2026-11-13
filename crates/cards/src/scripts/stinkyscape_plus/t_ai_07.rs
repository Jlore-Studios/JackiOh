//! SPEC §8.12 S+ #T-AI-7 KY's Line Search Bot, a card of the Stinkyscape+ set (R1420, R3535000). Built by S25 (#3571).
//!   Base:    "Cry: Choose an enemy tile. Deal 2 damage to every Unit on the line between this and it."
//!   Radiant: "Cry: Choose an enemy tile. Deal 4 damage to every Unit on the line between this and it."
//! Engine: a (2) Unit 2/2 → 4/4, KY, AI, Token (no printed rarity, R482). The Cry is E7's line (R3554005): a
//!   `mode` prompt over every enemy tile, which the AI, the random policy and the timeout answer, then the
//!   line from this to the tile hits every Unit on it, friend or foe, for 2 (Radiant 4). R3571000: its text
//!   prints no Pierce, no Cleave and no laser, so Armor reduces its hits. Declares no numbers (R482: its
//!   numbers are the script's).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-t-ai-07";

/// "Deal 2 damage".
const LINE_DAMAGE: i32 = 2;

/// "Deal 4 damage".
const RADIANT_LINE_DAMAGE: i32 = 4;

/// R3571000: any enemy tile, no Cleave, no laser and no Pierce (its text prints none).
const fn rules(amount: i32) -> LineRules {
    LineRules {
        amount,
        offers: None,
        cleave: false,
        laser: false,
        pierce: false,
    }
}

fn face(line: LineRules) -> Script {
    Script {
        cry: Some(hook(move |_ctx| line_damage_begin(line))),
        resume: IndexMap::from([(LINE_STEP, hook(move |ctx| line_damage_answered(line, ctx)))]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(rules(LINE_DAMAGE)),
        radiant: face(rules(RADIANT_LINE_DAMAGE)),
    }
}

// S+ #T-AI-7 KY's Line Search Bot — SPEC §8.12 row T-AI-7 (R3554005, R3571000).
#[cfg(test)]
mod tests {
    use super::{ID, LINE_DAMAGE, RADIANT_LINE_DAMAGE};
    use crate::weapon_board;
    use jackioh_engine::subsystems::ai_policy::policy_actions;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A Unit Armor 1 takes one less from.
    const ARMORED: &str = "stand-in (1) 4/60 Armor 1";
    const PLAIN: &str = "stand-in (1) 9/9";

    fn damage(radiant: bool) -> i32 {
        if radiant { RADIANT_LINE_DAMAGE } else { LINE_DAMAGE }
    }

    /// The bot played into lane 1 (its Cry opens the tile prompt), against `p2`'s board.
    fn bot_in(radiant: bool, p2: Value) -> Scenario {
        let mut s = weapon_board(
            "s-t-ai-07",
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 }),
            p2,
        );
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    /// The open tile prompt's offers, one per enemy tile.
    fn tile_offers(s: &Scenario) -> usize {
        legal_actions(s.state(), P1)
            .iter()
            .filter(|action| matches!(action, ActionBody::Answer { .. }))
            .count()
    }

    /// Answer the open tile with the cell (x, y) from the caster's seat, forcing it into the offer.
    fn pick_tile(s: &mut Scenario, x: i32, y: i32) {
        let wanted = format!("{x},{y}");
        let state = s.state_mut();
        let pending = state.pending.as_mut().expect("a tile prompt");
        if !pending.options.iter().any(|option| option.key == wanted) {
            pending.options[0] = PromptOption {
                key: wanted.clone(),
                label: format!("Lane {}, forced", x + 1),
                selection: Selection::Mode {
                    option: wanted.clone(),
                },
                cost: None,
                radiant: None,
            };
        }
        s.answer(json!([wanted]));
    }

    #[test]
    fn r3571000_the_line_hits_and_armor_reduces_it() {
        for radiant in [false, true] {
            let mut s = bot_in(radiant, json!({ "field": [{ "def": ARMORED, "lane": 3 }] }));
            // Straight across at the enemy Units at lane 3: no Pierce, so Armor 1 takes one off.
            pick_tile(&mut s, 2, 2);
            let armored = s.unit(P2, 3).map(|unit| unit.id).expect("the armored Unit");
            assert_eq!(
                s.stats(armored.as_str()).health,
                60 - (damage(radiant) - 1),
                "radiant {radiant}"
            );
        }
    }

    #[test]
    fn the_line_hits_a_plain_unit_for_its_damage() {
        for radiant in [false, true] {
            let mut s = bot_in(radiant, json!({ "field": [{ "def": PLAIN, "lane": 1 }] }));
            pick_tile(&mut s, 0, 2);
            let plain = s.unit_at(P2, 1).id;
            assert_eq!(
                s.stats(plain.as_str()).health,
                9 - damage(radiant),
                "radiant {radiant}"
            );
        }
    }

    #[test]
    fn neither_face_cleaves_to_a_neighbour_off_the_line() {
        for radiant in [false, true] {
            let mut s = bot_in(radiant, json!({ "field": [PLAIN, PLAIN] }));
            pick_tile(&mut s, 0, 2);
            let on_the_line = s.unit_at(P2, 1).id;
            let beside = s.unit_at(P2, 2).id;
            assert_eq!(
                s.stats(on_the_line.as_str()).health,
                9 - damage(radiant),
                "radiant {radiant}"
            );
            assert_eq!(s.stats(beside.as_str()).health, 9, "radiant {radiant}");
        }
    }

    #[test]
    fn every_enemy_tile_is_offered_and_the_prompt_is_answered() {
        for radiant in [false, true] {
            let s = bot_in(radiant, json!({ "field": [PLAIN] }));
            // Every enemy tile: 5 lanes times the Units and the backrow rows.
            assert_eq!(tile_offers(&s), 10, "radiant {radiant}");
            assert!(
                policy_actions(s.state(), P1, Default::default())
                    .iter()
                    .any(|action| matches!(action, ActionBody::Answer { .. })),
                "radiant {radiant}: the AI policy answers"
            );
            let timed_out = reduce(
                s.state(),
                &Action::new(ActionBody::Timeout, P1, "s-t-ai-07-timeout"),
            );
            assert_eq!(timed_out.error, None, "radiant {radiant}");
            assert!(
                timed_out.state.pending.is_none(),
                "radiant {radiant}: the timeout answers"
            );
        }
    }
}
