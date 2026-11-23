//! SPEC §8.12 S+ #17 Smoothie (3) Spell, Fruit, Rare, a card of the Stinkyscape+ set (R1420; R3558003,
//! R3566001, R4440006).
//!   Base:    "There is X and Y inside the smoothie. When added to your hand, select two random fruits
//!            for it."
//!   Radiant: "There is Radiant X and Radiant Y inside the smoothie."
//!
//! Engine: E8's `fuseRandomInto` with `count`, `unfusedOnly` (R3558003) and, since issue #4440, `asText`
//! (R4440006). As it is added to a hand (`entersHand`), two random Fruit Spells of the shipped sets are
//! drawn, and the Smoothie gains their text: their fusion is minted as a definition of its own and the
//! Smoothie runs it (R1850000's record), so it stays a Smoothie, its instance, name, price, rarity and art
//! its own, and is no fusion. Played, it does each Fruit's effect in turn, each asking its own targets
//! (R77), and plays no Fruit. The pool is every Fruit Spell but the two Smoothies (R3566001), never this
//! card (R387); a Smoothie that already runs Fruits' text is not filled again, and once it lands from a
//! play its record is reset (R215), so one that comes back is filled anew. The Radiant face's Fruits go in
//! on their Radiant faces (R469). Through v0.6.11d the Fruits were fused into it, a fusion
//! "Smoothie + X + Y" (`script_v0_6_11d`, R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-017";

/// "select two random fruits".
const FRUITS: i32 = 2;

/// R3566001: the Fruit Spells a Smoothie is filled from, the Smoothies left out.
pub(crate) fn fruit_pool() -> Value {
    json!({
        "tags": ["Fruit"],
        "type": "Spell",
        "excludeDefId": [ID, super::c018_large_smoothie::ID],
    })
}

/// `count` random Fruits for the arriving card, its face's (`radiant`): their text it gains (R4440006),
/// or, `as_text` false, fused into it (R3558003, through v0.6.11d).
fn filled(count: i32, radiant: bool, as_text: bool) -> Script {
    Script {
        enters_hand: Some(hook(move |_ctx| {
            let mut args = json!({
                "into": { "target": { "of": "self" } },
                "query": fruit_pool(),
                "count": count,
                "unfusedOnly": true,
                "radiant": radiant,
            });
            if as_text {
                args["asText"] = json!(true);
            }
            vec![fuse_random_into(json_as(args))]
        })),
        ..Script::default()
    }
}

/// R4440006: `count` random Fruits whose text the arriving card gains, its face's (`radiant`).
pub(crate) fn smoothie(count: i32, radiant: bool) -> Script {
    filled(count, radiant, true)
}

/// R767000: through v0.6.11d, `count` random Fruits fused into the arriving card (R3558003).
pub(crate) fn smoothie_v0_6_11d(count: i32, radiant: bool) -> Script {
    filled(count, radiant, false)
}

pub fn script() -> CardScripts {
    CardScripts {
        base: smoothie(FRUITS, false),
        radiant: smoothie(FRUITS, true),
    }
}

/// R767000: through v0.6.11d its Fruits were fused into it (issue #4440); `KEPT_SCRIPTS` folds those
/// replays under it.
pub fn script_v0_6_11d() -> CardScripts {
    CardScripts {
        base: smoothie_v0_6_11d(FRUITS, false),
        radiant: smoothie_v0_6_11d(FRUITS, true),
    }
}

// S+ #17 Smoothie — SPEC §8.12 row 17 (R3558003, R3566001, R4440006).
#[cfg(test)]
pub(crate) mod tests {
    use jackioh_engine::subsystems::fuse::fused_ingredients;
    use jackioh_engine::subsystems::unit_text::unit_text_of;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";

    /// The Fruits that add cards to a hand and ask for no target: a play of a Smoothie holding only these
    /// is read off the hand it fills.
    const ADDING_FRUITS: &[&str] = &[
        "classicplus-058",
        "classicplus-065",
        "classicplus-066",
        "meditative-055",
    ];

    /// p1 draws `id` (`radiant` its face) at the start of their turn: what it holds once it lands.
    pub(crate) fn handed(seed: &str, id: &str, radiant: bool) -> (Scenario, CardInstance) {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [FILLER], "library": [{ "def": id, "radiant": radiant }, FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.start_turn();
        let card = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id != FILLER)
            .expect("it was drawn");
        (s, card)
    }

    /// R4440006: the Fruits whose text `card` gained, in order; the card itself is still `own` and no
    /// fusion.
    pub(crate) fn fruits_in(s: &Scenario, card: &CardInstance, own: &str) -> Vec<String> {
        assert_eq!(card.def_id, own, "it stays itself");
        assert!(fused_ingredients(s.state(), &card.def_id).is_none(), "it is no fusion");
        let gained = unit_text_of(card).expect("a filled Smoothie runs its Fruits' text");
        fused_ingredients(s.state(), &gained.def_id).expect("the Fruits' fusion, minted alone")
    }

    /// The text the card gained, on the face it runs.
    pub(crate) fn gained_text(s: &Scenario, card: &CardInstance) -> String {
        let gained = unit_text_of(card).expect("a filled Smoothie");
        let def = def_of(Some(s.state()), &gained.def_id).clone();
        if gained.radiant {
            def.radiant.text
        } else {
            def.base.text
        }
    }

    /// Every Fruit is a Fruit Spell of a shipped set, never a Smoothie.
    pub(crate) fn assert_fruits(s: &Scenario, fruits: &[String], count: usize) {
        assert_eq!(fruits.len(), count, "{fruits:?}");
        for fruit in fruits {
            let def = def_of(Some(s.state()), fruit);
            assert!(def.tags.contains(&Tag::Fruit), "{fruit}");
            assert_eq!(def.type_, CardType::Spell, "{fruit}");
            assert!(set_ships(def.set), "{fruit}");
            assert!(
                ![super::ID, crate::scripts::stinkyscape_plus::c018_large_smoothie::ID].contains(&fruit.as_str()),
                "{fruit}: never a Smoothie"
            );
        }
    }

    #[test]
    fn r4440006_added_to_a_hand_it_stays_a_smoothie_gains_two_fruits_text_and_keeps_its_price() {
        for n in 0..8 {
            let (s, card) = handed(&format!("r3566001-smoothie-{n}"), ID, false);
            let fruits = fruits_in(&s, &card, ID);
            assert_fruits(&s, &fruits, 2);
            let text = gained_text(&s, &card);
            for fruit in &fruits {
                assert!(text.contains(&crate::card_def(fruit).base.text), "{n}: {fruit}");
            }
            assert_eq!(def_of(Some(s.state()), &card.def_id).name, "Smoothie");
            assert_eq!(effective_cost(s.state(), &card, Default::default()), 3);
        }
    }

    #[test]
    fn r4440006_radiant_it_gains_its_fruits_radiant_text() {
        for n in 0..4 {
            let (s, card) = handed(&format!("r3566001-smoothie-radiant-{n}"), ID, true);
            assert!(card.radiant);
            let fruits = fruits_in(&s, &card, ID);
            assert_fruits(&s, &fruits, 2);
            let text = gained_text(&s, &card);
            for fruit in &fruits {
                assert!(text.contains(&crate::card_def(fruit).radiant.text), "{fruit}");
            }
        }
    }

    /// R4440006: its owner's view shows the Fruits' text it runs (`copies`); the opponent's names neither
    /// the Smoothie nor its Fruits.
    #[test]
    fn r4440006_its_owner_sees_the_fruits_text_and_the_opponent_sees_none_of_it() {
        let (s, card) = handed("r4440006-view", ID, false);
        let gained = unit_text_of(&card).expect("filled");
        let mine = serde_json::to_value(s.view(P1)).unwrap_or_default();
        let shown = mine["you"]["hand"]
            .as_array()
            .and_then(|hand| hand.iter().find(|entry| entry["instanceId"] == card.id.as_str()).cloned())
            .expect("in its owner's hand");
        assert_eq!(shown["defId"], ID);
        assert_eq!(shown["copies"]["defId"], gained.def_id.as_str());
        let theirs = serde_json::to_value(s.view(PlayerId::P2)).unwrap_or_default();
        assert!(!theirs.to_string().contains(&gained.def_id), "the opponent reads no Fruit");
    }

    /// R4440006: played, it does its Fruits' effects and plays no Fruit, and it lands as itself, its
    /// record reset (R215).
    #[test]
    fn r4440006_played_it_does_its_fruits_effects_plays_no_fruit_and_lands_as_a_smoothie() {
        let found = (0..200).find_map(|n| {
            let (s, card) = handed(&format!("r4440006-play-{n}"), ID, false);
            let fruits = fruits_in(&s, &card, ID);
            fruits
                .iter()
                .all(|fruit| ADDING_FRUITS.contains(&fruit.as_str()))
                .then_some((s, card, fruits))
        });
        let (mut s, card, fruits) = found.expect("a seed whose Fruits both add cards");
        s.state_mut().players.p1.mana.current = MAX_MANA;
        let before = s.hand(P1).len();
        s.play(card.id.as_str(), json!({}));
        let played: Vec<String> = s
            .events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::CardPlayed { def_id, .. } => Some(def_id.clone()),
                _ => None,
            })
            .collect();
        for fruit in &fruits {
            assert!(!played.contains(fruit), "{fruit} is not played");
        }
        assert!(played.contains(&ID.to_string()));
        assert!(s.hand(P1).len() > before, "{fruits:?} added cards to the hand");
        let landed = s
            .pile(P1, "graveyard")
            .into_iter()
            .find(|entry| entry.id == card.id)
            .expect("it lands in its owner's graveyard");
        assert_eq!(landed.def_id, ID);
        assert!(unit_text_of(&landed).is_none(), "R215 resets its record");
    }

    /// R767000: the script kept for v0.6.11d fused the Fruits into it (R3558003).
    #[test]
    fn r767000_the_script_kept_for_v0_6_11d_fuses_its_fruits_into_it() {
        crate::under_kept(ID, super::script_v0_6_11d(), || {
            let (s, card) = handed("r767000-smoothie-kept", ID, false);
            assert_ne!(card.def_id, ID, "a fusion");
            let ingredients = fused_ingredients(s.state(), &card.def_id).expect("a filled Smoothie");
            assert!(ingredients.contains(&ID.to_string()), "{ingredients:?}");
            assert_eq!(ingredients.len(), 3, "{ingredients:?}");
            assert!(unit_text_of(&card).is_none());
        });
    }
}
