//! M #54 Money Machine (SPEC §8.8 row 54): (1) Field Spell, Common.
//!
//! Base:    "End of turn: Shuffle The Coin into each player's deck.\nActivate: Spend ({price}).
//!          Tribute this. Either player may activate this."
//! Radiant: "End of turn: Shuffle The Coin into your opponent's deck and a Radiant one into
//!          yours.\nActivate: Spend ({price}). Tribute this. Either player may activate this."
//! Engine:
//! - **End of turn** (its controller's): two fresh shuffles, The Coin (§7) into the opponent's deck,
//!   then a card into its controller's, R80's cap turning a full library away.
//! - **The hidden roll** (MD-D10, R1106): the controller's card is The Coin 89, Jade (M #39.2) 10 or
//!   Red Jade (M #39.4) 1, one match-rng draw over `MONEY_MACHINE_ODDS`, Radiant on the Radiant face.
//!   It is unprinted, as R673's Glitch roll is, so no Lucky or Luck reaches it, and a full library is
//!   rolled nothing: The Coin is turned away. Both cards go in unseen (`unseen`), counted as unknown
//!   in their owners' lists, so nothing tells a Jade from a Coin before it is drawn.
//! - **Activate** (MD-D9, R1105): either player, in their own main phase, pays {price} from their own
//!   mana and the card's own Tribute, a Sacrifice (R384); its uses are counted on the card, and
//!   `activated` names the activator. The price is the declared `price`, read through `mana_param`
//!   so a Nerf or a Buff reaches it (B3.4). Tunes: price 2 ↑.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-054";

/// "The Coin" (§7), the card the opponent always gets.
const THE_COIN: &str = "core-t-coin";

/// The card the Machine shuffles into its controller's deck: rolled over `MONEY_MACHINE_ODDS`, or
/// The Coin the full library turns away, rolled nothing (R1106).
fn yours(ctx: &mut EffectContext<'_>) -> String {
    if zone_count(&*ctx.state, ctx.controller, OffFieldZone::Library) >= LIBRARY_CAP {
        return THE_COIN.to_string();
    }
    roll_weighted(&mut *ctx.rng, MONEY_MACHINE_ODDS, 0)
}

fn money_machine(radiant: bool) -> Script {
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

pub fn script() -> CardScripts {
    CardScripts {
        base: money_machine(false),
        radiant: money_machine(true),
    }
}

// M #54 Money Machine — SPEC §8.8 row 54, BUILD M10 row M 54: "At your end of turn only, one The Coin
// into the opponent's deck and one card into yours that is The Coin 89%, a Jade 10% or a Red Jade 1%
// by one match-rng roll (MD-D10), both shuffled in unseen, so neither list names them; R80's cap
// refuses a full deck; Activate: its controller or the opponent, in their own main phase, pays 2 of
// their own mana and the Machine is Tributed (MD-D9), once per turn on the card, `activated` naming
// who; price reads through `param()`; radiant the opponent gets a base Coin, yours is Radiant, and so
// is the hidden Jade or Red Jade".
#[cfg(test)]
mod tests {
    use super::{ID, THE_COIN};
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

    /// The opponent activates the Machine (MD-D9).
    fn opponent_activates(s: &mut Scenario) {
        let id = s.card(ID).id.clone();
        s.act(
            ActionBody::Activate {
                instance_id: id,
                ability: Some("tribute".to_string()),
                targets: None,
                modes: None,
                tributes: None,
            },
            P2,
            "p2 activates Money Machine",
        );
    }

    #[test]
    fn r1106_the_odds_are_89_10_1_worst_to_best_and_add_to_100() {
        let table: Vec<(&str, i32)> = MONEY_MACHINE_ODDS
            .iter()
            .map(|entry| (entry.def_id, entry.percent))
            .collect();
        assert_eq!(table, vec![(THE_COIN, 89), (JADE, 10), (RED_JADE, 1)]);
    }

    mod base {
        use super::*;

        #[test]
        fn r1106_at_your_end_of_turn_a_coin_into_each_deck_both_unseen() {
            let mut s = machine("money-machine", false);
            let cursor = s.state().rng_cursor;
            s.end_turn();
            // p2's start of turn drew one of p2's three cards, so look at what is left plus the hand.
            let theirs: Vec<CardInstance> = s
                .pile(P2, "library")
                .into_iter()
                .chain(s.hand(P2))
                .filter(|card| card.def_id == THE_COIN)
                .collect();
            assert_eq!(theirs.len(), 1);
            assert!(!theirs[0].radiant);
            let mine = shuffled_in(&s, P1);
            assert_eq!(mine.len(), 1);
            assert!([THE_COIN, JADE, RED_JADE].contains(&mine[0].def_id.as_str()));
            assert!(!mine[0].radiant);
            // R1106: one roll for yours, then each shuffle's slot.
            let mut rng = create_rng(&s.state().seed, cursor);
            assert_eq!(roll_weighted(&mut rng, MONEY_MACHINE_ODDS, 0), mine[0].def_id);
            // R311, R312: p1's own list counts its card as unknown, never names it.
            let list = s.view(P1).you.own_library.expect("p1 reads its own list");
            assert_eq!(list.unknown, 1);
            assert!(list.cards.iter().all(|entry| entry.def_id == FILLER));
            // The shuffles' events name nothing to either seat (R97).
            for viewer in [P1, P2] {
                let view = serde_json::to_value(s.view(viewer)).expect("a view serialises");
                assert!(!view.to_string().contains(JADE));
                assert!(!view.to_string().contains(RED_JADE));
            }
        }

        #[test]
        fn r1106_the_roll_is_the_coin_89_jade_10_red_jade_1_by_seed() {
            let mut seen: Vec<String> = Vec::new();
            for n in 0..200 {
                let mut s = machine(&format!("money-machine-roll-{n}"), false);
                s.end_turn();
                let mine = shuffled_in(&s, P1);
                assert_eq!(mine.len(), 1);
                if !seen.contains(&mine[0].def_id) {
                    seen.push(mine[0].def_id.clone());
                }
            }
            // Over 200 seeds the Coin and the Jade both come up; the Red Jade is 1 in 100.
            assert!(seen.contains(&THE_COIN.to_string()));
            assert!(seen.contains(&JADE.to_string()));
            assert!(seen.iter().all(|id| [THE_COIN, JADE, RED_JADE].contains(&id.as_str())));
        }

        #[test]
        fn r1106_not_at_the_opponent_s_end_of_turn() {
            let mut s = machine("money-machine-theirs", false);
            s.end_turn();
            let (mine, theirs) = (s.pile(P1, "library").len(), s.pile(P2, "library").len());
            s.end_turn();
            // p2's end of turn adds nothing; p1's start of turn drew one.
            assert_eq!(s.pile(P1, "library").len(), mine - 1);
            assert_eq!(s.pile(P2, "library").len(), theirs);
        }

        #[test]
        fn r80_r1106_a_full_library_turns_the_coin_away_unrolled() {
            crate::register_all();
            let full: Vec<Value> = (0..LIBRARY_CAP).map(|_| json!(FILLER)).collect();
            let mut s = crate::scenario(json!({
                "seed": "money-machine-full",
                "p1": { "hand": [FILLER], "backrow": [ID], "library": full },
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
            }));
            s.end_turn();
            assert_eq!(s.pile(P1, "library").len() as i32, LIBRARY_CAP);
            // The card turned away is The Coin, so the overflow tells nothing of the odds.
            let overflow: Vec<&GameEvent> = s
                .events()
                .iter()
                .filter(|event| matches!(event, GameEvent::LibraryOverflow { .. }))
                .collect();
            assert_eq!(overflow.len(), 1);
            assert!(
                matches!(overflow[0], GameEvent::LibraryOverflow { def_id, player, .. } if def_id == THE_COIN && *player == P1)
            );
        }

        #[test]
        fn r1105_the_opponent_activates_it_in_their_main_phase_paying_their_own_mana() {
            let mut s = machine("money-machine-activate", false);
            s.end_turn();
            assert_eq!(s.state().active, P2);
            let machine_id = s.card(ID).id.clone();
            // Listed for p2 off p1's card, and in p2's view of it.
            let listed = legal_actions(s.state(), P2).into_iter().any(|body| {
                matches!(body, ActionBody::Activate { instance_id, .. } if instance_id == machine_id)
            });
            assert!(listed);
            let theirs = activation_views_for(s.state(), P2, s.card(ID)).unwrap_or_default();
            assert_eq!(theirs.len(), 1);
            assert!(theirs[0].usable);
            let (p1_mana, p2_mana) = (s.view(P1).you.mana.current, s.view(P2).you.mana.current);
            opponent_activates(&mut s);
            // p2 paid 2 of their own; the Machine is Tributed into p1's graveyard.
            assert_eq!(s.view(P2).you.mana.current, p2_mana - 2);
            assert_eq!(s.view(P1).you.mana.current, p1_mana);
            s.expect_in_zone(&machine_id, "graveyard");
            assert!(s.pile(P1, "graveyard").iter().any(|card| card.id == machine_id));
            // `activated` names the activator, and the Tribute is a death (§6.3).
            assert!(s.last_events().iter().any(|event| matches!(
                event,
                GameEvent::Activated { player, instance_id, .. } if *player == P2 && *instance_id == machine_id
            )));
            s.expect_events(json!(["activated", "manaChanged", "destroyed"]));
        }

        #[test]
        fn r1105_its_controller_activates_it_too() {
            let mut s = machine("money-machine-own", false);
            let mana = s.view(P1).you.mana.current;
            let machine_id = s.card(ID).id.clone();
            s.activate(ID, json!({ "ability": "tribute" }));
            assert_eq!(s.view(P1).you.mana.current, mana - 2);
            s.expect_in_zone(&machine_id, "graveyard");
            // Gone before the end of turn, it shuffles nothing.
            s.end_turn();
            assert!(shuffled_in(&s, P1).is_empty());
        }

        #[test]
        fn r1105_not_on_the_controller_s_turn_for_the_opponent_and_not_without_the_mana() {
            let mut s = machine("money-machine-refused", false);
            let machine_id = s.card(ID).id.clone();
            // p1's turn: p2 may not use it now.
            let refused = why_cannot_activate_ability(s.state(), P2, &machine_id, Some("tribute"))
                .err()
                .map(|error| error.message);
            assert_eq!(refused.as_deref(), Some("it is not your turn"));
            s.end_turn();
            s.state_mut().players.p2.mana.current = 1;
            let refused = why_cannot_activate_ability(s.state(), P2, &machine_id, Some("tribute"))
                .err()
                .map(|error| error.message);
            assert_eq!(refused.as_deref(), Some("that ability costs 2, more than your mana"));
        }

        #[test]
        fn r386_a_buff_of_its_price_reaches_the_cost() {
            let mut s = machine("money-machine-buff", false);
            assert_eq!(crate::upgrade_number(&mut s, ID, "price"), 3);
            let mana = s.view(P1).you.mana.current;
            s.activate(ID, json!({ "ability": "tribute" }));
            assert_eq!(s.view(P1).you.mana.current, mana - 3);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1106_a_base_coin_into_theirs_and_a_radiant_card_into_yours() {
            let mut seen_jade = false;
            for n in 0..200 {
                let mut s = machine(&format!("money-machine-radiant-{n}"), true);
                s.end_turn();
                let theirs: Vec<CardInstance> = s
                    .pile(P2, "library")
                    .into_iter()
                    .chain(s.hand(P2))
                    .filter(|card| card.def_id == THE_COIN)
                    .collect();
                assert_eq!(theirs.len(), 1);
                assert!(!theirs[0].radiant);
                let mine = shuffled_in(&s, P1);
                assert_eq!(mine.len(), 1);
                assert!(mine[0].radiant, "{}", mine[0].def_id);
                assert_eq!(s.view(P1).you.own_library.map(|list| list.unknown), Some(1));
                seen_jade |= mine[0].def_id == JADE;
            }
            // The hidden Jade is Radiant too.
            assert!(seen_jade);
        }

        #[test]
        fn r1105_the_radiant_face_s_activate_is_the_same() {
            let mut s = machine("money-machine-radiant-activate", true);
            s.end_turn();
            let machine_id = s.card(ID).id.clone();
            let mana = s.view(P2).you.mana.current;
            opponent_activates(&mut s);
            assert_eq!(s.view(P2).you.mana.current, mana - 2);
            s.expect_in_zone(&machine_id, "graveyard");
        }
    }
}
