//! FvH #54 Intrepid initiate (SPEC §8.14 row 54): (1) Unit, Human, Common, 2/3 → 2/6. Built by S08 (#5258).
//!   Base:    "The first time you play a Spell, gain +{attack} Attack and Divine Shield." — attack 3
//!   Radiant: "After you play a Spell, gain +{attack} Attack and Divine Shield." — attack 6
//!
//! A trigger on its controller's `cardPlayed` of a Spell (typed by `played_type_of`, R783005), a cast
//! included, since a cast is a play (R70); while it stands on the field. It runs a permanent `buff` of
//! its Attack by the declared `attack` (§10.4 layer 4), then `grant_keyword` Divine Shield, which brings
//! a spent shield back and leaves a standing one one (§6.1). Base: once per stay on the field, an
//! instance memory mark that leaving the field clears (R78); Radiant: every Spell.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-054";

/// The base face's mark: it has answered its first Spell on this stay.
const ANSWERED: &str = "firstSpell";

/// A Spell its controller played, as the play counts.
fn spell_by_you(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::CardPlayed { player, .. } = event else {
        return false;
    };
    *player == ctx.controller && played_type_of(&*ctx.state, event) == Some(CardType::Spell)
}

fn answered(ctx: &EffectContext<'_>) -> bool {
    recalled(ctx, ANSWERED) == Some(Value::Bool(true))
}

/// "Gain +{attack} Attack and Divine Shield", marked once on the base face.
fn embolden(ctx: &EffectContext<'_>, once: bool) -> Vec<Effect> {
    let mut effects = Vec::new();
    if once {
        effects.push(remember(RememberArgs {
            key: ANSWERED.to_string(),
            value: Value::Bool(true),
        }));
    }
    effects.push(buff(json_as(json!({
        "target": { "of": "self" },
        "attack": param(ctx, "attack"),
    }))));
    effects.push(grant_keyword(GrantKeywordArgs {
        target: TargetSpec::SelfCard,
        keyword: Keyword::DivineShield,
    }));
    effects
}

fn initiate(once: bool) -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("intrepid-initiate", &[GameEventType::CardPlayed], move |ctx, event| {
                if !spell_by_you(ctx, event) || (once && answered(ctx)) {
                    return vec![];
                }
                embolden(ctx, once)
            })
            .with_when(move |ctx, event| spell_by_you(ctx, event) && !(once && answered(ctx))),
        ],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: initiate(true),
        radiant: initiate(false),
    }
}

// FvH #54 Intrepid initiate — SPEC §8.14 row 54.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SPELL: &str = "stand-in (0) Spell";
    const UNIT: &str = "stand-in (0) 1/1";
    const FILLER: &str = "core-005"; // Stockpile: a card in hand

    /// The Initiate on p1's field, `hand` in p1's hand.
    fn ready(radiant: bool, hand: Value) -> Scenario {
        scenario(json!({
            "p1": { "field": [{ "def": ID, "radiant": radiant }], "hand": hand },
            "p2": { "hand": [SPELL, FILLER] },
        }))
    }

    fn shielded(s: &Scenario) -> bool {
        s.stats(ID).keywords.iter().any(|keyword| keyword.kind() == KeywordKind::DivineShield)
            && s.card(ID).divine_shield_spent != Some(true)
    }

    mod base {
        use super::*;

        #[test]
        fn the_first_spell_you_play_gives_3_attack_and_divine_shield() {
            let mut s = ready(false, json!([SPELL, FILLER]));
            let before = s.stats(ID).attack;
            assert!(!shielded(&s));
            s.play(SPELL, json!({}));
            assert_eq!(s.stats(ID).attack, before + 3);
            assert!(shielded(&s));
        }

        #[test]
        fn a_second_spell_gives_nothing_more() {
            let mut s = ready(false, json!([SPELL, SPELL, FILLER]));
            let before = s.stats(ID).attack;
            s.play(SPELL, json!({}));
            let id = s.card(ID).id.clone();
            must(find_instance_mut(s.state_mut(), &id), "the Initiate").divine_shield_spent = Some(true);
            s.play(SPELL, json!({}));
            assert_eq!(s.stats(ID).attack, before + 3);
            assert!(!shielded(&s), "no second shield");
        }

        #[test]
        fn a_unit_play_gives_nothing() {
            let mut s = ready(false, json!([UNIT, FILLER]));
            let before = s.stats(ID).attack;
            s.play(UNIT, json!({}));
            assert_eq!(s.stats(ID).attack, before);
            assert!(!shielded(&s));
        }

        #[test]
        fn the_opponents_spell_gives_nothing() {
            let mut s = ready(false, json!([FILLER]));
            let before = s.stats(ID).attack;
            s.end_turn();
            s.play(SPELL, json!({}));
            assert_eq!(s.stats(ID).attack, before);
            assert!(!shielded(&s));
        }

        #[test]
        fn the_buff_tunes() {
            let mut s = ready(false, json!([SPELL, FILLER]));
            let before = s.stats(ID).attack;
            set_param(s.card_mut(ID), "attack", 1);
            s.play(SPELL, json!({}));
            assert_eq!(s.stats(ID).attack, before + 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn every_spell_you_play_gives_6_attack_and_divine_shield() {
            let mut s = ready(true, json!([SPELL, SPELL, FILLER]));
            let before = s.stats(ID).attack;
            s.play(SPELL, json!({}));
            assert_eq!(s.stats(ID).attack, before + 6);
            assert!(shielded(&s));
            s.play(SPELL, json!({}));
            assert_eq!(s.stats(ID).attack, before + 12);
        }

        #[test]
        fn a_spent_shield_comes_back_with_the_next_spell() {
            let mut s = ready(true, json!([SPELL, SPELL, FILLER]));
            s.play(SPELL, json!({}));
            let id = s.card(ID).id.clone();
            must(find_instance_mut(s.state_mut(), &id), "the Initiate").divine_shield_spent = Some(true);
            assert!(!shielded(&s));
            s.play(SPELL, json!({}));
            assert!(shielded(&s));
        }

        #[test]
        fn a_unit_play_gives_nothing() {
            let mut s = ready(true, json!([UNIT, FILLER]));
            let before = s.stats(ID).attack;
            s.play(UNIT, json!({}));
            assert_eq!(s.stats(ID).attack, before);
        }
    }
}
