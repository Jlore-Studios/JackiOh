//! Meditative #11 Double Header (SPEC §8.8 row 11; docs/meditative-set.md M6 #11; R826).
//! (4) Field Spell, Legendary.
//!   Base:    "The first card you play each turn adds a copy of it to your hand. The copy isn't Radiant
//!            and costs ({setCost})." (setCost 0)
//!   Radiant: "The first card you play each turn adds a Radiant copy of it to your hand. It costs
//!            ({setCost})."
//!
//! A trigger on its controller's `cardResolved` (§10.5 step 7, as Core #33 Unstable Clone Machine's,
//! R17) for the card at the head of their plays this turn (R213: "first" is the turn's log, not a
//! flag), played that once: a card played again later in the turn is the turn's first card no more.
//! The copy is a fresh card of the definition the event names (R71), so a fused card copies its
//! transient definition (R77) and a card that has ceased to exist since (Core #41 Sheepish) still
//! copies. A cast is a play (R70); a countered card never reached the log (R448). Double Header does
//! not answer its own play (R119), so the turn it lands copies nothing (R826).

use jackioh_engine::effects::add_to_hand;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-011";

/// Whether `instance_id` is the first card `player` played this turn, and played that once (R826).
fn first_played(state: &GameState, player: PlayerId, instance_id: &str) -> bool {
    let played = played_ids_this_turn(state, player);
    played.first().is_some_and(|first| first == instance_id)
        && played.iter().filter(|id| *id == instance_id).count() == 1
}

/// `radiant`: the Radiant face's copy is Radiant; the base face's never is.
fn first_card_copied(radiant: bool) -> TriggerDef {
    TriggerDef::new(
        if radiant {
            "m11r-first-card-copied"
        } else {
            "m11-first-card-copied"
        },
        &[GameEventType::CardResolved],
        move |ctx, event| {
            let GameEvent::CardResolved {
                player,
                instance_id,
                def_id,
                ..
            } = event
            else {
                return vec![];
            };
            let Some(this) = ctx.self_.as_ref() else {
                return vec![];
            };
            if *player != ctx.controller || *instance_id == this.id {
                return vec![];
            }
            if !first_played(&*ctx.state, *player, instance_id) {
                return vec![];
            }
            vec![add_to_hand(json_as(json!({
                "defId": def_id,
                "costOverride": param(&*ctx, "setCost"),
                "radiant": radiant,
            })))]
        },
    )
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            triggers: vec![first_card_copied(false)],
            ..Script::default()
        },
        radiant: Script {
            triggers: vec![first_card_copied(true)],
            ..Script::default()
        },
    }
}

// Meditative #11 Double Header — SPEC §8.8 row 11, BUILD M10 row M 11: "From the turn after it
// lands, the first card you play each turn, once it resolves, adds a fresh base copy costing (0)
// to your hand; the second card adds nothing; the turn it lands it copies nothing, being or
// following the turn's first card (R119, R826); a cast first card counts (R70); a countered one
// does not, so the next is the first; a fused card's copy is the fused definition (R77); a card
// that has ceased to exist still copies; the opponent's plays copy nothing; the hand cap burns the
// copy; setCost reads through `param()`; radiant the copy is Radiant".
//
// Core #5 Stockpile is the played first card; Core #27 Blood Ridden Glowy Jelly Bean (cast on draw,
// R70) is the cast one; Classic #17 Counterspell counters; Core #41 Sheepish transforms the victim
// after its resolution; Core #99 Craft a Card builds the fused card (answered as its own tests
// answer: each Discover with its first option).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
    const VANILLA: &str = "core-008"; // a (1) 4/4 with no text.
    const SPARE: &str = "core-010"; // (0) Spell, never played: it only keeps the turn open (R82).
    const JELLY: &str = "core-027"; // (1) Spell, cast on draw.
    const SHEEPISH: &str = "core-041"; // (1) Trap: transform a played Unit into a Sheep Token.
    const SHEEP: &str = "core-t-sheep"; // Sheep Token.
    const COUNTER: &str = "classic-017"; // (2) Trap: counter an opponent's Spell.
    const CRAFT: &str = "core-099"; // (4) Spell: Discover 2 Units, fuse them, the result costs (0).

    /// A side with mana to play and a stocked library, so no turn auto-ends (§2.5) or fatigues;
    /// `extra` replaces its own entries.
    fn side(extra: Value) -> Value {
        crate::merged(
            json!({ "mana": 10, "hand": [SPARE], "library": [VANILLA, VANILLA, VANILLA] }),
            extra,
        )
    }

    /// The hand cards Double Header added: a copy carries a cost override (R71's fresh card).
    fn copies(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        s.hand(player)
            .into_iter()
            .filter(|card| card.cost_override.is_some())
            .collect()
    }

    fn must<T>(value: Option<T>, what: &str) -> T {
        value.unwrap_or_else(|| panic!("expected {what}"))
    }

    /// The open prompt, as its JSON.
    fn open(s: &Scenario) -> Value {
        crate::js(must(s.state().pending.as_ref(), "an open prompt"))
    }

    /// The prompt's first option key, the answer the tests give (as `c099_craft_a_card.rs` does).
    fn first_key(pending: &Value) -> Value {
        must(
            pending["options"].as_array().and_then(|options| options.first()),
            "a first option",
        )["key"]
            .clone()
    }

    #[test]
    fn is_a_4_cost_legendary_field_spell_answering_each_resolution() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.cost, CardCost::Fixed(4));
        assert_eq!(def.type_, CardType::FieldSpell);
        assert_eq!(def.rarity, Rarity::Legendary);
        assert_eq!(
            crate::js(&def.params),
            json!([{ "key": "setCost", "base": 0, "radiant": 0, "better": "down", "step": 1, "min": 0 }])
        );
        let scripts = super::script();
        assert_eq!(scripts.base.triggers.len(), 1);
        assert_eq!(scripts.radiant.triggers.len(), 1);
    }

    mod base {
        use super::*;

        #[test]
        fn r826_the_first_card_is_copied_at_zero_not_radiant() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID], "hand": [STOCKPILE, SPARE] })),
                "p2": side(json!({})),
            }));
            s.play(STOCKPILE, json!({}));
            let copies = copies(&s, P1);
            assert_eq!(copies.len(), 1);
            assert_eq!(copies[0].def_id, STOCKPILE);
            assert_eq!(copies[0].cost_override, Some(0));
            assert!(!copies[0].radiant);
        }

        #[test]
        fn r826_the_second_card_adds_nothing() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID], "hand": [STOCKPILE, VANILLA, SPARE] })),
                "p2": side(json!({})),
            }));
            s.play(STOCKPILE, json!({}));
            s.play(VANILLA, json!({}));
            let copies = copies(&s, P1);
            assert_eq!(copies.len(), 1);
            assert_eq!(copies[0].def_id, STOCKPILE);
        }

        #[test]
        fn r826_nothing_the_turn_it_lands() {
            let mut s = scenario(json!({
                "p1": side(json!({ "hand": [ID, STOCKPILE, SPARE] })),
                "p2": side(json!({})),
            }));
            // Its own play is the turn's first card, which it does not answer (R119).
            s.play(ID, json!({ "zone": 1 }));
            s.play(STOCKPILE, json!({}));
            assert!(copies(&s, P1).is_empty());
        }

        #[test]
        fn r826_a_cast_on_draw_first_card_counts() {
            let mut s = scenario(json!({
                "p1": side(json!({
                    "backrow": [ID],
                    "hand": [SPARE],
                    "health": 30,
                    "library": [JELLY, VANILLA, VANILLA],
                })),
                "p2": side(json!({})),
            }));
            // The draw casts the Jelly Bean free (R70): the turn's first card, though never in hand.
            s.start_turn();
            s.expect_in_zone(JELLY, "graveyard");
            s.expect_health(P1, 25);
            let copies = copies(&s, P1);
            assert_eq!(copies.len(), 1);
            assert_eq!(copies[0].def_id, JELLY);
            assert_eq!(copies[0].cost_override, Some(0));
            assert!(!copies[0].radiant);
        }

        #[test]
        fn r826_a_countered_card_was_never_played_so_the_next_is_first() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID], "hand": [STOCKPILE, VANILLA, SPARE] })),
                "p2": side(json!({
                    "backrow": [{ "def": COUNTER, "faceUp": false, "lane": 2 }],
                })),
            }));
            s.play(STOCKPILE, json!({}));
            s.expect_in_zone(STOCKPILE, "graveyard");
            assert!(copies(&s, P1).is_empty());
            s.play(VANILLA, json!({}));
            let copies = copies(&s, P1);
            assert_eq!(copies.len(), 1);
            assert_eq!(copies[0].def_id, VANILLA);
        }

        #[test]
        fn r826_sheepishs_victim_still_copies() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID], "hand": [VANILLA, SPARE] })),
                "p2": side(json!({ "backrow": [{ "def": SHEEPISH, "lane": 1 }] })),
            }));
            s.play(VANILLA, json!({ "zone": 1 }));
            // Sheepish transforms the Unit after its resolution; the copy names the played card.
            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(SHEEP.to_string()));
            let copies = copies(&s, P1);
            assert_eq!(copies.len(), 1);
            assert_eq!(copies[0].def_id, VANILLA);
            assert_eq!(copies[0].cost_override, Some(0));
        }

        #[test]
        fn r826_a_fused_card_copies_its_fused_definition() {
            let mut s = scenario(json!({
                "seed": "double-header-fuse",
                "p1": side(json!({ "backrow": [ID], "hand": [CRAFT, SPARE] })),
                "p2": side(json!({ "field": [VANILLA] })),
            }));
            s.play(CRAFT, json!({}));
            for _ in 0..2 {
                let pending = open(&s);
                s.answer(first_key(&pending));
            }
            let fused = must(
                s.hand(P1).iter().find(|card| card.def_id.starts_with("t-")).cloned(),
                "the fused card in hand",
            );
            let fused_id = fused.id.clone();
            let fused_def = fused.def_id.clone();
            s.end_turn();
            s.end_turn();
            // The fused card's first play on a later turn copies its transient definition (R77).
            s.play(&fused_id, json!({}));
            let copies: Vec<CardInstance> = copies(&s, P1)
                .into_iter()
                .filter(|card| card.def_id == fused_def)
                .collect();
            assert_eq!(copies.len(), 1);
            assert_eq!(copies[0].cost_override, Some(0));
        }

        #[test]
        fn r826_the_opponents_plays_copy_nothing() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": side(json!({ "backrow": [ID] })),
                "p2": side(json!({ "hand": [STOCKPILE, SPARE] })),
            }));
            s.play(STOCKPILE, json!({}));
            assert!(copies(&s, P1).is_empty());
            assert!(copies(&s, P2).is_empty());
        }

        #[test]
        fn r826_a_full_hand_burns_the_copy() {
            let mut hand = vec![json!(STOCKPILE)];
            hand.extend([SPARE; 8].iter().map(|id| json!(id)));
            let mut s = scenario(json!({
                "p1": side(json!({ "hand": hand, "library": [VANILLA, VANILLA, VANILLA] })),
                "p2": side(json!({})),
            }));
            // Nine cards in hand: the play leaves eight, the two draws fill it to ten, and the
            // copy has nowhere to go.
            s.play(STOCKPILE, json!({}));
            let types: Vec<String> = s
                .events()
                .iter()
                .map(|event| {
                    crate::js(event)["type"].as_str().unwrap_or("?").to_string()
                })
                .collect();
            assert_eq!(s.hand(P1).len(), 10);
            assert!(copies(&s, P1).is_empty());
            assert!(
                s.events().iter().any(|event| matches!(event, GameEvent::Burned { .. })),
                "events: {types:?}"
            );
        }

        #[test]
        fn r386_set_cost_reads_through_param_a_nerf_makes_it_1() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID], "hand": [STOCKPILE, SPARE] })),
                "p2": side(json!({})),
            }));
            assert_eq!(crate::degrade_number(&mut s, ID, "setCost"), 1);
            s.play(STOCKPILE, json!({}));
            let copies = copies(&s, P1);
            assert_eq!(copies.len(), 1);
            assert_eq!(copies[0].cost_override, Some(1));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r826_the_copy_is_radiant() {
            let mut s = scenario(json!({
                "p1": side(json!({
                    "backrow": [{ "def": ID, "radiant": true }],
                    "hand": [STOCKPILE, SPARE],
                })),
                "p2": side(json!({})),
            }));
            s.play(STOCKPILE, json!({}));
            let copies = copies(&s, P1);
            assert_eq!(copies.len(), 1);
            assert_eq!(copies[0].def_id, STOCKPILE);
            assert_eq!(copies[0].cost_override, Some(0));
            assert!(copies[0].radiant);
        }
    }
}
