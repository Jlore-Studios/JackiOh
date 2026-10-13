//! M #65 Keymaster Keenus (SPEC §8.8 row 65, R1086, R781004): (4) Unit, Legendary, 1/1 → 2/2.
//!
//! Both faces: "Equip\nAura: Has {keywords|random combat keyword|random combat keywords}." — 5 on the
//! base face, 10 on the Radiant face (the declared number `keywords`, R386).
//! Engine: Equip is printed (ME-EQUIP, R1086). §10.4's layers read with no rng, so the Aura's keywords
//! are drawn at an event and kept on the card: as it arrives on the field (played, summoned, Flickered,
//! back through Reborn) and as an Equip play fuses it into a Unit, which draws its own. They come from
//! the combat keyword pool (R21's), none the card has and no Taunt beside Indestructible
//! (`draw_combat_keywords`), and stay until it leaves the field (R78). The Aura reads them back
//! (`conditionalKeywords`), so a Vanilla takes them with the rest of its text (R781004). It printed
//! every keyword and, Radiant, a Death that gave them away until patch v0.3.7 (R1084, R1085).

use jackioh_engine::effects::{DrawCombatKeywordsArgs, draw_combat_keywords, drawn_combat_keywords};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-065";

/// R781004: this card arrived on the field (played, summoned, Flickered, back through Reborn).
fn arrived(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    let Some(me) = ctx.live_self() else {
        return false;
    };
    matches!(event, GameEvent::Summoned { instance_id, .. } if *instance_id == me.id)
}

fn draw_on_arrival() -> TriggerDef {
    TriggerDef::new("keenus-combat-keywords", &[GameEventType::Summoned], |ctx, event| {
        if !arrived(ctx, event) {
            return vec![];
        }
        vec![draw_for(param(&*ctx, "keywords"))]
    })
    .with_when(|ctx, event| arrived(ctx, event))
}

fn draw_for(count: i32) -> Effect {
    draw_combat_keywords(DrawCombatKeywordsArgs { count, key: None })
}

fn keenus() -> Script {
    Script {
        triggers: vec![draw_on_arrival()],
        // An Equip play fuses this into a Unit, which carries the Aura and draws its own (R1086).
        fused_into: Some(hook(|ctx| vec![draw_for(param(&*ctx, "keywords"))])),
        conditional_keywords: Some(read_hook(|args| drawn_combat_keywords(args.self_, None))),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // One script: the Radiant face's 10 is its declared number, which `param` reads off the face.
    CardScripts {
        base: keenus(),
        radiant: keenus(),
    }
}

// M #65 Keymaster Keenus — SPEC §8.8 row 65, BUILD M10 row M 65: "Prints Equip and an Aura that has
// 5 random combat keywords (Radiant 10, a declared number): drawn from the combat keyword pool as it
// arrives on the field, none repeated and none it has (never Equip, nor a keyword outside the pool),
// replayed by the seed, kept until it leaves the field and taken by a Vanilla; played as Equip onto your
// Unit it fuses in, and the host draws its own (R1086, R781004); radiant 2/2 and 10".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const KEENUS: &str = "meditative-065";
    /// (1) 4/4 Unit with no hooks: an Equip host.
    const VANILLA: &str = "core-008";
    /// (0) Spell: Vanilla a Unit.
    const SILENCE: &str = "classicplus-009";
    /// (4) Field Spell: Aura: your Units have +4 attack, Rush and First Strike.
    const WEAPONS: &str = "core-014";
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

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
                serde_json::to_value(&def.params).expect("params serialise"),
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
                let view = serde_json::to_string(&s.view(P1)).expect("the view serialises");
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
            fn r781004_never_taunt_beside_indestructible() {
                for n in 0..24 {
                    let s = played(&format!("keenus-r347-{n}"), true, json!({}));
                    let id = s.card(KEENUS).id.clone();
                    let drawn = drawn(&s, &id);
                    if drawn.contains(&KeywordKind::Indestructible) {
                        let at = drawn.iter().position(|kind| *kind == KeywordKind::Indestructible);
                        let taunt = drawn.iter().position(|kind| *kind == KeywordKind::Taunt);
                        // Taunt drawn first is held down by the later Indestructible (R347), never after.
                        assert!(taunt.is_none_or(|taunt| Some(taunt) < at), "{drawn:?}");
                    }
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
                crate::register_all();
                let _guard = preview_sets(&[SetName::Meditative]);
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
                crate::register_all();
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
        }
    }
}
