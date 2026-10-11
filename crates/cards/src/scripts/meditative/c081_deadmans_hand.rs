//! M #81 Deadman's Hand (SPEC §8.8 row 81): (0) embiggen (2) Spell, Rare.
//!
//! Base:    "Shuffle a copy of each card in your hand, and of this, into your deck.\nPaid (2): Each
//!          copy has a {chance}% chance to become Radiant."
//! Radiant: "Shuffle a copy of each card in your hand, and of this, into your deck. Then draw
//!          {draw|card|cards}.\nPaid (2): Each copy has a {chance}% chance to become Radiant."
//! Engine:
//! - **The copies:** for each card in this player's hand, in hand order, then for this card on the
//!   face it was played with, a library copy (`shuffle_into { copyOf }`: R57's Radiant flag, and the
//!   `tuning`, enchantments and `chinese` flag `copyOf` carries) at a random position (§6.3), R80's
//!   cap turning the rest away. The hand is kept. A copy is not generation (R387).
//! - **The roll** (R1144): "embiggen cards" is the embiggen price, the only text that names it. Paid
//!   at (2) (`ctx.embiggened`, §2.3), each copy whose card is not already Radiant rolls `chance` on the
//!   match rng once, before it goes in, so it goes in on the face it rolled (R311); at (0) none rolls,
//!   and a Radiant card's copy is Radiant without a roll (R129).
//! - **Then** the Radiant face's draw.
//! - **Hidden information** (R311): the owner's library list names the copies on the faces they went
//!   in with; the opponent reads how many went in.

use jackioh_engine::effects::shuffle_into;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-081";

/// The declared `chance` is a percentage.
const PERCENT: i32 = 100;

fn deadmans_hand(radiant: bool) -> Script {
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
            if radiant {
                effects.push(draw(json_as(json!({ "count": param(&*ctx, "draw") }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: deadmans_hand(false),
        radiant: deadmans_hand(true),
    }
}

// M #81 Deadman's Hand — SPEC §8.8 row 81, BUILD M10 row M 81: "Shuffles one copy of each hand card
// and one of itself (with R57's riders) into your deck at random positions, the hand kept, R80's cap
// refusing the rest; paid (2), each copy not already Radiant becomes Radiant on a 25% roll (R1144),
// and unpaid none rolls; the owner's list shows the copies, the opponent only the count; chance reads
// through `param()`; radiant paid, every copy becomes Radiant, then draw 1".
#[cfg(test)]
mod tests {
    use super::{ID, PERCENT};
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
            fn copies_hand_then_itself_hand_kept() {
                let mut s = holding("deadman-copies", false, json!([VANILLA, FILLER]), 3);
                let hand: Vec<String> = s.hand(P1).iter().skip(1).map(|card| card.id.clone()).collect();
                s.play(ID, json!({}));
                let defs: Vec<String> = copies(&s).into_iter().map(|card| card.def_id).collect();
                assert_eq!(defs, vec![VANILLA.to_string(), FILLER.to_string(), ID.to_string()]);
                // The hand is kept; the deck holds its three and the three copies.
                let kept: Vec<String> = s.hand(P1).iter().map(|card| card.id.clone()).collect();
                assert_eq!(kept, hand);
                assert_eq!(s.pile(P1, "library").len(), 6);
                // Deadman's Hand itself resolved into the graveyard, its copy into the deck.
                assert_eq!(s.pile(P1, "graveyard").iter().filter(|card| card.def_id == ID).count(), 1);
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
                let mut s = holding("deadman-r311", false, json!([{ "def": VANILLA, "radiant": true }]), 1);
                s.play(ID, json!({}));
                let mine = view(&s, P1);
                let list = &mine["you"]["ownLibrary"];
                assert_eq!(list["unknown"], json!(0));
                let cards = list["cards"].as_array().expect("the owner's library list has cards");
                assert!(cards.contains(&json!({ "defId": VANILLA, "radiant": true, "count": 1 })));
                assert!(cards.contains(&json!({ "defId": ID, "radiant": false, "count": 1 })));
                assert!(cards.contains(&json!({ "defId": FILLER, "radiant": false, "count": 1 })));
                // p2 reads p1's library as a count, and the shuffle-ins as hidden cards.
                let theirs = view(&s, P2);
                assert!(theirs["opponent"]["ownLibrary"].is_null());
                assert_eq!(theirs["opponent"]["libraryCount"], json!(3));
                let shuffled: Vec<Value> = theirs["events"]
                    .as_array()
                    .expect("events")
                    .iter()
                    .filter(|event| event["type"] == "shuffledIn")
                    .cloned()
                    .collect();
                assert_eq!(shuffled.len(), 2);
                assert!(shuffled.iter().all(|event| event["defId"] == json!(HIDDEN_ID)));
            }

            #[test]
            fn r80_a_full_deck_turns_the_rest_away() {
                let mut s = holding("deadman-full", false, json!([VANILLA, FILLER]), LIBRARY_CAP as usize - 1);
                s.play(ID, json!({}));
                assert_eq!(s.pile(P1, "library").len() as i32, LIBRARY_CAP);
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
            fn radiant_paid_all_radiant_then_draws_1() {
                let mut s = holding("deadman-radiant", true, json!([VANILLA, FILLER]), 0);
                s.play(ID, paid(true));
                let copied = copies(&s);
                assert_eq!(copied.len(), 3);
                assert!(copied.iter().all(|card| card.radiant));
                // Then one draw, from a deck of the three copies.
                assert_eq!(s.hand(P1).len(), 3);
                assert_eq!(s.pile(P1, "library").len(), 2);
                assert_eq!(s.state().players[P1].fatigue_count, 0);
            }

            #[test]
            fn radiant_unpaid_rolls_nothing_and_copies_this_radiant() {
                let mut s = holding("deadman-radiant-free", true, json!([VANILLA]), 2);
                s.play(ID, json!({}));
                let faces: Vec<bool> = copies(&s).iter().map(|card| card.radiant).collect();
                // The hand card is not Radiant and nothing rolled; this card was played Radiant.
                assert_eq!(faces, vec![false, true]);
                assert_eq!(s.hand(P1).len(), 2);
            }
        }
    }
}
