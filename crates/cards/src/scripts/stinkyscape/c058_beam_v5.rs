//! Stinkyscape #58 Beam v5 (SPEC §8.10 row 58). Unit 0/8 → 0/16, Weapon, cost 4, Legendary; held (R1420).
//!   Base:    "Starship Piece, Equip
//!            Activate: Deal {damage} damage to an enemy, with Cleave and Pierce. Has an extra turn
//!            cooldown. Deals half damage to heroes." — damage 15
//!   Radiant: the same text — damage 30
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it
//! fuses into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one
//! (R1852001); played alone, its death fuses a copy into its owner's Dormant Starship (R1852000).
//! R1862000: the Activate's target is a declared enemy Unit or the enemy hero (R81); Cleave is the same
//! hit on each Unit beside a Unit target, read before the first hit, and Pierce is `ignoreArmor`
//! (R346). Once a turn (R384), with an extra turn cooldown (R1841000); a hit on a hero is halved,
//! rounded down (R1841001). The number is the declared `damage` (R386), read through `param`.

use jackioh_engine::effects::{
    BoardScope, DamageEffectArgs, DamageFlagArgs, TargetSpec, adjacent_to_aimed, damage,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-058";

/// R1862000: the chosen enemy's hit, then the same hit on each Unit beside it, read before the first.
fn shot(ctx: &EffectContext<'_>) -> Vec<Effect> {
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

fn beam() -> ActivationDecl {
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

// Stinkyscape #58 Beam v5 — SPEC §8.10 row 58, BUILD M12 row S 58: its Activate deals 15 to a chosen
// enemy Unit and 15 to each Unit beside it, through Armor (Pierce); on the enemy hero 7 (half, rounded
// down, R1841001); once a turn, a second use refused; refused on its user's next turn and usable the
// turn after, a bounce clearing it (R1841000, R78); equipped onto a Unit, the host takes the Activate;
// damage reads through `param()`; radiant 0/16, 30 damage.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hits, next_turn_of, weapon_board};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MENACE: &str = "core-019"; // (3) 9/9 Taunt.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FLOOD: &str = "core-017"; // (4) Spell: "Bounce all Units."

    /// The printed `damage`, and the Radiant face's, and its Radiant health.
    const DAMAGE: i32 = 15;
    const RADIANT_DAMAGE: i32 = 30;
    const RADIANT_HEALTH: i32 = 16;
    /// R1841001: half of it, rounded down, on a hero.
    const HALVED: i32 = DAMAGE / HALF_DAMAGE_TO_HEROES_DIVISOR;

    fn at(target: &str) -> Value {
        json!({ "ability": "beam", "targets": pick::id(target) })
    }

    fn at_hero() -> Value {
        json!({ "ability": "beam", "targets": [{ "pick": "hero", "player": "p2" }] })
    }

    fn beam_on(radiant: bool, p1: Value, p2: Value) -> Scenario {
        let mut p1 = p1;
        p1["field"] = json!([{ "def": ID, "radiant": radiant }]);
        weapon_board("s058", p1, p2)
    }

    /// p2's Menaces in lanes 1 to 3, the middle one with Armor 3, and a Vanilla in lane 4.
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

    fn halves_on_the_hero(radiant: bool, amount: i32) {
        let halved = amount / HALF_DAMAGE_TO_HEROES_DIVISOR;
        let mut s = beam_on(radiant, json!({}), json!({ "field": [MENACE, VANILLA] }));
        s.activate(ID, at_hero());
        assert_eq!(
            hits(s.last_events()),
            vec![("hero-p2".to_string(), halved)],
            "no Cleave off a hero"
        );
        s.expect_health(P2, HERO_HEALTH - halved);
    }

    mod base {
        use super::*;

        #[test]
        fn r1862000_hits_the_chosen_unit_and_each_unit_beside_it_through_armor() {
            cleaves(false, DAMAGE);
        }

        #[test]
        fn r1862000_its_target_is_an_enemy_never_a_unit_of_yours() {
            let mut s = weapon_board("s058-own", json!({ "field": [ID, VANILLA] }), json!({}));
            let mine = s.unit_at(P1, 2).id;
            s.expect_refused(|s| s.activate(ID, at(&mine)));
        }

        #[test]
        fn r1841001_the_enemy_hero_takes_half_rounded_down() {
            halves_on_the_hero(false, DAMAGE);
        }

        #[test]
        fn r1841000_once_a_turn_then_cooling_down_then_back() {
            let mut s = beam_on(false, json!({}), json!({}));
            s.activate(ID, at_hero());
            s.expect_refused_with(|s| s.activate(ID, at_hero()), "already been used");
            next_turn_of(&mut s, P1);
            s.expect_refused_with(|s| s.activate(ID, at_hero()), "cooling down");
            next_turn_of(&mut s, P1);
            s.activate(ID, at_hero());
            s.expect_health(P2, HERO_HEALTH - 2 * HALVED);
        }

        #[test]
        fn r1841000_r78_a_bounce_clears_the_cooldown() {
            let mut s = beam_on(false, json!({ "hand": [FLOOD], "mana": 4 }), json!({}));
            s.activate(ID, at_hero());
            s.play(FLOOD, json!({}));
            s.expect_in_zone(ID, "hand");
            next_turn_of(&mut s, P1);
            s.play(ID, json!({ "zone": 1 }));
            s.activate(ID, at_hero());
            s.expect_health(P2, HERO_HEALTH - 2 * HALVED);
        }

        #[test]
        fn r1086_r1852001_an_equipped_host_takes_the_activate_and_is_no_starship_piece() {
            let mut s = weapon_board(
                "s058-equip",
                json!({ "hand": [ID], "field": [VANILLA] }),
                json!({}),
            );
            let host = s.unit_at(P1, 1).id;
            s.play(ID, json!({ "zone": 1, "equip": true }));
            let fused = s.card(host.as_str()).clone();
            assert!(!unit_has(s.state(), &fused, KeywordKind::StarshipPiece));
            s.activate(host.as_str(), at_hero());
            s.expect_health(P2, HERO_HEALTH - HALVED);
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
        fn r1841001_the_enemy_hero_takes_half_of_its_doubled_damage() {
            halves_on_the_hero(true, RADIANT_DAMAGE);
        }

        #[test]
        fn its_health_is_doubled() {
            let s = beam_on(true, json!({}), json!({}));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (0, RADIANT_HEALTH));
        }
    }
}
