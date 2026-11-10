//! M+ #5 Spirit Lash (SPEC §8.9 row 5, BUILD M11 row M+ 5): (0, Embiggened 1) Spell, Rare.
//!   Base:    "Flashback / Deal {damage} damage to all Units. Embiggened (1): With Lifesteal and Pierce."
//!   Radiant: "Echo 1, Flashback" plus the same: it resolves a second time.
//!
//! Flashback (issue #3138) is the catalog keyword (§6.1, R783012): the script is unchanged. The card lies
//! in the graveyard after its play and is offered from there once, at both of its prices, (0) and
//! Embiggened (1) (R3138000); either play spends the Flashback (R783013, upheld by R3039001).
//!
//! One `damage_all` over every Unit on both sides ("any", no hero), one hit each, all landing
//! before the state check (R59, R1179000). Played Embiggened, each hit carries Lifesteal (R85)
//! and Pierce (R346), stated on the effect, so the card prints neither keyword. The Radiant face
//! prints Echo 1 (`StaticFlags.echo`, §6.2): the repeat is Embiggened if the play was (the
//! instance keeps the flag, R81). The damage is declared and read through `param` (R386).

use jackioh_engine::effects::damage_all;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-005";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            let embiggened = ctx.embiggened;
            vec![damage_all(json_as(json!({
                "amount": param(&*ctx, "damage"),
                "side": "any",
                "lifesteal": embiggened,
                "ignoreArmor": embiggened,
            })))]
        })),
        ..Script::default()
    };

    // The Radiant face adds Echo 1 and keeps the base script, so the repeat is Embiggened when
    // the play was (C+ #36.1's shape).
    let radiant = Script {
        static_flags: Some(StaticFlags {
            echo: Some(1),
            ..StaticFlags::default()
        }),
        ..base.clone()
    };

    CardScripts { base, radiant }
}

// M+ #5 Spirit Lash — SPEC §8.9 row 5, BUILD M11 row M+ 5.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const LASH: &str = "meditativeplus-005";
    const ARMORED: &str = "stand-in (4) 7/7 Armor 7 R14/14 Armor 7, Reborn";
    const SHIELDED: &str = "core-056"; // Jilliax: Divine Shield
    const BODY: &str = "core-019"; // Midrange Menace
    const FILLER: &str = "core-005";

    use crate::scenario;
    use crate::unit_or_blank;

    /// A play of the graveyard card `id` as `legal_actions` offers it, at its Embiggened price when
    /// `embiggen` (`reduce`, since the harness's `play` takes a card from a hand: C #28 Second Wind's shape).
    fn graveyard_play(state: &GameState, id: &str, embiggen: bool, attempt: u32) -> (Option<String>, GameState) {
        let action = json_as::<Action>(json!({
            "type": "play",
            "playerId": "p1",
            "instanceId": id,
            "embiggen": embiggen,
            "nonce": format!("spirit-lash-flashback-{id}-{attempt}"),
        }));
        let result = reduce(state, &action);
        (result.error, result.state)
    }

    /// The prices `legal_actions` offers p1 for the graveyard card `id`: `Some(false)` is the base price,
    /// `Some(true)` the Embiggened one.
    fn graveyard_prices(state: &GameState, id: &str) -> Vec<Option<bool>> {
        legal_actions(state, P1)
            .iter()
            .filter_map(|action| match action {
                ActionBody::Play { instance_id, embiggen, .. } if instance_id == id => Some(*embiggen),
                _ => None,
            })
            .collect()
    }

    fn hits(s: &Scenario) -> Vec<i32> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { amount, .. } => Some(*amount),
                _ => None,
            })
            .collect()
    }

    fn has_keyword(s: &Scenario, card: &str, kind: KeywordKind) -> bool {
        s.stats(card).keywords.iter().any(|keyword| keyword.kind() == kind)
    }

    /// Issue #3138: both faces print Flashback, so after a play the card lies in the graveyard and is played
    /// from there once at (0), dealing its hit to every Unit again; that play spends the Flashback, so it is
    /// never offered from there again (R783013, upheld by R3039001).
    #[test]
    fn issue_3138_r3039001_it_is_played_from_the_graveyard_once_at_0_on_both_faces() {
        for radiant in [false, true] {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": LASH, "radiant": radiant }, FILLER], "field": [{ "def": BODY, "lane": 1 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": BODY, "lane": 1 }] },
            }));
            let (own, enemy) = (s.unit(P1, 1).expect("own").id, s.unit(P2, 1).expect("enemy").id);
            s.play(LASH, json!({}));
            let grave = s.pile(P1, "graveyard").into_iter().find(|card| card.def_id == LASH).expect("played");
            assert!(has_keyword(&s, &grave.id, KeywordKind::Flashback), "radiant {radiant}: it prints Flashback");
            let per_play = if radiant { 2 } else { 1 };

            let state = s.state().clone();
            let mana = state.players.p1.mana.current;
            let (error, state) = graveyard_play(&state, &grave.id, false, 1);
            assert!(error.is_none(), "radiant {radiant}: {error:?}");
            assert_eq!(state.players.p1.mana.current, mana, "radiant {radiant}: (0)");
            // Every Unit took the first play's hits and the Flashback's.
            for id in [&own, &enemy] {
                let unit = find_instance(&state, id).expect("the 9/9");
                assert_eq!(unit.damage, per_play * 2, "radiant {radiant}");
            }
            // Back in the graveyard with its Flashback spent: never offered from there again.
            let spent =
                state.players.p1.graveyard.iter().find(|card| card.id == grave.id).expect("back in the graveyard");
            assert!(!has_flashback(&state, spent), "radiant {radiant}: spent");
            assert_eq!(graveyard_prices(&state, &grave.id), Vec::<Option<bool>>::new(), "radiant {radiant}: no replay");
        }
    }

    /// R3138000: a Flashback play of an embiggen card is offered at both of its prices, (0) and
    /// Embiggened (1), as a play from the hand is; the Embiggened one carries Lifesteal and Pierce, and
    /// spends the Flashback as the base one does (R783013).
    #[test]
    fn r3138000_a_flashback_of_an_embiggen_card_offers_both_prices_and_either_spends_it() {
        for radiant in [false, true] {
            let mut s = scenario(json!({
                "p1": {
                    "health": 20,
                    "hand": [{ "def": LASH, "radiant": radiant }, FILLER],
                    "field": [{ "def": BODY, "lane": 1 }],
                },
                "p2": { "hand": [FILLER], "field": [{ "def": ARMORED, "lane": 1 }] },
            }));
            let armored = s.unit(P2, 1).expect("the Armored 7/7").id;
            s.play(LASH, json!({}));
            let grave = s.pile(P1, "graveyard").into_iter().find(|card| card.def_id == LASH).expect("played");
            let mut state = s.state().clone();
            assert_eq!(graveyard_prices(&state, &grave.id), vec![Some(false), Some(true)], "radiant {radiant}: both prices");
            let held = state.players.p1.graveyard.iter().find(|card| card.id == grave.id).expect("held");
            assert_eq!(embiggen_play_cost(&state, held), Some(1), "the Embiggened price, as a hand card's (the view's embiggenCost)");
            assert_eq!(find_instance(&state, &armored).expect("armored").damage, 0, "Armor 7 took the base hits");

            // Embiggened from the graveyard: (1), each hit ignores Armor and heals the hero by the damage dealt.
            let mana = state.players.p1.mana.current;
            let (error, next) = graveyard_play(&state, &grave.id, true, 1);
            assert!(error.is_none(), "radiant {radiant}: {error:?}");
            state = next;
            let hits_each = if radiant { 2 } else { 1 };
            assert_eq!(state.players.p1.mana.current, mana - 1, "radiant {radiant}: (1)");
            assert_eq!(state.players.p1.hero.health, 20 + 2 * hits_each, "radiant {radiant}: Lifesteal, one heal a hit");
            assert_eq!(find_instance(&state, &armored).expect("armored").damage, hits_each, "radiant {radiant}: Pierce");

            // It lies there again, its Flashback spent: neither price is on offer.
            assert_eq!(graveyard_prices(&state, &grave.id), Vec::<Option<bool>>::new(), "radiant {radiant}: spent");
        }
    }

    #[test]
    fn r1179000_one_hit_on_every_unit_of_both_sides_and_none_on_a_hero() {
        let mut s = scenario(json!({
            "p1": { "hand": [LASH, FILLER], "field": [{ "def": BODY, "lane": 2 }] },
            "p2": { "hand": [FILLER], "field": [{ "def": BODY, "lane": 1 }, { "def": BODY, "lane": 3 }] },
        }));
        s.play(LASH, json!({}));
        let own = unit_or_blank(&s, P1, 2);
        s.expect_stats(&own, json!({ "health": 8 }));
        for lane in [1, 3] {
            let enemy = unit_or_blank(&s, P2, lane);
            s.expect_stats(&enemy, json!({ "health": 8 }));
        }
        assert_eq!(hits(&s), vec![1, 1, 1]);
        s.expect_health(P1, 30).expect_health(P2, 30);
    }

    #[test]
    fn a_divine_shield_is_popped() {
        let mut s = scenario(json!({
            "p1": { "hand": [LASH, FILLER] },
            "p2": { "hand": [FILLER], "field": [SHIELDED] },
        }));
        s.play(LASH, json!({}));
        let shielded = s.unit(P2, 1).expect("gone");
        s.expect_stats(&shielded, json!({ "health": 2 }));
        assert!(!has_keyword(&s, &shielded.id, KeywordKind::DivineShield));
    }

    #[test]
    fn the_normal_version_does_not_pierce_armor() {
        let mut s = scenario(json!({
            "p1": { "hand": [LASH, FILLER] },
            "p2": { "hand": [FILLER], "field": [ARMORED] },
        }));
        s.play(LASH, json!({}));
        let armored = unit_or_blank(&s, P2, 1);
        s.expect_stats(&armored, json!({ "health": 7 }));
    }

    #[test]
    fn r1179000_embiggened_each_hit_ignores_armor_and_heals_your_hero_by_the_damage_dealt() {
        let mut s = scenario(json!({
            "p1": { "health": 20, "hand": [LASH, FILLER], "field": [BODY] },
            "p2": { "hand": [FILLER], "field": [ARMORED] },
        }));
        s.play(LASH, json!({ "embiggen": true }));
        s.expect_mana(P1, 3);
        let armored = unit_or_blank(&s, P2, 1);
        s.expect_stats(&armored, json!({ "health": 6 }));
        let own = unit_or_blank(&s, P1, 1);
        s.expect_stats(&own, json!({ "health": 8 }));
        assert_eq!(hits(&s), vec![1, 1]);
        s.expect_health(P1, 22);
    }

    #[test]
    fn r386_damage_reads_through_param() {
        let mut s = scenario(json!({
            "p1": { "hand": [LASH, FILLER] },
            "p2": { "hand": [FILLER], "field": [BODY] },
        }));
        step_param(s.card_mut(LASH), "damage", 1);
        s.play(LASH, json!({}));
        let body = unit_or_blank(&s, P2, 1);
        s.expect_stats(&body, json!({ "health": 7 }));
    }

    #[test]
    fn radiant_echo_1_hits_every_unit_twice() {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": LASH, "radiant": true }, FILLER], "field": [BODY] },
            "p2": { "hand": [FILLER], "field": [BODY] },
        }));
        s.play(LASH, json!({}));
        let own = unit_or_blank(&s, P1, 1);
        s.expect_stats(&own, json!({ "health": 7 }));
        let enemy = unit_or_blank(&s, P2, 1);
        s.expect_stats(&enemy, json!({ "health": 7 }));
        assert_eq!(hits(&s), vec![1, 1, 1, 1]);
        s.expect_health(P1, 30).expect_health(P2, 30);
    }

    #[test]
    fn r1179000_radiant_an_embiggened_play_repeats_embiggened() {
        let mut s = scenario(json!({
            "p1": { "health": 20, "hand": [{ "def": LASH, "radiant": true }, FILLER], "field": [BODY] },
            "p2": { "hand": [FILLER], "field": [ARMORED] },
        }));
        s.play(LASH, json!({ "embiggen": true }));
        let armored = unit_or_blank(&s, P2, 1);
        s.expect_stats(&armored, json!({ "health": 5 }));
        assert_eq!(hits(&s), vec![1, 1, 1, 1]);
        s.expect_health(P1, 24);
    }
}
