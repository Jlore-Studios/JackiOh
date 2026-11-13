//! M #65 Keymaster Keenus (SPEC §8.8 row 65, R1086, R781004): (4) Unit, Legendary, 1/1 → 2/2.
//!
//! Both faces: "Equip\nAura: Has {keywords|random combat keyword|random combat keywords}." — 5 on the
//! base face, 10 on the Radiant face (the declared number `keywords`, R386).
//! Engine: Equip is printed (ME-EQUIP, R1086). §10.4's layers read with no rng, so the Aura's keywords
//! are drawn at an event and kept on the card: as it arrives on the field (played, summoned, Flickered,
//! back through Reborn) and as an Equip play fuses it into a Unit, which draws its own. They come from
//! the combat keyword pool (R21's), none the card has and no Taunt beside Indestructible, into one
//! ordered list the card keeps until it leaves the field (R78). The Aura gives the first `keywords` of
//! that list (`aura_combat_keywords`): a Buff of the number, a number set higher or the Radiant face
//! draws only the rest (`combat_keywords_follow`), a lower number takes the newest off, and the number
//! back returns them (R3778000). The Aura reads them back (`conditionalKeywords`), so a Vanilla takes
//! them with the rest of its text (R781004). It printed every keyword and, Radiant, a Death that gave
//! them away until patch v0.3.7 (R1084, R1085).

use jackioh_engine::effects::{
    DrawCombatKeywordsArgs, aura_combat_keywords, draw_combat_keywords, drawn_combat_keywords,
    top_up_combat_keywords,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-065";

/// R781004: this card arrived on the field (played, summoned, Flickered, back through Reborn).
fn arrived(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    let Some(me) = ctx.live_self() else {
        return false;
    };
    matches!(event, GameEvent::Summoned { instance_id, .. } if *instance_id == me.id)
}

fn draw_on_arrival(top_up: bool) -> TriggerDef {
    TriggerDef::new(
        "keenus-combat-keywords",
        &[GameEventType::Summoned],
        move |ctx, event| {
            if !arrived(ctx, event) {
                return vec![];
            }
            vec![draw_for(param(&*ctx, "keywords"), top_up)]
        },
    )
    .with_when(|ctx, event| arrived(ctx, event))
}

/// R3778000: `top_up` draws what the number asks beyond the list; the kept script's draw keeps any it
/// holds (R781004).
fn draw_for(count: i32, top_up: bool) -> Effect {
    let args = DrawCombatKeywordsArgs { count, key: None };
    if top_up {
        top_up_combat_keywords(args)
    } else {
        draw_combat_keywords(args)
    }
}

fn keenus(top_up: bool) -> Script {
    Script {
        triggers: vec![draw_on_arrival(top_up)],
        // R3778000: a Buff, a number set or the Radiant face that raises its number draws the rest.
        combat_keywords_follow: top_up.then_some("keywords"),
        // An Equip play fuses this into a Unit, which carries the Aura and draws its own (R1086).
        fused_into: Some(hook(move |ctx| vec![draw_for(param(&*ctx, "keywords"), top_up)])),
        conditional_keywords: Some(if top_up {
            read_hook(|args| aura_combat_keywords(&args, "keywords", None))
        } else {
            read_hook(|args| drawn_combat_keywords(args.self_, None))
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // One script: the Radiant face's 10 is its declared number, which `param` reads off the face.
    CardScripts {
        base: keenus(true),
        radiant: keenus(true),
    }
}

/// R767000: through v0.6.0b the Aura drew once as it arrived and kept its keywords whatever its
/// number did (R781004); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_0b() -> CardScripts {
    CardScripts {
        base: keenus(false),
        radiant: keenus(false),
    }
}

// M #65 Keymaster Keenus — SPEC §8.8 row 65, BUILD M10 row M 65.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::filler;
    use crate::js;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const KEENUS: &str = "meditative-065";
    /// (1) 4/4 Unit with no hooks: an Equip host.
    const VANILLA: &str = "core-008";
    /// (0) Spell: Vanilla a Unit.
    const SILENCE: &str = "classicplus-009";
    /// (4) Field Spell: Aura: your Units have +4 attack, Rush and First Strike.
    const WEAPONS: &str = "core-014";
    /// (4) 10/10 Unit with Indestructible: an Equip host that keeps Taunt out (R347), and not Immutable
    /// as State of the Game is since #2179.
    const THE_ROCK: &str = "core-066";
    const FILLER: &str = "core-005";
    /// (0) Spell: a Unit gains Reborn; Radiant, then it is made Radiant.
    const NEW_WRAPS: &str = "classicplus-010";
    use crate::BOOK_OF_BUFF;

    fn played(seed: &str, radiant: bool, p1: Value) -> Scenario {
        crate::register_all();
        let mut hand = vec![json!({ "def": KEENUS, "radiant": radiant }), json!(FILLER)];
        if let Some(extra) = p1.get("hand").and_then(Value::as_array) {
            hand.extend(extra.iter().cloned());
        }
        let mut p1 = p1;
        p1["hand"] = json!(hand);
        p1["library"] = filler(4);
        p1["mana"] = json!(10);
        let mut s = scenario(json!({
            "seed": seed,
            "p1": p1,
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(KEENUS, json!({ "zone": 1 }));
        s
    }

    /// The combat keywords the Aura holds now, as kinds in draw order.
    fn drawn(s: &Scenario, id: &str) -> Vec<KeywordKind> {
        drawn_combat_keywords(s.card(id), None)
            .iter()
            .map(Keyword::kind)
            .collect()
    }

    fn acting(s: &Scenario, id: &str) -> Vec<KeywordKind> {
        keywords_of(s.state(), s.card(id)).iter().map(Keyword::kind).collect()
    }

    fn pool_kinds() -> Vec<KeywordKind> {
        random_pool_keywords().iter().map(Keyword::kind).collect()
    }

    /// R3778000: the drawn keywords the card acts with, in draw order: what its Aura gives.
    fn aura(s: &Scenario, id: &str) -> Vec<KeywordKind> {
        let acting = acting(s, id);
        drawn(s, id).into_iter().filter(|kind| acting.contains(kind)).collect()
    }

    /// Keenus on the field, the Book of Buff in hand; its id.
    fn with_book(seed: &str) -> (Scenario, String) {
        let s = played(seed, false, json!({ "hand": [BOOK_OF_BUFF] }));
        let id = s.card(KEENUS).id.clone();
        (s, id)
    }

    mod m65_keymaster_keenus {
        use super::*;

        #[test]
        fn r1084_r781004_prints_equip_and_the_aura_on_both_faces_with_5_and_10_declared() {
            crate::register_all();
            let def = crate::card_def(KEENUS);
            for face in [&def.base, &def.radiant] {
                assert_eq!(face.keywords, vec![Keyword::Equip], "it no longer prints every keyword");
                assert_eq!(
                    face.text,
                    "Equip\nAura: Has {keywords|random combat keyword|random combat keywords}."
                );
            }
            assert_eq!(
                js(&def.params),
                json!([{ "key": "keywords", "base": 5, "radiant": 10, "better": "up", "step": 1, "min": 1 }])
            );
            assert_eq!((def.base.attack, def.base.health), (Some(1), Some(1)));
            assert_eq!((def.radiant.attack, def.radiant.health), (Some(2), Some(2)));
        }

        mod base {
            use super::*;

            #[test]
            fn r781004_played_it_has_5_different_combat_keywords_from_the_pool_and_never_equip_again() {
                let s = played("keenus-draw", false, json!({}));
                let id = s.card(KEENUS).id.clone();
                let drawn = drawn(&s, &id);
                assert_eq!(drawn.len(), 5, "{drawn:?}");
                for kind in &drawn {
                    assert!(pool_kinds().contains(kind), "{kind:?} is a combat keyword");
                    assert_ne!(*kind, KeywordKind::Equip);
                }
                let mut unique = drawn.clone();
                unique.sort();
                unique.dedup();
                assert_eq!(unique.len(), 5, "none repeats: {drawn:?}");
                let acting = acting(&s, &id);
                assert!(acting.contains(&KeywordKind::Equip));
                for kind in &drawn {
                    // R347: an Indestructible card has no Taunt, and the draw never wastes one on it.
                    assert!(acting.contains(kind), "the Aura gives it {kind:?}");
                }
                // The view lists them, as every keyword it has.
                let view = serde_json::to_string(&s.view(P1)).expect("the view serializes");
                assert!(view.contains("Equip"));
            }

            #[test]
            fn r781004_the_draw_replays_under_the_same_seed() {
                let first = played("keenus-replay", false, json!({}));
                let second = played("keenus-replay", false, json!({}));
                let (a, b) = (first.card(KEENUS).id.clone(), second.card(KEENUS).id.clone());
                assert_eq!(drawn(&first, &a), drawn(&second, &b));
            }

            #[test]
            fn r781004_it_draws_none_it_already_has() {
                for n in 0..6 {
                    let s = played(&format!("keenus-held-{n}"), false, json!({ "backrow": [WEAPONS] }));
                    let id = s.card(KEENUS).id.clone();
                    let drawn = drawn(&s, &id);
                    assert_eq!(drawn.len(), 5);
                    assert!(!drawn.contains(&KeywordKind::Rush), "the Field Spell already gives it Rush");
                    assert!(!drawn.contains(&KeywordKind::FirstStrike), "and First Strike");
                }
            }

            #[test]
            fn r781004_r1615000_never_taunt_beside_indestructible() {
                crate::register_all();
                for n in 0..24 {
                    let mut s = scenario(json!({
                        "seed": format!("keenus-r347-{n}"),
                        "p1": { "mana": 10, "hand": [THE_ROCK, KEENUS, FILLER], "field": ["core-008"], "library": filler(4) },
                        "p2": { "hand": [FILLER], "library": filler(4) },
                    }));
                    // The Rock's Tribute 1 is a Mr. Vanilla.
                    let tribute = s.unit(P1, 1).expect("the Mr. Vanilla").id;
                    s.play(THE_ROCK, json!({ "zone": 1, "tributes": [tribute] }));
                    s.play(KEENUS, json!({ "zone": 1, "equip": true }));
                    let host = s.unit(P1, 1).expect("the fused host");
                    let drawn = drawn(&s, &host.id);
                    // The pool holds no Indestructible (R1615000), and the host's own keeps Taunt out (R347).
                    assert_eq!(drawn.len(), 5, "{drawn:?}");
                    assert!(!drawn.contains(&KeywordKind::Indestructible), "{drawn:?}");
                    assert!(!drawn.contains(&KeywordKind::Taunt), "{drawn:?}");
                }
            }

            #[test]
            fn r781004_a_vanilla_takes_the_auras_keywords_with_its_text() {
                let mut s = played("keenus-vanilla", false, json!({ "hand": [SILENCE] }));
                let id = s.card(KEENUS).id.clone();
                assert_eq!(drawn(&s, &id).len(), 5);

                s.play(SILENCE, json!({ "targets": [{ "pick": "instance", "instanceId": id }] }));

                assert!(acting(&s, &id).is_empty(), "no Equip and no Aura: {:?}", acting(&s, &id));
            }

            #[test]
            fn r781004_r1086_equip_onto_your_unit_fuses_and_the_host_draws_its_own() {
                let mut s = scenario(json!({
                    "seed": "keenus-equip",
                    "p1": { "mana": 10, "hand": [VANILLA, KEENUS, FILLER], "library": filler(4) },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(VANILLA, json!({}));
                s.play(KEENUS, json!({ "zone": 1, "equip": true }));

                // The host is kept and united with Keenus: a 4/4 that is also a 1/1, with Equip and the
                // Aura, whose keywords it drew for itself as it fused.
                let host = s.unit(P1, 1).expect("the fused host");
                let stats = s.stats(host.id.as_str());
                assert_eq!((stats.attack, stats.health), (5, 5));
                let acting = acting(&s, &host.id);
                assert!(acting.contains(&KeywordKind::Equip));
                assert_eq!(drawn(&s, &host.id).len(), 5);
            }

            #[test]
            fn r3778000_a_buff_of_its_number_keeps_its_5_and_draws_a_6th() {
                let (seed, s, id) = crate::buff_number_once("keenus-3778", "keywords", with_book);
                let (before, first) = with_book(&seed);
                let five = drawn(&before, &first);
                assert_eq!(five.len(), 5);

                let six = drawn(&s, &id);
                assert_eq!(six.len(), 6, "{six:?}");
                assert_eq!(six[..5], five[..], "it keeps the 5 it drew, in their order");
                assert!(!five.contains(&six[5]), "the 6th is none it had: {six:?}");
                assert_eq!(aura(&s, &id), six, "it acts with all 6");
            }

            #[test]
            fn r3778000_a_nerf_takes_off_only_its_newest_and_a_buff_back_returns_it() {
                let (mut s, id) = with_book("keenus-3778-nerf");
                let five = drawn(&s, &id);

                assert_eq!(crate::degrade_number(&mut s, &id, "keywords"), 4);
                assert_eq!(aura(&s, &id), five[..4], "only the newest, {:?}, comes off", five[4]);
                assert!(!acting(&s, &id).contains(&five[4]));
                assert_eq!(drawn(&s, &id), five, "the list stays whole");

                assert_eq!(crate::upgrade_number(&mut s, &id, "keywords"), 5);
                assert_eq!(aura(&s, &id), five, "the same one comes back");
                assert_eq!(drawn(&s, &id), five);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r781004_the_radiant_face_is_2_2_and_draws_10() {
                let s = played("keenus-radiant", true, json!({}));
                let id = s.card(KEENUS).id.clone();
                let stats = s.stats(id.as_str());
                assert_eq!((stats.attack, stats.health), (2, 2));
                let drawn = drawn(&s, &id);
                assert_eq!(drawn.len(), 10, "{drawn:?}");
                let mut unique = drawn.clone();
                unique.sort();
                unique.dedup();
                assert_eq!(unique.len(), 10);
            }

            #[test]
            fn r386_r781004_a_buff_steps_the_10_to_11() {
                let mut s = scenario(json!({
                    "seed": "keenus-buff",
                    "p1": { "hand": [{ "def": KEENUS, "radiant": true }, FILLER], "library": filler(4), "mana": 10 },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                step_param(s.card_mut(KEENUS), "keywords", 1);
                s.play(KEENUS, json!({ "zone": 1 }));
                let id = s.card(KEENUS).id.clone();
                assert_eq!(drawn(&s, &id).len(), 11);
            }

            #[test]
            fn r3778000_made_radiant_on_the_field_it_tops_up_from_5_to_10_keeping_the_first_5() {
                let mut s = played(
                    "keenus-3778-radiant",
                    false,
                    json!({ "hand": [{ "def": NEW_WRAPS, "radiant": true }] }),
                );
                let id = s.card(KEENUS).id.clone();
                let five = drawn(&s, &id);
                assert_eq!(five.len(), 5);

                s.play(NEW_WRAPS, json!({ "targets": [{ "pick": "instance", "instanceId": id }] }));

                assert!(s.card(id.as_str()).radiant);
                let ten = drawn(&s, &id);
                assert_eq!(ten.len(), 10, "{ten:?}");
                assert_eq!(ten[..5], five[..], "it keeps the 5 it drew");
                let mut unique = ten.clone();
                unique.sort();
                unique.dedup();
                assert_eq!(unique.len(), 10, "and draws 5 more it does not have");
                assert!(!ten[5..].contains(&KeywordKind::Reborn), "New Wraps gave it Reborn first");
                assert_eq!(aura(&s, &id), ten, "it acts with all 10");
            }
        }

        #[test]
        fn r767000_the_script_kept_for_v0_6_0b_keeps_its_5_whatever_its_number() {
            crate::under_kept(KEENUS, script_v0_6_0b(), || {
                let (mut s, id) = with_book("keenus-kept");
                let five = drawn(&s, &id);
                assert_eq!(five.len(), 5);
                assert_eq!(crate::degrade_number(&mut s, &id, "keywords"), 4);
                assert_eq!(aura(&s, &id), five, "it kept all 5 through v0.6.0b");

                let mut s = played(
                    "keenus-kept-radiant",
                    false,
                    json!({ "hand": [{ "def": NEW_WRAPS, "radiant": true }] }),
                );
                let id = s.card(KEENUS).id.clone();
                let five = drawn(&s, &id);
                s.play(NEW_WRAPS, json!({ "targets": [{ "pick": "instance", "instanceId": id }] }));
                assert!(s.card(id.as_str()).radiant);
                assert_eq!(drawn(&s, &id), five, "made Radiant, it drew none more");
            });
        }
    }
}
