//! SPEC §8.12 S+ #126 Vela, a card of the Stinkyscape+ set (R1420, R3535000). Built by S13 (#3560).
//! 0/10
//! **Can’t attack, Can’t defend**
//! **Tribute 2**
//! **Activate 1: Choose** an enemy tile deal 8 **Pierce** deal damage to all Units between that row and this
//! \~\~\~
//! 0/20
//! **Can’t attack, Can’t defend**
//! **Tribute 2**
//! **Activate 1: Choose** an enemy tile deal 8 **Cleave Pierce** deal damage to all Units between that row and this
//!
//! Tribute 2 is the script's `tribute` (R101). R3554005: the Activate takes any enemy tile and the
//! line between this and it hits every Unit on it — friend or foe — piercing Armor; the Radiant face
//! Cleaves each neighbour not yet hit.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-126";

/// "Tribute 2" (R101).
const TRIBUTE: i32 = 2;

/// "Activate 1": one use a turn (R384).
const USES: i32 = 1;

/// Vela draws no laser.
const LASER: bool = false;

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

// S+ #126 Vela — SPEC §8.12 row 126.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hits, next_turn_of, weapon_board};
    use jackioh_engine::subsystems::ai_policy::policy_actions;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const ARMORED: &str = "stand-in (0) 40/60 Armor 5";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    fn vela_on(radiant: bool, lane: i32, p2: Value) -> Scenario {
        weapon_board(
            "s126",
            json!({ "field": [{ "def": ID, "radiant": radiant, "lane": lane }] }),
            p2,
        )
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
    fn r3554005_every_enemy_tile_is_offered_and_the_line_hits_with_pierce() {
        for radiant in [false, true] {
            let mut s = vela_on(radiant, 1, json!({ "field": [{ "def": ARMORED, "lane": 3 }] }));
            s.activate(ID, json!({ "ability": "line" }));
            // Every enemy tile: 5 lanes times the Units and the backrow rows.
            assert_eq!(tile_offers(&s), 10, "radiant {radiant}");
            // Straight ahead at the enemy Units at lane 3: Pierce goes through Armor.
            pick_tile(&mut s, 2, 2);
            let hits = hits(s.last_events());
            assert!(!hits.is_empty(), "radiant {radiant}");
            // An armored 60-health Unit took the whole 8.
            let armored = s.unit(P2, 3).map(|unit| unit.id).expect("the armored Unit");
            assert_eq!(s.stats(armored.as_str()).health, 60 - 8, "radiant {radiant}");
            assert!(
                hits.iter().any(|(id, amount)| id == &armored && *amount == 8),
                "radiant {radiant}"
            );
        }
    }

    #[test]
    fn r3554005_radiant_cleaves() {
        let mut s = vela_on(true, 1, json!({ "field": [VANILLA, VANILLA, VANILLA] }));
        // The ids are read before the line: its hits kill the 4/4 Units, which then leave the field.
        let first = s.unit_at(P2, 1).id;
        let second = s.unit_at(P2, 2).id;
        s.activate(ID, json!({ "ability": "line" }));
        // At the enemy Units at lane 1: the line hits it, the Cleave its lane-2 neighbour once.
        pick_tile(&mut s, 0, 2);
        let dealt: Vec<(String, i32)> = hits(s.last_events());
        assert!(dealt.contains(&(first.clone(), 8)));
        assert!(dealt.contains(&(second.clone(), 8)), "the Cleave hit");
    }

    #[test]
    fn the_tile_prompt_is_answered_by_the_policy_and_the_timeout() {
        let mut s = vela_on(false, 1, json!({ "field": [VANILLA] }));
        s.activate(ID, json!({ "ability": "line" }));
        assert!(
            policy_actions(s.state(), P1, Default::default())
                .iter()
                .any(|action| matches!(action, ActionBody::Answer { .. })),
            "the AI policy answers"
        );
        let timed_out = reduce(s.state(), &Action::new(ActionBody::Timeout, P1, "s126-t"));
        assert_eq!(timed_out.error, None);
        assert!(timed_out.state.pending.is_none());
    }

    #[test]
    fn once_a_turn_then_back() {
        let mut s = vela_on(false, 1, json!({ "field": [VANILLA] }));
        s.activate(ID, json!({ "ability": "line" }));
        pick_tile(&mut s, 0, 2);
        s.expect_refused(|s| s.activate(ID, json!({ "ability": "line" })));
        next_turn_of(&mut s, P1);
        s.activate(ID, json!({ "ability": "line" }));
        pick_tile(&mut s, 0, 2);
    }
}
