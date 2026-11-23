//! SPEC §8.12 S+ #127 Corvus, a card of the Stinkyscape+ set (R1420, R3535000). Built by S13 (#3560).
//! 0/20
//! **Can’t attack, Can’t defend**
//! **Tribute 3**
//! **Activate 1: Choose** an enemy tile deal 80 **Pierce** deal damage to all Units between that row and this
//! \~\~\~
//! 0/40
//! **Can’t attack, Can’t defend**
//! **Tribute 3**
//! **Activate 1: Choose** an enemy tile deal 80 **Cleave Pierce** deal damage to all Units between that row and this
//!
//! Tribute 3 is the script's `tribute` (R101). R3554005: the Activate takes any enemy tile and the
//! line between this and it hits every Unit on it — friend or foe — piercing Armor, drawn as a laser;
//! the Radiant face Cleaves each neighbour not yet hit.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-127";

/// "Tribute 3" (R101).
const TRIBUTE: i32 = 3;

/// "Activate 1": one use a turn (R384).
const USES: i32 = 1;

/// Corvus draws the line as a laser.
const LASER: bool = true;

/// "Pierce": its hits ignore Armor (R3554005).
const PIERCE: bool = true;

/// R3554005: the line's rules now.
fn rules(ctx: &EffectContext<'_>) -> LineRules {
    LineRules {
        amount: param(ctx, "damage"),
        offers: None,
        cleave: ctx.radiant,
        laser: LASER,
        pierce: PIERCE,
    }
}

fn face() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE),
            ..StaticFlags::default()
        }),
        activations: vec![ActivationDecl {
            id: "line".into(),
            label: "Choose an enemy tile: deal damage along the line".into(),
            uses: ActivationUses::Count(USES),
            cost: None,
            targets: vec![],
            modes: vec![],
            can_activate: None,
            has: None,
            run: hook(|ctx| line_damage_begin(rules(ctx))),
        }],
        resume: IndexMap::from([(LINE_STEP, hook(|ctx| line_damage_answered(rules(ctx), ctx)))]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(),
        radiant: face(),
    }
}

// S+ #127 Corvus — SPEC §8.12 row 127.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hits, next_turn_of, weapon_board};
    use jackioh_engine::subsystems::ai_policy::policy_actions;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const ARMORED: &str = "stand-in (0) 40/90 Armor 5";

    fn corvus_on(radiant: bool, lane: i32, p2: Value) -> Scenario {
        weapon_board(
            "s127",
            json!({ "field": [{ "def": ID, "radiant": radiant, "lane": lane }] }),
            p2,
        )
    }

    fn tile_offers(s: &Scenario) -> usize {
        legal_actions(s.state(), P1)
            .iter()
            .filter(|action| matches!(action, ActionBody::Answer { .. }))
            .count()
    }

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
    fn r3554005_every_enemy_tile_is_offered_and_the_line_hits_with_pierce() {
        for radiant in [false, true] {
            let mut s = corvus_on(radiant, 1, json!({ "field": [{ "def": ARMORED, "lane": 3 }] }));
            s.activate(ID, json!({ "ability": "line" }));
            assert_eq!(tile_offers(&s), 10, "radiant {radiant}");
            pick_tile(&mut s, 2, 2);
            let armored = s.unit(P2, 3).map(|unit| unit.id).expect("the armored Unit");
            assert_eq!(
                s.stats(armored.as_str()).health,
                90 - 80,
                "radiant {radiant}: Pierce"
            );
            assert!(
                hits(s.last_events())
                    .iter()
                    .any(|(id, amount)| id == &armored && *amount == 80),
                "radiant {radiant}"
            );
        }
    }

    #[test]
    fn r3554005_radiant_cleaves() {
        let mut s = corvus_on(true, 1, json!({ "field": [VANILLA, VANILLA, VANILLA] }));
        // The ids are read before the line: its hits kill the 4/4 Units, which then leave the field.
        let first = s.unit_at(P2, 1).id;
        let second = s.unit_at(P2, 2).id;
        s.activate(ID, json!({ "ability": "line" }));
        pick_tile(&mut s, 0, 2);
        let dealt = hits(s.last_events());
        assert!(dealt.contains(&(first, 80)));
        assert!(dealt.contains(&(second, 80)), "the Cleave hit");
    }

    #[test]
    fn r3554005_corvus_draws_a_laser() {
        for radiant in [false, true] {
            let mut s = corvus_on(radiant, 1, json!({ "field": [VANILLA] }));
            s.activate(ID, json!({ "ability": "line" }));
            pick_tile(&mut s, 0, 2);
            assert!(
                s.last_events().iter().any(|event| matches!(
                    event,
                    GameEvent::LineDrawn {
                        laser: Some(true),
                        ..
                    }
                )),
                "radiant {radiant}: a laser"
            );
        }
    }

    #[test]
    fn the_tile_prompt_is_answered_by_the_policy_and_the_timeout() {
        let mut s = corvus_on(false, 1, json!({ "field": [VANILLA] }));
        s.activate(ID, json!({ "ability": "line" }));
        assert!(
            policy_actions(s.state(), P1, Default::default())
                .iter()
                .any(|action| matches!(action, ActionBody::Answer { .. })),
            "the AI policy answers"
        );
        let timed_out = reduce(s.state(), &Action::new(ActionBody::Timeout, P1, "s127-t"));
        assert_eq!(timed_out.error, None);
        assert!(timed_out.state.pending.is_none());
    }

    #[test]
    fn once_a_turn_then_back() {
        let mut s = corvus_on(false, 1, json!({ "field": [VANILLA] }));
        s.activate(ID, json!({ "ability": "line" }));
        pick_tile(&mut s, 0, 2);
        s.expect_refused(|s| s.activate(ID, json!({ "ability": "line" })));
        next_turn_of(&mut s, P1);
        s.activate(ID, json!({ "ability": "line" }));
        pick_tile(&mut s, 0, 2);
    }
}
