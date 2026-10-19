//! M+ #17 Dogpile (SPEC §8.9 row 17, BUILD M11 row M+ 17): (1) Spell, Rare. The designer's "Pile On",
//! renamed because Classic #60 holds the name (R381, R1178002).
//!   Base:    "Deal {damage} damage to an enemy.\nIn your hand or deck: After another Spell of yours that
//!            targeted an enemy resolves, cast this at it." (2)
//!   Radiant: the same, {damage} 4.
//!
//! Played, a targeted Spell: its damage to a declared enemy Unit or the enemy hero (R81). In its
//! owner's hand or deck it answers their Spells that targeted an enemy (R1180005, `targeted.rs`):
//! while one resolves, the enemies it targets — declared, chosen as a cast's or an Echo repeat's
//! choice, or picked in a `target` prompt its own script opens — are kept on it, and at §10.5 step 7
//! the engine names one answering card on its `cardResolved` (`answer`): the leftmost in the hand,
//! else the topmost in the deck, cast at the first of those enemies still an enemy on the field. This
//! card's hand and deck triggers (R464) read the event, and the one named casts itself there for
//! free (§6.3 Cast, R70: a play), aimed, or casts nothing if the aim is no longer a legal pick. Every
//! other copy does nothing, so one answers each Spell; its own cast targeted an enemy, so it can call
//! the next. Tunes: damage 2 ↑ (Radiant 4).

use jackioh_engine::effects::damage;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-017";

/// R1180005: cast this card at the enemy the engine named, when it is the card named.
fn answer() -> TriggerDef {
    // Not a trap, so the condition is read in `run` (only traps consult `when`, R99).
    TriggerDef::new("dogpile", &[GameEventType::CardResolved], |ctx, event| {
        let GameEvent::CardResolved {
            answer: Some(answer), ..
        } = event
        else {
            return vec![];
        };
        if ctx.self_.as_ref().map(|card| card.id.as_str()) != Some(answer.by.as_str()) {
            return vec![];
        }
        vec![cast(json_as(json!({
            "target": { "of": "self" },
            "aimAt": option_key(&answer.at),
            "aimOnly": true,
        })))]
    })
}

fn dogpile() -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit", "hero"] }))],
        cry: Some(hook(|ctx| {
            vec![damage(json_as(json!({
                "to": { "of": "chosen" },
                "amount": param(&*ctx, "damage"),
            })))]
        })),
        static_flags: Some(StaticFlags {
            answers_targeted_spells: Some(true),
            ..StaticFlags::default()
        }),
        hand_triggers: vec![answer()],
        deck_triggers: vec![answer()],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The faces differ only in the declared `damage`, which `param` reads off the running face.
    let base = dogpile();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// M+ #17 Dogpile — SPEC §8.9 row 17, BUILD M11 row M+ 17: "played, 2 damage to a declared enemy Unit
// or hero; in your hand, after your targeted Spell that hit an enemy resolves (declared or chosen in a
// prompt), it is cast free at that enemy for 2 (a play, R70), leaving your hand; from your deck the
// same, the hand's first; one answers each Spell; a second Dogpile answers the first's cast; an
// untargeted Spell, a Spell at your own Unit and the opponent's Spells call nothing; a target gone
// calls nothing; damage reads through `param()`; radiant 4".
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// (1) Spell, Pierce: 4 damage to a declared Unit or hero, either side.
    const TRUE_STRIKE: &str = "core-044";
    /// (2) Field Spell: your next Spell gains Echo +1.
    const TWINSPELL: &str = "core-079";
    /// (1) Spell: draw 2, heal your hero 2.
    const STOCKPILE: &str = "core-005";
    /// (1) Spell: heal a target 9.
    const BOOK_OF_HEAL: &str = "classic-003";
    /// (3) 9/9 Taunt.
    const MENACE: &str = "core-019";
    /// (1) 1/1.
    const TOKEN_MAN: &str = "core-015";
    /// A (1) 4/4 with no text.
    const FILLER: &str = "core-008";
    /// (2) Field Spell: after you play a card, shuffle 3 copies of it into your library.
    const CLONE_MACHINE: &str = "core-033";

    fn game(seed: &str, p1: Value, p2: Value) -> Scenario {
        let mut sides = json!({
            "seed": seed,
            "p1": { "library": [FILLER, FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
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

    fn hero(player: PlayerId) -> Value {
        json!([{ "pick": "hero", "player": player.as_str() }])
    }

    fn instance(card: &CardInstance) -> Value {
        json!([{ "pick": "instance", "instanceId": card.id }])
    }

    fn health(s: &Scenario, player: PlayerId) -> i32 {
        s.state().players[player].hero.health
    }

    fn dogpiles_in(s: &Scenario, player: PlayerId, zone: &str) -> usize {
        s.pile(player, zone).into_iter().filter(|card| card.def_id == ID).count()
    }

    /// Every hit's amount on the target `target_id` names, in order.
    fn hits_on(s: &Scenario, target_id: &str) -> Vec<i32> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage {
                    target_id: hit, amount, ..
                } if hit == target_id => Some(*amount),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn played_it_deals_2_to_a_declared_enemy_unit() {
        let mut s = game("m+017-unit", json!({ "hand": [ID] }), json!({ "field": [MENACE] }));
        let menace = s.unit(P2, 1).expect("the menace");
        s.play(ID, json!({ "targets": instance(&menace) }));
        assert_eq!(s.card(&menace.id).damage, 2);
    }

    #[test]
    fn played_it_deals_2_to_the_enemy_hero() {
        let mut s = game("m+017-hero", json!({ "hand": [ID] }), json!({}));
        s.play(ID, json!({ "targets": hero(P2) }));
        assert_eq!(health(&s, P2), 28);
    }

    #[test]
    fn r1180005_from_your_hand_it_is_cast_at_the_enemy_your_spell_targeted() {
        // A card left in hand, so the turn does not end itself before the log is read (§2.5).
        let mut s = game("m+017-hand", json!({ "hand": [TRUE_STRIKE, ID, FILLER] }), json!({}));
        let dogpile = s.card(ID).id.clone();
        s.play(TRUE_STRIKE, json!({ "targets": hero(P2) }));
        assert_eq!(hits_on(&s, "hero-p2"), vec![4, 2]);
        assert_eq!(dogpiles_in(&s, P1, "hand"), 0);
        assert_eq!(dogpiles_in(&s, P1, "graveyard"), 1);
        // A cast is a play (R70).
        assert!(was_played_this_turn(s.state(), P1, dogpile.as_str()));
        assert_eq!(s.state().players.p1.mana.current, 3, "the cast was free");
    }

    #[test]
    fn r1180005_an_enemy_chosen_in_a_prompt_calls_it_too() {
        let mut s = game(
            "m+017-prompt",
            json!({ "hand": [TWINSPELL, TRUE_STRIKE, ID], "field": [MENACE], "mana": 9 }),
            json!({}),
        );
        s.play(TWINSPELL, json!({}));
        let menace = s.unit(P1, 1).expect("the menace");
        // Declared at your own Unit; the Echo repeat's prompt picks the enemy hero.
        s.play(TRUE_STRIKE, json!({ "targets": instance(&menace) }));
        s.answer(hero(P2));
        assert_eq!(hits_on(&s, "hero-p2"), vec![4, 2]);
        assert_eq!(dogpiles_in(&s, P1, "graveyard"), 1);
    }

    #[test]
    fn r1180005_from_your_deck_too() {
        let mut s = game(
            "m+017-deck",
            json!({ "hand": [TRUE_STRIKE], "library": [ID, FILLER, FILLER] }),
            json!({}),
        );
        s.play(TRUE_STRIKE, json!({ "targets": hero(P2) }));
        assert_eq!(hits_on(&s, "hero-p2"), vec![4, 2]);
        assert_eq!(dogpiles_in(&s, P1, "library"), 0);
        assert_eq!(dogpiles_in(&s, P1, "graveyard"), 1);
    }

    #[test]
    fn r1180005_the_hand_answers_first_and_the_deck_answers_its_cast() {
        let mut s = game(
            "m+017-hand-first",
            json!({ "hand": [TRUE_STRIKE, ID], "library": [FILLER, ID, FILLER] }),
            json!({}),
        );
        let in_hand = s.hand(P1)[1].id.clone();
        s.play(TRUE_STRIKE, json!({ "targets": hero(P2) }));
        let first_answer = s.events().iter().find_map(|event| match event {
            GameEvent::CardResolved {
                answer: Some(answer), ..
            } => Some(answer.by.clone()),
            _ => None,
        });
        assert_eq!(first_answer, Some(in_hand));
        assert_eq!(hits_on(&s, "hero-p2"), vec![4, 2, 2]);
        assert_eq!(dogpiles_in(&s, P1, "graveyard"), 2);
    }

    #[test]
    fn r1180005_one_answers_each_spell_and_a_second_answers_the_firsts_cast() {
        let mut s = game("m+017-chain", json!({ "hand": [TRUE_STRIKE, ID, ID] }), json!({}));
        let first = s.hand(P1)[1].id.clone();
        let second = s.hand(P1)[2].id.clone();
        s.play(TRUE_STRIKE, json!({ "targets": hero(P2) }));
        let answered: Vec<(String, String)> = s
            .events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::CardResolved {
                    instance_id,
                    answer: Some(answer),
                    ..
                } => Some((instance_id.clone(), answer.by.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(answered.len(), 2);
        assert_eq!(answered[0].1, first);
        assert_eq!(answered[1], (first, second));
        assert_eq!(hits_on(&s, "hero-p2"), vec![4, 2, 2]);
    }

    #[test]
    fn r1180005_an_untargeted_spell_calls_nothing() {
        let mut s = game("m+017-untargeted", json!({ "hand": [STOCKPILE, ID] }), json!({}));
        s.play(STOCKPILE, json!({}));
        assert_eq!(dogpiles_in(&s, P1, "hand"), 1);
    }

    #[test]
    fn r1180005_a_spell_at_your_own_unit_calls_nothing() {
        let mut s = game(
            "m+017-own",
            json!({ "hand": [BOOK_OF_HEAL, ID], "field": [{ "def": MENACE, "damage": 3 }] }),
            json!({}),
        );
        let menace = s.unit(P1, 1).expect("the menace");
        s.play(BOOK_OF_HEAL, json!({ "targets": instance(&menace) }));
        assert_eq!(dogpiles_in(&s, P1, "hand"), 1);
    }

    #[test]
    fn r1180005_the_opponents_spells_call_nothing() {
        let mut s = game(
            "m+017-theirs",
            json!({ "hand": [ID] }),
            json!({ "hand": [TRUE_STRIKE, FILLER] }),
        );
        s.end_turn();
        s.play(TRUE_STRIKE, json!({ "targets": hero(P1) }));
        assert_eq!(dogpiles_in(&s, P1, "hand"), 1);
        assert_eq!(health(&s, P2), 30);
    }

    #[test]
    fn r1180005_a_target_gone_calls_nothing() {
        let mut s = game(
            "m+017-gone",
            json!({ "hand": [TRUE_STRIKE, ID] }),
            json!({ "field": [TOKEN_MAN] }),
        );
        let victim = s.unit(P2, 1).expect("the 1/1");
        s.play(TRUE_STRIKE, json!({ "targets": instance(&victim) }));
        s.expect_in_zone(victim.id.as_str(), "graveyard");
        assert_eq!(dogpiles_in(&s, P1, "hand"), 1);
    }

    #[test]
    fn damage_reads_through_param() {
        let mut s = game("m+017-param", json!({ "hand": [TRUE_STRIKE, ID] }), json!({}));
        let dogpile = s.card(ID).id.clone();
        set_param(s.card_mut(&dogpile), "damage", 3);
        s.play(TRUE_STRIKE, json!({ "targets": hero(P2) }));
        assert_eq!(hits_on(&s, "hero-p2"), vec![4, 3]);
    }

    #[test]
    fn radiant_it_deals_4() {
        let mut s = game(
            "m+017-radiant",
            json!({ "hand": [TRUE_STRIKE, { "def": ID, "radiant": true }] }),
            json!({}),
        );
        s.play(TRUE_STRIKE, json!({ "targets": hero(P2) }));
        assert_eq!(hits_on(&s, "hero-p2"), vec![4, 4]);
    }

    #[test]
    fn radiant_played_it_deals_4() {
        let radiant = json!({ "hand": [{ "def": ID, "radiant": true }] });
        let mut s = game("m+017-radiant-played", radiant, json!({}));
        s.play(ID, json!({ "targets": hero(P2) }));
        assert_eq!(health(&s, P2), 26);
    }

    /// R1180006, fuzz seeds 934, 2632, 2995 and 5204 (#1216): Core #33 Unstable Clone Machine shuffles 3
    /// copies of every answered cast into the library, so each answer mints its successor; the armored
    /// hero survives them all, the chain still ends at its cap of casts, and the turn can pass.
    #[test]
    fn r1180006_clone_machine_answers_end_at_the_chain_cap() {
        let mut s = game(
            "m+017-clone-machine",
            json!({ "hand": [FILLER, FILLER], "armor": 1000 }),
            json!({
                "hand": [TRUE_STRIKE, ID, FILLER],
                "library": [ID, FILLER, FILLER],
                "field": [CLONE_MACHINE],
            }),
        );
        s.end_turn();
        s.play(TRUE_STRIKE, json!({ "targets": hero(P1) }));
        let cast = s
            .events()
            .iter()
            .filter(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id == ID))
            .count();
        let cap = usize::try_from(jackioh_engine::config::SPELL_ANSWER_CHAIN_CAP).unwrap_or(0);
        assert_eq!(cast, cap);
        assert!(s.state().result.is_none());
        s.end_turn();
    }
}
