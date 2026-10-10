//! M #49.1 YileGPT Unleashed (SPEC §8.8 row 49.1): (10) Unit, CN, Acclaimed, Token (printed
//! Mythic), 8/20 → 16/40, Can't attack (Radiant: Can't attack, Armor 5).
//!
//! "Cry: Do {picks} of these at random …" / Radiant "Cry: [all five] …" plus "Start of turn: Make
//! [a random permanent / {radiants} random permanents] of yours Radiant" and "Whenever you end your
//! turn with unspent mana, this costs that much less."
//! Engine:
//! - The discount is a hand trigger and a deck trigger (R464) on its owner's `turnEnded`: they read
//!   the event's `unspentMana` (the number Bread and Butter reads) into a `costMod` of minus that
//!   (MD-C22). The `costMod` keeps in hand and deck until a graveyard or exile (R766), and the price
//!   floors at 0.
//! - The Cry is ME-PICK-N (MD-C23) over the five entries below: the base face does `picks` different
//!   ones by the match rng, named to both players and resolved in list order; the Radiant face does
//!   all five in order, with two Radiant Viruses for the opponent in place of the fill (MD-C24).
//! - Start of turn: `set_radiant_random` among your non-Radiant permanents on the field (R60, R242),
//!   this one included.

use jackioh_engine::effects::{
    DoRandomEntriesArgs, RandomEntry, add_to_hand, degrade, do_random_entries, fill_board,
    set_cost_mod, set_radiant_random, summon,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-049-1";

/// C+ #78 Claude's Datacenter, summoned twice into your backrow.
const DATACENTERS: &str = "classicplus-078";
/// C #18 Glitch in the System, added Radiant at (0).
const GLITCH: &str = "classic-018";
/// M #49.3 AI Girlfriend, summoned to animate at once.
const GIRLFRIEND: &str = "meditative-049-3";
/// M #49.2 Yile's Virus, filling the opponent's board (or two Radiant ones, Radiant face).
const VIRUS: &str = "meditative-049-2";

/// One clause the way the card prints it, with its current number.
fn plural(count: i32, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// The five entries in the list's order, with the face's current numbers (MD-C23).
fn entries(ctx: &EffectContext, radiant: bool) -> Vec<RandomEntry> {
    let datacenters = param(ctx, "datacenters");
    let glitches = param(ctx, "glitches");
    let set_cost = param(ctx, "setCost");
    let nerfs = param(ctx, "nerfs");
    let viruses = param(ctx, "viruses");
    vec![
        RandomEntry {
            label: format!(
                "Summon {}",
                plural(datacenters, "Claude's Datacenter", "Claude's Datacenters")
            ),
            effects: (0..datacenters)
                .map(|_| summon(json_as(json!({ "defId": DATACENTERS }))))
                .collect(),
        },
        RandomEntry {
            label: format!(
                "Add {} to your hand",
                plural(
                    glitches,
                    "Radiant Glitch in the System card",
                    "Radiant Glitch in the System cards"
                )
            ),
            effects: (0..glitches)
                .map(|_| {
                    add_to_hand(json_as(json!({
                        "defId": GLITCH,
                        "radiant": true,
                        "costOverride": set_cost,
                    })))
                })
                .collect(),
        },
        RandomEntry {
            label: "Summon an AI Girlfriend".to_string(),
            effects: vec![summon(json_as(json!({ "defId": GIRLFRIEND })))],
        },
        if radiant {
            RandomEntry {
                label: format!(
                    "Summon {} for your opponent",
                    plural(
                        viruses,
                        "Radiant Yile's Virus",
                        "Radiant Yile's Viruses"
                    )
                ),
                effects: (0..viruses)
                    .map(|_| {
                        summon(json_as(json!({
                            "defId": VIRUS,
                            "player": "enemy",
                            "radiant": true,
                        })))
                    })
                    .collect(),
            }
        } else {
            RandomEntry {
                label: "Fill your opponent's board with Yile's Viruses".to_string(),
                effects: vec![fill_board(json_as(json!({
                    "defId": VIRUS,
                    "player": "enemy",
                })))],
            }
        },
        RandomEntry {
            label: format!(
                "Nerf every card on your opponent's field, in their hand and in their deck {}",
                plural(nerfs, "time", "times")
            ),
            effects: vec![degrade(json_as(json!({
                "scope": { "side": "enemy", "zones": ["field", "hand", "library"] },
                "times": nerfs,
            })))],
        },
    ]
}

/// MD-C22: the ending player's unspent mana off the event — the number Bread and Butter reads — for
/// this card's owner only.
fn discount_for(ctx: &mut EffectContext, event: &GameEvent) -> Vec<Effect> {
    let (player, unspent) = match event {
        GameEvent::TurnEnded {
            player,
            unspent_mana,
            ..
        } => (*player, *unspent_mana),
        _ => return vec![],
    };
    if player != ctx.controller || unspent <= 0 {
        return vec![];
    }
    vec![set_cost_mod(json_as(json!({
        "target": { "of": "self" },
        "amount": -unspent,
    })))]
}

fn discount_trigger(id: &str) -> TriggerDef {
    TriggerDef::new(id, &[GameEventType::TurnEnded], discount_for).with_when(|ctx, event| {
        matches!(
            event,
            GameEvent::TurnEnded {
                player,
                unspent_mana,
                ..
            } if *player == ctx.controller && *unspent_mana > 0
        )
    })
}

fn unleashed(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            if radiant {
                // The Radiant face does all five, in order, with no roll (MD-C23).
                entries(&*ctx, true)
                    .into_iter()
                    .flat_map(|entry| entry.effects)
                    .collect()
            } else {
                vec![do_random_entries(DoRandomEntriesArgs {
                    entries: entries(&*ctx, false),
                    count: param(&*ctx, "picks"),
                })]
            }
        })),
        start_of_turn: Some(hook(move |ctx| {
            vec![set_radiant_random(json_as(json!({
                "zones": "field",
                "count": param(&*ctx, "radiants"),
            })))]
        })),
        hand_triggers: vec![discount_trigger("unleashed-discount-hand")],
        deck_triggers: vec![discount_trigger("unleashed-discount-deck")],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: unleashed(false),
        radiant: unleashed(true),
    }
}

// M #49.1 YileGPT Unleashed — SPEC §8.8 row 49.1, BUILD M10 row M 49.1.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const UNLEASHED: &str = "meditative-049-1";
    const DATACENTERS: &str = "classicplus-078";
    const GLITCH: &str = "classic-018";
    const GIRLFRIEND: &str = "meditative-049-3";
    const VIRUS: &str = "meditative-049-2";
    const FILLER: &str = "core-005";

    /// p1 holds the Unleashed (base unless `radiant_face`) with 10 mana to play it.
    fn holding(seed: &str, radiant_face: bool) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": UNLEASHED, "radiant": radiant_face }, FILLER],
                "library": [FILLER, FILLER, FILLER, FILLER],
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }))
    }

    /// The labels the Cry named to both players, in order.
    fn rolled(s: &Scenario) -> Vec<String> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::ChaosRolled { effects, .. } => Some(effects.clone()),
                _ => None,
            })
            .flatten()
            .collect()
    }

    /// What playing this copy now costs in its owner's hand (R65): never below 0.
    fn price_of(s: &Scenario, id: &str) -> i32 {
        jackioh_engine::mana::effective_cost(
            s.state(),
            s.card(id),
            jackioh_engine::mana::CostOptions::default(),
        )
    }

    fn count_def(s: &Scenario, player: PlayerId, zone: &str, def: &str) -> usize {
        s.pile(player, zone)
            .iter()
            .filter(|card| card.def_id == def)
            .count()
    }

    /// Units of this def on this side's field, by lane.
    fn field_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        (1..=5)
            .filter_map(|lane| s.unit(player, lane).map(|unit| unit.def_id))
            .collect()
    }

    /// Backrow cards on this side, by lane.
    fn backrow_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        (1..=5)
            .filter_map(|lane| s.backrow(player, lane).map(|card| card.def_id))
            .collect()
    }

    /// How many `degraded` reports the game emitted.
    fn degraded(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Degraded { .. }))
            .count()
    }

    mod m49_1_yilegpt_unleashed {
        use super::*;

        mod discount {
            use super::*;

            #[test]
            fn r1022_unspent_mana_lowers_its_cost_in_hand_and_deck_never_below_0() {
                let mut s = scenario(json!({
                    "seed": "unleashed-discount",
                    "p1": {
                        "hand": [UNLEASHED, FILLER],
                        "library": [UNLEASHED, FILLER, FILLER],
                        "mana": 10,
                    },
                    "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                }));
                let hand_id = s.hand(P1)[0].id.clone();
                let deck_id = s.pile(P1, "library")[0].id.clone();

                s.end_turn();

                // Each copy costs 10 less: a permanent `costMod`, kept in hand and deck (R766).
                assert_eq!(s.card(hand_id.as_str()).cost_mod, -10);
                assert_eq!(s.card(deck_id.as_str()).cost_mod, -10);
                assert_eq!(price_of(&s, &hand_id), 0);
                // Ending another turn with mana left lowers it again, and the price floors at 0.
                s.end_turn();
                s.end_turn();
                assert!(s.card(hand_id.as_str()).cost_mod < -10);
                assert_eq!(price_of(&s, &hand_id), 0);
            }

            #[test]
            fn r1022_no_unspent_mana_changes_nothing() {
                let mut s = scenario(json!({
                    "seed": "unleashed-discount-none",
                    "p1": {
                        "hand": [UNLEASHED, FILLER],
                        "library": [FILLER, FILLER],
                        "mana": 0,
                    },
                    "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                }));
                let hand_id = s.hand(P1)[0].id.clone();

                s.end_turn();

                assert_eq!(s.card(hand_id.as_str()).cost_mod, 0);
            }
        }

        mod base {
            use super::*;

            #[test]
            fn r1023_two_different_entries_named_and_in_order() {
                let mut pairs: Vec<Vec<String>> = Vec::new();
                for n in 0..16 {
                    let mut s = holding(&format!("unleashed-pick-{n}"), false);
                    s.play(UNLEASHED, json!({ "zone": 1 }));
                    let labels = rolled(&s);
                    // Two different entries (R60), named to both players.
                    assert_eq!(labels.len(), 2, "{n}");
                    assert_ne!(labels[0], labels[1], "{n}");
                    pairs.push(labels.clone());
                    // …and both resolved: the named summons stand, the named adds arrived.
                    if labels.iter().any(|label| label.contains("Datacenter")) {
                        assert_eq!(
                            backrow_defs(&s, P1)
                                .iter()
                                .filter(|def| *def == DATACENTERS)
                                .count(),
                            2,
                            "{n}"
                        );
                    }
                    if labels.iter().any(|label| label.contains("Glitch")) {
                        assert_eq!(count_def(&s, P1, "hand", GLITCH), 3, "{n}");
                    }
                }
                pairs.sort();
                pairs.dedup();
                assert!(pairs.len() > 1, "more than one pair comes up: {pairs:?}");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r1024_radiant_does_all_five_with_two_radiant_viruses_for_the_opponent() {
                let mut s = holding("unleashed-radiant", true);
                s.play(UNLEASHED, json!({ "zone": 1 }));

                // No roll on the Radiant face: all five entries resolved in order (MD-C23).
                assert!(rolled(&s).is_empty());
                // Two Datacenters, three Radiant Glitches at (0), an AI Girlfriend …
                assert_eq!(
                    backrow_defs(&s, P1)
                        .iter()
                        .filter(|def| *def == DATACENTERS)
                        .count(),
                    2
                );
                assert_eq!(count_def(&s, P1, "hand", GLITCH), 3);
                for card in s.hand(P1) {
                    if card.def_id == GLITCH {
                        assert!(card.radiant);
                        assert_eq!(card.cost_override, Some(0));
                    }
                }
                assert!(field_defs(&s, P1).contains(&GIRLFRIEND.to_string()));
                // … two Radiant Viruses for the opponent in place of the fill (MD-C24) …
                let foe = field_defs(&s, P2);
                assert_eq!(
                    foe.iter().filter(|def| *def == VIRUS).count(),
                    2
                );
                for lane in 1..=5 {
                    if let Some(unit) = s.unit(P2, lane)
                        && unit.def_id == VIRUS
                    {
                        assert!(unit.radiant);
                    }
                }
                // … and every enemy card Nerfed twice: the hand card alone drew two Degrades.
                assert!(degraded(&s) >= 2, "every enemy card Nerfed twice");
            }
        }
    }
}
