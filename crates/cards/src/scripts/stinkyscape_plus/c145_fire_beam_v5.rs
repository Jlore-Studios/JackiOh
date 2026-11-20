//! SPEC §8.12 S+ #145 Fire Beam v5 (Unit 0/4 → 0/8, Weapon, cost 4, Legendary), a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Starship Piece
//!            Activate: Burn {burn} an enemy Unit, Cleave" — burn 6
//!   Radiant: "Starship Piece
//!            Activate 2: Burn {burn} an enemy Unit, Cleave" — burn 6
//!
//! Starship Piece is a catalog keyword the engine plays, and Equip was one through v0.6.7k (R4285001): Equipped onto one of your Units it fuses
//! into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one (R1852001).
//! The Activate is once a turn on the base face and twice on the Radiant one (R384, R1857003). R3544000:
//! its target is a declared enemy Unit, never a hero (R81, R1862000); the verb adds its Burn and deals no
//! damage (R3540000). R3544001: Cleave puts the same Burn on each Unit beside the chosen one, read before
//! the first Burn. The number is the declared `burn` (R386), read through `param`. Through v0.6.5b both
//! faces were "Activate 2" (burn 4, Radiant 8); `script_v0_6_5b` keeps the base face's two uses for those
//! replays (R767000).

use jackioh_engine::effects::{AfflictArgs, BoardScope, TargetSpec, adjacent_to_aimed, burn};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-145";

/// "Activate": 1 use a turn on the base face (R384, R1857003).
const USES: i32 = 1;
/// "Activate 2": 2 uses a turn on the Radiant face, and on both faces through v0.6.5b.
const RADIANT_USES: i32 = 2;

/// R3544001: the chosen enemy Unit's Burn, then the same Burn on each Unit beside it, read before the
/// first.
fn shot(ctx: &EffectContext<'_>) -> Vec<Effect> {
    let amount = param(ctx, "burn");
    let one = |to: TargetSpec| burn(AfflictArgs { to, amount });
    let chosen = TargetSpec::Chosen { index: None };
    let beside = adjacent_to_aimed(ctx, &chosen, &BoardScope::default(), TargetAim::Harm);
    let mut out = vec![one(chosen)];
    out.extend(
        beside
            .into_iter()
            .map(|unit| one(TargetSpec::Instance { instance_id: unit.id })),
    );
    out
}

/// The Activate, `uses` a turn.
fn beam(uses: i32) -> ActivationDecl {
    ActivationDecl {
        id: "burn".into(),
        label: "Burn an enemy Unit, and each Unit beside it".into(),
        uses: ActivationUses::Count(uses),
        cost: None,
        // "an enemy Unit": enemy Units only, never a hero.
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"] }))],
        modes: vec![],
        can_activate: Some(condition_hook(|c| {
            // R430: "Burn N an enemy unit" needs an enemy Unit to name.
            !active_units_of(c.state, opponent_of(c.controller)).is_empty()
        })),
        has: None,
        run: hook(|ctx| shot(ctx)),
    }
}

fn face(uses: i32) -> Script {
    Script {
        activations: vec![beam(uses)],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(USES),
        radiant: face(RADIANT_USES),
    }
}

/// R767000: through v0.6.5b, "Activate 2" on both faces; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_5b() -> CardScripts {
    CardScripts {
        base: face(RADIANT_USES),
        radiant: face(RADIANT_USES),
    }
}

// S+ #145 Fire Beam v5 — SPEC §8.12 row 145 (R3544000, R3544001; R81, R384, R767000, R1086, R1751000, R3540000).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hits, next_turn_of, upgrade_number, weapon_board};
    use jackioh_engine::query::{burn_on, hero_burn_of};
    use jackioh_engine::subsystems::activate::activate_actions_for;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A durable enemy: it survives any Burn's tick.
    const TOUGH: &str = "stand-in (1) 2/30";
    /// An equipped host.
    const HOST: &str = "stand-in (1) 4/4 R12/12";

    /// The printed Burn, and the Radiant face's, and its Radiant health.
    const BURN: i32 = 6;
    const RADIANT_BURN: i32 = 6;
    const RADIANT_HEALTH: i32 = 8;

    fn at(target: &str) -> Value {
        json!({ "ability": "burn", "targets": pick::id(target) })
    }

    fn at_hero() -> Value {
        json!({ "ability": "burn", "targets": [{ "pick": "hero", "player": "p2" }] })
    }

    fn beam_on(radiant: bool, p2: Value) -> Scenario {
        weapon_board(
            "s145",
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            p2,
        )
    }

    /// The Activate's offers: one per enemy Unit it may name.
    fn offers(s: &Scenario) -> usize {
        activate_actions_for(s.state(), P1, s.card(ID)).len()
    }

    fn burn_of(s: &Scenario, id: &str) -> i32 {
        burn_on(s.card(id))
    }

    /// The Burn gains of a step, as (target, placed), in no fixed order.
    fn placed(events: &[GameEvent]) -> Vec<(String, i32)> {
        let mut out: Vec<(String, i32)> = events
            .iter()
            .filter_map(|event| match event {
                GameEvent::CounterChanged {
                    instance_id,
                    counter: CounterKind::Burn,
                    placed: Some(n),
                    ..
                } => Some((instance_id.clone(), *n)),
                _ => None,
            })
            .collect();
        out.sort();
        out
    }

    /// R3544001: the chosen Unit and each Unit beside it gain N Burn, no other Unit, with no damage.
    fn cleaves(radiant: bool, amount: i32) {
        let mut s = beam_on(radiant, json!({ "field": [TOUGH, TOUGH, TOUGH, TOUGH] }));
        let lanes: Vec<String> = [1, 2, 3, 4].map(|lane| s.unit_at(P2, lane).id).to_vec();
        s.activate(ID, at(&lanes[1]));
        assert_eq!(burn_of(&s, &lanes[0]), amount, "radiant {radiant}");
        assert_eq!(burn_of(&s, &lanes[1]), amount, "radiant {radiant}");
        assert_eq!(burn_of(&s, &lanes[2]), amount, "radiant {radiant}");
        assert_eq!(burn_of(&s, &lanes[3]), 0, "radiant {radiant}");
        // A verb, no damage: three gains of N, and none on the enemy hero.
        assert!(hits(s.last_events()).is_empty(), "radiant {radiant}");
        let mut want = vec![
            (lanes[0].clone(), amount),
            (lanes[1].clone(), amount),
            (lanes[2].clone(), amount),
        ];
        want.sort();
        assert_eq!(placed(s.last_events()), want, "radiant {radiant}");
        assert_eq!(hero_burn_of(s.state(), P2), 0, "radiant {radiant}");
    }

    /// R384: `uses` a turn, each adding its Burn to the last (R3540000), then refused until its user's
    /// next turn.
    fn uses_a_turn(radiant: bool, uses: i32, amount: i32) {
        let mut s = beam_on(radiant, json!({ "field": [TOUGH] }));
        let target = s.unit_at(P2, 1).id;
        for _ in 0..uses {
            assert_eq!(offers(&s), 1, "radiant {radiant}");
            s.activate(ID, at(&target));
        }
        assert_eq!(burn_of(&s, &target), uses * amount, "radiant {radiant}");
        assert_eq!(offers(&s), 0, "radiant {radiant}");
        s.expect_refused(|s| s.activate(ID, at(&target)));
        next_turn_of(&mut s, P1);
        assert_eq!(offers(&s), 1, "radiant {radiant}");
    }

    mod base {
        use super::*;

        #[test]
        fn r3544001_burns_the_chosen_unit_and_each_unit_beside_it_and_no_other() {
            cleaves(false, BURN);
        }

        #[test]
        fn r3544000_its_target_is_an_enemy_unit_never_your_own_or_the_hero() {
            let mut s = weapon_board(
                "s145-target",
                json!({ "field": [ID, HOST] }),
                json!({ "field": [TOUGH, TOUGH] }),
            );
            let mine = s.unit_at(P1, 2).id;
            s.expect_refused(|s| s.activate(ID, at(&mine)));
            s.expect_refused(|s| s.activate(ID, at_hero()));
            // One offer for each enemy Unit it may name.
            assert_eq!(offers(&s), 2);
        }

        #[test]
        fn r3544000_it_is_not_offered_while_the_enemy_has_no_unit() {
            let mut s = beam_on(false, json!({}));
            assert_eq!(offers(&s), 0);
            s.expect_refused(|s| s.activate(ID, at_hero()));
            assert!(placed(s.events()).is_empty());
            s.expect_health(P2, HERO_HEALTH);
        }

        #[test]
        fn r384_once_a_turn_and_back_the_next() {
            uses_a_turn(false, USES, BURN);
        }

        #[test]
        fn r767000_the_script_kept_for_v0_6_5b_is_used_twice_a_turn() {
            crate::under_kept(ID, script_v0_6_5b(), || uses_a_turn(false, RADIANT_USES, BURN));
        }

        #[test]
        fn r1086_r1751000_an_equipped_host_takes_the_activate() {
            let mut s = weapon_board(
                "s145-equip",
                json!({ "hand": [ID], "field": [HOST], "mana": 10 }),
                json!({ "field": [TOUGH, TOUGH] }),
            );
            let host = s.unit_at(P1, 1).id;
            let target = s.unit_at(P2, 1).id;
            let beside = s.unit_at(P2, 2).id;
            crate::give_equip(&mut s, ID);
            s.play(ID, json!({ "zone": 1, "equip": true }));
            s.activate(host.as_str(), at(&target));
            assert_eq!(burn_of(&s, &target), BURN);
            assert_eq!(burn_of(&s, &beside), BURN);
            assert!(hits(s.last_events()).is_empty());
        }

        #[test]
        fn burn_reads_through_param() {
            let mut s = beam_on(false, json!({ "field": [TOUGH] }));
            let amount = upgrade_number(&mut s, ID, "burn");
            let target = s.unit_at(P2, 1).id;
            s.activate(ID, at(&target));
            assert_eq!(burn_of(&s, &target), amount);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3544001_burns_the_chosen_unit_and_each_unit_beside_it_too() {
            cleaves(true, RADIANT_BURN);
        }

        #[test]
        fn r384_twice_a_turn_and_back_the_next() {
            uses_a_turn(true, RADIANT_USES, RADIANT_BURN);
        }

        #[test]
        fn r3544000_the_hero_is_refused_here_too() {
            let mut s = beam_on(true, json!({ "field": [TOUGH] }));
            assert_eq!(offers(&s), 1);
            s.expect_refused(|s| s.activate(ID, at_hero()));
            assert_eq!(hero_burn_of(s.state(), P2), 0);
        }

        #[test]
        fn its_health_is_doubled() {
            let s = beam_on(true, json!({}));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (0, RADIANT_HEALTH));
        }
    }
}
