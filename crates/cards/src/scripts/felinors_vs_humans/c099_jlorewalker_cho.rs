//! SPEC §8.14 FvH #99 Jlorewalker Cho ((2) Unit, Legendary, 0/4 → 0/8), a card of the held Felinors vs
//! Humans set. Built by S10 (#5260).
//!   Base:    "Whenever a player casts a Spell, shuffle a copy of it with Cast on draw into your deck."
//!   Radiant: "Whenever a player casts a Spell, shuffle a Radiant copy of it with Cast on draw into your
//!            deck."
//!
//! A trigger on `cardPlayed` for either player whose played type (`played_type_of`) is Spell, a Cast
//! included (R70); a cast on draw is no play and wakes no trigger (R2179002), so its own copies never feed
//! it. R5260001: `shuffle_into` its controller's deck of the Spell's definition with `copy_of` the played
//! card (R57, a full deck refusing it, R316) and `enchantments` Cast on draw (R5238004): a fresh copy,
//! carrying the played card's Radiant flag on the base face and Radiant on the Radiant face, never its
//! buffs. Two Chos make two copies. Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-099";

/// One copy of each Spell cast.
const COPIES: i32 = 1;

/// "Whenever a player casts a Spell": a play or cast of a Spell, either side's, never a cast on draw.
fn a_spell_was_cast(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    matches!(event, GameEvent::CardPlayed { .. })
        && !cast_on_draw_event(event)
        && played_type_of(ctx.state, event) == Some(CardType::Spell)
}

fn cho(radiant: bool) -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("jlorewalker-cho", &[GameEventType::CardPlayed], move |ctx, event| {
                let GameEvent::CardPlayed { instance_id, def_id, .. } = event else {
                    return vec![];
                };
                // R57: a copy keeps the Radiant flag of the card copied; the Radiant face's is Radiant.
                let copied_radiant = find_instance(ctx.state, instance_id).is_some_and(|card| card.radiant);
                vec![shuffle_into(json_as(json!({
                    "defId": def_id,
                    "count": COPIES,
                    "player": "self",
                    "radiant": radiant || copied_radiant,
                    "copyOf": instance_id,
                    "enchantments": [{ "kind": "castOnDraw" }],
                })))]
            })
            .with_when(a_spell_was_cast),
        ],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: cho(false),
        radiant: cho(true),
    }
}

// FvH #99 Jlorewalker Cho — SPEC §8.14 row 99 (R57, R70, R316, R2179002, R5238004, R5260001).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::weapon_board;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A textless Spell to cast, and a Unit to play.
    const SPELL: &str = "stand-in (0) Spell";
    const UNIT: &str = "stand-in (0) 1/1";

    /// p1's library copies of `def_id`: each one's Radiant flag and whether it casts on draw.
    fn copies(s: &Scenario, def_id: &str) -> Vec<(bool, bool)> {
        s.pile(P1, "library")
            .into_iter()
            .filter(|card| card.def_id == def_id)
            .map(|card| {
                let casts = card
                    .enchantments
                    .iter()
                    .flatten()
                    .any(|enchantment| matches!(enchantment, Enchantment::CastOnDraw));
                (card.radiant, casts)
            })
            .collect()
    }

    /// p1's Cho on the field over an empty deck; each side holds `hand`; `active` takes the turn.
    fn cho_out(radiant: bool, hand: Value, active: &str) -> Scenario {
        let mut s = weapon_board(
            "fvh099",
            json!({
                "field": [{ "def": ID, "radiant": radiant }],
                "hand": hand.clone(),
                "library": [],
                "mana": 10,
            }),
            json!({ "hand": hand, "mana": 10 }),
        );
        if active == "p2" {
            s.end_turn();
        }
        s
    }

    mod base {
        use super::*;

        #[test]
        fn r5260001_your_spell_shuffles_a_copy_with_cast_on_draw_into_your_deck() {
            let mut s = cho_out(false, json!([SPELL]), "p1");
            s.play(SPELL, json!({}));
            assert_eq!(copies(&s, SPELL), [(false, true)]);
        }

        #[test]
        fn r5260001_the_opponents_spell_shuffles_a_copy_into_your_deck() {
            let mut s = cho_out(false, json!([SPELL]), "p2");
            let library = s.pile(P1, "library").len();
            s.play(SPELL, json!({}));
            assert_eq!(copies(&s, SPELL), [(false, true)]);
            assert_eq!(s.pile(P1, "library").len(), library + 1);
            assert!(s.pile(P2, "library").iter().all(|card| card.def_id != SPELL));
        }

        #[test]
        fn a_unit_played_shuffles_nothing() {
            let mut s = cho_out(false, json!([UNIT]), "p1");
            s.play(UNIT, json!({}));
            assert!(s.pile(P1, "library").is_empty());
        }

        #[test]
        fn r5260001_a_copy_of_a_radiant_spell_is_radiant() {
            let mut s = cho_out(false, json!([{ "def": SPELL, "radiant": true }]), "p1");
            s.play(SPELL, json!({}));
            assert_eq!(copies(&s, SPELL), [(true, true)]);
        }

        #[test]
        fn r2179002_a_copy_cast_on_draw_shuffles_no_new_copy() {
            let mut s = cho_out(false, json!([SPELL]), "p1");
            s.play(SPELL, json!({}));
            assert_eq!(copies(&s, SPELL), [(false, true)]);
            // The copy is p1's whole deck: the next draw casts it, which feeds Cho nothing.
            s.start_turn();
            assert_eq!(s.last_count("drawn"), 1);
            assert!(copies(&s, SPELL).is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5260001_your_spell_shuffles_a_radiant_copy_with_cast_on_draw_into_your_deck() {
            let mut s = cho_out(true, json!([SPELL]), "p1");
            s.play(SPELL, json!({}));
            assert_eq!(copies(&s, SPELL), [(true, true)]);
        }

        #[test]
        fn r5260001_the_opponents_spell_shuffles_a_radiant_copy_into_your_deck() {
            let mut s = cho_out(true, json!([SPELL]), "p2");
            s.play(SPELL, json!({}));
            assert_eq!(copies(&s, SPELL), [(true, true)]);
        }
    }
}
