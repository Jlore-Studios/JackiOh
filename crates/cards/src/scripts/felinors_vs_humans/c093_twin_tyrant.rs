//! SPEC §8.14 FvH #93 Twin Tyrant ((3) Unit, Common, 4/10 → 8/20), a card of the held Felinors vs Humans set
//! (R5235000).
//!   Base:    "Cry: Deal {damage} damage to two different random enemies." (damage 4)
//!   Radiant: "Lucky 1\nCry: Deal {damage} damage to two different random enemies." (damage 8)
//!
//! Engine: the Cry (R1) hits two different standing enemies, the hero or Units, one hit each, as
//! `damage_random_enemies` draws them (R60, R1860004): the designer's "must be separate"; fewer stand, fewer
//! are hit. The pair is drawn as one luck roll (R1438): Lucky 1 is the Radiant face's catalog data, read with
//! `lucky_on` beside its controller's Luck (R987), and the roll keeps the pair that would destroy more, the
//! earlier draw on a tie (R5251000), as R3567005 ruled Snom Bunny's. With no luck it is one plain draw, so
//! both faces run the one script. The pair is drawn as the list reaches it (`for_each_card`, Soul Shot's
//! shape), so a pause after the first hit resumes over the same pair. Built by S01 (#5251).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-093";

/// "two different random enemies".
const TARGETS: usize = 2;
/// The name the enemy hero goes by among the enemy Units' instance ids in the drawn pair.
const ENEMY_HERO: &str = "enemy-hero";

/// The enemies the Cry may hit, each with whether `amount` would destroy it: the standing enemy Units a
/// harmful effect may reach (R60), then the enemy hero.
fn enemies(ctx: &EffectContext<'_>, amount: i32) -> Vec<(String, bool)> {
    let state: &GameState = &*ctx.state;
    let scope: BoardScope = json_as(json!({ "side": "enemy" }));
    let mut pool: Vec<(String, bool)> = cards_in_scope_aimed(ctx, &scope, TargetAim::Harm)
        .into_iter()
        .filter_map(|unit| {
            let view = unit_view(state, &unit);
            let standing =
                view.health > 0 && unit.marked_destroyed != Some(true) && unit.marked_exiled != Some(true);
            // R5251000: a hit destroys a Unit when its Health and Armor come to no more than the damage
            // and no Divine Shield takes it.
            let destroys = !has_keyword(&view.keywords, KeywordKind::DivineShield)
                && view.health + view.armor <= amount;
            standing.then(|| (unit.id.clone(), destroys))
        })
        .collect();
    let hero = hero_of(state, opponent_of(ctx.controller));
    pool.push((ENEMY_HERO.to_string(), hero.health + hero.armor <= amount));
    pool
}

/// R5251000: the two different enemies the Cry hits, in the order drawn. One draw shuffles the enemies and
/// takes the first two; Lucky X (the card's, plus its controller's Luck, R987) draws X more and keeps the
/// pair that would destroy more, the earlier draw on a tie (below 0, the worst, R5236020).
fn pair(ctx: &mut EffectContext<'_>, amount: i32) -> Vec<String> {
    let pool = enemies(ctx, amount);
    let lucky = {
        let state: &GameState = &*ctx.state;
        ctx.live_self().map_or(0, |me| lucky_on(state, me)) + luck_of(state, ctx.controller)
    };
    let destroys: Vec<bool> = pool.iter().map(|(_, destroys)| *destroys).collect();
    let kills = |drawn: &[usize]| drawn.iter().filter(|index| destroys[**index]).count();
    let indices: Vec<usize> = (0..pool.len()).collect();
    let roll = |rng: &mut Rng| -> Vec<usize> { rng.shuffle(&indices).into_iter().take(TARGETS).collect() };
    let drawn = ctx.rng.lucky(lucky, roll, |kept, next| {
        if kills(next.as_slice()) > kills(kept.as_slice()) {
            next
        } else {
            kept
        }
    });
    drawn.into_iter().map(|index| pool[index].0.clone()).collect()
}

fn twin_tyrant() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let amount = param(&*ctx, "damage");
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(move |c: &mut EffectContext<'_>| -> Vec<String> { pair(c, amount) }),
                each: Arc::new(move |id: &str| {
                    let to = if id == ENEMY_HERO {
                        json!({ "of": "enemyHero" })
                    } else {
                        json!({ "of": "instance", "instanceId": id })
                    };
                    damage(json_as(json!({ "to": to, "amount": amount })))
                }),
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: Lucky 1 is the Radiant face's catalog keyword, which `pair` reads.
    CardScripts {
        base: twin_tyrant(),
        radiant: twin_tyrant(),
    }
}

// FvH #93 Twin Tyrant — SPEC §8.14 row 93 (R60, R1438, R5251000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    /// Survives any one hit of the Cry.
    const STURDY: &str = "stand-in (1) 1/20";
    /// Dies to either face's hit, 4 or 8.
    const FRAIL: &str = "stand-in (1) 1/4";
    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    /// p1 plays Twin Tyrant (`radiant` its face, given `lucky` more Lucky) into lane 1, facing `foes`.
    fn cast(radiant: bool, lucky: i32, foes: Value) -> Scenario {
        let mut s = scenario(json!({
            "seed": "fvh-93",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "field": foes, "library": filler(4) },
        }));
        if lucky > 0 {
            crate::give_lucky(&mut s, ID, lucky);
        }
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    /// Facing one Sturdy, the only two enemies are it and the hero: each is hit once for `amount`.
    fn hits_both_once(radiant: bool, amount: i32) {
        let s = cast(radiant, 0, json!([STURDY]));
        let sturdy = s.unit_at(P2, 1);
        let mut hits = crate::hits(s.last_events());
        hits.sort();
        let mut expected = vec![(sturdy.id.clone(), amount), ("hero-p2".to_string(), amount)];
        expected.sort();
        assert_eq!(hits, expected, "two different enemies, one hit each");
        assert_eq!(s.unit_at(P1, 1).def_id, ID);
    }

    /// R5251000: given Lucky 30 more, of the three pairs among two Frails and the hero the roll keeps the
    /// one that destroys both Frails, whatever the seed's first draw.
    fn lucky_keeps_the_pair_that_destroys_more(radiant: bool) {
        let mut s = cast(radiant, 30, json!([FRAIL, FRAIL]));
        assert!(s.unit(P2, 1).is_none() && s.unit(P2, 2).is_none(), "both Frails were hit and destroyed");
        s.expect_health(P2, HERO_HEALTH);
        assert_eq!(crate::hits(s.last_events()).len(), 2, "two hits, no more");
    }

    #[test]
    fn r60_with_the_hero_alone_standing_one_enemy_is_hit_once() {
        let mut s = cast(false, 0, json!([]));
        assert_eq!(crate::hits(s.last_events()), vec![("hero-p2".to_string(), 4)]);
        s.expect_health(P2, HERO_HEALTH - 4);
    }

    mod base {
        use super::*;

        #[test]
        fn r60_its_cry_hits_two_different_enemies_for_4() {
            hits_both_once(false, 4);
        }

        #[test]
        fn r5251000_given_lucky_it_keeps_the_pair_that_destroys_more() {
            lucky_keeps_the_pair_that_destroys_more(false);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r60_its_cry_hits_two_different_enemies_for_8() {
            hits_both_once(true, 8);
        }

        #[test]
        fn r5251000_its_lucky_keeps_the_pair_that_destroys_more() {
            lucky_keeps_the_pair_that_destroys_more(true);
        }
    }
}
