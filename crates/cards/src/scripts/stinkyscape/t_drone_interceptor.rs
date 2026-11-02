//! S T-drone-interceptor Drone Interceptor (SPEC §8.10 row T-drone-interceptor): (1) Unit, Token
//! (printed Common), 2/2 → 4/4, a token of the Stinkyscape set.
//!   Both faces: "Ranged, Flying\nAura: Has {keywords|random combat keyword|random combat keywords}." — 1
//!   on the base face, 2 on the Radiant face (the declared number `keywords`, R386).
//! Engine: Ranged and Flying are catalog keywords (§6.1). The Aura is Keymaster Keenus's (R781004): §10.4's
//!   layers read with no rng, so its keywords are drawn with the match rng as it arrives on the field
//!   (summoned, as every drone arrives, or played) and kept on the card until it leaves (R78), from R21's
//!   combat keyword pool, none it has (so neither Ranged nor Flying) and no Taunt beside Indestructible
//!   (R347); the Aura reads them back (`conditional_keywords`). A Fuse that eats it (M #47 饕餮's, from the
//!   enemy field) gives the kept card the Aura, which draws its own as it fuses (`fused_into`, R781004).
//!   A unit token: it ceases to exist off the field (R11).

use jackioh_engine::effects::{DrawCombatKeywordsArgs, draw_combat_keywords, drawn_combat_keywords};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-drone-interceptor";

/// R781004: this card arrived on the field.
fn arrived(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    let Some(me) = ctx.live_self() else {
        return false;
    };
    matches!(event, GameEvent::Summoned { instance_id, .. } if *instance_id == me.id)
}

fn draw_on_arrival() -> TriggerDef {
    TriggerDef::new(
        "drone-interceptor-combat-keywords",
        &[GameEventType::Summoned],
        |ctx, event| {
            if !arrived(ctx, event) {
                return vec![];
            }
            vec![draw_for(param(&*ctx, "keywords"))]
        },
    )
    .with_when(|ctx, event| arrived(ctx, event))
}

fn draw_for(count: i32) -> Effect {
    draw_combat_keywords(DrawCombatKeywordsArgs { count, key: None })
}

fn interceptor() -> Script {
    Script {
        triggers: vec![draw_on_arrival()],
        // A Fuse that keeps another card makes it carry the Aura, and it draws its own (R781004).
        fused_into: Some(hook(|ctx| vec![draw_for(param(&*ctx, "keywords"))])),
        conditional_keywords: Some(read_hook(|args| drawn_combat_keywords(args.self_, None))),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // One script: the Radiant face's 2 is its declared number, which `param` reads off the face, and its
    // 4/4 is catalog data.
    CardScripts {
        base: interceptor(),
        radiant: interceptor(),
    }
}

// S T-drone-interceptor — SPEC §8.10 row T-drone-interceptor, BUILD M12 row S T-drone-interceptor
// (R781004).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{filler, scenario};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
    /// S #28 Drone Core II: summon a Drone Interceptor.
    const CORE_II: &str = "stinkyscape-028";
    const FILLER: &str = "core-005";
    const TAOTIE: &str = "meditative-047";

    fn summoned(seed: &str, radiant: bool) -> (Scenario, String) {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": CORE_II, "radiant": radiant }, FILLER], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(CORE_II, json!({}));
        let id = s.unit(P1, 1).expect("the Interceptor").id;
        (s, id)
    }

    fn drawn(s: &Scenario, id: &str) -> Vec<KeywordKind> {
        drawn_combat_keywords(s.card(id), None)
            .iter()
            .map(Keyword::kind)
            .collect()
    }

    fn acting(s: &Scenario, id: &str) -> Vec<KeywordKind> {
        keywords_of(s.state(), s.card(id)).iter().map(Keyword::kind).collect()
    }

    #[test]
    fn r781004_summoned_it_has_ranged_flying_and_one_more_from_the_pool() {
        let (s, id) = summoned("interceptor-draw", false);
        assert_eq!(s.card(id.as_str()).def_id, INTERCEPTOR);
        let drawn = drawn(&s, &id);
        assert_eq!(drawn.len(), 1, "{drawn:?}");
        let pool: Vec<KeywordKind> = random_pool_keywords().iter().map(Keyword::kind).collect();
        assert!(pool.contains(&drawn[0]), "{drawn:?} is a combat keyword");
        assert!(![KeywordKind::Ranged, KeywordKind::Flying].contains(&drawn[0]));
        let acting = acting(&s, &id);
        for kind in [KeywordKind::Ranged, KeywordKind::Flying, drawn[0]] {
            assert!(acting.contains(&kind), "it has {kind:?}: {acting:?}");
        }
    }

    #[test]
    fn r781004_different_keywords_over_the_seeds() {
        let mut seen: Vec<KeywordKind> = (0..12)
            .flat_map(|n| {
                let (s, id) = summoned(&format!("interceptor-seed-{n}"), false);
                drawn(&s, &id)
            })
            .collect();
        seen.sort();
        seen.dedup();
        assert!(seen.len() > 1, "{seen:?}");
    }

    #[test]
    fn r781004_the_draw_replays_under_the_same_seed() {
        let (first, a) = summoned("interceptor-replay", false);
        let (second, b) = summoned("interceptor-replay", false);
        assert_eq!(drawn(&first, &a), drawn(&second, &b));
    }

    #[test]
    fn keywords_reads_through_param() {
        // A unit token may sit in a hand and be played from it (R11).
        let mut s = scenario(json!({
            "seed": "interceptor-param",
            "p1": { "hand": [INTERCEPTOR, FILLER], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        set_param(s.card_mut(INTERCEPTOR), "keywords", 3);
        s.play(INTERCEPTOR, json!({ "zone": 1 }));
        let id = s.unit(P1, 1).expect("the Interceptor").id;
        assert_eq!(drawn(&s, &id).len(), 3);
    }

    #[test]
    fn r781004_radiant_twice_the_keywords() {
        let (s, id) = summoned("interceptor-radiant", true);
        assert!(s.card(id.as_str()).radiant);
        let stats = s.stats(id.as_str());
        assert_eq!((stats.attack, stats.health), (4, 4));
        let mut drawn = drawn(&s, &id);
        assert_eq!(drawn.len(), 2, "{drawn:?}");
        drawn.dedup();
        assert_eq!(drawn.len(), 2, "different ones");
    }

    #[test]
    fn r781004_fused_into_another_card_the_kept_card_draws_its_own() {
        // M #47 饕餮 eats an enemy permanent at its end of turn: the kept 饕餮 carries the Aura and draws
        // its number for itself, Ranged and Flying already its own. The fusion wears the kept card's face
        // (R77), so a Radiant 饕餮 draws the Radiant number; p2's empty deck leaves its second meal out.
        for (radiant, count) in [(false, 1), (true, 2)] {
            let mut s = scenario(json!({
                "seed": "interceptor-fused",
                "p1": {
                    "field": [{ "def": TAOTIE, "lane": 1, "radiant": radiant }],
                    "library": filler(4),
                },
                "p2": { "field": [{ "def": INTERCEPTOR, "lane": 1 }], "library": [] },
            }));
            let taotie = s.unit(P1, 1).expect("饕餮").id;
            s.end_turn();
            assert!(s.unit(P2, 1).is_none(), "eaten");
            let drawn = drawn(&s, &taotie);
            assert_eq!(drawn.len(), count, "radiant {radiant}: {drawn:?}");
            let acting: Vec<KeywordKind> =
                keywords_of(s.state(), s.card(taotie.as_str())).iter().map(Keyword::kind).collect();
            for kind in [KeywordKind::Ranged, KeywordKind::Flying].iter().chain(&drawn) {
                assert!(acting.contains(kind), "it has {kind:?}: {acting:?}");
            }
        }
    }
}
