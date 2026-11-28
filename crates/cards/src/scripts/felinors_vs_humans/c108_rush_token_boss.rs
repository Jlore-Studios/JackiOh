//! SPEC §8.14 FvH #108 Rush Token Boss ((4) Unit, Human, Rare, 8/10 → 16/20), a card of the Felinors vs
//! Humans set (R5235000).
//!   Base:    "End of turn: Summon {tokens|Rush Token|Rush Tokens}. Each attacks a random enemy." (2)
//!   Radiant: "End of turn: Summon {tokens|Radiant Rush Token|Radiant Rush Tokens}. Each attacks a random
//!            enemy." (2)
//!
//! At the end of its controller's turn (R62): `tokens` summons of the Rush Token (`core-t-rush`), each
//! into R64's zone, Radiant on the Radiant face, each remembered (TG #15 In a hurry's shape, R1126), and
//! then each makes one `forced_attack_random` (R53) on an enemy it may attack, the hero included, drawn as
//! that attack begins, so two may pick the same one. A token with no zone is not summoned and attacks
//! nothing (R129), and a token an earlier end of turn summoned never attacks again: each key is cleared
//! before its summon (R5261005). Built by S11 (#5261).

use jackioh_engine::effects::{forced_attack_random, remember};
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-108";

/// The token summoned.
const RUSH_TOKEN: &str = "core-t-rush";

/// The key the `n`th token of this end of turn is remembered under.
fn token_key(n: i32) -> String {
    format!("token{n}")
}

/// The tokens this end of turn summoned, by id: a key its summon left cleared names none.
fn summoned(ctx: &mut EffectContext<'_>, tokens: i32) -> Vec<String> {
    (0..tokens)
        .filter_map(|n| recalled(ctx, &token_key(n)))
        .filter_map(|id| id.as_str().map(str::to_string))
        .collect()
}

fn boss() -> Script {
    Script {
        end_of_turn: Some(hook(|ctx| {
            let tokens = param(&*ctx, "tokens");
            let radiant = ctx.radiant;
            let mut effects: Vec<Effect> = Vec::new();
            for n in 0..tokens {
                // R5261005: last turn's token under this key is no token of this one.
                effects.push(remember(json_as(json!({ "key": token_key(n), "value": null }))));
                effects.push(summon(json_as(json!({
                    "defId": RUSH_TOKEN,
                    "radiant": radiant,
                    "rememberAs": token_key(n),
                }))));
            }
            effects.push(for_each_card(ForEachCardArgs {
                cards: Arc::new(move |ctx| summoned(ctx, tokens)),
                each: Arc::new(|id| {
                    let attacker = TargetSpec::Instance { instance_id: id.to_string() };
                    forced_attack_random(ForcedAttackRandomArgs { attacker, among: None, times: None })
                }),
            }));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: the Radiant face's tokens are Radiant by its face (`ctx.radiant`).
    CardScripts {
        base: boss(),
        radiant: boss(),
    }
}

// FvH #108 Rush Token Boss — SPEC §8.14 row 108 (R5261005).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself
    const VANILLA: &str = "stand-in (1) 1/1";
    /// An enemy that strikes back for nothing and outlives any token's hit.
    const SPONGE: &str = "stand-in (1) 0/30";

    fn board(radiant: bool, mine: Value, theirs: Value, seed: &str) -> Scenario {
        let mut field = vec![json!({ "def": ID, "lane": 1, "radiant": radiant })];
        field.extend(mine.as_array().cloned().unwrap_or_default());
        scenario(json!({
            "seed": seed,
            "p1": { "field": field, "hand": [FILLER], "library": filler(4) },
            "p2": { "field": theirs, "hand": [FILLER], "library": filler(4) },
        }))
    }

    fn tokens(s: &Scenario) -> Vec<CardInstance> {
        (1..=UNIT_ZONES)
            .filter_map(|lane| s.unit(P1, lane))
            .filter(|card| card.def_id == RUSH_TOKEN)
            .collect()
    }

    fn enemy_hero_health(s: &Scenario) -> i32 {
        hero_of(s.state(), P2).health
    }

    /// The token's base Attack and its Radiant face's.
    fn token_attack(radiant: bool) -> i32 {
        let def = def_of(None, RUSH_TOKEN);
        let face = if radiant { &def.radiant } else { &def.base };
        must(face.attack, "the token's Attack")
    }

    fn summons_two_that_hit_the_empty_board_s_hero(radiant: bool) {
        let mut s = board(radiant, json!([]), json!([]), "rush-token-boss-hero");
        s.end_turn();
        let made = tokens(&s);
        assert_eq!(made.len(), 2, "two Rush Tokens");
        for token in nonempty(made, "the tokens") {
            assert_eq!(token.radiant, radiant);
        }
        assert_eq!(enemy_hero_health(&s), HERO_HEALTH - 2 * token_attack(radiant), "each hit the hero once");
    }

    mod base {
        use super::*;

        #[test]
        fn at_the_end_of_your_turn_two_rush_tokens_each_attack_the_only_enemy_the_hero() {
            summons_two_that_hit_the_empty_board_s_hero(false);
        }

        #[test]
        fn each_attacks_a_random_enemy_unit_or_hero_once() {
            for seed in 0..6 {
                let mut s = board(false, json!([]), json!([SPONGE]), &format!("rush-token-boss-{seed}"));
                s.end_turn();
                let sponge = s.unit_at(P2, 1);
                let dealt = HERO_HEALTH - enemy_hero_health(&s) + sponge.damage;
                assert_eq!(dealt, 2 * token_attack(false), "two attacks, one each, on an enemy");
                assert_eq!(tokens(&s).len(), 2, "both survive the 0-Attack sponge");
            }
        }

        #[test]
        fn r5261005_a_token_with_no_zone_is_not_summoned_and_attacks_nothing() {
            let crowd = json!([VANILLA, VANILLA, VANILLA]);
            let mut s = board(false, crowd, json!([]), "rush-token-boss-crowded");
            s.end_turn();
            assert_eq!(tokens(&s).len(), 1, "one zone, one token");
            assert_eq!(enemy_hero_health(&s), HERO_HEALTH - token_attack(false));
        }

        #[test]
        fn r5261005_a_token_of_an_earlier_turn_never_attacks_again() {
            let mut s = board(false, json!([VANILLA, VANILLA]), json!([]), "rush-token-boss-full");
            s.end_turn();
            assert_eq!(tokens(&s).len(), 2, "the row is full now");
            let after_first = enemy_hero_health(&s);
            assert_eq!(after_first, HERO_HEALTH - 2 * token_attack(false));
            s.end_turn();
            s.end_turn();
            assert_eq!(tokens(&s).len(), 2, "no zone, no token");
            assert_eq!(enemy_hero_health(&s), after_first, "and last turn's tokens attack nothing");
        }

        #[test]
        fn nothing_happens_at_the_end_of_the_opponent_s_turn() {
            let mut s = board(false, json!([]), json!([]), "rush-token-boss-theirs");
            s.end_turn();
            let made = tokens(&s).len();
            let health = enemy_hero_health(&s);
            s.end_turn();
            assert_eq!(tokens(&s).len(), made);
            assert_eq!(enemy_hero_health(&s), health);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn at_the_end_of_your_turn_two_radiant_rush_tokens_each_attack_the_only_enemy_the_hero() {
            summons_two_that_hit_the_empty_board_s_hero(true);
        }
    }
}
