//! #30 Archivist (SPEC §8.2): "Cry: choose one: draw the highest-cost card in your library, or the
//! lowest", radiant "Cry: draw both". The radiant cell restates the whole Cry, so it replaces the
//! base clause (§8 Conventions) — the radiant face asks nothing and takes both cards.
//!
//! The mode is a DECLARED play-time choice: it travels in the play action's `modes` and never pauses
//! resolution (R81); `chosenOptions` reads it back. The radiant face declares no modes.
//!
//! An empty library draws nothing and the Cry fizzles; the unit still enters (§8 Conventions). With
//! one card it is both highest and lowest, so the radiant face draws it once (compared by instance).
//!
//! The draw is §6.3's `drawFromLibrary` (#94 Genn's Greed too): the card itself leaves as a draw — a
//! `drawn` event, R55's counter, the hand cap (R4), the cast-on-draw path (§2.4, R58).

use jackioh_engine::prelude::*;

pub const ID: &str = "core-030";

/// §8.2's two modes, as the play action spells them (R81).
const HIGHEST: &str = "highest";
const LOWEST: &str = "lowest";

/// R24: the library card with the extreme current cost, ties going to the card nearest the top.
/// Costs are R65's out-of-play numbers: X counts 0, embiggen its base price, the instance's `costMod`
/// persists (R78), and player discounts never price a library card. `effective_cost` is the one
/// calculation; `queryCost` reads a DEFINITION, blind to #7's `costMod` and #95's library discount.
/// Reads through `zoneCards` and writes nothing (CLAUDE.md rule 5, BUILD M3-T1).
fn extreme(ctx: &EffectContext<'_>, want: &str) -> Option<CardInstance> {
    let mut best: Option<CardInstance> = None;
    let mut best_cost = 0;
    for card in zone_cards(ctx.state, ctx.controller, OffFieldZone::Library) {
        let cost = effective_cost(ctx.state, &card, Default::default());
        let better = if want == HIGHEST { cost > best_cost } else { cost < best_cost };
        // Strict, and top down, so a tie keeps the card already held — the one nearer the top (R24).
        if best.is_none() || better {
            best = Some(card.clone());
            best_cost = cost;
        }
    }
    best
}

/// §6.3 Draw of one named library card (R24), or nothing for an empty library.
fn draw_named(card: Option<CardInstance>) -> Vec<Effect> {
    match card {
        None => vec![],
        Some(card) => vec![draw_from_library(json_as(json!({ "instanceId": card.id })))],
    }
}

pub fn script() -> CardScripts {
    let base = Script {
        modes: vec![ModeDecl {
            kind: PromptKind::Mode,
            options: vec![HIGHEST.to_string(), LOWEST.to_string()],
        }],
        cry: Some(hook(|ctx| {
            // R81: the mode arrived with the play, so there is nothing to wait for. R90 validates the
            // declaration, so anything else means no mode was carried at all — fizzle rather than guess.
            let mode = chosen_options(ctx).into_iter().next();
            match mode.as_deref() {
                Some(want) if want == HIGHEST || want == LOWEST => draw_named(extreme(ctx, want)),
                _ => vec![],
            }
        })),
        ..Script::default()
    };

    let radiant = Script {
        cry: Some(hook(|_ctx| {
            // A one-card library is its own highest and lowest, so "draw both" draws it once.
            // The two are read once, as the Cry begins: a highest card that is cast on draw and asks has left
            // the library by the answer, and a rebuilt pair would name other cards (R113, `forEachCard`).
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(|at: &mut EffectContext<'_>| {
                    let high = extreme(at, HIGHEST);
                    let low = extreme(at, LOWEST);
                    let both = match (&high, &low) {
                        (Some(high), Some(low)) if high.id == low.id => vec![Some(high.clone())],
                        _ => vec![high.clone(), low.clone()],
                    };
                    both.into_iter().flatten().map(|card| card.id).collect()
                }),
                each: Arc::new(|instance_id: &str| draw_from_library(json_as(json!({ "instanceId": instance_id })))),
            })]
        })),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// #30 Archivist (SPEC §8.2, BUILD M4-T4 row 30): "Mode chosen with the play (R81); highest/lowest
// by current cost, ties nearest top, X counts 0 (R24); radiant draws both".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    fn archivist(library: &[&str], radiant: bool) -> Scenario {
        scenario(json!({ "p1": { "hand": [{ "def": "core-030", "radiant": radiant }], "library": library, "mana": 2 } }))
    }

    fn hand_defs(s: &Scenario) -> Vec<String> {
        s.state().players.p1.hand.iter().map(|card| card.def_id.clone()).collect()
    }

    fn sorted(mut ids: Vec<String>) -> Vec<String> {
        ids.sort();
        ids
    }

    fn strings(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    mod n30_archivist_base {
        use super::*;

        #[test]
        fn r81_draws_the_highest_cost_card_in_your_library_with_the_mode_carried_by_the_play() {
            crate::register_all();
            let mut s = archivist(&["core-005", "core-025", "core-010"], false);

            s.play("core-030", json!({ "modes": ["highest"] }));

            // R81: the mode travelled in the play action, so resolution never paused.
            assert!(s.state().pending.is_none());
            assert_eq!(hand_defs(&s), strings(&["core-025"])); // #25 at 4 is the dearest
            assert_eq!(s.unit(P1, 1).map(|card| card.def_id.clone()).as_deref(), Some("core-030"));
        }

        #[test]
        fn r81_draws_the_lowest_cost_card_when_the_play_carries_that_mode_instead() {
            crate::register_all();
            let mut s = archivist(&["core-005", "core-025", "core-010"], false);

            s.play("core-030", json!({ "modes": ["lowest"] }));

            assert_eq!(hand_defs(&s), strings(&["core-010"])); // #10 Rapid Replenish at 0
        }

        #[test]
        fn r24_ties_go_to_the_card_nearest_the_top() {
            crate::register_all();
            // #13 and #19 both cost 3, and #13 is nearer the top, so #13 wins the highest.
            let mut highest = archivist(&["core-013", "core-019", "core-005"], false);
            highest.play("core-030", json!({ "modes": ["highest"] }));
            assert_eq!(hand_defs(&highest), strings(&["core-013"]));

            // The same tie on the other side: #5 and #51 both cost 1, and #5 is nearer the top.
            let mut lowest = archivist(&["core-025", "core-005", "core-051"], false);
            lowest.play("core-030", json!({ "modes": ["lowest"] }));
            assert_eq!(hand_defs(&lowest), strings(&["core-005"]));
        }

        #[test]
        fn r24_r65_an_x_cost_card_in_a_library_counts_0_so_it_is_the_lowest_and_never_the_highest() {
            crate::register_all();
            let mut lowest = archivist(&["core-005", "core-024", "core-025"], false);
            lowest.play("core-030", json!({ "modes": ["lowest"] }));
            assert_eq!(hand_defs(&lowest), strings(&["core-024"])); // #24 Efficiency Dividend, X → 0

            let mut highest = archivist(&["core-024", "core-005"], false);
            highest.play("core-030", json!({ "modes": ["highest"] }));
            assert_eq!(hand_defs(&highest), strings(&["core-005"])); // 1 beats X's 0
        }

        #[test]
        fn r65_an_embiggen_card_counts_its_base_price_out_of_play_not_its_embiggen_price() {
            crate::register_all();
            // #46 Suppressive Aura is "2 embiggen 4". At its base price of 2 it loses to #13's 3; if the
            // embiggen price counted it would be a 4 and would win.
            let mut s = archivist(&["core-046", "core-013"], false);

            s.play("core-030", json!({ "modes": ["highest"] }));

            assert_eq!(hand_defs(&s), strings(&["core-013"]));
        }

        #[test]
        fn s8_conventions_an_empty_library_fizzles_the_cry_and_the_unit_still_enters() {
            crate::register_all();
            let mut s = archivist(&[], false);

            s.play("core-030", json!({ "modes": ["highest"] }));

            assert!(hand_defs(&s).is_empty());
            assert_eq!(s.unit(P1, 1).map(|card| card.def_id.clone()).as_deref(), Some("core-030"));
            assert_eq!(s.state().players.p1.hero.health, 30); // no fatigue: nothing was drawn
        }

        // §6.3 Draw: "take the top card of your library" is the one Draw, and Archivist's names the card,
        // so the card leaves the library as a draw (`drawFromLibrary`) rather than a copy landing in hand.
        #[test]
        fn r24_draws_the_card_out_of_the_library_so_the_library_no_longer_holds_it_and_it_arrives_as_a_draw_s6_3_draw_s2_4_r55() {
            crate::register_all();
            let mut s = archivist(&["core-005", "core-025", "core-010"], false);
            let dearest = s.pile(P1, "library").get(1).map(|card| card.id.clone());
            let drawn_before = s.state().counters.drawn;

            s.play("core-030", json!({ "modes": ["highest"] }));

            assert_eq!(
                s.state().players.p1.library.iter().map(|card| card.def_id.clone()).collect::<Vec<_>>(),
                strings(&["core-005", "core-010"])
            );
            // The library card itself, not a fresh copy of its definition, and counted as a draw.
            let held: Vec<Option<String>> = s.hand(P1).iter().map(|card| Some(card.id.clone())).collect();
            assert!(held.contains(&dearest));
            assert_eq!(
                s.last_events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::Drawn { .. }))
                    .count(),
                1
            );
            assert_eq!(s.state().counters.drawn, drawn_before + 1);
        }
    }

    mod n30_archivist_radiant {
        use super::*;

        #[test]
        fn draws_both_the_highest_cost_and_the_lowest_cost_card_and_asks_nothing_s8_conventions() {
            crate::register_all();
            let mut s = archivist(&["core-005", "core-025", "core-010"], true);

            s.play("core-030", json!({ "modes": [] }));

            assert!(s.state().pending.is_none());
            assert_eq!(sorted(hand_defs(&s)), strings(&["core-010", "core-025"])); // 0 and 4
            // The radiant face is an 8/10 (§8.2).
            let unit_id = s.unit(P1, 1).map(|card| card.id.clone()).unwrap_or_default();
            s.expect_stats(&unit_id, json!({ "attack": 8, "maxHealth": 10 }));
        }

        #[test]
        fn draws_one_card_when_the_library_holds_only_one_which_is_both_the_highest_and_the_lowest() {
            crate::register_all();
            let mut s = archivist(&["core-005"], true);

            s.play("core-030", json!({ "modes": [] }));

            assert_eq!(hand_defs(&s), strings(&["core-005"]));
        }

        #[test]
        fn r24_applies_the_same_tie_rule_to_both_ends() {
            crate::register_all();
            // #13 and #19 both cost 3 (the highest); #5 and #51 both cost 1 (the lowest). Top-down wins.
            let mut s = archivist(&["core-013", "core-019", "core-005", "core-051"], true);

            s.play("core-030", json!({ "modes": [] }));

            assert_eq!(sorted(hand_defs(&s)), strings(&["core-005", "core-013"]));
        }

        #[test]
        fn s8_conventions_an_empty_library_fizzles_both_draws_and_the_unit_still_enters() {
            crate::register_all();
            let mut s = archivist(&[], true);

            s.play("core-030", json!({ "modes": [] }));

            assert!(hand_defs(&s).is_empty());
            assert_eq!(s.unit(P1, 1).map(|card| card.def_id.clone()).as_deref(), Some("core-030"));
        }
    }
}
