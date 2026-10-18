//! Meditative+ #1.1 Base Camp (SPEC §8.9 row 1.1, §7, R1185001, R429, R840, R850, R1178002).
//! (0) Spell, AI, Token: an AI generated card M+ #1 Bob's Plan adds after M #8 Reach the Summit.
//!   Base:    "Every Reach the Summit you own goes up {levels|Ascent level|Ascent levels}. Add
//!            {coins|Coin|Coins} to your hand."
//!   Radiant: "Every Reach the Summit you own goes up {levels|Ascent level|Ascent levels}. Add
//!            {coins|Radiant Coin|Radiant Coins} to your hand."
//!
//! M #8's Ascent level is R429's play count (R840), so the levels are `raise_times_played` on every
//! Reach the Summit the caster owns, wherever it is (R1185001): its next play resolves the level it
//! reached (R841) and costs that much (R842); a level of 10 wins only when the card is played (R850).
//! A copy made later starts at 0 (R57). Then The Coin (§7), Radiant on the Radiant face, a full hand
//! burning what does not fit (§2.4).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-001-1";

/// M #8 Reach the Summit, whose Ascent level this raises.
const REACH_THE_SUMMIT: &str = "meditative-008";
/// The Coin (§7).
const COIN: &str = "core-t-coin";

fn base_camp(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut out = vec![raise_times_played(json_as(json!({
                "defId": REACH_THE_SUMMIT,
                "amount": param(&*ctx, "levels"),
            })))];
            for _ in 0..param(&*ctx, "coins").max(0) {
                out.push(add_to_hand(json_as(json!({ "defId": COIN, "radiant": radiant }))));
            }
            out
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: base_camp(false),
        radiant: base_camp(true),
    }
}

// Meditative+ #1.1 Base Camp — SPEC §8.9 row 1.1, BUILD M11 row M+ 1.1: "every Reach the Summit its
// caster owns, in hand, deck and graveyard alike, goes up 2 levels (its next play resolves that
// level's line, R841, and costs that much, R842); one the opponent owns is untouched; a Reach the
// Summit at 8 raised to 10 does not win until it is played, then wins (R850); adds 2 Coins, a full
// hand burning them; levels and coins read through `param()`; radiant 3 levels and 2 Radiant Coins".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";

    fn camp(seed: &str, radiant: bool, p1: Value, p2_hand: Value) -> Scenario {
        crate::register_all();
        let mut p1 = p1;
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend(p1["hand"].as_array().cloned().unwrap_or_default());
        p1["hand"] = json!(hand);
        if p1.get("library").is_none() {
            p1["library"] = json!([FILLER, FILLER, FILLER]);
        }
        scenario(json!({
            "seed": seed,
            "p1": p1,
            "p2": { "hand": p2_hand, "library": [FILLER, FILLER, FILLER] },
        }))
    }

    fn summits(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        [s.hand(player), s.pile(player, "library"), s.pile(player, "graveyard")]
            .concat()
            .into_iter()
            .filter(|card| card.def_id == REACH_THE_SUMMIT)
            .collect()
    }

    fn coins(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id == COIN).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r1185001_every_reach_the_summit_you_own_in_hand_deck_and_graveyard_goes_up_2_levels() {
            let mut s = camp(
                "base-camp",
                false,
                json!({
                    "hand": [REACH_THE_SUMMIT],
                    "library": [REACH_THE_SUMMIT, FILLER, FILLER],
                    "graveyard": [REACH_THE_SUMMIT],
                }),
                json!([REACH_THE_SUMMIT, FILLER]),
            );
            s.play(ID, json!({}));
            let mine = summits(&s, P1);
            assert_eq!(mine.len(), 3);
            for card in &mine {
                assert_eq!(times_played_of(card), 2, "{:?}", card.zone);
            }
            assert_eq!(summits(&s, P2)[0].times_played, None, "the opponent's is untouched");
        }

        #[test]
        fn r842_its_next_play_costs_that_level_and_resolves_that_levels_line() {
            let mut s = camp("base-camp-next", false, json!({ "hand": [REACH_THE_SUMMIT] }), json!([FILLER]));
            s.play(ID, json!({}));
            let summit = summits(&s, P1)[0].clone();
            assert_eq!(play_cost(s.state(), &summit), 2, "R842: it costs its level");
            let hand_before = s.hand(P1).len();
            s.play(&summit, json!({}));
            // Ascent 2: draw 2 (R841); the card itself left the hand.
            assert_eq!(s.hand(P1).len(), hand_before - 1 + 2);
        }

        #[test]
        fn r850_a_reach_the_summit_raised_to_10_wins_only_when_it_is_played() {
            let mut s = camp("base-camp-win", false, json!({ "hand": [REACH_THE_SUMMIT] }), json!([FILLER]));
            let summit = summits(&s, P1)[0].clone();
            s.card_mut(&summit).times_played = Some(8);
            s.play(ID, json!({}));
            assert_eq!(times_played_of(s.card(&summit)), 10);
            assert!(s.state().result.is_none(), "raising the level never wins by itself");
            s.state_mut().players.p1.mana.current = 10;
            s.play(&summit, json!({}));
            let result = s.state().result.expect("Ascent 10 wins as it is played");
            assert_eq!((result.winner, result.reason), (Winner::P1, GameOverReason::WonByEffect));
        }

        #[test]
        fn adds_2_coins_and_a_full_hand_burns_them() {
            let mut s = camp("base-camp-coins", false, json!({ "hand": [FILLER] }), json!([FILLER]));
            s.play(ID, json!({}));
            let made = coins(&s);
            assert_eq!(made.len(), 2);
            assert!(made.iter().all(|coin| !coin.radiant));
            let mut s = camp(
                "base-camp-full",
                false,
                json!({ "hand": vec![FILLER; 9] }),
                json!([FILLER]),
            );
            s.play(ID, json!({}));
            assert_eq!(coins(&s).len(), 1, "one Coin fills the hand");
            assert_eq!(
                s.pile(P1, "graveyard").iter().filter(|card| card.def_id == COIN).count(),
                1,
                "§2.4: the other is burned"
            );
        }

        #[test]
        fn levels_and_coins_read_through_param() {
            let mut s = camp("base-camp-param", false, json!({ "hand": [REACH_THE_SUMMIT] }), json!([FILLER]));
            assert_eq!(crate::upgrade_number(&mut s, ID, "levels"), 3);
            assert_eq!(crate::upgrade_number(&mut s, ID, "coins"), 3);
            s.play(ID, json!({}));
            assert_eq!(times_played_of(&summits(&s, P1)[0]), 3);
            assert_eq!(coins(&s).len(), 3);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1185001_3_levels_and_2_radiant_coins() {
            let mut s = camp("base-camp-radiant", true, json!({ "hand": [REACH_THE_SUMMIT] }), json!([FILLER]));
            s.play(ID, json!({}));
            assert_eq!(times_played_of(&summits(&s, P1)[0]), 3);
            let made = coins(&s);
            assert_eq!(made.len(), 2);
            assert!(made.iter().all(|coin| coin.radiant), "Radiant Coins, 2 mana each");
        }
    }
}
