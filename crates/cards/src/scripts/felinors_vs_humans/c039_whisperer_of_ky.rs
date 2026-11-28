//! SPEC §8.14 FvH #39 Whisperer of KY ((4) Unit, KY, Legendary; 5/8 → 10/16), a card of the Felinors vs
//! Humans set (R5235000).
//!   Base and Radiant: "Cry: Shuffle {copies} of each Spell in your deck into your deck, with Cast on
//!   draw." (1; Radiant 3)
//!
//! Cry (R1), FV2 #39's reading (R5256001): the Spells in its controller's deck as the Cry runs, read once
//! (`for_each_card`, R113: the copies are Spells too), one `shuffle_into` each with `copy_of` (a copy, its
//! tuning riding, R57, and its face kept), `count` the declared `copies` and `enchantments: [castOnDraw]`
//! (R5238004); the deck cap holds, so copies past it are not added (R80), and cast-on-draw chains stop at
//! the engine's cap (R58). A deck with no Spell gets nothing.

use jackioh_engine::effects::{ForEachCardArgs, for_each_card, shuffle_into};
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-039";

/// One Spell of the deck as the read keeps it: "<instanceId> <radiant> <defId>", the def id last since a
/// stand-in's holds spaces.
fn entry(card: &CardInstance) -> String {
    format!("{} {} {}", card.id, card.radiant, card.def_id)
}

/// The Spells in the controller's deck now, as entries.
fn spells_in_deck(ctx: &mut EffectContext<'_>) -> Vec<String> {
    zone_cards(&*ctx.state, ctx.controller, OffFieldZone::Library)
        .iter()
        .filter(|card| card_type_of(&*ctx.state, card) == CardType::Spell)
        .map(entry)
        .collect()
}

/// `copies` copies of the Spell an entry names, each with Cast on draw.
fn shuffle_copies(entry: &str, copies: i32) -> Effect {
    let mut parts = entry.splitn(3, ' ');
    let id = parts.next().unwrap_or_default();
    let radiant = parts.next() == Some("true");
    let def_id = parts.next().unwrap_or_default();
    shuffle_into(json_as(json!({
        "defId": def_id,
        "count": copies,
        "copyOf": id,
        "radiant": radiant,
        "enchantments": [{ "kind": "castOnDraw" }],
    })))
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            let copies = param(&*ctx, "copies");
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(spells_in_deck),
                each: Arc::new(move |entry: &str| shuffle_copies(entry, copies)),
            })]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 3 is its declared `copies`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #39 Whisperer of KY — SPEC §8.14 row 39 (R5238004, R57, R80, R1748000).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::weapon_board;

    const FILLER: &str = "core-005"; // Stockpile: a Spell, each side's spare card (§2.5).
    const SPELL: &str = "stand-in (0) Spell";
    const UNIT: &str = "stand-in (1) 1/1";

    /// p1 plays the Whisperer (`radiant` its face) over `library`.
    fn whispered(seed: &str, radiant: bool, library: Value) -> Scenario {
        let mut s = weapon_board(
            seed,
            json!({ "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": library }),
            json!({ "hand": [FILLER] }),
        );
        s.play(ID, json!({}));
        s
    }

    fn casts_on_draw(card: &CardInstance) -> bool {
        card.enchantments.as_ref().is_some_and(|all| all.contains(&Enchantment::CastOnDraw))
    }

    /// The copies the Cry shuffled in: p1's deck cards with Cast on draw, by def id and face.
    fn copies(s: &Scenario) -> Vec<(String, bool)> {
        let mut found: Vec<(String, bool)> = s
            .pile(P1, "library")
            .into_iter()
            .filter(casts_on_draw)
            .map(|card| (card.def_id, card.radiant))
            .collect();
        found.sort();
        found
    }

    mod base {
        use super::*;

        #[test]
        fn r5256001_r5238004_one_copy_of_each_spell_in_your_deck_with_cast_on_draw() {
            let s = whispered(
                "fvh039-base",
                false,
                json!([SPELL, UNIT, { "def": FILLER, "radiant": true }]),
            );
            assert_eq!(s.pile(P1, "library").len(), 3 + 2);
            assert_eq!(
                copies(&s),
                vec![(FILLER.to_string(), true), (SPELL.to_string(), false)],
                "no Unit is copied, and a Radiant Spell's copy is Radiant"
            );
        }

        #[test]
        fn r5256001_a_deck_with_no_spell_gets_nothing() {
            let s = whispered("fvh039-none", false, json!([UNIT, UNIT]));
            assert_eq!(s.pile(P1, "library").len(), 2);
            assert!(copies(&s).is_empty());
        }

        #[test]
        fn r1748000_the_count_moves_with_a_buff() {
            let mut s = weapon_board(
                "fvh039-buff",
                json!({ "hand": [ID, FILLER], "library": [SPELL] }),
                json!({ "hand": [FILLER] }),
            );
            assert_eq!(crate::upgrade_number(&mut s, ID, "copies"), 2);
            s.play(ID, json!({}));
            assert_eq!(copies(&s), vec![(SPELL.to_string(), false); 2]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5256001_r5238004_three_copies_of_each_spell_in_your_deck() {
            let s = whispered("fvh039-radiant", true, json!([SPELL, UNIT]));
            assert_eq!(s.pile(P1, "library").len(), 2 + 3);
            assert_eq!(copies(&s), vec![(SPELL.to_string(), false); 3]);
        }
    }
}
