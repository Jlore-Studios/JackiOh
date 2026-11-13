//! S T-drone-corvette Drone Corvette (SPEC §8.10 row T-drone-corvette): (3) Unit, Token (printed Rare),
//! 8/8 → 16/16, a token of the Stinkyscape set.
//!   Both faces: "Ranged, Flying\nAura: Has {keywords|random combat keyword|random combat keywords}." — 2
//!   on the base face, 4 on the Radiant face (the declared number `keywords`, R386).
//! Engine: as the Drone Interceptor: Ranged and Flying are catalog keywords (§6.1), and the Aura is
//!   Keymaster Keenus's (R781004), its keywords drawn with the match rng as it arrives on the field into
//!   one list kept until it leaves (R78), from R21's pool, all different, none it has and no Taunt beside
//!   Indestructible (R347), of which the Aura gives the first `keywords`: a Buff, a number set higher or
//!   the Radiant face draws only the rest, a lower number takes the newest off (R3778000). A Fuse that
//!   eats it gives the kept card the Aura, which draws its own as it fuses (`fused_into`, R781004). A
//!   unit token: it ceases to exist off the field (R11).

use jackioh_engine::effects::{
    DrawCombatKeywordsArgs, aura_combat_keywords, draw_combat_keywords, drawn_combat_keywords,
    top_up_combat_keywords,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-drone-corvette";

/// R781004: this card arrived on the field.
fn arrived(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    let Some(me) = ctx.live_self() else {
        return false;
    };
    matches!(event, GameEvent::Summoned { instance_id, .. } if *instance_id == me.id)
}

fn draw_on_arrival(top_up: bool) -> TriggerDef {
    TriggerDef::new(
        "drone-corvette-combat-keywords",
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

fn corvette(top_up: bool) -> Script {
    Script {
        triggers: vec![draw_on_arrival(top_up)],
        // R3778000: a Buff, a number set or the Radiant face that raises its number draws the rest.
        combat_keywords_follow: top_up.then_some("keywords"),
        // A Fuse that keeps another card makes it carry the Aura, and it draws its own (R781004).
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
    // One script: the Radiant face's 4 is its declared number, which `param` reads off the face, and its
    // 16/16 is catalog data.
    CardScripts {
        base: corvette(true),
        radiant: corvette(true),
    }
}

/// R767000: through v0.6.0b the Aura drew once as it arrived and kept its keywords whatever its
/// number did (R781004); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_0b() -> CardScripts {
    CardScripts {
        base: corvette(false),
        radiant: corvette(false),
    }
}

// S T-drone-corvette — SPEC §8.10 row T-drone-corvette, BUILD M12 row S T-drone-corvette
// (R781004).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{filler, scenario};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CORVETTE: &str = "stinkyscape-t-drone-corvette";
    /// S #31 Drone Core V: summon a Drone Corvette.
    const CORE_V: &str = "stinkyscape-031";
    const FILLER: &str = "core-005";
    const TAOTIE: &str = "meditative-047";
    use crate::BOOK_OF_BUFF;
    /// (0) Spell: a Unit gains Reborn; Radiant, then it is made Radiant.
    const NEW_WRAPS: &str = "classicplus-010";

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

    /// R3778000: the drawn keywords it acts with, in draw order: what its Aura gives.
    fn aura(s: &Scenario, id: &str) -> Vec<KeywordKind> {
        let acting: Vec<KeywordKind> = keywords_of(s.state(), s.card(id)).iter().map(Keyword::kind).collect();
        drawn(s, id).into_iter().filter(|kind| acting.contains(kind)).collect()
    }

    /// A Drone Core summons it, `card` in hand.
    fn with_in_hand(seed: &str, card: Value) -> (Scenario, String) {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [CORE_V, card, FILLER], "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(CORE_V, json!({}));
        let id = s.unit(P1, 1).expect("the drone").id;
        (s, id)
    }

    /// The Book of Buff in hand.
    fn with_book(seed: &str) -> (Scenario, String) {
        with_in_hand(seed, json!(BOOK_OF_BUFF))
    }

    /// A Radiant New Wraps cast on it: Reborn, then made Radiant on the field.
    fn wrapped(s: &mut Scenario, id: &str) {
        s.play(NEW_WRAPS, json!({ "targets": [{ "pick": "instance", "instanceId": id }] }));
        assert!(s.card(id).radiant);
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
        let list = drawn(&s, &id);
        assert_eq!(list.len(), 3);
        // R3778000: and on the field it follows the number, a Nerf taking the newest off.
        assert_eq!(crate::degrade_number(&mut s, &id, "keywords"), 2);
        assert_eq!(aura(&s, &id), list[..2]);
    }

    #[test]
    fn r3778000_a_buff_of_its_number_keeps_its_2_and_draws_one_more() {
        let (seed, s, id) = crate::buff_number_once("corvette-3778", "keywords", with_book);
        let (before, first) = with_book(&seed);
        let old = drawn(&before, &first);
        assert_eq!(old.len(), 2);

        let now = drawn(&s, &id);
        assert_eq!(now.len(), 3, "{now:?}");
        assert_eq!(now[..2], old[..], "it keeps what it drew");
        assert!(!old.contains(&now[2]), "one more, none it had: {now:?}");
        assert!(![KeywordKind::Ranged, KeywordKind::Flying].contains(&now[2]));
        assert_eq!(aura(&s, &id), now, "it acts with them all");
    }

    #[test]
    fn r3778000_a_nerf_takes_off_only_its_newest_and_a_buff_back_returns_it() {
        // The base face's 2 is a floor of 1 away from a Nerf; the Radiant 4 has room.
        let (mut s, id) = summoned("corvette-3778-nerf", true);
        let list = drawn(&s, &id);
        assert_eq!(list.len(), 4);

        assert_eq!(crate::degrade_number(&mut s, &id, "keywords"), 3);
        assert_eq!(aura(&s, &id), list[..3], "only the newest, {:?}, comes off", list[3]);
        assert_eq!(drawn(&s, &id), list, "the list stays whole");

        assert_eq!(crate::upgrade_number(&mut s, &id, "keywords"), 4);
        assert_eq!(aura(&s, &id), list, "the same one comes back");
    }

    #[test]
    fn r3778000_made_radiant_on_the_field_it_draws_the_radiant_faces_rest() {
        let (mut s, id) = with_in_hand("corvette-3778-radiant", json!({ "def": NEW_WRAPS, "radiant": true }));
        let old = drawn(&s, &id);
        assert_eq!(old.len(), 2);

        wrapped(&mut s, &id);

        let now = drawn(&s, &id);
        assert_eq!(now.len(), 4, "{now:?}");
        assert_eq!(now[..2], old[..], "it keeps what it drew");
        assert!(!now[2..].contains(&KeywordKind::Reborn), "New Wraps gave it Reborn first");
        assert_eq!(aura(&s, &id), now);
    }

    #[test]
    fn r767000_the_script_kept_for_v0_6_0b_keeps_its_keywords_whatever_its_number() {
        crate::under_kept(CORVETTE, script_v0_6_0b(), || {
            let (mut s, id) = summoned("corvette-kept", true);
            let list = drawn(&s, &id);
            assert_eq!(list.len(), 4);
            assert_eq!(crate::degrade_number(&mut s, &id, "keywords"), 3);
            assert_eq!(aura(&s, &id), list, "it kept all 4 through v0.6.0b");

            let (mut s, id) = with_in_hand("corvette-kept-radiant", json!({ "def": NEW_WRAPS, "radiant": true }));
            let list = drawn(&s, &id);
            wrapped(&mut s, &id);
            assert_eq!(drawn(&s, &id), list, "made Radiant, it drew none more");
        });
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

    #[test]
    fn r781004_fused_into_another_card_the_kept_card_draws_its_own() {
        // M #47 饕餮 eats an enemy permanent at its end of turn: the kept 饕餮 carries the Aura and draws
        // its number for itself, Ranged and Flying already its own. The fusion wears the kept card's face
        // (R77), so a Radiant 饕餮 draws the Radiant number; p2's empty deck leaves its second meal out.
        for (radiant, count) in [(false, 2), (true, 4)] {
            let mut s = scenario(json!({
                "seed": "corvette-fused",
                "p1": {
                    "field": [{ "def": TAOTIE, "lane": 1, "radiant": radiant }],
                    "library": filler(4),
                },
                "p2": { "field": [{ "def": CORVETTE, "lane": 1 }], "library": [] },
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
