//! FvH #41 Icecream Vender (SPEC §8.14 row 41): (2) Unit, Human, Rare, 2/8 → 4/16.
//!   Base:    "Durability 2\nActivate: Choose one: Vanilla a Unit, or Freeze a Unit."
//!   Radiant: "Durability 4\nActivate 2: Choose one: Vanilla a Unit, or Freeze a Unit."
//!
//! Engine: Activate, once a turn on the base face and twice on the Radiant (R384), its mode and its
//! target, any Unit on either side, declared in the action (R81): `vanilla` of the chosen Unit, or
//! `freeze` of it (R1174000). Durability 2 (Radiant 4) is the engine's keyword, one counter both a turn's
//! uses share (R5236000); at 0 the Activate is refused and the Unit lives on with none it may use
//! (R5236001); Refresh restores none (R5236003). Its Durability is a keyword count (R5236005); it
//! declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-041";

const VANILLA_MODE: &str = "Vanilla a Unit";
const FREEZE_MODE: &str = "Freeze a Unit";
/// "Activate": once a turn (R384).
const USES: i32 = 1;
/// "Activate 2": twice a turn (R384).
const RADIANT_USES: i32 = 2;

/// "Choose one: Vanilla a Unit, or Freeze a Unit", the mode and the Unit both declared (R81).
fn scoop(ctx: &EffectContext<'_>) -> Vec<Effect> {
    let chosen = json!({ "target": { "of": "chosen" } });
    match ctx.modes.first().map(String::as_str) {
        Some(VANILLA_MODE) => vec![vanilla(json_as(chosen))],
        Some(FREEZE_MODE) => vec![freeze(json_as(chosen))],
        _ => vec![],
    }
}

fn face(uses: i32) -> Script {
    let ability = ActivationDecl {
        id: "scoop".to_string(),
        label: "Choose one: Vanilla a Unit, or Freeze a Unit".to_string(),
        uses: ActivationUses::Count(uses),
        cost: None,
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))],
        modes: vec![ModeDecl {
            kind: PromptKind::Mode,
            options: vec![VANILLA_MODE.to_string(), FREEZE_MODE.to_string()],
        }],
        can_activate: None,
        has: None,
        run: hook(|ctx| scoop(&*ctx)),
    };
    Script {
        activations: vec![ability],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(USES),
        radiant: face(RADIANT_USES),
    }
}

// FvH #41 Icecream Vender — SPEC §8.14 row 41 (R384, R81, R1174000, R5236000, R5236001, R5236003).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, run_as, scenario};

    const P1: PlayerId = PlayerId::P1;

    /// A textless Spell each seat can always play, so no turn ends by itself (§2.5).
    const SPELL: &str = "stand-in (1) Spell";
    const GUARD: &str = "stand-in (1) 3/3 Taunt";
    const FRIEND: &str = "stand-in (2) 2/2 Rush";
    /// Its printed Durability, base and Radiant.
    const DURABILITY: i32 = 2;
    const RADIANT_DURABILITY: i32 = 4;

    fn vender(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [SPELL],
                "field": [{ "def": ID, "radiant": radiant }, FRIEND],
                "library": filler(6),
            },
            "p2": { "hand": [SPELL], "field": [GUARD], "library": filler(6) },
        }))
    }

    fn scoop_on<'a>(s: &'a mut Scenario, mode: &str, unit: &str) -> &'a mut Scenario {
        let target = s.card(unit).clone();
        s.activate(ID, json!({ "modes": [mode], "targets": pick::instance(&target) }))
    }

    fn kinds(s: &Scenario, unit: &str) -> Vec<KeywordKind> {
        s.stats(unit).keywords.iter().map(|keyword| keyword.kind()).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r81_vanilla_mode_vanillas_the_chosen_enemy_unit() {
            let mut s = vender(false);
            scoop_on(&mut s, VANILLA_MODE, GUARD);
            assert!(s.card(GUARD).vanilla);
            assert!(!kinds(&s, GUARD).contains(&KeywordKind::Taunt));
            assert_eq!(s.card(GUARD).frozen, None);
        }

        #[test]
        fn r81_either_side_vanilla_mode_reaches_your_own_unit_too() {
            let mut s = vender(false);
            scoop_on(&mut s, VANILLA_MODE, FRIEND);
            assert!(s.card(FRIEND).vanilla);
            assert!(!kinds(&s, FRIEND).contains(&KeywordKind::Rush));
        }

        #[test]
        fn r1174000_freeze_mode_freezes_the_chosen_unit_and_leaves_its_text() {
            let mut s = vender(false);
            scoop_on(&mut s, FREEZE_MODE, GUARD);
            assert!(s.card(GUARD).frozen.is_some());
            assert!(!s.card(GUARD).vanilla);
            s.end_turn();
            assert!(s.attacks_of(GUARD).is_empty(), "a Frozen Unit declares no attack");
        }

        #[test]
        fn r81_an_activate_with_no_unit_chosen_is_refused() {
            let mut s = vender(false);
            s.expect_refused(|s| s.activate(ID, json!({ "modes": [FREEZE_MODE] })));
            assert_eq!(durability_left(s.state(), s.card(ID)), Some(DURABILITY));
        }

        #[test]
        fn r5236001_two_uses_spend_its_durability_2_then_the_activate_is_refused_and_it_lives_on() {
            let mut s = vender(false);
            scoop_on(&mut s, FREEZE_MODE, GUARD);
            s.expect_refused(|s| scoop_on(s, FREEZE_MODE, GUARD));
            run_as(&mut s, refresh_activates(), P1);
            scoop_on(&mut s, FREEZE_MODE, GUARD);
            assert_eq!(durability_left(s.state(), s.card(ID)), Some(0));
            run_as(&mut s, refresh_activates(), P1);
            s.expect_refused_with(|s| scoop_on(s, FREEZE_MODE, GUARD), "Durability");
            s.expect_in_zone(ID, "field");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r384_activate_2_serves_twice_a_turn_a_third_use_is_refused() {
            let mut s = vender(true);
            scoop_on(&mut s, VANILLA_MODE, GUARD);
            scoop_on(&mut s, FREEZE_MODE, FRIEND);
            assert!(s.card(GUARD).vanilla);
            assert!(s.card(FRIEND).frozen.is_some());
            s.expect_refused(|s| scoop_on(s, FREEZE_MODE, GUARD));
            assert_eq!(durability_left(s.state(), s.card(ID)), Some(RADIANT_DURABILITY - 2));
        }

        #[test]
        fn r5236001_r5236003_four_uses_spend_its_durability_4_and_a_refresh_gives_none_back() {
            let mut s = vender(true);
            for _ in 0..RADIANT_USES {
                scoop_on(&mut s, FREEZE_MODE, GUARD);
            }
            run_as(&mut s, refresh_activates(), P1);
            assert_eq!(durability_left(s.state(), s.card(ID)), Some(RADIANT_DURABILITY - RADIANT_USES));
            for _ in 0..RADIANT_USES {
                scoop_on(&mut s, FREEZE_MODE, GUARD);
            }
            run_as(&mut s, refresh_activates(), P1);
            s.expect_refused_with(|s| scoop_on(s, FREEZE_MODE, GUARD), "Durability");
            s.expect_in_zone(ID, "field");
        }
    }
}
