//! SPEC §8.14 FvH #125 Plagiarized: Fingerprinter, a frozen copy of IA #175 Fingerprinter (R2195003, FV4), a
//! card of the held Felinors vs Humans set, built by #5235.
//!
//! The code is a copy of the IA card's as it stood when the set was founded, not a re-export, so a later IA
//! patch does not move it (R2195003). Its `ID` is its own, so every read of it names this card; it is named
//! `Plagiarized: Fingerprinter`, tagged Plagiarized only, and a token it makes stays the IA token. Its change
//! from IA #175: none. The IA file's notes, as they stood:
//!
//! SPEC §8.13 IA #175 Fingerprinter ((1) Unit, Common, 1/3 → 2/6), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Add a copy of a random Spell in your opponent's hand to your hand."
//!   Radiant: "Cry: Look at your opponent's hand. Add a copy of a card of your choice in it to your hand."
//!
//! Built by C8 (#4592). R4592000, R4592002, R4592004.

use jackioh_engine::effects::{ChooseFromHandArgs, add_to_hand, choose_from_hand};
use jackioh_engine::faces::card_type_of;
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-125";

const STEP_PICK: &str = "pick";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            let opp = opponent_of(ctx.controller);
            let spells: Vec<String> = zone_cards(&*ctx.state, opp, OffFieldZone::Hand)
                .iter()
                .filter(|c| card_type_of(&*ctx.state, c) == CardType::Spell)
                .map(|c| c.id.clone())
                .collect();
            if let Some(picked) = ctx.rng.pick(&spells) {
                vec![add_to_hand(json_as(json!({ "copyOf": picked })))]
            } else {
                vec![]
            }
        })),
        ..Script::default()
    };

    let radiant = Script {
        cry: Some(hook(|_ctx| {
            vec![choose_from_hand(ChooseFromHandArgs {
                step: STEP_PICK.into(),
                count: Some(1),
                of: Some(PlayerSpec::Enemy),
                by: Some(PlayerSpec::SelfSide),
                prompt: Some("Choose a card from your opponent's hand".into()),
                ..Default::default()
            })]
        })),
        resume: IndexMap::from([(
            STEP_PICK,
            hook(|ctx| {
                let Some(Selection::Instance { instance_id }) = ctx.targets.first() else {
                    return vec![];
                };
                vec![add_to_hand(json_as(json!({ "copyOf": instance_id })))]
            }),
        )]),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// FvH #125 Plagiarized: Fingerprinter — SPEC §8.14 row 125; IA #175's tests (SPEC §8.13 row 175), on this
// card.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "core-019";
    const SPELL: &str = "core-016";

    fn board(radiant: bool, opp_hand: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 },
            "p2": { "hand": opp_hand },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r4592001_adds_copy_of_random_spell_in_opponents_hand() {
            let mut s = board(false, json!([SPELL, UNIT]));
            s.play(ID, json!({ "zone": 1 }));
            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 1, "health": 3 }));
            let hand = s.hand(P1);
            assert_eq!(hand.len(), 1);
            assert_eq!(hand[0].def_id, SPELL);
            assert_eq!(s.hand(P2).len(), 2);
        }

        #[test]
        fn r4592001_with_no_spells_in_hand_adds_nothing() {
            let mut s = board(false, json!([UNIT]));
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(s.hand(P1).len(), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4592001_lets_player_choose_any_card_from_opponents_hand() {
            let mut s = board(true, json!([UNIT]));
            s.play(ID, json!({ "zone": 1 }));
            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 2, "health": 6 }));
            assert!(s.state().pending.is_some());
            let pick = match &s.state().pending.as_ref().unwrap().options[0].selection {
                Selection::Instance { instance_id } => instance_id.clone(),
                _ => panic!("expected instance selection"),
            };
            s.answer(json!(pick));
            assert_eq!(s.hand(P1).len(), 1);
            assert_eq!(s.hand(P1)[0].def_id, UNIT);
        }
    }
}
