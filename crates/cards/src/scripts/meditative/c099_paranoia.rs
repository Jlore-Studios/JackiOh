//! M #99 Paranoia (SPEC §8.8 row 99): (2) Field Spell, Epic.
//!
//! Base:    "Aura: You may play your Spells face-down as Traps. Each reveals at the time you choose
//!           as you play it: the end of this turn, the start of your next turn, or the end of your
//!           next turn.
//!           Cry: Draw {draw|card|cards}."
//! Radiant: "Aura: You may play your Spells face-down as Traps. Each reveals at the time you choose
//!           as you play it: the end of this turn, the start of your next turn, or the end of your
//!           next turn. Each has Echo +{echo}.
//!           Cry: Draw {draw|card|cards}."
//! Engine: ME-ALTPLAY permission for Spells (Radiant adds Echo), Cry draws.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-099";

fn face_down_spells(radiant: bool) -> FaceDownPlayHook {
    Arc::new(move |args| {
        let mut permission = FaceDownPlayPermission {
            spells: Some(true),
            ..FaceDownPlayPermission::default()
        };
        if radiant {
            permission.echo = Some(param(&args, "echo"));
        }
        vec![permission]
    })
}

fn paranoia(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let count = param(&*ctx, "draw");
            vec![draw(json_as(json!({ "count": count })))]
        })),
        face_down_play: Some(face_down_spells(radiant)),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: paranoia(false),
        radiant: paranoia(true),
    }
}

// M #99 Paranoia — SPEC §8.8 row 99, BUILD M10 row M 99.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PARANOIA: &str = "meditative-099";
    const STOCKPILE: &str = "core-005"; // Draw, heal; no declared targets.
    const ECLIPSE: &str = "core-035"; // Deal 3 to a target.
    const VANILLA: &str = "core-008";
    const SOLARIUS: &str = "classicplus-038"; // Spell Damage +2.

    /// Answer the open Target prompt at the enemy hero.
    fn answer_hero(s: &mut Scenario) {
        let pending = s.state().pending.clone().expect("an open prompt");
        let picked = pending
            .options
            .iter()
            .find(|option| option.selection == Selection::Hero { player: P2 })
            .expect("the enemy hero")
            .selection
            .clone();
        s.answer(json!([picked]));
    }

    fn set_plays(s: &Scenario, id: &str) -> Vec<ActionBody> {
        legal_actions(s.state(), P1)
            .into_iter()
            .filter(|action| {
                matches!(
                    action,
                    ActionBody::Play { instance_id, face_down: Some(_), .. }
                    if instance_id == id
                )
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn cry_draws_one() {
            let mut s = scenario(json!({
                "seed": "paranoia-draw",
                "p1": { "mana": 8, "hand": [PARANOIA], "library": [VANILLA, VANILLA] },
                "p2": { "hand": [VANILLA], "library": [VANILLA] },
            }));
            let before = s.hand(P1).len();
            let paranoia = s.hand(P1).iter().find(|card| card.def_id == PARANOIA).cloned().unwrap();
            s.play(&paranoia.id, json!({}));
            // Played one, drew one.
            assert_eq!(s.hand(P1).len(), before);
            assert!(s.state().pending.is_none());
        }

        #[test]
        fn r1044_a_spell_offers_three_timings_a_unit_none() {
            let s = scenario(json!({
                "seed": "paranoia-timings",
                "p1": {
                    "mana": 8,
                    "hand": [PARANOIA, STOCKPILE, VANILLA],
                    "field": [{ "def": PARANOIA, "lane": 1 }],
                    "library": [VANILLA],
                },
                "p2": { "hand": [VANILLA], "library": [VANILLA] },
            }));
            let spell = s.hand(P1).iter().find(|card| card.def_id == STOCKPILE).cloned().unwrap();
            let mut timings: Vec<String> = set_plays(&s, &spell.id)
                .iter()
                .map(|action| match action {
                    ActionBody::Play { face_down: Some(timing), .. } => format!("{timing:?}"),
                    _ => unreachable!(),
                })
                .collect();
            timings.sort();
            timings.dedup();
            assert_eq!(timings, vec!["EndOfNextTurn", "EndOfThisTurn", "StartOfNextTurn"]);
            // A Unit gets no Spell permission.
            let unit = s.hand(P1).iter().find(|card| card.def_id == VANILLA).cloned().unwrap();
            assert_eq!(set_plays(&s, &unit.id), vec![]);
        }

        #[test]
        fn r1044_no_spell_damage() {
            let mut s = scenario(json!({
                "seed": "paranoia-nodamage",
                "p1": {
                    "mana": 8,
                    "hand": [ECLIPSE],
                    "field": [{ "def": PARANOIA, "lane": 1 }, { "def": SOLARIUS, "lane": 1 }],
                    "library": [VANILLA],
                },
                "p2": { "hand": [VANILLA], "library": [VANILLA] },
            }));
            let eclipse = s.hand(P1).iter().find(|card| card.def_id == ECLIPSE).cloned().unwrap();
            let health = s.state().players.p2.hero.health;
            // Setting the Spell deals nothing: its text waits for the reveal.
            s.play(&eclipse.id, json!({ "zone": 2, "row": "backrow", "faceDown": "endOfThisTurn" }));
            assert_eq!(s.state().players.p2.hero.health, health);
            // Revealed, it deals its 3 to the chosen hero: its damage is a Trap's, so Solarius's
            // Spell Damage +2 adds nothing (MD-E17).
            s.end_turn();
            answer_hero(&mut s);
            assert_eq!(s.state().players.p2.hero.health, health - 3);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1045_radiant_gives_echo_plus_one() {
            let mut s = scenario(json!({
                "seed": "paranoia-echo",
                "p1": {
                    "mana": 8,
                    // A Vanilla still to play, so the set does not end the turn by itself (R82)
                    // and `end_turn` below is what reveals it.
                    "hand": [ECLIPSE, VANILLA],
                    "field": [{ "def": PARANOIA, "lane": 1, "radiant": true }],
                    "library": [VANILLA],
                },
                "p2": { "hand": [VANILLA], "library": [VANILLA] },
            }));
            let eclipse = s.hand(P1).iter().find(|card| card.def_id == ECLIPSE).cloned().unwrap();
            let health = s.state().players.p2.hero.health;
            s.play(&eclipse.id, json!({ "zone": 2, "row": "backrow", "faceDown": "endOfThisTurn" }));
            s.end_turn();
            // First resolution asks with a prompt; so does the Echo repeat.
            assert_eq!(s.state().pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Target));
            answer_hero(&mut s);
            assert_eq!(s.state().pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Target));
            answer_hero(&mut s);
            assert_eq!(s.state().players.p2.hero.health, health - 6);
        }
    }
}
