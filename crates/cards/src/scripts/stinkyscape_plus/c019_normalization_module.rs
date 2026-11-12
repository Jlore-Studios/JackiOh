//! SPEC §8.12 S+ #19 Normalization Module (Unit 0/2 → 0/4, Starship Piece, Equip, cost 2, Rare), a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Starship Piece, Equip
//!            Activate: Vanilla a Unit"
//!   Radiant: "Starship Piece, Equip
//!            Activate 2: Vanilla a Unit" — vanilla 2
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it fuses
//! into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one (R1852001).
//! The Activate is 1 use a turn on the base face and 2 on the Radiant face (R384, R1857003; the issue's
//! reading of the base face's "Activate" as "Activate 1"). R3552002: its target is a declared Unit on either
//! side, as Silence's (R81), never a hero; `vanilla` (§6.3) strips its printed keywords and scripts and
//! returns it to its printed stats keeping its damage, and Immutable refuses it (R23). Another Activate may
//! name the same Unit again; Vanilla on a Vanilla Unit does nothing.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-019";

/// "Activate": 1 use a turn on the base face (R384, R1857003).
const USES: i32 = 1;

/// "Activate 2": 2 uses a turn on the Radiant face, a label and no param (R482).
const RADIANT_USES: i32 = 2;

/// R3552002: "Vanilla a Unit" on the chosen Unit, on either side.
fn normalize(uses: i32) -> Script {
    let activation = ActivationDecl {
        id: "normalize".to_string(),
        label: "Vanilla a Unit".to_string(),
        uses: ActivationUses::Count(uses),
        cost: None,
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))],
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(|ctx| {
            // A declaration with no legal pick is passed (R81): no Unit chosen, nothing Vanilla.
            if ctx.targets.is_empty() {
                return Vec::new();
            }
            vec![vanilla(Default::default())]
        }),
    };
    Script {
        activations: vec![activation],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: normalize(USES),
        radiant: normalize(RADIANT_USES),
    }
}

// S+ #19 Normalization Module — SPEC §8.12 row 19 (R3552002; R23, R81, R384, R1086, R1751000, R1852001).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{next_turn_of, weapon_board};
    use jackioh_engine::subsystems::activate::activate_actions_for;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A Unit with text to lose: its printed 3/5 and two keywords.
    const TEXTED: &str = "stand-in (1) 3/5 Taunt, Rush";
    const IMMUTABLE: &str = "stand-in (1) 3/5 Taunt, Immutable";
    /// An equipped host.
    const HOST: &str = "stand-in (1) 4/4 R12/12";

    fn at(target: &str) -> Value {
        json!({ "ability": "normalize", "targets": pick::id(target) })
    }

    fn offers(s: &Scenario) -> usize {
        activate_actions_for(s.state(), P1, s.card(ID)).len()
    }

    fn offered(s: &Scenario) -> Vec<Selection> {
        activate_actions_for(s.state(), P1, s.card(ID))
            .into_iter()
            .flat_map(|action| action.targets.unwrap_or_default())
            .collect()
    }

    fn kinds(s: &Scenario, id: &str) -> Vec<KeywordKind> {
        s.stats(id).keywords.iter().map(Keyword::kind).collect()
    }

    fn board(radiant: bool, p1: Value, p2: Value) -> Scenario {
        let mut field = vec![json!({ "def": ID, "radiant": radiant })];
        field.extend(p1.as_array().cloned().unwrap_or_default());
        weapon_board("s019", json!({ "field": field }), json!({ "field": p2 }))
    }

    mod base {
        use super::*;

        #[test]
        fn r3552002_vanilla_applies_to_a_unit_on_either_side() {
            let mut s = board(false, json!([TEXTED]), json!([TEXTED]));
            let (own, enemy) = (s.unit_at(P1, 2).id, s.unit_at(P2, 1).id);
            s.card_mut(own.as_str()).buffs.attack = 2;
            s.card_mut(own.as_str()).buffs.health = 2;
            s.card_mut(enemy.as_str()).damage = 2;
            assert_eq!(kinds(&s, &enemy), [KeywordKind::Taunt, KeywordKind::Rush]);
            s.activate(ID, at(&enemy));
            assert!(kinds(&s, &enemy).is_empty(), "the enemy Unit lost its keywords");
            s.expect_stats(enemy.as_str(), json!({ "attack": 3, "health": 3, "maxHealth": 5 }));
            // The one use is spent; next turn it Vanillas its own side's Unit.
            s.expect_refused(|s| s.activate(ID, at(&own)));
            next_turn_of(&mut s, P1);
            s.activate(ID, at(&own));
            assert!(kinds(&s, &own).is_empty(), "the own Unit lost its keywords");
            s.expect_stats(own.as_str(), json!({ "attack": 3, "maxHealth": 5 }));
        }

        #[test]
        fn r3552002_it_offers_any_unit_and_no_hero() {
            let s = board(false, json!([TEXTED]), json!([TEXTED]));
            let offered = offered(&s);
            for unit in [s.unit_at(P1, 1), s.unit_at(P1, 2), s.unit_at(P2, 1)] {
                assert!(offered.contains(&Selection::Instance { instance_id: unit.id }));
            }
            assert!(!offered.iter().any(|pick| matches!(pick, Selection::Hero { .. })));
        }

        #[test]
        fn r3552002_a_hero_or_no_pick_is_refused() {
            let mut s = board(false, json!([]), json!([TEXTED]));
            s.expect_refused(|s| {
                s.activate(ID, json!({ "ability": "normalize", "targets": [{ "pick": "hero", "player": "p2" }] }))
            });
            s.expect_refused(|s| s.activate(ID, json!({ "ability": "normalize", "targets": [] })));
            let enemy = s.unit_at(P2, 1).id;
            assert_eq!(kinds(&s, &enemy).len(), 2);
        }

        #[test]
        fn r23_an_immutable_unit_keeps_its_text() {
            let mut s = board(false, json!([]), json!([IMMUTABLE]));
            let enemy = s.unit_at(P2, 1).id;
            s.activate(ID, at(&enemy));
            assert!(kinds(&s, &enemy).contains(&KeywordKind::Taunt));
        }

        #[test]
        fn r384_the_use_is_spent_a_turn_and_comes_back_the_next() {
            let mut s = board(false, json!([]), json!([TEXTED, TEXTED]));
            let (first, second) = (s.unit_at(P2, 1).id, s.unit_at(P2, 2).id);
            assert!(offers(&s) > 0);
            s.activate(ID, at(&first));
            assert_eq!(offers(&s), 0);
            s.expect_refused(|s| s.activate(ID, at(&second)));
            next_turn_of(&mut s, P1);
            assert!(offers(&s) > 0);
        }

        #[test]
        fn r1086_r1751000_an_equipped_host_takes_the_activate() {
            let mut s = weapon_board(
                "s019-equip",
                json!({ "hand": [ID], "field": [HOST], "mana": 10 }),
                json!({ "field": [TEXTED] }),
            );
            let host = s.unit_at(P1, 1).id;
            let enemy = s.unit_at(P2, 1).id;
            s.play(ID, json!({ "zone": 1, "equip": true }));
            s.activate(host.as_str(), at(&enemy));
            assert!(kinds(&s, &enemy).is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r384_it_may_vanilla_two_units_a_turn_and_no_third() {
            let mut s = board(true, json!([]), json!([TEXTED, TEXTED, TEXTED]));
            let ids: Vec<String> = (1..=3).map(|lane| s.unit_at(P2, lane).id).collect();
            s.activate(ID, at(&ids[0]));
            assert!(offers(&s) > 0, "one use left");
            s.activate(ID, at(&ids[1]));
            assert_eq!(offers(&s), 0);
            s.expect_refused(|s| s.activate(ID, at(&ids[2])));
            assert!(kinds(&s, &ids[0]).is_empty() && kinds(&s, &ids[1]).is_empty());
            assert_eq!(kinds(&s, &ids[2]).len(), 2);
        }

        #[test]
        fn its_health_is_doubled() {
            let s = board(true, json!([]), json!([]));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (0, 4));
        }
    }
}
