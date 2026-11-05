//! M #81 Deadman's Hand (SPEC §8.8 row 81): (0) embiggen (1) Spell, Rare ((2) through v0.5.11o).
//!
//! Base:    "Shuffle a copy of each card in your hand, and of this, into your deck. Then draw
//!          {draw|card|cards}.\nEmbiggened (1): Each copy has a {chance}% chance to become Radiant."
//!          (draw 1)
//! Radiant: the same, draw 3.
//! Engine:
//! - **The copies:** for each card in this player's hand, in hand order, then for this card on the
//!   face it was played with, a library copy (`shuffle_into { copyOf }`, R57's riders) at a random
//!   position (§6.3), R80's cap turning the rest away. A copy is not generation (R387).
//! - **The roll** (R1144): played at (1), (2) through v0.5.11o (`ctx.embiggened`, §2.3), each copy whose card is not
//!   already Radiant rolls `chance` on the match rng once, before it goes in, so it goes in on the
//!   face it rolled (R311); at (0) none rolls, and a Radiant card's copy is Radiant without a roll (R129).
//! - **The draw** (since #3140; through v0.5.11o the Radiant face alone drew, one card, `script_v0_5_11o`):
//!   `draw` cards after the copies went in, on either face, read through `param` (R386): 1 on the base face,
//!   3 on the Radiant one, both tuned.
//! - **Hidden information** (R311): the owner's library list names the copies on the faces they went
//!   in with; the opponent reads how many went in.

use jackioh_engine::effects::shuffle_into;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-081";

/// The declared `chance` is a percentage.
const PERCENT: i32 = 100;

/// One face. `base_draws` is whether the base face draws too: it does since #3140, and the Radiant face
/// always did.
fn deadmans_hand(radiant: bool, base_draws: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let chance = param(&*ctx, "chance");
            // (instance id, definition, whether that card is Radiant), the hand first, then this.
            let mut sources: Vec<(String, String, bool)> = zone_cards(ctx.state, ctx.controller, OffFieldZone::Hand)
                .into_iter()
                .map(|card| (card.id, card.def_id, card.radiant))
                .collect();
            if let Some(this) = ctx.self_.as_ref() {
                sources.push((this.id.clone(), this.def_id.clone(), ctx.radiant));
            }
            let mut effects: Vec<Effect> = Vec::new();
            for (id, def_id, already) in sources {
                let radiant_copy =
                    already || (ctx.embiggened && ctx.rng.chance(f64::from(chance) / f64::from(PERCENT)));
                effects.push(shuffle_into(json_as(json!({
                    "defId": def_id,
                    "count": 1,
                    "radiant": radiant_copy,
                    "copyOf": id,
                }))));
            }
            if radiant || base_draws {
                effects.push(draw(json_as(json!({ "count": param(&*ctx, "draw") }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: deadmans_hand(false, true),
        radiant: deadmans_hand(true, true),
    }
}

/// R767000: through v0.5.11o only the Radiant face drew (one card); `KEPT_SCRIPTS` folds those replays
/// under it.
pub fn script_v0_5_11o() -> CardScripts {
    CardScripts {
        base: deadmans_hand(false, false),
        radiant: deadmans_hand(true, false),
    }
}

// M #81 Deadman's Hand — SPEC §8.8 row 81, BUILD M10 row M 81.
#[cfg(test)]
mod tests {
    use super::{ID, PERCENT, script_v0_5_11o};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008";
    const FILLER: &str = "core-016";

    /// p1 holds Deadman's Hand (Radiant on `radiant`) and `hand`, with a deck of `deck` fillers.
    fn holding(seed: &str, radiant: bool, hand: Value, deck: usize) -> Scenario {
        let mut cards = vec![json!({ "def": ID, "radiant": radiant })];
        cards.extend(hand.as_array().cloned().unwrap_or_default());
        crate::scenario(json!({
            "seed": seed,
            "p1": { "hand": cards, "library": vec![FILLER; deck] },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }))
    }

    fn paid(embiggen: bool) -> Value {
        if embiggen { json!({ "embiggen": true }) } else { json!({}) }
    }

    /// The copies the play shuffled in, in the order they went in.
    fn copies(s: &Scenario) -> Vec<CardInstance> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::ShuffledIn { instance_id, .. } => Some(s.card(instance_id.as_str()).clone()),
                _ => None,
            })
            .collect()
    }

    fn view(s: &Scenario, seat: PlayerId) -> Value {
        crate::js(&s.view(seat))
    }

    mod m81_deadmans_hand {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn copies_hand_then_itself_hand_kept_and_then_draws_1() {
                let mut s = holding("deadman-copies", false, json!([VANILLA, FILLER]), 3);
                let hand: Vec<String> = s.hand(P1).iter().skip(1).map(|card| card.id.clone()).collect();
                s.play(ID, json!({}));
                let defs: Vec<String> = copies(&s).into_iter().map(|card| card.def_id).collect();
                assert_eq!(defs, vec![VANILLA.to_string(), FILLER.to_string(), ID.to_string()]);
                // The hand is kept and one card is drawn after the copies went in; the deck holds its three and
                // the three copies, less the card drawn.
                let kept: Vec<String> = s.hand(P1).iter().map(|card| card.id.clone()).collect();
                assert_eq!(kept[..hand.len()], hand[..]);
                assert_eq!(kept.len(), hand.len() + 1);
                assert_eq!(s.pile(P1, "library").len(), 5);
                // Deadman's Hand itself resolved into the graveyard, its copy into the deck.
                assert_eq!(s.pile(P1, "graveyard").iter().filter(|card| card.def_id == ID).count(), 1);
            }

            #[test]
            fn r3140000_the_draw_follows_the_last_copy() {
                let mut s = holding("deadman-order", false, json!([VANILLA]), 2);
                s.play(ID, json!({}));
                let events = s.events();
                let last_copy = events
                    .iter()
                    .rposition(|event| matches!(event, GameEvent::ShuffledIn { .. }))
                    .expect("the copies went in");
                let drawn = events
                    .iter()
                    .position(|event| matches!(event, GameEvent::Drawn { player: P1, .. }))
                    .expect("the card was drawn");
                assert!(last_copy < drawn);
            }

            #[test]
            fn r1144_unpaid_none_rolls() {
                for n in 0..8 {
                    let mut s = holding(&format!("deadman-free-{n}"), false, json!([VANILLA, FILLER, VANILLA]), 3);
                    s.play(ID, json!({}));
                    assert!(copies(&s).iter().all(|card| !card.radiant), "deadman-free-{n}");
                }
            }

            #[test]
            fn r1144_paid_rolls_per_copy() {
                // Each copy rolls 25% once, before it goes in, in hand order and then this card's: the first
                // four numbers the match rng gives as the hook runs. Over these seeds both outcomes occur.
                let (mut saw_radiant, mut saw_plain) = (false, false);
                for n in 0..16 {
                    let seed = format!("deadman-{n}");
                    let mut s = holding(&seed, false, json!([VANILLA, FILLER, VANILLA]), 3);
                    let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
                    let rolls: Vec<bool> = (0..4).map(|_| rng.chance(f64::from(25) / f64::from(PERCENT))).collect();
                    s.play(ID, paid(true));
                    let faces: Vec<bool> = copies(&s).iter().map(|card| card.radiant).collect();
                    assert_eq!(faces, rolls, "{seed}");
                    saw_radiant |= faces.iter().any(|radiant| *radiant);
                    saw_plain |= faces.iter().any(|radiant| !radiant);
                }
                assert!(saw_radiant && saw_plain);
            }

            #[test]
            fn r1144_radiant_source_copy_radiant() {
                let mut s = holding("deadman-radiant-source", false, json!([{ "def": VANILLA, "radiant": true }, FILLER]), 3);
                s.play(ID, json!({}));
                let faces: Vec<bool> = copies(&s).iter().map(|card| card.radiant).collect();
                assert_eq!(faces, vec![true, false, false]);
            }

            #[test]
            fn chance_reads_through_param() {
                let mut s = holding("deadman-param", false, json!([VANILLA, FILLER]), 3);
                set_param(s.card_mut(ID), "chance", 100);
                s.play(ID, paid(true));
                assert!(copies(&s).iter().all(|card| card.radiant));
            }

            #[test]
            fn r311_owner_lists_faces_opponent_a_count() {
                // The draw takes the top of a deck the copies were shuffled into at random: take a seed whose draw
                // is the dealt Filler, so the two copies are what the deck holds.
                for n in 0..60 {
                    let mut s = holding(&format!("deadman-r311-{n}"), false, json!([{ "def": VANILLA, "radiant": true }]), 1);
                    s.play(ID, json!({}));
                    if s.hand(P1).last().map(|card| card.def_id.as_str()) != Some(FILLER) {
                        continue;
                    }
                    let mine = view(&s, P1);
                    let list = &mine["you"]["ownLibrary"];
                    assert_eq!(list["unknown"], json!(0));
                    let cards = list["cards"].as_array().expect("the owner's library list has cards");
                    // The copies are fresh cards, so Created (R943); the dealt card is not.
                    let vanilla = json!({ "defId": VANILLA, "radiant": true, "created": true, "count": 1 });
                    let this = json!({ "defId": ID, "radiant": false, "created": true, "count": 1 });
                    assert!(cards.contains(&vanilla));
                    assert!(cards.contains(&this));
                    assert_eq!(cards.len(), 2);
                    // p2 reads p1's library as a count, and the shuffle-ins as hidden cards.
                    let theirs = view(&s, P2);
                    assert!(theirs["opponent"]["ownLibrary"].is_null());
                    assert_eq!(theirs["opponent"]["libraryCount"], json!(2));
                    let shuffled: Vec<Value> = theirs["events"]
                        .as_array()
                        .expect("events")
                        .iter()
                        .filter(|event| event["type"] == "shuffledIn")
                        .cloned()
                        .collect();
                    assert_eq!(shuffled.len(), 2);
                    assert!(shuffled.iter().all(|event| event["defId"] == json!(HIDDEN_ID)));
                    return;
                }
                panic!("no seed drew the dealt Filler");
            }

            #[test]
            fn r80_a_full_deck_turns_the_rest_away() {
                let mut s = holding("deadman-full", false, json!([VANILLA, FILLER]), LIBRARY_CAP as usize - 1);
                s.play(ID, json!({}));
                // The deck was full but for one place, took the first copy, and then gave up the card drawn.
                assert_eq!(s.pile(P1, "library").len() as i32, LIBRARY_CAP - 1);
                assert_eq!(copies(&s).iter().map(|card| card.def_id.clone()).collect::<Vec<String>>(), vec![VANILLA]);
                let refused = s
                    .events()
                    .iter()
                    .filter(|event| event.event_type() == GameEventType::LibraryOverflow)
                    .count();
                assert_eq!(refused, 2);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn radiant_paid_all_radiant_then_draws_3() {
                let mut s = holding("deadman-radiant", true, json!([VANILLA, FILLER]), 0);
                s.play(ID, paid(true));
                let copied = copies(&s);
                assert_eq!(copied.len(), 3);
                assert!(copied.iter().all(|card| card.radiant));
                // Then three draws, the whole deck of the three copies.
                assert_eq!(s.hand(P1).len(), 5);
                assert_eq!(s.pile(P1, "library").len(), 0);
                assert_eq!(s.state().players[P1].fatigue_count, 0);
            }

            #[test]
            fn radiant_unpaid_rolls_nothing_and_copies_this_radiant() {
                let mut s = holding("deadman-radiant-free", true, json!([VANILLA]), 2);
                s.play(ID, json!({}));
                let faces: Vec<bool> = copies(&s).iter().map(|card| card.radiant).collect();
                // The hand card is not Radiant and nothing rolled; this card was played Radiant.
                assert_eq!(faces, vec![false, true]);
                assert_eq!(s.hand(P1).len(), 4, "the hand card and three drawn");
            }
        }

        mod price_and_draw {
            use super::*;

            /// The price a play of Deadman's Hand paid, from its announcement.
            fn paid_for(s: &Scenario) -> i32 {
                s.events()
                    .iter()
                    .find_map(|event| match event {
                        GameEvent::CardPlayed { cost_paid, .. } => Some(*cost_paid),
                        _ => None,
                    })
                    .expect("the play is announced")
            }

            #[test]
            fn its_embiggened_version_costs_1_and_the_normal_one_nothing() {
                crate::register_all();
                assert_eq!(crate::card_def(ID).cost, CardCost::Embiggen { base: 0, embiggen: 1 });
                for radiant in [false, true] {
                    let mut s = holding("deadman-price", radiant, json!([VANILLA]), 3);
                    s.state_mut().players.p1.mana.current = 0;
                    s.play(ID, json!({}));
                    assert_eq!(paid_for(&s), 0, "radiant {radiant}");

                    let mut s = holding("deadman-price-embiggened", radiant, json!([VANILLA]), 3);
                    s.state_mut().players.p1.mana.current = 1;
                    s.play(ID, paid(true));
                    assert_eq!(paid_for(&s), 1, "radiant {radiant}");
                    assert_eq!(s.state().players.p1.mana.current, 0);
                }
            }

            #[test]
            fn its_embiggened_version_is_refused_without_that_mana() {
                let mut s = holding("deadman-price-short", false, json!([VANILLA]), 3);
                s.state_mut().players.p1.mana.current = 0;
                s.expect_refused(|s| s.play(ID, paid(true)));
            }

            #[test]
            fn r386_the_draw_is_tuned_on_both_faces_one_card_more_from_an_upgrade() {
                for (radiant, draws) in [(false, 1), (true, 3)] {
                    let mut s = holding("deadman-draw", radiant, json!([VANILLA]), 6);
                    assert!(crate::can_upgrade_number(&s, ID, "draw"), "radiant {radiant}");
                    assert_eq!(crate::upgrade_number(&mut s, ID, "draw"), draws + 1, "radiant {radiant}");
                    s.play(ID, json!({}));
                    assert_eq!(s.hand(P1).len(), 1 + draws as usize + 1, "radiant {radiant}");
                }
            }

            /// R767000: Deadman's Hand through v0.5.11o: the Radiant face alone drew, one card, and its
            /// Embiggened version cost (2).
            #[test]
            fn r767000_a_game_on_v0_5_11o_draws_on_the_radiant_face_alone_and_one_card() {
                let through_v0_5_11o = |def: &mut CardDef| {
                    def.cost = CardCost::Embiggen { base: 0, embiggen: 2 };
                    for param in def.params.iter_mut().flatten().filter(|param| param.key == "draw") {
                        param.radiant = 1;
                        param.tuned_on = Some(ParamTunedOn::Radiant);
                    }
                };
                for (radiant, hand) in [(false, 1), (true, 2)] {
                    crate::under_kept_as(ID, script_v0_5_11o(), through_v0_5_11o, || {
                        let mut s = holding("deadman-kept", radiant, json!([VANILLA]), 4);
                        s.play(ID, json!({}));
                        assert_eq!(s.hand(P1).len(), hand, "radiant {radiant}");
                    });
                }
            }
        }
    }
}
