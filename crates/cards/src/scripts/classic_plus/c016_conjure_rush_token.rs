//! C+ #16 Conjure Rush Token+ (SPEC §8.7 row 16, R3140002): (2) Spell, Rare.
//!   Base:    "Summon a Rush Token. Flip a coin: it gains {keywords|random combat keyword|random combat
//!            keywords} on heads, {tails} on tails." (3 on heads, 2 on tails)
//!   Radiant: "Summon {tokens|Rush Token|Rush Tokens}. Each gains {keywords|random combat keyword|random
//!            combat keywords}." (3 Rush Tokens, 5 keywords each)
//! Engine: the Cry flips one coin on the base face (R1440: a luck-based roll whose better side is heads,
//! so the card's own Lucky and its controller's Luck flip more coins and keep heads) and summons a Rush
//! Token with `keywords` different random keywords from R21's pool it lacks on heads and `tails` on
//! tails; the coin is shown to both players before the summon it decided (R2652001). The Radiant face
//! flips nothing: `tokens` Rush Tokens, each with `keywords`. A summon with no open zone rolls no
//! keywords (R129), though the coin was flipped as the Spell resolved. Through v0.5.11p the base face
//! gave 3 keywords with no coin and the Radiant face 3 (`script_v0_5_11o`, R767000).

use jackioh_engine::effects::summon;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-016";

/// §7's shared Rush Token, 3/3 with Rush.
const RUSH_TOKEN: &str = "core-t-rush";

/// "Summon 3 Rush Tokens" on the Radiant face: the declared number `tokens` (R386), tuned on the
/// Radiant face only — the base face's "a Rush Token" is 1 and never moves (R749), so both faces read
/// the one number.
fn summons(keywords: i32, ctx: &EffectContext<'_>) -> Vec<Effect> {
    (0..param(ctx, "tokens"))
        .map(|_| summon(json_as(json!({ "defId": RUSH_TOKEN, "randomKeywords": keywords }))))
        .collect()
}

/// The base face since #3140: the coin, shown, then the token with the keywords it decided.
fn flip_then_summon() -> Hook {
    hook(|ctx| {
        // R987, R1440: the controller's Luck flips extra times beside the card's own Lucky.
        let lucky = ctx.live_self().map_or(0, |me| lucky_on(&*ctx.state, me)) + luck_of(&*ctx.state, ctx.controller);
        let coin = flip_coin(ctx, lucky);
        let keywords = param(&*ctx, if coin.heads() { "keywords" } else { "tails" });
        let mut effects = vec![report_rolls(RollKind::Coin, ROLL_COIN_SIDES, vec![coin])];
        effects.extend(summons(keywords, ctx));
        effects
    })
}

/// The Radiant face, and the base face through v0.5.11p: no coin.
fn conjure() -> Hook {
    hook(|ctx| summons(param(&*ctx, "keywords"), ctx))
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(flip_then_summon()),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(conjure()),
            ..Script::default()
        },
    }
}

/// R767000: through v0.5.11p both faces summoned with `keywords` and no coin; `KEPT_SCRIPTS` folds those
/// replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(conjure()),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(conjure()),
            ..Script::default()
        },
    }
}

// C+ #16 Conjure Rush Token+ — SPEC §8.7 row 16, BUILD M9 Classic+ row C+ 16: as C+ #15, with 3 keywords on heads
// and 2 on tails (R1440, R3140002), and 5 on each of the Radiant's 3 tokens.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const CARD: &str = "classicplus-016";
    const RUSH_TOKEN: &str = "core-t-rush";
    /// The keywords a heads gives, `keywords` on the base face.
    const HEADS: usize = 3;
    /// The keywords a tails gives, `tails`.
    const TAILS: usize = 2;
    /// The Radiant face's keywords on each token.
    const RADIANT_KEYWORDS: usize = 5;
    const FILLER: &str = "core-005"; // a card in hand, so a turn never auto-ends under the test
    const BODY: &str = "core-019"; // Midrange Menace, a unit to fill a zone

    use crate::scenario;

    use crate::js;

    fn tokens(s: &Scenario) -> Vec<CardInstance> {
        (1..=5)
            .filter_map(|lane| s.unit(P1, lane))
            .filter(|unit| unit.def_id == RUSH_TOKEN)
            .collect()
    }

    /// The keywords a token gained beyond its printed Rush: all from R21's pool, all different, never Rush.
    fn gained(s: &Scenario, token: &str) -> Vec<String> {
        // Read off the card's grants, since an Indestructible one shows no Taunt (R347, R781004).
        let keys: Vec<String> = s
            .card(token)
            .granted_keywords
            .iter()
            .map(keyword_key)
            .filter(|key| key != "Rush")
            .collect();
        for key in &keys {
            assert!(RANDOM_KEYWORD_POOL.contains(&key.as_str()), "{key} is in R21's pool");
        }
        let distinct: IndexSet<&String> = keys.iter().collect();
        assert_eq!(distinct.len(), keys.len());
        keys
    }

    /// The id of p1's first Rush Token, or "".
    fn first_token(s: &Scenario) -> String {
        tokens(s).into_iter().next().map(|token| token.id).unwrap_or_default()
    }

    /// p1 holds the Conjure (base) and a filler, so the turn goes on.
    fn conjuring(seed: &str) -> Scenario {
        scenario(json!({ "seed": seed, "p1": { "hand": [CARD, FILLER] }, "p2": { "hand": [FILLER] } }))
    }

    /// The coin the Cry flips first, for a game built and not yet played: the first draw of the match rng
    /// (R1440: with Lucky `lucky`, that many more coins, heads kept).
    fn heads_at(s: &Scenario, lucky: i32) -> bool {
        Rng::new(&s.state().seed, s.state().rng_cursor).lucky_coin(lucky)
    }

    /// The first seed of `seed-N` whose coin, at Lucky `lucky`, lands `heads`.
    fn seed_with(heads: bool, lucky: i32) -> String {
        for n in 0..200 {
            let seed = format!("c16-coin-{n}");
            if heads_at(&conjuring(&seed), lucky) == heads {
                return seed;
            }
        }
        panic!("no seed flips heads {heads}");
    }

    fn coins(s: &Scenario) -> Vec<GameEvent> {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Rolled { kind: RollKind::Coin, .. }))
            .cloned()
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r21_r1440_r3140002_flips_a_coin_and_summons_one_rush_token_3_3_rush_with_3_keywords_on_heads_2_on_tails() {
            let (mut heads_seen, mut tails_seen) = (0, 0);
            let mut seen: IndexSet<String> = IndexSet::new();
            for seed in 1..=24 {
                let mut s = conjuring(&format!("c16-{seed}"));
                let heads = heads_at(&s, 0);
                s.play(CARD, json!({}));
                let made = tokens(&s);
                assert_eq!(made.len(), 1);
                let token = made.into_iter().next().expect("no token");
                assert_eq!(s.unit(P1, 1).map(|unit| unit.id), Some(token.id.clone()));
                s.expect_stats(&token, json!({ "attack": 3, "health": 3 }));
                assert!(s.stats(&token).keywords.iter().map(keyword_key).any(|key| key == "Rush"));
                let keys = gained(&s, &token.id);
                assert_eq!(keys.len(), if heads { HEADS } else { TAILS }, "seed {seed}");
                if heads {
                    heads_seen += 1;
                } else {
                    tails_seen += 1;
                }
                seen.extend(keys);
            }
            assert!(heads_seen > 0 && tails_seen > 0, "{heads_seen} heads, {tails_seen} tails");
            assert!(seen.len() > HEADS);
        }

        #[test]
        fn r2652001_the_coin_is_shown_to_both_players_before_the_token_it_decided() {
            let mut s = conjuring(&seed_with(true, 0));
            s.play(CARD, json!({}));
            let events = s.events();
            let rolled = events
                .iter()
                .position(|event| matches!(event, GameEvent::Rolled { kind: RollKind::Coin, .. }))
                .expect("the coin is reported");
            let summoned = events
                .iter()
                .position(|event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == RUSH_TOKEN))
                .expect("the token is summoned");
            assert!(rolled < summoned);
            assert_eq!(coins(&s).len(), 1);
            for viewer in [PlayerId::P1, PlayerId::P2] {
                assert!(
                    s.view(viewer).events.iter().any(|event| matches!(event, GameEvent::Rolled { kind: RollKind::Coin, .. })),
                    "{viewer} sees the coin"
                );
            }
        }

        #[test]
        fn r1440_the_cards_lucky_flips_more_coins_and_keeps_heads() {
            // A plain tails that Lucky 2 turns to heads: 3 keywords where the plain card gave 2.
            let mut turned = false;
            for n in 0..200 {
                let seed = format!("c16-lucky-{n}");
                let plain = heads_at(&conjuring(&seed), 0);
                let mut s = conjuring(&seed);
                crate::give_lucky(&mut s, CARD, 2);
                let lucky = heads_at(&s, 2);
                assert!(lucky || !plain, "Lucky never turns heads to tails");
                s.play(CARD, json!({}));
                let keys = gained(&s, &first_token(&s)).len();
                assert_eq!(keys, if lucky { HEADS } else { TAILS }, "{seed}");
                turned |= lucky && !plain;
            }
            assert!(turned, "Lucky 2 turned a tails to heads on some seed");
        }

        #[test]
        fn r64_r129_a_full_board_summons_nothing_and_rolls_no_keywords_though_the_coin_flipped() {
            let mut s = scenario(json!({
                "seed": "c16-full",
                "p1": { "hand": [CARD, FILLER], "field": [BODY, BODY, BODY, BODY, BODY] },
                "p2": { "hand": [FILLER] },
            }));
            let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
            rng.coin();
            let after_the_coin = rng.cursor();
            s.play(CARD, json!({}));
            assert!(tokens(&s).is_empty());
            assert_eq!(s.state().rng_cursor, after_the_coin, "one coin, no keyword draws");
            s.expect_in_zone(CARD, "graveyard");
        }

        #[test]
        fn r386_the_keywords_read_through_param_an_upgrade_gives_one_more_on_heads_a_degrade_one_fewer() {
            let heads = seed_with(true, 0);
            let mut up = conjuring(&heads);
            step_param(up.card_mut(CARD), "keywords", 1);
            up.play(CARD, json!({}));
            assert_eq!(gained(&up, &first_token(&up)).len(), HEADS + 1);

            let mut down = conjuring(&heads);
            step_param(down.card_mut(CARD), "keywords", -1);
            down.play(CARD, json!({}));
            assert_eq!(gained(&down, &first_token(&down)).len(), HEADS - 1);
        }

        #[test]
        fn r386_the_tails_count_is_its_own_number_an_upgrade_gives_one_more_on_tails_alone() {
            let tails = seed_with(false, 0);
            let mut up = conjuring(&tails);
            assert_eq!(crate::upgrade_number(&mut up, CARD, "tails"), TAILS as i32 + 1);
            up.play(CARD, json!({}));
            assert_eq!(gained(&up, &first_token(&up)).len(), TAILS + 1);

            // The heads count is the other number: raising it leaves a tails where it was.
            let mut heads_up = conjuring(&tails);
            step_param(heads_up.card_mut(CARD), "keywords", 1);
            heads_up.play(CARD, json!({}));
            assert_eq!(gained(&heads_up, &first_token(&heads_up)).len(), TAILS);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn summons_three_rush_tokens_into_the_leftmost_open_zones_each_with_its_own_5_random_keywords() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": CARD, "radiant": true }, FILLER], "field": [{ "def": BODY, "lane": 2 }] },
                "p2": { "hand": [FILLER] },
            }));
            s.play(CARD, json!({}));
            let made = tokens(&s);
            let zones: Vec<Value> = made.iter().map(|token| js(&s.card(token).zone)).collect();
            let expected: Vec<Value> = [1, 3, 4]
                .into_iter()
                .map(|lane| json!({ "z": "field", "player": "p1", "row": "units", "lane": lane }))
                .collect();
            assert_eq!(zones, expected);
            for token in &made {
                assert!(!token.radiant);
                assert_eq!(gained(&s, &token.id).len(), RADIANT_KEYWORDS);
            }
            assert!(coins(&s).is_empty(), "the Radiant face flips no coin");
        }

        /// R64 a nearly full board summons fewer: one open zone, one token
        #[test]
        fn r64_a_nearly_full_board_summons_fewer_one_open_zone_one_token() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": CARD, "radiant": true }, FILLER], "field": [BODY, BODY, BODY, BODY] },
                "p2": { "hand": [FILLER] },
            }));
            s.play(CARD, json!({}));
            assert_eq!(tokens(&s).len(), 1);
            assert_eq!(s.unit(P1, 5).map(|unit| unit.def_id).as_deref(), Some(RUSH_TOKEN));
        }

        /// R386 an Upgrade summons 4 tokens and a Degrade 2; the base face's one token is never tuned
        #[test]
        fn r386_an_upgrade_summons_4_tokens_and_a_degrade_2_and_the_base_face_s_one_is_never_tuned() {
            for (upgrade, count) in [(true, 4), (false, 2)] {
                let mut s =
                    scenario(json!({ "p1": { "hand": [{ "def": CARD, "radiant": true }, FILLER] }, "p2": { "hand": [FILLER] } }));
                let moved = if upgrade {
                    crate::upgrade_number(&mut s, CARD, "tokens")
                } else {
                    crate::degrade_number(&mut s, CARD, "tokens")
                };
                assert_eq!(moved, count);
                s.play(CARD, json!({}));
                assert_eq!(tokens(&s).len(), count as usize);
            }
            let s = scenario(json!({ "p1": { "hand": [CARD, FILLER] } }));
            assert!(!crate::can_upgrade_number(&s, CARD, "tokens"));
            assert!(!crate::can_degrade_number(&s, CARD, "tokens"));
        }

        #[test]
        fn r386_the_keywords_are_tuned_on_the_radiant_face_and_the_tails_count_on_the_base_face_alone() {
            let s = scenario(json!({ "p1": { "hand": [{ "def": CARD, "radiant": true }, FILLER] } }));
            assert!(crate::can_upgrade_number(&s, CARD, "keywords"));
            assert!(!crate::can_upgrade_number(&s, CARD, "tails"));
        }
    }

    /// R767000: the script kept for the replays of v0.5.11p and before: no coin, 3 keywords on a base
    /// token, 3 on each of the Radiant's.
    mod v0_5_11o {
        use super::*;

        #[test]
        fn r767000_a_game_on_v0_5_11o_flips_no_coin_and_gives_3_keywords_on_both_faces() {
            let through_v0_5_11o = |def: &mut CardDef| {
                for param in def.params.iter_mut().flatten().filter(|param| param.key == "keywords") {
                    param.radiant = 3;
                }
            };
            for radiant in [false, true] {
                crate::under_kept_as(CARD, script_v0_5_11o(), through_v0_5_11o, || {
                    for n in 0..8 {
                        let mut s = scenario(json!({
                            "seed": format!("c16-kept-{n}"),
                            "p1": { "hand": [{ "def": CARD, "radiant": radiant }, FILLER] },
                            "p2": { "hand": [FILLER] },
                        }));
                        s.play(CARD, json!({}));
                        assert!(coins(&s).is_empty());
                        let made = tokens(&s);
                        assert_eq!(made.len(), if radiant { 3 } else { 1 });
                        for token in &made {
                            assert_eq!(gained(&s, &token.id).len(), 3, "radiant {radiant}, seed {n}");
                        }
                    }
                });
            }
        }
    }
}
