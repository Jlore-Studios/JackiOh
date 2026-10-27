//! S T-drone-corvette Drone Corvette (SPEC §8.10 row T-drone-corvette): (3) Unit, Token (printed Rare),
//! 8/8 → 16/16, a token of the held Stinkyscape set (R1420).
//!   Both faces: "Ranged, Flying\nAura: Has {keywords|random combat keyword|random combat keywords}." — 2
//!   on the base face, 4 on the Radiant face (the declared number `keywords`, R386).
//! Engine: as the Drone Interceptor: Ranged and Flying are catalog keywords (§6.1), and the Aura is
//!   Keymaster Keenus's (R781004), its keywords drawn with the match rng as it arrives on the field and
//!   kept until it leaves (R78), from R21's pool, all different, none it has and no Taunt beside
//!   Indestructible (R347). A unit token: it ceases to exist off the field (R11).

use jackioh_engine::effects::{DrawCombatKeywordsArgs, draw_combat_keywords, drawn_combat_keywords};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-drone-corvette";

/// R781004: this card arrived on the field.
fn arrived(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    let Some(me) = ctx.live_self() else {
        return false;
    };
    matches!(event, GameEvent::Summoned { instance_id, .. } if *instance_id == me.id)
}

fn draw_on_arrival() -> TriggerDef {
    TriggerDef::new(
        "drone-corvette-combat-keywords",
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

fn corvette() -> Script {
    Script {
        triggers: vec![draw_on_arrival()],
        conditional_keywords: Some(read_hook(|args| drawn_combat_keywords(args.self_, None))),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // One script: the Radiant face's 4 is its declared number, which `param` reads off the face, and its
    // 16/16 is catalog data.
    CardScripts {
        base: corvette(),
        radiant: corvette(),
    }
}

// S T-drone-corvette — SPEC §8.10 row T-drone-corvette, BUILD M12 row S T-drone-corvette: "summoned, it
// has Ranged, Flying and two more different combat keywords; keywords reads through `param()`; radiant
// 16/16 and four".
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{filler, scenario};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const CORVETTE: &str = "stinkyscape-t-drone-corvette";
    /// S #31 Drone Core V: summon a Drone Corvette.
    const CORE_V: &str = "stinkyscape-031";
    const FILLER: &str = "core-005";

    /// p1 casts Drone Core V (Radiant on `radiant`) under `seed`; the Corvette lands in lane 1.
    fn summoned(seed: &str, radiant: bool) -> (Scenario, String) {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": CORE_V, "radiant": radiant }, FILLER], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(CORE_V, json!({}));
        let id = s.unit(P1, 1).expect("the Corvette").id;
        (s, id)
    }

    fn drawn(s: &Scenario, id: &str) -> Vec<KeywordKind> {
        drawn_combat_keywords(s.card(id), None)
            .iter()
            .map(Keyword::kind)
            .collect()
    }

    fn distinct(mut kinds: Vec<KeywordKind>) -> usize {
        kinds.sort();
        kinds.dedup();
        kinds.len()
    }

    #[test]
    fn r781004_summoned_it_has_ranged_flying_and_two_more_from_the_pool() {
        let (s, id) = summoned("corvette-draw", false);
        assert_eq!(s.card(id.as_str()).def_id, CORVETTE);
        let drawn = drawn(&s, &id);
        assert_eq!(drawn.len(), 2, "{drawn:?}");
        assert_eq!(distinct(drawn.clone()), 2, "different ones: {drawn:?}");
        let pool: Vec<KeywordKind> = random_pool_keywords().iter().map(Keyword::kind).collect();
        let acting: Vec<KeywordKind> = keywords_of(s.state(), s.card(id.as_str()))
            .iter()
            .map(Keyword::kind)
            .collect();
        for kind in &drawn {
            assert!(pool.contains(kind), "{kind:?} is a combat keyword");
            assert!(![KeywordKind::Ranged, KeywordKind::Flying].contains(kind));
        }
        for kind in [KeywordKind::Ranged, KeywordKind::Flying, drawn[0], drawn[1]] {
            assert!(acting.contains(&kind), "it has {kind:?}: {acting:?}");
        }
    }

    #[test]
    fn r781004_different_keywords_over_the_seeds() {
        let seen: Vec<KeywordKind> = (0..12)
            .flat_map(|n| {
                let (s, id) = summoned(&format!("corvette-seed-{n}"), false);
                drawn(&s, &id)
            })
            .collect();
        assert!(distinct(seen) > 2);
    }

    #[test]
    fn r781004_the_draw_replays_under_the_same_seed() {
        let (first, a) = summoned("corvette-replay", false);
        let (second, b) = summoned("corvette-replay", false);
        assert_eq!(drawn(&first, &a), drawn(&second, &b));
    }

    #[test]
    fn keywords_reads_through_param() {
        // A unit token may sit in a hand and be played from it (R11).
        let mut s = scenario(json!({
            "seed": "corvette-param",
            "p1": { "hand": [CORVETTE, FILLER], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        set_param(s.card_mut(CORVETTE), "keywords", 3);
        s.play(CORVETTE, json!({ "zone": 1 }));
        let id = s.unit(P1, 1).expect("the Corvette").id;
        assert_eq!(drawn(&s, &id).len(), 3);
    }

    #[test]
    fn r781004_radiant_16_16_and_four_keywords() {
        let (s, id) = summoned("corvette-radiant", true);
        assert!(s.card(id.as_str()).radiant);
        let stats = s.stats(id.as_str());
        assert_eq!((stats.attack, stats.health), (16, 16));
        let drawn = drawn(&s, &id);
        assert_eq!(drawn.len(), 4, "{drawn:?}");
        assert_eq!(distinct(drawn), 4);
    }
}
