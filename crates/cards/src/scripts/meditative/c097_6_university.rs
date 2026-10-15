//! M #97.6 University (SPEC §8.8 row 97.6). (4) Unit, Token (printed Epic), 0/12 → 0/24.
//!   Base:    "Can't attack\nActivate: Buff each of your permanents {times|time|times}."
//!   Radiant: "Can't attack\nActivate: Buff each of your permanents {times|time|times}."
//!   Engine:  "Activate (R384): separate Buffs (Upgrade, R386) on each permanent acting on your side (unit
//!            tops, backrow cards, face-down ones, this one included), each application its own draw; an
//!            Immutable card is left alone (R23); a face-down card's applications are reported as R440
//!            reports hidden ones. Tunes: times 2 ↑ (Radiant 5)."
//!
//! One script serves both faces: only the declared `times` (2, Radiant 5) and the health differ.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-097-6";

/// §8.8: "Activate" is once a turn.
const USES: i32 = 1;

fn study() -> ActivationDecl {
    ActivationDecl {
        id: "study".to_string(),
        label: "Buff your permanents".to_string(),
        uses: ActivationUses::Count(USES),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(|ctx| {
            vec![upgrade(json_as(json!({
                "scope": { "side": "self", "zones": ["field"] },
                "times": param(&*ctx, "times"),
            })))]
        }),
    }
}

pub fn script() -> CardScripts {
    let base = Script {
        activations: vec![study()],
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// M 97.6 University — SPEC §8.8 row 97.6, BUILD M10 row M 97.6: "Can't attack; Activate, once a turn: two
// Buffs on each permanent on your side, itself and face-down cards included (the latter cued per R440), an
// Immutable one unchanged; enemy cards untouched; times reads through `param()`; radiant 0/24 and five
// Buffs each".
//
// A Buff is counted by the `upgraded` events naming each card. Every scenario keeps a 0-cost Spell in each
// hand and spares in the libraries, so R82 does not end the turn after the Activate.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use crate::js;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-010"; // (0) Spell.
    const SPARE: &str = "core-005"; // (1) Spell: library spare.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt, Immutable on its Radiant face.
    const TRAP: &str = "core-060"; // a Trap.

    /// The University in lane 1, a Vanilla in lane 2, a Radiant Menace (Immutable) in lane 3 and a face-down
    /// Trap in backrow lane 1 on p1's side; a Vanilla on p2's.
    fn setup(radiant: bool) -> Scenario {
        crate::scenario(json!({
            "p1": {
                "hand": [FILLER],
                "field": [
                    { "def": ID, "radiant": radiant, "lane": 1 },
                    { "def": VANILLA, "lane": 2 },
                    { "def": MENACE, "radiant": true, "lane": 3 },
                ],
                "backrow": [{ "def": TRAP, "faceUp": false, "lane": 1 }],
                "library": [SPARE, SPARE],
            },
            "p2": { "hand": [FILLER], "field": [VANILLA], "library": [SPARE, SPARE] },
        }))
    }

    /// The `upgraded` events of the last step naming `id`.
    fn upgraded(s: &Scenario, id: &str) -> Vec<GameEvent> {
        s.last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Upgraded { instance_id, .. } if instance_id == id))
            .cloned()
            .collect()
    }

    /// Each p1 card's Buffs, in the order university, vanilla, menace, trap, and p2's vanilla.
    fn tally(s: &Scenario) -> [usize; 5] {
        let id = |card: &CardInstance| card.id.clone();
        let ids = [
            id(s.card(ID)),
            id(&s.unit(P1, 2).expect("p1's Vanilla")),
            id(&s.unit(P1, 3).expect("p1's Menace")),
            id(&s.backrow(P1, 1).expect("p1's Trap")),
            id(&s.unit(P2, 1).expect("p2's Vanilla")),
        ];
        ids.map(|id| upgraded(s, &id).len())
    }

    #[test]
    fn is_a_4_cost_0_12_token_printed_epic_with_one_activate_on_each_face_and_a_declared_times() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.id, "meditative-097-6");
        assert_eq!(js(&def.cost), json!(4));
        assert!(def.token);
        assert_eq!(def.printed_rarity, Some(PrintedRarity::Epic));
        assert_eq!(
            [
                js(&def.base.attack),
                js(&def.base.health),
                js(&def.radiant.attack),
                js(&def.radiant.health)
            ],
            [json!(0), json!(12), json!(0), json!(24)]
        );
        assert_eq!(js(&def.base.keywords), json!([{ "kind": "Can't attack" }]));
        assert_eq!(js(&def.radiant.keywords), json!([{ "kind": "Can't attack" }]));
        assert_eq!(
            js(&def.params),
            json!([{ "key": "times", "base": 2, "radiant": 5, "better": "up", "step": 1, "min": 1 }])
        );
        let scripts = script();
        let uses = |script: &Script| {
            script
                .activations
                .iter()
                .map(|ability| js(&ability.uses))
                .collect::<Vec<_>>()
        };
        assert_eq!(uses(&scripts.base), vec![json!(1)]);
        assert_eq!(uses(&scripts.radiant), vec![json!(1)]);
    }

    mod base {
        use super::*;

        #[test]
        fn r386_buffs_each_of_your_permanents_twice_itself_and_a_face_down_card_included() {
            let mut s = setup(false);
            s.activate(ID, json!({}));
            // The University, the Vanilla and the face-down Trap twice; the Immutable Menace never (R23);
            // the enemy Vanilla never.
            assert_eq!(tally(&s), [2, 2, 0, 2, 0]);
        }

        #[test]
        fn r440_the_face_down_cards_buffs_are_hidden_from_the_opponent() {
            let mut s = setup(false);
            s.activate(ID, json!({}));
            let trap = s.backrow(P1, 1).expect("p1's Trap").id;
            let vanilla = s.unit(P1, 2).expect("p1's Vanilla").id;
            let hidden = upgraded(&s, &trap);
            assert!(!hidden.is_empty());
            for event in &hidden {
                assert_eq!(js(event)["hiddenFrom"], json!(["p2"]));
            }
            for event in &upgraded(&s, &vanilla) {
                assert_eq!(js(event)["hiddenFrom"], Value::Null);
            }
            assert!(!serde_json::to_string(&s.view(P2)).is_ok_and(|view| view.contains(&trap)));
        }

        #[test]
        fn the_times_read_through_param() {
            let mut s = setup(false);
            set_param(s.card_mut(ID), "times", 3);
            s.activate(ID, json!({}));
            assert_eq!(tally(&s), [3, 3, 0, 3, 0]);
        }

        #[test]
        fn r384_once_a_turn() {
            let mut s = setup(false);
            s.activate(ID, json!({}));
            s.expect_refused(|s| s.activate(ID, json!({})));
        }

        #[test]
        fn cant_attack() {
            let mut s = setup(false);
            s.expect_refused(|s| s.attack(ID, "hero"));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn five_buffs_each_and_0_24() {
            let mut s = setup(true);
            s.expect_stats(ID, json!({ "attack": 0, "maxHealth": 24 }));
            s.activate(ID, json!({}));
            assert_eq!(tally(&s), [5, 5, 0, 5, 0]);
        }
    }
}
