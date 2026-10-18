//! M+ #12 Living Armor Potion (SPEC §8.9 row 12, BUILD M11 row M+ 12): (1) Unit, Potion,
//! Rare, 1/5 → 2/10.
//!   Base:    "Activate: Tribute this. Your hero has +{armor} Armor until the start of your next
//!            turn." (5)
//!   Radiant: "Activate: Your hero has +{armor} Armor until the start of your next turn." (5)
//!
//! An Activate (§6.2, R384): in its controller's main phase, the turn it lands included (no
//! summoning sickness), once a turn. The base face's cost is the card's own Tribute
//! (`tribute_self`, C #84), paid first; the Radiant face's has none, so it stays and activates
//! again each turn. Either way the activator's hero gains Core #98 Armor Up's `heroArmor` player
//! modifier (R757), expiring at the start of its controller's next turn (`nextTurnOf`), so it
//! lasts through the opponent's turn and is gone when the next own turn begins. The number is the
//! declared `armor`, read as the ability runs (R386). Tunes: armor 5 ↑.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-012";

fn armor_until_next_turn() -> Hook {
    hook(|ctx| {
        // Exactly Core #98 Armor Up's modifier (R757): gone when the next own turn begins.
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

fn potion(radiant: bool) -> Script {
    Script {
        activations: vec![ActivationDecl {
            id: "armor".to_string(),
            label: "Armor until your next turn".to_string(),
            uses: ActivationUses::Count(1),
            cost: (!radiant).then(|| ActivationCost {
                tribute_self: Some(true),
                ..ActivationCost::default()
            }),
            targets: Vec::new(),
            modes: Vec::new(),
            can_activate: None,
            has: None,
            run: armor_until_next_turn(),
        }],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: potion(false),
        radiant: potion(true),
    }
}

// M+ #12 Living Armor Potion — SPEC §8.9 row 12, BUILD M11 row M+ 12: "Activate (the turn it
// lands included): tributed, your hero has +5 Armor through the opponent's turn, absorbing their
// hits (§4.4), gone at your next start of turn; radiant 2/10, kept on the field, once a turn, two
// activations on two turns each their own; armor reads through `param()`".
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::damage::hero_armor_of;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    /// (1) Spell: Draw 2. A spare that keeps the turn alive (§2.5).
    const STOCKPILE: &str = "core-005";
    /// (1) Spell.
    const FILLER: &str = "core-010";
    /// (3) 9/9 Taunt.
    const MENACE: &str = "core-019";
    /// Library filler.
    const X: &str = "core-020";

    fn game(seed: &str, p1: Value, p2: Value) -> Scenario {
        let mut sides = json!({
            "seed": seed,
            "p1": { "library": [X, X, X] },
            "p2": { "hand": [FILLER], "library": [X, X, X] },
        });
        for (seat, extra) in [("p1", p1), ("p2", p2)] {
            if let Value::Object(entries) = extra {
                for (key, value) in entries {
                    sides[seat][key.as_str()] = value;
                }
            }
        }
        crate::scenario(sides)
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

    #[test]
    fn your_hero_has_5_armor_through_the_opponents_turn_absorbing_their_hits() {
        let mut s = game(
            "m+012-hits",
            json!({ "field": [ID], "hand": [STOCKPILE] }),
            json!({ "field": [MENACE] }),
        );
        activate(ID, &mut s);
        assert_eq!(hero_armor_of(s.state(), P1), 5);
        s.end_turn();
        // Still there on the opponent's turn: their 9 attack hits the 5 armor for 4.
        assert_eq!(hero_armor_of(s.state(), P1), 5);
        s.attack(MENACE, "hero");
        s.expect_health(P1, 26);
    }

    #[test]
    fn gone_at_your_next_start_of_turn() {
        let mut s = game("m+012-gone", json!({ "field": [ID], "hand": [STOCKPILE] }), json!({}));
        activate(ID, &mut s);
        assert_eq!(hero_armor_of(s.state(), P1), 5);
        s.end_turn();
        s.end_turn();
        assert_eq!(hero_armor_of(s.state(), P1), 0);
        assert!(s.state().players.p1.mods.is_empty());
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
        // The first modifier expired at the start of this turn: one activation, one modifier.
        assert_eq!(hero_armor_of(s.state(), P1), 0);
        activate(ID, &mut s);
        assert_eq!(hero_armor_of(s.state(), P1), 5);
        assert_eq!(s.state().players.p1.mods.len(), 1);
    }
}
