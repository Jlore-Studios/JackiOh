//! M+ #12 Living Armor Potion (SPEC §8.9 row 12, BUILD M11 row M+ 12): (1) Unit, Potion,
//! Rare, 1/5 → 2/10.
//!   Base:    "Activate: Tribute this. Your hero gains {armor} Armor." (5)
//!   Radiant: "Activate: Your hero gains {armor} Armor." (5)
//!
//! Activate in main phase (§6.2, R384): base tributes self (C #84); Radiant stays on field. Since
//! #5183 the Armor is the hero's to keep (`gain_hero_armor`), which each hit it meets wears
//! (R5183000, R5183003); before it, Core #98 Armor Up's `heroArmor` player modifier expiring
//! nextTurnOf (R757, `script_kept_5183`, kept, R767000). Read via param (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-012";

fn armor_kept() -> Hook {
    hook(|ctx| vec![gain_hero_armor(json_as(json!({ "amount": param(&*ctx, "armor") })))])
}

/// R767000, R5183003: the Armor before #5183, exactly Core #98 Armor Up's modifier (R757): gone when
/// the next own turn begins.
fn armor_until_next_turn() -> Hook {
    hook(|ctx| {
        let opponent = opponent_of(ctx.controller);
        vec![add_player_modifier(json_as(json!({
            "mod": {
                "kind": "heroArmor",
                "amount": param(&*ctx, "armor"),
                "expiry": { "until": "nextTurnOf", "player": opponent, "fromTurn": ctx.sink.state.turn },
            },
        })))]
    })
}

fn potion(radiant: bool, label: &str, run: Hook) -> Script {
    Script {
        activations: vec![ActivationDecl {
            id: "armor".to_string(),
            label: label.to_string(),
            uses: ActivationUses::Count(1),
            cost: (!radiant).then(|| ActivationCost {
                tribute_self: Some(true),
                ..ActivationCost::default()
            }),
            targets: Vec::new(),
            modes: Vec::new(),
            can_activate: None,
            has: None,
            run,
        }],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: potion(false, "Your hero gains Armor", armor_kept()),
        radiant: potion(true, "Your hero gains Armor", armor_kept()),
    }
}

/// R767000, R5183003: `KEPT_SCRIPTS` folds the games before #5183 under it.
pub fn script_kept_5183() -> CardScripts {
    CardScripts {
        base: potion(false, "Armor until your next turn", armor_until_next_turn()),
        radiant: potion(true, "Armor until your next turn", armor_until_next_turn()),
    }
}

// M+ #12 Living Armor Potion — SPEC §8.9 row 12, BUILD M11 row M+ 12 (§4.4).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::damage::hero_armor_of;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const STOCKPILE: &str = "core-005";
    const FILLER: &str = "core-010";
    const MENACE: &str = "core-019";
    const X: &str = "core-020";

    fn game(seed: &str, p1: Value, p2: Value) -> Scenario {
        crate::scenario(json!({
            "seed": seed,
            "p1": crate::merged(json!({ "library": [X, X, X] }), p1),
            "p2": crate::merged(json!({ "hand": [FILLER], "library": [X, X, X] }), p2),
        }))
    }

    fn activate(id: &str, s: &mut Scenario) {
        let potion = s.card(id).id.clone();
        s.activate(potion.as_str(), json!({}));
    }

    #[test]
    fn activates_the_turn_it_lands_and_is_tributed() {
        let mut s = game("m+012-lands", json!({ "hand": [ID, STOCKPILE] }), json!({}));
        s.play(ID, json!({ "zone": 1 }));
        let potion = s.unit(P1, 1).expect("the potion");
        s.activate(potion.id.as_str(), json!({}));
        s.expect_in_zone(potion.id.as_str(), "graveyard");
        assert_eq!(hero_armor_of(s.state(), P1), 5);
    }

    /// R5183003: the 5 Armor absorbs the opponent's hits, each wearing 1 off it (R5183000).
    #[test]
    fn r5183003_your_hero_has_5_armor_that_absorbs_their_hits_and_wears() {
        let mut s = game(
            "m+012-hits",
            json!({ "field": [ID], "hand": [STOCKPILE] }),
            json!({ "field": [MENACE] }),
        );
        activate(ID, &mut s);
        assert_eq!(hero_armor_of(s.state(), P1), 5);
        s.end_turn();
        assert_eq!(hero_armor_of(s.state(), P1), 5);
        s.attack(MENACE, "hero");
        s.expect_health(P1, 26);
        assert_eq!(hero_armor_of(s.state(), P1), 4, "the hit wore 1 off it");
    }

    /// R5183003: the Armor is the hero's own, so it stays past your next start of turn.
    #[test]
    fn r5183003_kept_past_your_next_start_of_turn() {
        let mut s = game("m+012-kept", json!({ "field": [ID], "hand": [STOCKPILE] }), json!({}));
        activate(ID, &mut s);
        assert_eq!(hero_armor_of(s.state(), P1), 5);
        s.end_turn();
        s.end_turn();
        assert_eq!(hero_armor_of(s.state(), P1), 5);
        assert_eq!(s.state().players.p1.hero.armor, 5);
        assert!(s.state().players.p1.mods.is_empty());
    }

    /// R767000, R5183003: the kept script's Armor is a modifier, gone at your next start of turn.
    #[test]
    fn r5183003_the_kept_script_s_armor_is_gone_at_your_next_start_of_turn() {
        crate::under_kept(ID, super::script_kept_5183(), || {
            let mut s = game("m+012-gone", json!({ "field": [ID], "hand": [STOCKPILE] }), json!({}));
            activate(ID, &mut s);
            assert_eq!(hero_armor_of(s.state(), P1), 5);
            s.end_turn();
            s.end_turn();
            assert_eq!(hero_armor_of(s.state(), P1), 0);
            assert!(s.state().players.p1.mods.is_empty());
        });
    }

    #[test]
    fn armor_reads_through_param() {
        // A card in hand, so the turn does not end itself once the potion is gone (§2.5).
        let mut s = game("m+012-param", json!({ "field": [ID], "hand": [FILLER] }), json!({}));
        let potion = s.unit(P1, 1).expect("the potion").id.clone();
        set_param(s.card_mut(&potion), "armor", 3);
        s.activate(potion.as_str(), json!({}));
        assert_eq!(hero_armor_of(s.state(), P1), 3);
    }

    #[test]
    fn radiant_stays_once_a_turn_and_two_activations_on_two_turns_are_each_their_own() {
        let mut s = game(
            "m+012-radiant",
            json!({ "field": [{ "def": ID, "radiant": true }], "hand": [FILLER] }),
            json!({ "hand": [FILLER, FILLER] }),
        );
        activate(ID, &mut s);
        s.expect_in_zone(ID, "field");
        assert_eq!(hero_armor_of(s.state(), P1), 5);
        s.expect_refused(|s| s.activate(ID, json!({})));
        s.end_turn();
        s.end_turn();
        // R5183003: the first 5 are kept, so the second activation's add to them.
        assert_eq!(hero_armor_of(s.state(), P1), 5);
        activate(ID, &mut s);
        assert_eq!(hero_armor_of(s.state(), P1), 10);
        assert!(s.state().players.p1.mods.is_empty());
    }
}
