//! M #14 Prime Time (SPEC §8.8 row 14, BUILD M10 row M 14). (2) Spell, Epic.
//!   Base:    "Draw {draws|card|cards} with a prime index from your deck." (3)
//!   Radiant: "Draw every card with a prime index from your deck."
//!
//! "Prime indexed" (R804) means the card's catalog index, read as a whole number, is a prime (2, 3, 5
//! … 97), in any set: indices repeat across sets, so Core #2 and Meditative #2 both count. It never
//! means a position in the deck, whose order is hidden. A token's "N.k" or T-name index, and a
//! transient card's (whose index is its id), fail the parse and never match.
//!
//! Each draw is `draw_from_library`, topmost match first (R801). The base face takes the first
//! `draws` matches; the Radiant face takes every match in the deck as it stood when the effect began
//! (§2.4's "draw your whole library" reading) — one memoized `for_each_card`, so a cast-on-draw pause
//! cannot move them. The hand cap burns the overflow. The number is declared and read through `param`
//! (R386); the Radiant face ignores it (`tunedOn: "base"`, R1431).

use jackioh_engine::effects::{ForEachCardArgs, draw_from_library, for_each_card};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-014";

/// The smallest prime: 0 and 1 are not prime.
const SMALLEST_PRIME: u32 = 2;

/// Trial division: the indices that can occur stay under a few thousand.
fn is_prime(n: u32) -> bool {
    if n < SMALLEST_PRIME {
        return false;
    }
    let mut divisor = SMALLEST_PRIME;
    while divisor * divisor <= n {
        if n % divisor == 0 {
            return false;
        }
        divisor += 1;
    }
    true
}

/// R804: the card's catalog index is a prime number. A token's "N.k" or T-name, and a transient
/// card's id, fail the parse and never match.
fn prime_indexed(state: &GameState, card: &CardInstance) -> bool {
    def_of(Some(state), &card.def_id).index.parse::<u32>().is_ok_and(is_prime)
}

/// The library's prime-indexed ids, top-down, at most `take` of them (`None` takes every match).
fn prime_ids(ctx: &EffectContext<'_>, take: Option<i32>) -> Vec<String> {
    let mut ids: Vec<String> = zone_cards(&*ctx.state, ctx.controller, OffFieldZone::Library)
        .iter()
        .filter(|card| prime_indexed(&ctx.state, card))
        .map(|card| card.id.clone())
        .collect();
    if let Some(take) = take {
        ids.truncate(take.max(0) as usize);
    }
    ids
}

/// One memoized `for_each_card` over the prime-indexed ids: read once as the list reaches it, so a
/// cast-on-draw pause resumes over the same set (R113).
fn prime_time(take: Option<i32>) -> Vec<Effect> {
    vec![for_each_card(ForEachCardArgs {
        cards: Arc::new(move |now: &mut EffectContext<'_>| prime_ids(now, take)),
        each: Arc::new(|instance_id: &str| {
            draw_from_library(json_as(json!({ "instanceId": instance_id })))
        }),
    })]
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            // "Draw 3 …": the declared number `draws` (R386, R1431).
            cry: Some(hook(|ctx| {
                let draws = param(&*ctx, "draws");
                prime_time(Some(draws))
            })),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| prime_time(None))),
            ..Script::default()
        },
    }
}

// M #14 Prime Time — SPEC §8.8 row 14, BUILD M10 row M 14: "Draw the 3 topmost prime-indexed cards
// of any set (R804); a token index never matches; fewer matches draw fewer without fatigue (R801);
// radiant draws every prime-indexed card and burns the overflow; its tuned number (draws) reads
// through `param()` (R386, R1431)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const PRIME: &str = "meditative-014";
    const FIVE: &str = "core-005"; // Index 5: prime. (1) Spell.
    const THREE: &str = "core-003"; // Index 3: prime. (1) Unit.
    const ELEVEN: &str = "core-011"; // Index 11: prime. (1) Unit.
    const NINETEEN: &str = "core-019"; // Index 19: prime. (3) Unit.
    const MED_FIVE: &str = "meditative-005"; // Index 5: prime, another set. (1) Spell.
    const EIGHT: &str = "core-008"; // Index 8: not prime.
    const TEN: &str = "core-010"; // Index 10: not prime.
    const FIFTEEN: &str = "core-015"; // Index 15: not prime.
    const TWENTY_FIVE: &str = "core-025"; // Index 25: not prime.
    const RUSH_TOKEN: &str = "core-t-rush"; // Index "T-rush": never prime.
    const FILLER: &str = "core-005"; // (1) Spell.

    /// The def ids of p1's `drawn` events, in order.
    fn drawn_defs(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Drawn { def_id, .. } => Some(def_id.clone()),
                _ => None,
            })
            .collect()
    }

    mod meditative_014 {
        use super::*;

        #[test]
        fn r804_draws_the_3_topmost_prime_indexed_cards_of_any_set() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [PRIME, FILLER],
                    "mana": 9,
                    "library": [FIVE, EIGHT, ELEVEN, FIFTEEN, MED_FIVE, TWENTY_FIVE, NINETEEN],
                },
                "p2": { "hand": [FILLER] },
            }));

            s.play(PRIME, json!({}));

            // Indices 5, 11 and 5 (Core, then Meditative: any set counts), skipping 8 and 15.
            assert_eq!(drawn_defs(&s), vec![FIVE, ELEVEN, MED_FIVE]);
            let library: Vec<String> = s.pile(P1, "library").iter().map(|card| card.def_id.clone()).collect();
            assert_eq!(library, vec![EIGHT, FIFTEEN, TWENTY_FIVE, NINETEEN]);
        }

        #[test]
        fn r804_a_token_index_never_matches() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [PRIME, FILLER],
                    "mana": 9,
                    "library": [RUSH_TOKEN, FIVE, EIGHT],
                },
                "p2": { "hand": [FILLER] },
            }));

            s.play(PRIME, json!({}));

            assert_eq!(drawn_defs(&s), vec![FIVE]);
            let library: Vec<String> = s.pile(P1, "library").iter().map(|card| card.def_id.clone()).collect();
            assert_eq!(library, vec![RUSH_TOKEN, EIGHT]);
        }

        #[test]
        fn r801_fewer_matches_draw_fewer_without_fatigue() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [PRIME, FILLER], "mana": 9, "library": [EIGHT, FIVE] },
                "p2": { "hand": [FILLER] },
            }));

            s.play(PRIME, json!({}));

            assert_eq!(drawn_defs(&s), vec![FIVE]);
            assert_eq!(s.pile(P1, "library").len(), 1);
            s.expect_health(P1, HERO_HEALTH);
            assert_eq!(s.state().players.p1.fatigue_count, 0);
        }

        #[test]
        fn r386_draws_reads_through_param() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [PRIME, FILLER],
                    "mana": 9,
                    "library": [FIVE, ELEVEN, MED_FIVE, NINETEEN],
                },
                "p2": { "hand": [FILLER] },
            }));
            step_param(s.card_mut(PRIME), "draws", 1);

            s.play(PRIME, json!({}));

            assert_eq!(drawn_defs(&s), vec![FIVE, ELEVEN, MED_FIVE, NINETEEN]);
        }

        #[test]
        fn radiant_draws_every_prime_indexed_card_and_burns_the_overflow() {
            crate::register_all();
            let mut hand: Vec<Value> = vec![json!({ "def": PRIME, "radiant": true })];
            hand.extend(vec![json!(TEN); 8]);
            let mut s = scenario(json!({
                "p1": {
                    "hand": hand,
                    "mana": 9,
                    "library": [FIVE, THREE, ELEVEN, MED_FIVE, NINETEEN, EIGHT],
                },
                "p2": { "hand": [FILLER] },
            }));

            s.play(PRIME, json!({}));

            // Nine cards plus the first draw: the other four burn into the graveyard (R4).
            assert_eq!(s.hand(P1).len(), 10);
            let library: Vec<String> = s.pile(P1, "library").iter().map(|card| card.def_id.clone()).collect();
            assert_eq!(library, vec![EIGHT]);
            let burned: Vec<String> =
                s.pile(P1, "graveyard").iter().map(|card| card.def_id.clone()).collect();
            assert_eq!(burned.len(), 4);
            for def in [THREE, ELEVEN, MED_FIVE, NINETEEN] {
                assert!(burned.contains(&def.to_string()), "{def} burns");
            }
        }
    }
}
