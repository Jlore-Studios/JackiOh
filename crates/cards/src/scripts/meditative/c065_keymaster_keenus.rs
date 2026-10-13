//! M #65 Keymaster Keenus (SPEC §8.8 row 65, R1084, R1085): (4) Unit, Legendary, 1/1 → 2/2.
//!
//! Base:    the 21 keywords, then "This has every keyword."
//! Radiant: the same, then "Death: Give every keyword this has to another random Unit of yours."
//! Engine: the base face is `Script::default()` — the keywords are printed data (R1084). The Radiant
//! face's Death grants the keywords it had as it died (R78's last-known state) to another random
//! Unit of yours (R1085). Equip and Stack are two play options (ME-EQUIP, R1086).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-065";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = Script {
        death: Some(hook(|ctx| {
            let Some(me) = ctx.self_.clone() else {
                return vec![];
            };
            vec![grant_keywords_random_unit(json_as(json!({
                "keywords": keywords_of(ctx.state, &me),
            })))]
        })),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// M #65 Keymaster Keenus — SPEC §8.8 row 65, BUILD M10 row M 65: "Prints the 21 keywords (R1084),
// and the client lists them all; Taunt is dropped while it is Indestructible (R347); played as
// Equip onto your Unit it fuses in (R1086), or with Stack onto a pile; its Armor, Lucky and
// Spell Damage move by Nerf and Buff's X row; destroyed, it survives (Indestructible), so its
// Death needs a Sacrifice or a Tribute; radiant 2/2, and Death: every keyword it had as it died is
// granted to a random other Unit of yours (R1085), none with no other Unit; Reborn returns it, and
// a second death grants again".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const KEENUS: &str = "meditative-065";
    /// (1) 4/4 Unit with no hooks: the Death's receiver.
    const VANILLA: &str = "core-008";
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    /// The 21 printed keywords, in the catalog's order.
    fn printed_kinds() -> Vec<KeywordKind> {
        use KeywordKind::*;
        vec![
            Taunt,
            Armor,
            Rush,
            Charge,
            FirstStrike,
            Poisonous,
            Lifesteal,
            Reborn,
            DivineShield,
            Trample,
            Cleave,
            Pierce,
            Indestructible,
            Immutable,
            Stack,
            Lucky,
            SpellDamage,
            ImmuneToSpells,
            Windfury,
            Deft,
            Equip,
        ]
    }

    mod m65_keymaster_keenus {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r1084_prints_the_21_keywords() {
                crate::register_all();
                // Every engine keyword kind plus Equip, leaving out Can't attack, Brittle,
                // Temporary and Untributable (which only hurt, R1220), Animated and Animated on
                // your turn (a backrow meaning only), and Immune to tribal tag based hate (R940),
                // a keyword the 21 do not name: a later keyword joins only by a patch (R1084). The
                // print order is the brief's, not the enum's, so the whole-set comparison is
                // order-insensitive.
                let excluded = [
                    KeywordKind::CantAttack,
                    KeywordKind::Brittle,
                    KeywordKind::Temporary,
                    KeywordKind::Untributable,
                    KeywordKind::Animated,
                    KeywordKind::AnimatedOnYourTurn,
                    KeywordKind::ImmuneToTribalHate,
                ];
                let expected: Vec<KeywordKind> = KEYWORD_KINDS
                    .iter()
                    .copied()
                    .filter(|kind| !excluded.contains(kind))
                    .collect();
                assert_eq!(printed_kinds().len(), 21);
                for kind in &expected {
                    assert!(printed_kinds().contains(kind), "Keenus prints {kind:?}");
                }
                let def = crate::card_def(KEENUS);
                let kinds: Vec<KeywordKind> =
                    def.base.keywords.iter().map(|keyword| keyword.kind()).collect();
                assert_eq!(kinds, printed_kinds());
                assert_eq!(
                    def.radiant
                        .keywords
                        .iter()
                        .map(|keyword| keyword.kind())
                        .collect::<Vec<_>>(),
                    printed_kinds()
                );
                // Armor, Lucky and Spell Damage print numbered (R386's X row).
                assert_eq!(def.base.keywords[1], Keyword::Armor { n: 1 });
                assert_eq!(def.base.keywords[15], Keyword::Lucky { n: 1 });
                assert_eq!(def.base.keywords[16], Keyword::SpellDamage { n: 1 });
            }

            #[test]
            fn r347_taunt_drops_while_indestructible() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "keenus-taunt",
                    "p1": { "hand": [KEENUS, FILLER], "library": filler(3) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(KEENUS, json!({ "zone": 1 }));

                // Indestructible holds Taunt down (R347): no Taunt among the acting keywords.
                let me = s.card(KEENUS);
                let kinds: Vec<KeywordKind> = keywords_of(s.state(), me)
                    .iter()
                    .map(|keyword| keyword.kind())
                    .collect();
                assert!(kinds.contains(&KeywordKind::Indestructible));
                assert!(!kinds.contains(&KeywordKind::Taunt));
            }

            #[test]
            fn r1086_equip_onto_your_unit_fuses() {
                crate::register_all();
                let _guard = preview_sets(&[SetName::Meditative]);
                let mut s = scenario(json!({
                    "seed": "keenus-equip",
                    "p1": { "mana": 10, "hand": [VANILLA, KEENUS, FILLER], "library": filler(4) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(VANILLA, json!({}));
                s.play(KEENUS, json!({ "zone": 1, "equip": true }));

                // The host is kept and united with Keenus: a 4/4 that is also a 1/1, and Felinor...
                // (Keenus carries no tags, so the host keeps its own).
                let host = s.unit(P1, 1).expect("the fused host");
                let stats = s.stats(host.id.as_str());
                assert_eq!((stats.attack, stats.health), (5, 5));
                assert!(
                    keywords_of(s.state(), &host)
                        .iter()
                        .any(|keyword| keyword.kind() == KeywordKind::Equip)
                );
            }
        }

        /// Apply `effects` as p1's, outside any card, then run the state check.
        fn run_and_check(s: &mut Scenario, effects: &[Effect]) -> Vec<GameEvent> {
            let mut events = Vec::new();
            let (seed, cursor) = (s.state().seed.clone(), s.state().rng_cursor);
            let mut rng = Rng::new(&seed, cursor);
            {
                let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
                {
                    let mut ctx = make_context(
                        &mut sink,
                        None,
                        HookOptions {
                            controller: Some(P1),
                            ..HookOptions::default()
                        },
                    );
                    apply_effects(effects, &mut ctx);
                }
                state_check(&mut sink);
            }
            s.state_mut().rng_cursor = rng.cursor();
            events
        }

        mod base_more {
            use super::*;

            #[test]
            fn played_with_stack_it_goes_on_top_of_a_pile() {
                crate::register_all();
                let _guard = preview_sets(&[SetName::Meditative]);
                let mut s = scenario(json!({
                    "seed": "keenus-stack",
                    "p1": { "mana": 10, "field": [VANILLA], "hand": [KEENUS, FILLER], "library": filler(4) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                let under = s.unit(P1, 1).expect("the Vanilla").id;
                s.play(KEENUS, json!({ "zone": 1 }));
                // Keenus tops lane 1's pile, the Vanilla dormant under it (§3.2 Stack, R13).
                assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(KEENUS.to_string()));
                assert!(matches!(
                    s.card(under.as_str()).zone,
                    Zone::Field {
                        player: PlayerId::P1,
                        row: Row::Units,
                        lane: 1
                    }
                ));
                let stats = s.stats(KEENUS);
                assert_eq!((stats.attack, stats.health), (1, 1), "not fused: Keenus alone");
            }

            #[test]
            fn destroyed_it_survives_indestructible_and_its_death_does_not_fire() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "keenus-destroy",
                    "p1": {
                        "field": [{ "def": KEENUS, "radiant": true }, VANILLA],
                        "hand": [FILLER],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                let id = s.unit(P1, 1).expect("Keenus").id;
                let events = run_and_check(
                    &mut s,
                    &[destroy(json_as(
                        json!({ "target": { "of": "instance", "instanceId": id } }),
                    ))],
                );
                assert_eq!(
                    s.unit(P1, 1).map(|unit| unit.id),
                    Some(id.clone()),
                    "Keenus stands"
                );
                assert!(
                    !events
                        .iter()
                        .any(|event| matches!(event, GameEvent::KeywordGranted { .. })),
                    "no Death: nothing granted"
                );
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_radiant_face_is_2_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "keenus-stats",
                    "p1": { "mana": 10, "hand": [{ "def": KEENUS, "radiant": true }, FILLER], "library": filler(4) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(KEENUS, json!({ "zone": 1 }));
                let stats = s.stats(KEENUS);
                assert_eq!((stats.attack, stats.health), (2, 2));
            }

            /// Carnivorous Cube (Core #22): (3) Unit, "Cry: Tribute one of your other Units".
            const CUBE: &str = "core-022";
            /// The Rock (Core #66): (4) Unit, Tribute 1 — a play cost, paid before the card lands.
            const ROCK: &str = "core-066";

            fn granted_for(s: &Scenario, id: &str) -> usize {
                s.events()
                    .iter()
                    .filter(|event| {
                        matches!(event, GameEvent::KeywordGranted { instance_id, .. } if instance_id == id)
                    })
                    .count()
            }

            /// p1 holds a Radiant Keenus and two Cubes with a Vanilla on the field to receive.
            fn death_board(seed: &str) -> Scenario {
                crate::register_all();
                scenario(json!({
                    "seed": seed,
                    "p1": {
                        "mana": 10,
                        "hand": [{ "def": KEENUS, "radiant": true }, CUBE, CUBE, FILLER],
                        "field": [VANILLA],
                        "library": filler(3),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }))
            }

            fn cube_tribute(s: &mut Scenario, keenus: &str) {
                let meal = s.card(keenus).id.clone();
                s.play(
                    CUBE,
                    json!({ "targets": [{ "pick": "instance", "instanceId": meal }] }),
                );
            }

            /// The units the `keywordGranted` events since `mark` went to, each once, in order.
            fn receivers_since(s: &Scenario, mark: usize) -> Vec<String> {
                let mut out: Vec<String> = Vec::new();
                for event in &s.events()[mark..] {
                    if let GameEvent::KeywordGranted {
                        instance_id,
                        lost: None,
                        ..
                    } = event
                        && !out.contains(instance_id)
                    {
                        out.push(instance_id.clone());
                    }
                }
                out
            }

            /// The keywords Keenus has as it acts now: its 21 but Taunt, which R347 drops while
            /// it is Indestructible.
            fn acting_kinds(s: &Scenario, id: &str) -> Vec<KeywordKind> {
                keywords_of(s.state(), s.card(id))
                    .iter()
                    .map(|keyword| keyword.kind())
                    .collect()
            }

            #[test]
            fn r1085_death_grants_every_keyword_to_another_random_unit() {
                let mut s = death_board("keenus-death");
                s.play(KEENUS, json!({}));
                let keenus = s.card(KEENUS).id.clone();
                let had = acting_kinds(&s, &keenus);
                assert_eq!(had.len(), printed_kinds().len() - 1);
                assert!(!had.contains(&KeywordKind::Taunt));
                // Indestructible survives destroy, so the Death needs a Sacrifice: the Cube's Cry
                // tributes Keenus. One other Unit of p1's (the Vanilla or the Cube) receives
                // every keyword Keenus had as it died (R78): its 21 but Taunt (R347).
                let mark = s.events().len();
                cube_tribute(&mut s, KEENUS);
                let receivers = receivers_since(&s, mark);
                assert_eq!(receivers.len(), 1, "one random Unit receives");
                let receiver = receivers[0].clone();
                assert_ne!(receiver, keenus, "another Unit than Keenus");
                assert_eq!(s.card(receiver.as_str()).controller, P1);
                assert_eq!(granted_for(&s, &receiver), had.len());
                let kinds = acting_kinds(&s, &receiver);
                for kind in &had {
                    assert!(kinds.contains(kind), "the receiver holds {kind:?}");
                }
                // It is Indestructible now, so it has no Taunt either (R347).
                assert!(!kinds.contains(&KeywordKind::Taunt));
                // Reborn returns Keenus.
                assert_eq!(s.card(KEENUS).def_id, KEENUS);
            }

            #[test]
            fn r1085_a_second_death_grants_again() {
                let mut s = death_board("keenus-death-twice");
                s.play(KEENUS, json!({}));
                let first = acting_kinds(&s, KEENUS).len();
                let mark = s.events().len();
                cube_tribute(&mut s, KEENUS);
                let granted: usize = receivers_since(&s, mark)
                    .iter()
                    .map(|receiver| granted_for(&s, receiver))
                    .sum();
                assert_eq!(granted, first);
                // The returned Keenus dies again: its Death fires again, granting every keyword
                // the returned card had as it died to one other Unit of p1's.
                let returned = s.card(KEENUS).id.clone();
                let second = acting_kinds(&s, &returned);
                let mark = s.events().len();
                cube_tribute(&mut s, KEENUS);
                let receivers = receivers_since(&s, mark);
                assert_eq!(receivers.len(), 1, "one random Unit receives again");
                assert_ne!(receivers[0], returned);
                let again = s.events()[mark..]
                    .iter()
                    .filter(|event| matches!(event, GameEvent::KeywordGranted { lost: None, .. }))
                    .count();
                assert_eq!(again, second.len());
            }

            #[test]
            fn r1085_no_other_unit_grants_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "keenus-alone",
                    "p1": {
                        "mana": 10,
                        "hand": [{ "def": KEENUS, "radiant": true }, ROCK, FILLER],
                        "library": filler(3),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(KEENUS, json!({}));
                // The Rock's Tribute cost sacrifices Keenus at step 2, before the Rock lands: the
                // Death finds no other Unit and draws nothing (R129).
                let me = s.card(KEENUS).id.clone();
                s.play(ROCK, json!({ "tributes": [me] }));
                assert!(
                    s.events()
                        .iter()
                        .all(|event| !matches!(event, GameEvent::KeywordGranted { .. }))
                );
                // Reborn still returns Keenus.
                assert_eq!(s.card(KEENUS).def_id, KEENUS);
            }
        }
    }
}
