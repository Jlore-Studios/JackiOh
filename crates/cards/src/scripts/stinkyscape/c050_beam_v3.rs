//! Stinkyscape #50 Beam v3 (SPEC §8.10 row 50). Unit 0/4 → 0/8, Weapon, cost 2, Rare.
//!   Base:    "Starship Piece, Equip
//!            Activate: Deal {damage} damage to an enemy Unit, with Cleave and Pierce. Has an extra turn
//!            cooldown." — damage 5
//!   Radiant: the same text — damage 10
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it
//! fuses into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one
//! (R1852001); played alone, its death fuses a copy into its owner's Dormant Starship (R1852000).
//! R1862000: the Activate's target is a declared enemy Unit (R81), never a hero (#2521). Cleave is the
//! same hit on each Unit beside the target, read before the first hit; Pierce is `ignoreArmor` (R346).
//! Once a turn (R384), with an extra turn cooldown (R1841000); the number is the declared `damage`
//! (R386), read through `param`. Through v0.5.11 the enemy hero was a target too, its hit halved
//! (R1841001), which `script_v0_5_11` keeps (R767000).

use jackioh_engine::effects::{
    BoardScope, DamageEffectArgs, DamageFlagArgs, TargetSpec, adjacent_to_aimed, damage,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-050";

/// R1862000: the chosen enemy Unit's hit, then the same hit on each Unit beside it, read before the
/// first.
fn shot(ctx: &EffectContext<'_>) -> Vec<Effect> {
    let amount = param(ctx, "damage");
    let hit = |to: TargetSpec| {
        damage(DamageEffectArgs {
            to,
            amount,
            flags: DamageFlagArgs {
                ignore_armor: Some(true),
                ..DamageFlagArgs::default()
            },
        })
    };
    let chosen = TargetSpec::Chosen { index: None };
    let beside = adjacent_to_aimed(ctx, &chosen, &BoardScope::default(), TargetAim::Harm);
    let mut hits = vec![hit(chosen)];
    hits.extend(
        beside
            .into_iter()
            .map(|unit| hit(TargetSpec::Instance { instance_id: unit.id })),
    );
    hits
}

fn beam() -> ActivationDecl {
    ActivationDecl {
        id: "beam".into(),
        label: "Deal damage to an enemy Unit, with Cleave and Pierce".into(),
        uses: ActivationUses::Count(1),
        cost: Some(ActivationCost {
            extra_turn_cooldown: Some(true),
            ..ActivationCost::default()
        }),
        // "an enemy Unit": enemy Units only, never a hero.
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"] }))],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(|ctx| shot(ctx)),
    }
}

pub fn script() -> CardScripts {
    let base = Script {
        activations: vec![beam()],
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

/// R767000: the chosen enemy's hit, then the same hit on each Unit beside it, read before the first,
/// as v0.5.11 ran it: the enemy hero a target too, its hit halved, rounded down (R1841001).
fn shot_v0_5_11(ctx: &EffectContext<'_>) -> Vec<Effect> {
    let amount = param(ctx, "damage");
    let hit = |to: TargetSpec| {
        damage(DamageEffectArgs {
            to,
            amount,
            flags: DamageFlagArgs {
                ignore_armor: Some(true),
                half_to_heroes: Some(true),
                ..DamageFlagArgs::default()
            },
        })
    };
    let chosen = TargetSpec::Chosen { index: None };
    let beside = adjacent_to_aimed(ctx, &chosen, &BoardScope::default(), TargetAim::Harm);
    let mut hits = vec![hit(chosen)];
    hits.extend(
        beside
            .into_iter()
            .map(|unit| hit(TargetSpec::Instance { instance_id: unit.id })),
    );
    hits
}

/// R767000: the Activate as v0.5.11 ran it, aimed at an enemy Unit or the enemy hero.
fn beam_v0_5_11() -> ActivationDecl {
    ActivationDecl {
        id: "beam".into(),
        label: "Deal damage to an enemy, with Cleave and Pierce".into(),
        uses: ActivationUses::Count(1),
        cost: Some(ActivationCost {
            extra_turn_cooldown: Some(true),
            ..ActivationCost::default()
        }),
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "enemy", "of": ["unit", "hero"] }),
        )],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(|ctx| shot_v0_5_11(ctx)),
    }
}

/// R767000: through v0.5.11 its Activate could aim at the enemy hero, for half; `KEPT_SCRIPTS` folds
/// those replays under it.
pub fn script_v0_5_11() -> CardScripts {
    let base = Script {
        activations: vec![beam_v0_5_11()],
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #50 Beam v3 — SPEC §8.10 row 50, BUILD M12 row S 50: its Activate deals 5 to a chosen
// enemy Unit and 5 to each Unit beside it, through Armor (Pierce); the enemy hero is never offered as
// its target, and aiming at it is refused; once a turn, a second use refused; refused on its user's next
// turn and usable the turn after, a bounce clearing it (R1841000, R78); equipped onto a Unit, the host
// takes the Activate; damage reads through `param()`; radiant 0/8, 10 damage.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hits, js, next_turn_of, weapon_board};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MENACE: &str = "core-019"; // (3) 9/9 Taunt.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FLOOD: &str = "core-017"; // (4) Spell: "Bounce all Units."

    /// The printed `damage`, and the Radiant face's, and its Radiant health.
    const DAMAGE: i32 = 5;
    const RADIANT_DAMAGE: i32 = 10;
    const RADIANT_HEALTH: i32 = 8;

    fn at(target: &str) -> Value {
        json!({ "ability": "beam", "targets": pick::id(target) })
    }

    fn at_hero() -> Value {
        json!({ "ability": "beam", "targets": [{ "pick": "hero", "player": "p2" }] })
    }

    fn beam_on(radiant: bool, p1: Value, p2: Value) -> Scenario {
        let mut p1 = p1;
        p1["field"] = json!([{ "def": ID, "radiant": radiant }]);
        weapon_board("s050", p1, p2)
    }

    /// The targets `legal_actions` offers p1 for the Beam's Activate, one per action.
    fn offered(s: &Scenario) -> Vec<Value> {
        let beam = s.card(ID).id.clone();
        legal_actions(s.state(), P1)
            .iter()
            .map(js)
            .filter(|action| action["type"] == "activate" && action["instanceId"] == beam.as_str())
            .map(|action| action["targets"].clone())
            .collect()
    }

    fn wall(radiant: bool) -> (Scenario, [String; 4]) {
        let mut s = beam_on(
            radiant,
            json!({}),
            json!({ "field": [MENACE, MENACE, MENACE, VANILLA] }),
        );
        let lanes = [1, 2, 3, 4].map(|lane| s.unit_at(P2, lane).id);
        s.card_mut(lanes[1].as_str())
            .granted_keywords
            .push(Keyword::Armor { n: 3 });
        (s, lanes)
    }

    fn cleaves(radiant: bool, amount: i32) {
        let (mut s, lanes) = wall(radiant);
        s.activate(ID, at(&lanes[1]));
        let expected: Vec<(String, i32)> = [&lanes[1], &lanes[0], &lanes[2]]
            .iter()
            .map(|id| ((*id).clone(), amount))
            .collect();
        // The Armored Menace's hit is whole: Armor would have taken 3 of it (R346).
        assert_eq!(hits(s.last_events()), expected, "radiant {radiant}");
        s.expect_stats(lanes[3].as_str(), json!({ "health": 4 }));
        s.expect_health(P2, HERO_HEALTH);
    }

    /// "an enemy Unit": the enemy Unit is the one target offered, the enemy hero never is, and aiming
    /// at the hero is refused, leaving it untouched.
    fn never_the_hero(radiant: bool) {
        let mut s = beam_on(radiant, json!({}), json!({ "field": [VANILLA] }));
        let unit = s.unit_at(P2, 1).id;
        assert_eq!(offered(&s), vec![pick::id(&unit)], "radiant {radiant}");
        s.expect_refused(|s| s.activate(ID, at_hero()));
        s.expect_health(P2, HERO_HEALTH);
    }

    mod base {
        use super::*;

        /// R767000: the script kept for the replays of v0.5.11 and before may aim at the enemy hero,
        /// whose hit is halved, rounded down, with no Cleave off it (R1841001).
        #[test]
        fn r767000_the_script_kept_for_v0_5_11_halves_its_hit_on_the_enemy_hero() {
            crate::under_kept(ID, script_v0_5_11(), || {
                let halved = DAMAGE / HALF_DAMAGE_TO_HEROES_DIVISOR;
                let mut s = beam_on(false, json!({}), json!({ "field": [MENACE, VANILLA] }));
                s.activate(ID, at_hero());
                assert_eq!(hits(s.last_events()), vec![("hero-p2".to_string(), halved)]);
                s.expect_health(P2, HERO_HEALTH - halved);
            });
        }

        #[test]
        fn r1862000_hits_the_chosen_unit_and_each_unit_beside_it_through_armor() {
            cleaves(false, DAMAGE);
        }

        #[test]
        fn r1862000_its_target_is_an_enemy_never_a_unit_of_yours() {
            let mut s = weapon_board("s050-own", json!({ "field": [ID, VANILLA] }), json!({}));
            let mine = s.unit_at(P1, 2).id;
            s.expect_refused(|s| s.activate(ID, at(&mine)));
        }

        #[test]
        fn r1862000_the_enemy_hero_is_never_offered_and_is_refused() {
            never_the_hero(false);
        }

        #[test]
        fn r1841000_once_a_turn_then_cooling_down_then_back() {
            // Menaces in lanes 1 and 3: neither is beside the other, so each shot hits its own alone.
            let row = json!({ "field": [{ "def": MENACE, "lane": 1 }, { "def": MENACE, "lane": 3 }] });
            let mut s = beam_on(false, json!({}), row);
            let [first, second] = [1, 3].map(|lane| s.unit_at(P2, lane).id);
            s.activate(ID, at(&first));
            s.expect_refused_with(|s| s.activate(ID, at(&second)), "already been used");
            next_turn_of(&mut s, P1);
            s.expect_refused_with(|s| s.activate(ID, at(&second)), "cooling down");
            next_turn_of(&mut s, P1);
            s.activate(ID, at(&second));
            assert_eq!(hits(s.last_events()), vec![(second, DAMAGE)]);
        }

        #[test]
        fn r1841000_r78_a_bounce_clears_the_cooldown() {
            let mut s = beam_on(
                false,
                json!({ "hand": [FLOOD], "mana": 4 }),
                json!({ "hand": [VANILLA], "field": [MENACE] }),
            );
            let first = s.unit_at(P2, 1).id;
            s.activate(ID, at(&first));
            s.play(FLOOD, json!({}));
            s.expect_in_zone(ID, "hand");
            // p2 puts a Unit back on the field for the replayed Beam to aim at.
            s.end_turn();
            s.play(VANILLA, json!({ "zone": 1 }));
            s.end_turn();
            s.play(ID, json!({ "zone": 1 }));
            let target = s.unit_at(P2, 1).id;
            s.activate(ID, at(&target));
            assert_eq!(hits(s.last_events()), vec![(target, DAMAGE)]);
        }

        #[test]
        fn r1086_r1852001_an_equipped_host_takes_the_activate_and_is_no_starship_piece() {
            let mut s = weapon_board(
                "s050-equip",
                json!({ "hand": [ID], "field": [VANILLA] }),
                json!({ "field": [VANILLA] }),
            );
            let host = s.unit_at(P1, 1).id;
            let target = s.unit_at(P2, 1).id;
            s.play(ID, json!({ "zone": 1, "equip": true }));
            let fused = s.card(host.as_str()).clone();
            assert!(!unit_has(s.state(), &fused, KeywordKind::StarshipPiece));
            s.activate(host.as_str(), at(&target));
            assert_eq!(hits(s.last_events()), vec![(target, DAMAGE)]);
        }

        #[test]
        fn damage_reads_through_param() {
            let mut s = beam_on(false, json!({}), json!({ "field": [VANILLA] }));
            let amount = crate::upgrade_number(&mut s, ID, "damage");
            let target = s.unit_at(P2, 1).id;
            s.activate(ID, at(&target));
            assert_eq!(hits(s.last_events()), vec![(target, amount)]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1862000_hits_the_chosen_unit_and_each_unit_beside_it_through_armor_twice_as_hard() {
            cleaves(true, RADIANT_DAMAGE);
        }

        #[test]
        fn r1862000_the_enemy_hero_is_never_offered_and_is_refused() {
            never_the_hero(true);
        }

        #[test]
        fn its_health_is_doubled() {
            let s = beam_on(true, json!({}), json!({}));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (0, RADIANT_HEALTH));
        }
    }
}
