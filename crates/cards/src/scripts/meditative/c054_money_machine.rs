//! M #54 Money Machine (SPEC §8.8 row 54): (1) Field Spell, Common.
//!
//! Base:    "Cry: Add a The Coin to your hand.\nEnd of turn: Shuffle a The Coin into your opponent's
//!          deck.\nActivate: Shuffle a The Coin into your deck."
//! Radiant: "Cry: Add a Radiant The Coin to your hand.\nEnd of turn: Shuffle a The Coin into your
//!          opponent's deck.\nActivate: Shuffle a Radiant The Coin into your deck."
//! Engine (#1194, #3140):
//! - **Cry** (since #3140, `script_v0_5_11o` has none): one `add_to_hand` of The Coin (§7), Radiant on the
//!   Radiant face, as it is played; a full hand burns it (§2.4). It is a Cry, so it runs when the Machine
//!   is played or cast (R1), never when it is put into a backrow zone by a summon or a fusion.
//! - **End of turn** (its controller's): one fresh shuffle of The Coin (§7) into the opponent's deck, a
//!   base Coin on either face, R80's cap turning a full library away. Nothing is rolled, and the
//!   shuffle is open: the opponent's list names the Coin it was shown going in (R311).
//! - **Activate** (R384): its controller's alone, once a turn, with no cost: one open shuffle of The
//!   Coin into its controller's deck, Radiant on the Radiant face. The Machine stays.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-054";

/// "The Coin" (§7), the card the opponent always gets.
const THE_COIN: &str = "core-t-coin";

/// "Shuffle a The Coin into your opponent's deck." — open, a base Coin on either face (R311).
fn coin_for_the_opponent() -> Effect {
    shuffle_into(json_as(json!({ "defId": THE_COIN, "count": 1, "player": "enemy" })))
}

/// One face. `cry` is the Cry the Machine has had since #3140, which `script_v0_5_11o` lacks.
fn money_machine(radiant: bool, cry: bool) -> Script {
    Script {
        cry: cry.then(|| {
            hook(move |_ctx| vec![add_to_hand(json_as(json!({ "defId": THE_COIN, "radiant": radiant })))])
        }),
        end_of_turn: Some(hook(|_ctx| vec![coin_for_the_opponent()])),
        activations: vec![ActivationDecl {
            id: "coin".to_string(),
            label: if radiant {
                "Shuffle a Radiant The Coin into your deck".to_string()
            } else {
                "Shuffle a The Coin into your deck".to_string()
            },
            uses: ActivationUses::Count(1),
            cost: None,
            targets: Vec::new(),
            modes: Vec::new(),
            can_activate: None,
            has: None,
            run: hook(move |_ctx| {
                vec![shuffle_into(json_as(json!({ "defId": THE_COIN, "count": 1, "radiant": radiant })))]
            }),
        }],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: money_machine(false, true),
        radiant: money_machine(true, true),
    }
}

/// R767000: through v0.5.11p the Machine had no Cry; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    CardScripts {
        base: money_machine(false, false),
        radiant: money_machine(true, false),
    }
}

/// The card the v0.3.7b Machine shuffles into its controller's deck: rolled over `MONEY_MACHINE_ODDS`,
/// or The Coin the full library turns away, rolled nothing (R1106).
fn yours(ctx: &mut EffectContext<'_>) -> String {
    if room_in(&*ctx.state, ctx.controller, OffFieldZone::Library) == 0 {
        return THE_COIN.to_string();
    }
    roll_weighted(&mut *ctx.rng, MONEY_MACHINE_ODDS, 0)
}

/// R767000: the Machine v0.3.7b and every version before it ran: at its controller's end of turn two
/// unseen shuffles, The Coin into the opponent's deck and the rolled card into its controller's
/// (MD-D10, R1106), and an Activate either player pays {price} for to Tribute it (MD-D9, R1105).
fn money_machine_v0_3_7b(radiant: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| {
            let rolled = yours(ctx);
            vec![
                shuffle_into(json_as(json!({
                    "defId": THE_COIN,
                    "count": 1,
                    "player": "enemy",
                    "unseen": true,
                }))),
                shuffle_into(json_as(json!({
                    "defId": rolled,
                    "count": 1,
                    "radiant": radiant,
                    "unseen": true,
                }))),
            ]
        })),
        activations: vec![ActivationDecl {
            id: "tribute".to_string(),
            label: "Tribute this".to_string(),
            uses: ActivationUses::Count(1),
            cost: Some(ActivationCost {
                mana_param: Some("price"),
                tribute_self: Some(true),
                either_player: Some(true),
                ..ActivationCost::default()
            }),
            targets: Vec::new(),
            modes: Vec::new(),
            can_activate: None,
            has: None,
            run: hook(|_ctx| vec![]),
        }],
        ..Script::default()
    }
}

/// R767000: the scripts v0.3.7b and every version before it ran (`money_machine_v0_3_7b`);
/// `KEPT_SCRIPTS` folds those versions' replays under them.
pub fn script_v0_3_7b() -> CardScripts {
    CardScripts {
        base: money_machine_v0_3_7b(false),
        radiant: money_machine_v0_3_7b(true),
    }
}

// M #54 Money Machine — SPEC §8.8 row 54, BUILD M10 row M 54.
#[cfg(test)]
mod tests {
    use super::{ID, THE_COIN, script_v0_5_11o};
    use jackioh_engine::subsystems::activate::{activation_views_for, why_cannot_activate_ability};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-016";
    const JADE: &str = "meditative-039-2";
    const RED_JADE: &str = "meditative-039-4";

    /// p1 has the Machine (Radiant on `radiant`) in the backrow; both sides hold a card and have a
    /// two-card library, so a turn passes without auto-ending.
    fn machine(seed: &str, radiant: bool) -> Scenario {
        crate::scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [FILLER],
                "backrow": [{ "def": ID, "radiant": radiant }],
                "library": [FILLER, FILLER],
            },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }))
    }

    /// The library cards of `player` that are not the fillers the scenario dealt.
    fn shuffled_in(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        s.pile(player, "library")
            .into_iter()
            .filter(|card| card.def_id != FILLER)
            .collect()
    }

    /// The Coins `player` has in their library and hand.
    fn coins_of(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        s.pile(player, "library")
            .into_iter()
            .chain(s.hand(player))
            .filter(|card| card.def_id == THE_COIN)
            .collect()
    }

    /// Whether `legal_actions` offers `player` an activation of the Machine.
    fn listed_for(s: &Scenario, player: PlayerId) -> bool {
        let machine_id = s.card(ID).id.clone();
        legal_actions(s.state(), player)
            .into_iter()
            .any(|body| matches!(body, ActionBody::Activate { instance_id, .. } if instance_id == machine_id))
    }

    /// p1 holds the Machine (Radiant on `radiant`) and `others` more cards in hand, to be played.
    fn holding(seed: &str, radiant: bool, others: usize) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend((0..others).map(|_| json!(FILLER)));
        crate::scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": [FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }))
    }

    /// The Coins in p1's hand.
    fn hand_coins(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id == THE_COIN).collect()
    }

    #[test]
    fn r1106_the_odds_are_89_10_1_worst_to_best_and_add_to_100() {
        // The v0.3.7b script's roll (`script_v0_3_7b`, R767000).
        let table: Vec<(&str, i32)> = MONEY_MACHINE_ODDS
            .iter()
            .map(|entry| (entry.def_id, entry.percent))
            .collect();
        assert_eq!(table, vec![(THE_COIN, 89), (JADE, 10), (RED_JADE, 1)]);
    }

    #[test]
    fn declares_no_tuned_numbers() {
        crate::register_all();
        assert!(crate::card_def(ID).params.is_none());
    }

    /// R767000: through v0.5.11p the Machine had no Cry, so playing it added nothing to the hand.
    #[test]
    fn r767000_a_game_on_v0_5_11o_adds_no_coin_when_it_is_played() {
        for radiant in [false, true] {
            crate::under_kept(ID, script_v0_5_11o(), || {
                let mut s = holding("money-machine-kept", radiant, 2);
                s.play(ID, json!({}));
                assert!(hand_coins(&s).is_empty(), "radiant {radiant}");
                s.expect_in_zone(ID, "field");
                // The rest of the card is the same: its End of turn still gives the opponent a Coin.
                s.end_turn();
                assert_eq!(coins_of(&s, P2).len(), 1);
            });
        }
    }

    mod base {
        use super::*;

        #[test]
        fn at_your_end_of_turn_one_coin_into_the_opponent_s_deck_and_none_into_yours() {
            let mut s = machine("money-machine", false);
            s.end_turn();
            // p2's start of turn drew one of p2's three cards, so look at what is left plus the hand.
            let theirs = coins_of(&s, P2);
            assert_eq!(theirs.len(), 1);
            assert!(!theirs[0].radiant);
            assert!(shuffled_in(&s, P1).is_empty());
            // R311: the shuffle is open, so p2's own list names the Coin it was shown going in.
            let list = s.view(P2).you.own_library.expect("p2 reads its own list");
            assert_eq!(list.unknown, 0);
            let in_hand = s.hand(P2).iter().any(|card| card.def_id == THE_COIN);
            assert!(in_hand || list.cards.iter().any(|entry| entry.def_id == THE_COIN));
        }

        #[test]
        fn not_at_the_opponent_s_end_of_turn() {
            let mut s = machine("money-machine-theirs", false);
            s.end_turn();
            let (mine, theirs) = (s.pile(P1, "library").len(), s.pile(P2, "library").len());
            s.end_turn();
            // p2's end of turn adds nothing; p1's start of turn drew one.
            assert_eq!(s.pile(P1, "library").len(), mine - 1);
            assert_eq!(s.pile(P2, "library").len(), theirs);
        }

        #[test]
        fn r80_a_full_opponent_library_turns_the_coin_away() {
            crate::register_all();
            let full: Vec<Value> = (0..LIBRARY_CAP).map(|_| json!(FILLER)).collect();
            let mut s = crate::scenario(json!({
                "seed": "money-machine-full",
                "p1": { "hand": [FILLER], "backrow": [ID], "library": [FILLER, FILLER] },
                "p2": { "hand": [FILLER], "library": full },
            }));
            s.end_turn();
            assert!(s.events().iter().any(|event| matches!(
                event,
                GameEvent::LibraryOverflow { def_id, player, .. } if def_id == THE_COIN && *player == P2
            )));
            assert!(coins_of(&s, P2).is_empty());
        }

        #[test]
        fn r384_activate_shuffles_a_coin_into_your_deck_once_a_turn_and_the_machine_stays() {
            let mut s = machine("money-machine-activate", false);
            let mana = s.view(P1).you.mana.current;
            let machine_id = s.card(ID).id.clone();
            assert!(listed_for(&s, P1));
            s.activate(ID, json!({}));
            let mine = shuffled_in(&s, P1);
            assert_eq!(mine.len(), 1);
            assert_eq!(mine[0].def_id, THE_COIN);
            assert!(!mine[0].radiant);
            // R311: open, so p1's own list names it.
            assert_eq!(s.view(P1).you.own_library.map(|list| list.unknown), Some(0));
            assert_eq!(s.view(P1).you.mana.current, mana);
            s.expect_in_zone(&machine_id, "field");
            // R384: "Activate" is once a turn.
            assert!(!listed_for(&s, P1));
            let refused =
                why_cannot_activate_ability(s.state(), P1, &machine_id, None).err().map(|error| error.message);
            assert_eq!(refused.as_deref(), Some("that ability has already been used this turn"));
        }

        #[test]
        fn r1105_r384_only_its_controller_activates_it_the_opponent_is_told_it_is_not_theirs() {
            let mut s = machine("money-machine-theirs-to-use", false);
            s.end_turn();
            assert_eq!(s.state().active, P2);
            let machine_id = s.card(ID).id.clone();
            let refused =
                why_cannot_activate_ability(s.state(), P2, &machine_id, Some("coin")).err().map(|error| error.message);
            assert_eq!(refused.as_deref(), Some("that card is not yours"));
            assert!(!listed_for(&s, P2));
            assert!(activation_views_for(s.state(), P2, s.card(ID)).unwrap_or_default().is_empty());
        }

        #[test]
        fn r1_cry_adds_a_the_coin_to_your_hand_as_it_is_played() {
            let mut s = holding("money-machine-cry", false, 2);
            s.play(ID, json!({}));
            let coins = hand_coins(&s);
            assert_eq!(coins.len(), 1);
            assert!(!coins[0].radiant);
            // The Machine, played, stands; its Coin is in the hand, not the deck.
            s.expect_in_zone(ID, "field");
            assert_eq!(s.hand(P1).len(), 3);
            assert!(coins_of(&s, P1).len() == 1 && shuffled_in(&s, P1).is_empty());
            // The Cry added a Coin to the hand and shuffled none into the opponent's deck.
            assert!(coins_of(&s, P2).is_empty());
        }

        #[test]
        fn r1_a_full_hand_burns_the_cry_s_coin() {
            // Ten cards beside the Machine: the hand is full once it is played, so the Coin burns (§2.4).
            let mut s = holding("money-machine-cry-full", false, HAND_CAP as usize);
            s.play(ID, json!({}));
            assert!(hand_coins(&s).is_empty());
            assert_eq!(s.hand(P1).len() as i32, HAND_CAP);
            assert!(s
                .events()
                .iter()
                .any(|event| matches!(event, GameEvent::Burned { def_id, .. } if def_id == THE_COIN)));
            s.expect_in_zone(ID, "field");
        }

        #[test]
        fn r1_a_machine_that_is_not_played_adds_no_coin() {
            // Set straight into the backrow (no play), it has no Cry to run.
            let s = machine("money-machine-no-cry", false);
            assert!(hand_coins(&s).is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1_cry_adds_a_radiant_the_coin_to_your_hand() {
            let mut s = holding("money-machine-cry-radiant", true, 2);
            s.play(ID, json!({}));
            let coins = hand_coins(&s);
            assert_eq!(coins.len(), 1);
            assert!(coins[0].radiant);
        }

        #[test]
        fn the_opponent_still_gets_a_base_coin_and_its_activate_shuffles_a_radiant_one_into_yours() {
            let mut s = machine("money-machine-radiant", true);
            s.activate(ID, json!({}));
            let mine = shuffled_in(&s, P1);
            assert_eq!(mine.len(), 1);
            assert_eq!(mine[0].def_id, THE_COIN);
            assert!(mine[0].radiant);
            assert_eq!(s.view(P1).you.own_library.map(|list| list.unknown), Some(0));
            s.end_turn();
            let theirs = coins_of(&s, P2);
            assert_eq!(theirs.len(), 1);
            assert!(!theirs[0].radiant);
            // Only the one the Activate made is in p1's deck.
            assert_eq!(shuffled_in(&s, P1).len(), 1);
        }
    }
}
