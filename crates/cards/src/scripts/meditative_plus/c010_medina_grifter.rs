//! M+ #10 Medina Grifter (SPEC §8.9 row 10, BUILD M11 row M+ 10): (1) Unit, Human,
//! Common, 4/1 → 8/2.
//!   Base:    "Cry: Steal {cards|random card|random cards} from your opponent's hand." (1)
//!   Radiant: the same, plus "It costs (0)."
//!
//! `give_from_hand { from: enemy, cards: random }` (M #21, C #9): `cards` distinct random cards
//! (R60, R1179001) become the stealer's (R12), hidden per zone (R97), the hand cap burning the
//! overflow into the stealer's graveyard (§2.4); an empty hand takes nothing and draws no random
//! number (R129). The Radiant face's `TakenRiders.cost_override` is 0. The Cry runs when the
//! Grifter is played or cast (R70). The count is declared and read through `param` (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-010";

/// The Radiant face's price for the stolen card.
const TAKEN_COST: i32 = 0;

fn grifter(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut args = json!({
                "from": "enemy",
                "cards": "random",
                "count": param(&*ctx, "cards"),
            });
            if radiant {
                args["costOverride"] = json!(TAKEN_COST);
            }
            vec![give_from_hand(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: grifter(false),
        radiant: grifter(true),
    }
}

// M+ #10 Medina Grifter — SPEC §8.9 row 10, BUILD M11 row M+ 10: "Cry (played or cast): one
// random card of the opponent's hand moves to yours, owned by you, the opponent seeing only that
// one left; an empty hand takes nothing and draws no random number; a full hand burns it into
// your graveyard; cards reads through `param()`; radiant 8/2 and the card costs (0)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::effects::{CastNewArgs, cast_new};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const GRIFTER: &str = "meditativeplus-010";
    const MENACE: &str = "core-019"; // (3) Unit
    const VANILLA: &str = "core-008"; // (1) Unit
    const FILLER: &str = "core-005"; // (1) Spell

    use crate::scenario;

    use crate::js;

    fn hand_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.hand(player).into_iter().map(|card| card.def_id).collect()
    }

    #[test]
    fn r1179001_steals_one_random_card_of_the_opponents_hand_as_yours() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [GRIFTER, FILLER] },
            "p2": { "hand": [MENACE], "library": [VANILLA, VANILLA] },
        }));
        s.play(GRIFTER, json!({ "zone": 1 }));
        s.expect_in_zone(GRIFTER, "field");
        assert_eq!(hand_defs(&s, P1), [FILLER, MENACE]);
        let taken = s.hand(P1).into_iter().find(|card| card.def_id == MENACE).expect("the stolen card");
        assert_eq!(taken.owner, P1);
        assert_eq!(taken.controller, P1);
        assert_eq!(hand_defs(&s, P2), Vec::<String>::new());
    }

    #[test]
    fn r1179001_an_empty_hand_steals_nothing_and_draws_no_random_number() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [GRIFTER, FILLER] },
            "p2": { "hand": [], "library": [VANILLA, VANILLA] },
        }));
        let cursor = s.state().rng_cursor;
        s.play(GRIFTER, json!({ "zone": 1 }));
        assert_eq!(s.state().rng_cursor, cursor);
        s.expect_in_zone(GRIFTER, "field");
        assert_eq!(hand_defs(&s, P1), [FILLER]);
    }

    #[test]
    fn s2_4_a_full_hand_burns_it_into_your_graveyard() {
        crate::register_all();
        // The Grifter and ten more: after the Grifter is played the hand is at its cap, so the
        // stolen card burns.
        let mut hand = vec![json!(GRIFTER)];
        hand.extend((0..HAND_CAP).map(|_| json!(FILLER)));
        let mut s = scenario(json!({
            "p1": { "hand": hand },
            "p2": { "hand": [MENACE], "library": [VANILLA, VANILLA] },
        }));
        s.play(GRIFTER, json!({ "zone": 1 }));
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
        let burned = s.pile(P1, "graveyard").into_iter().find(|card| card.def_id == MENACE).expect("the burned card");
        assert_eq!(burned.owner, P1);
        assert!(!s.pile(P2, "graveyard").iter().any(|card| card.def_id == MENACE));
    }

    #[test]
    fn r97_the_opponent_sees_only_that_a_card_left() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [GRIFTER, FILLER] },
            "p2": { "hand": [MENACE, VANILLA], "library": [FILLER, FILLER] },
        }));
        s.play(GRIFTER, json!({ "zone": 1 }));
        // p2's view of p1's hand is a count: the stolen card is hidden there.
        let theirs = js(&s.view(P2));
        assert_eq!(theirs["opponent"]["hand"], json!({ "count": 2 }));
        // The steal itself names p2's own former card (it was theirs); every other event names
        // neither its id nor its def.
        let mut not_steals = theirs.clone();
        not_steals["events"] = Value::Array(
            theirs["events"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|event| event["type"] != "stolen")
                .collect(),
        );
        let not_steals = not_steals.to_string();
        let taken = s.hand(P1).into_iter().find(|card| card.owner == P1 && card.def_id != FILLER).expect("the stolen card");
        assert!(!not_steals.contains(&format!("\"{}\"", taken.id)));
        assert!(!not_steals.contains(&taken.def_id));
    }

    #[test]
    fn r70_a_cast_grifter_steals_too() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [VANILLA] },
            "p2": { "hand": [MENACE], "library": [VANILLA, VANILLA] },
        }));
        // A fixture Spell that casts a Grifter, as an effect's cast would (R70).
        let face = json!({ "keywords": [], "text": "fx-cast-grifter" });
        let caster: CardDef = json_as(json!({
            "id": "fx-cast-grifter", "index": "fx-cast-grifter", "name": "fx-cast-grifter", "set": "Core",
            "type": "Spell", "tags": [], "rarity": "Common", "token": false, "cost": 0,
            "base": face, "radiant": face,
        }));
        s.state_mut().transient_defs.insert(caster.id.clone(), caster);
        let script = Script {
            cry: Some(hook(|_ctx| {
                vec![cast_new(CastNewArgs {
                    def: ID.into(),
                    radiant: None,
                    how: Default::default(),
                })]
            })),
            ..Script::default()
        };
        let mut scripts = registered_scripts();
        scripts.insert(
            "fx-cast-grifter".to_string(),
            CardScripts {
                base: script.clone(),
                radiant: script,
            },
        );
        register_scripts(scripts);
        let spell = new_instance(s.state_mut(), "fx-cast-grifter", P1, Zone::Hand { player: P1 });
        s.state_mut().players.p1.hand.push(spell.clone());
        s.play(&spell, json!({}));
        // The cast Grifter entered the field and its Cry stole the Menace.
        s.expect_in_zone(GRIFTER, "field");
        assert_eq!(hand_defs(&s, P1), [VANILLA, MENACE]);
        let taken = s.hand(P1).into_iter().find(|card| card.def_id == MENACE).expect("the stolen card");
        assert_eq!(taken.owner, P1);
    }

    #[test]
    fn r386_cards_reads_through_param() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [GRIFTER, FILLER] },
            "p2": { "hand": [MENACE, VANILLA], "library": [FILLER, FILLER] },
        }));
        step_param(s.card_mut(GRIFTER), "cards", 1);
        s.play(GRIFTER, json!({ "zone": 1 }));
        // Both cards of their hand, in whatever order the shuffle took them.
        let mut hand = hand_defs(&s, P1);
        hand.sort_unstable();
        assert_eq!(hand, vec![FILLER, VANILLA, MENACE]);
    }

    #[test]
    fn radiant_the_stolen_card_costs_0() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": GRIFTER, "radiant": true }, FILLER] },
            "p2": { "hand": [MENACE], "library": [VANILLA, VANILLA] },
        }));
        s.play(GRIFTER, json!({ "zone": 1 }));
        let taken = s.hand(P1).into_iter().find(|card| card.def_id == MENACE).expect("the stolen card");
        assert_eq!(taken.cost_override, Some(0));
        assert_eq!(effective_cost(s.state(), &taken, Default::default()), 0);
    }
}
