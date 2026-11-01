//! S #137.1 Land Doctrine (SPEC §8.10 row 137.1; R53, R60, R1852005, R1859007, R1859008): (4) Spell,
//! War Doctrine, Token (printed Legendary), a token of the Stinkyscape set, which #137
//! Starfleet Commander Discovers and casts.
//!
//! Base:    "For the rest of the game, your Units have Trample. Your Starship attacks a random enemy Unit."
//! Radiant: "For the rest of the game, your Units have Trample. Your Starship attacks each enemy Unit."
//! Engine: `give_units_keyword { Trample }`, a `unitsHaveKeyword` player modifier for the rest of the
//! game: every Unit its caster controls has Trample, those that arrive later too, but a Vanilla one,
//! and it stays once the Doctrine is in the graveyard (R1859007). Then, once, as it is cast: a forced
//! attack (R53) by its caster's launched Starship (`launched_starship_of`, R1852005) on an enemy Unit
//! drawn from the match rng among those it may attack (§4.2 step 2, R60), the target striking back
//! (`forced_attack_random`); the Radiant face's attacks each enemy Unit in lane order, each its own
//! combat and state check, stopping once the Starship leaves the field (`forced_attacks`, R1859008).
//! With no launched Starship or no enemy Unit, only the keyword. Declares no numbers.

use std::sync::Arc;

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-137-1";

fn land_doctrine(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut effects = vec![give_units_keyword(GiveUnitsKeywordArgs {
                keyword: Keyword::Trample,
            })];
            let Some(ship) = launched_starship_of(&*ctx.state, ctx.controller) else {
                return effects;
            };
            if radiant {
                // A fused definition's id is minted for one card (R179), so it names this Starship alone.
                let ship_def = ship.def_id.clone();
                effects.push(for_each_card(ForEachCardArgs {
                    cards: Arc::new(|ctx: &mut EffectContext<'_>| -> Vec<String> {
                        active_units_of(&*ctx.state, opponent_of(ctx.controller))
                            .iter()
                            .map(|card| card.id.clone())
                            .collect()
                    }),
                    each: Arc::new(move |enemy: &str| {
                        forced_attacks(json_as(json!({
                            "attackers": { "side": "self", "defId": ship_def },
                            "target": { "instanceId": enemy },
                        })))
                    }),
                }));
            } else {
                effects.push(forced_attack_random(json_as(json!({
                    "attacker": { "of": "instance", "instanceId": ship.id },
                    "among": "enemyUnits",
                }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: land_doctrine(false),
        radiant: land_doctrine(true),
    }
}

// S #137.1 Land Doctrine — SPEC §8.10 row 137.1, BUILD M12 row S 137.1 (R53).
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::{built_starship, launch_starship};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    // Issue #2521: Stratos (S #68) is no Starship Piece any more, so a piece of 6 Health stands in.
    const CRYSTAL: &str = "stinkyscape-132"; // (2) 3/6 Starship Piece, "Death: Your hero gains 1 Armor."
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const TIMMY: &str = "core-011"; // (1) 3/3 Rush, First Strike.

    fn has_trample(s: &Scenario, id: &str) -> bool {
        s.stats(id)
            .keywords
            .iter()
            .any(|keyword| keyword.kind() == KeywordKind::Trample)
    }

    fn forced_attacks(s: &Scenario) -> Vec<(String, String)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AttackDeclared {
                    attacker_id,
                    target_id,
                    forced: true,
                    ..
                } => Some((attacker_id.clone(), target_id.clone())),
                _ => None,
            })
            .collect()
    }

    /// p1's launched Starship (one Defensive Crystal, 3/6), a Mr. Vanilla of p1's in lane 3, the Doctrine
    /// (Radiant if `radiant`) and a Mr. Vanilla in hand; p2's `enemies` in their unit zones.
    fn launched(seed: &str, radiant: bool, enemies: Value) -> (Scenario, String) {
        let (mut s, ship) = built_starship(
            seed,
            json!([{ "def": CRYSTAL, "lane": 1 }]),
            json!({
                "hand": [{ "def": ID, "radiant": radiant }, VANILLA],
                "field": [{ "def": VANILLA, "lane": 3 }],
            }),
            json!({ "field": enemies }),
        );
        launch_starship(&mut s, &ship);
        s.state_mut().players.p1.mana.current = 10;
        (s, ship)
    }

    #[test]
    fn r1859007_its_casters_units_have_trample_now_and_later_the_doctrine_in_the_graveyard() {
        let (mut s, ship) = launched("r1859007-land", false, json!([]));
        let mine = s.unit_at(P1, 3).id;
        assert!(!has_trample(&s, &mine));
        s.play(ID, json!({}));
        s.expect_in_zone(ID, "graveyard");
        assert!(has_trample(&s, &mine) && has_trample(&s, &ship));
        s.play(VANILLA, json!({ "zone": 4 }));
        assert!(has_trample(&s, s.unit_at(P1, 4).id.as_str()), "a Unit played later has it");
        s.end_turn();
        s.end_turn();
        assert!(has_trample(&s, &mine), "for the rest of the game");
        assert!(forced_attacks(&s).is_empty());
    }

    #[test]
    fn r1859008_its_launched_starship_attacks_one_random_enemy_unit() {
        let (mut s, ship) = launched(
            "r1859008-land",
            false,
            json!([{ "def": VANILLA, "lane": 1 }, { "def": TIMMY, "lane": 2 }]),
        );
        let enemies = [s.unit_at(P2, 1).id, s.unit_at(P2, 2).id];
        s.play(ID, json!({}));
        let attacks = forced_attacks(&s);
        assert_eq!(attacks.len(), 1, "{attacks:?}");
        assert_eq!(attacks[0].0, ship);
        assert!(enemies.contains(&attacks[0].1));
    }

    #[test]
    fn r1859008_radiant_it_attacks_each_enemy_unit_in_lane_order() {
        let (mut s, ship) = launched(
            "r1859008-land-radiant",
            true,
            json!([{ "def": TIMMY, "lane": 1 }, { "def": TIMMY, "lane": 2 }]),
        );
        let enemies = [s.unit_at(P2, 1).id, s.unit_at(P2, 2).id];
        s.play(ID, json!({}));
        let attacks = forced_attacks(&s);
        assert_eq!(
            attacks,
            enemies.iter().map(|enemy| (ship.clone(), enemy.clone())).collect::<Vec<_>>()
        );
    }

    #[test]
    fn r1859008_radiant_the_attacks_stop_once_the_starship_dies() {
        let (mut s, ship) = launched(
            "r1859008-land-dies",
            true,
            json!([
                { "def": VANILLA, "lane": 1 },
                { "def": VANILLA, "lane": 2 },
                { "def": VANILLA, "lane": 3 },
            ]),
        );
        s.card_mut(ship.as_str()).damage = 4;
        s.play(ID, json!({}));
        let attacks = forced_attacks(&s);
        assert_eq!(attacks.len(), 1, "it died in the first combat: {attacks:?}");
        s.expect_in_zone(ship.as_str(), "graveyard");
    }

    #[test]
    fn r1859007_with_no_launched_starship_only_the_keyword() {
        let (mut s, _ship) = built_starship(
            "r1859007-land-none",
            json!([{ "def": CRYSTAL, "lane": 1 }]),
            json!({ "hand": [ID], "field": [{ "def": VANILLA, "lane": 3 }] }),
            json!({ "field": [{ "def": VANILLA, "lane": 1 }] }),
        );
        s.state_mut().players.p1.mana.current = 10;
        s.play(ID, json!({}));
        assert!(forced_attacks(&s).is_empty());
        assert!(has_trample(&s, s.unit_at(P1, 3).id.as_str()));
    }
}
