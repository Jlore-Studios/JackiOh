//! #31 KY's Math Equation (SPEC §8.2 row 31), patch v0.2.0 (R429): "Deal Fib(times played + 1) damage
//! to a target. End of turn: Return this to your hand. It costs (1) more, to a maximum of (4).",
//! radiant "Deal Fib(times played + 3) damage to a target. …" (R275 raised the offset).
//!
//! Four rulings drive the whole card:
//!   R429 the Fib index is the times this card has been played, THIS play included, + 1 (radiant + 3).
//!        The engine counts the plays on the instance at §10.5 step 4 (`StaticFlags.countsPlays`,
//!        `timesPlayedOf`), casts included (R70) and countered plays never, and the count rides the
//!        card through every zone like `radiant` (R78). So the 1st play deals Fib(2) = 1, the 2nd
//!        Fib(3) = 2, the 3rd 3, the 4th 5, the 5th 8 (radiant 3, 5, 8, 13, 21).
//!   R67  its cost plays no part in the damage any more: `costMod`, player discounts and the cost
//!        paid change its price, never what it deals.
//!   R25  Fib = 0,1,1,2,3,5,8,13,21,34,55,89 and the index clamps at 11, which is what `fib` in
//!        engine/src/config.ts already does — nothing here re-derives Fibonacci.
//!   R280 the damage it would deal now is the card's `preview`, labelled with the running face's
//!        formula as its catalog text prints it, off the same `damageFor` its Cry deals. In hand that
//!        is the play it would be (one more than it has had); it reads the card's own count and
//!        nothing else, so it shows wherever the card may be read (§10.8).
//!
//! The return is an `endOfTurn` hook on a Spell that is sitting in its owner's graveyard: §5.1's
//! "add this back to your hand" spells are found there by `triggerHoldersWithHook` (triggers.ts,
//! R68), which unlike `turn.ts`'s `triggerOrder` reaches the hand and the graveyard. The hook is
//! one-shot by reading the turn log: the return happens on the turn the card was played, not at
//! every end of turn for the rest of the game. R429: the return raises the card's own cost (R65:
//! `costOverride` or printed, plus `costMod`) by 1, but never above (4) — at (4) it adds nothing.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-031";

/// §8 row 31: what each face adds to the times played before taking the Fibonacci number.
#[derive(Clone, Copy)]
struct FibOffset {
    base: i32,
    radiant: i32,
}

const FIB_OFFSET: FibOffset = FibOffset { base: 1, radiant: 3 };

/// R429: "It costs (1) more, to a maximum of (4)."
const RETURN_COST_STEP: i32 = 1;
const RETURN_COST_CAP: i32 = 4;

/// R280: the formula as each face prints it — "Fib(times played + 1)" — which the preview labels its
/// number with. Read off the catalog text, so the label is always a substring of the face it names.
#[derive(Clone)]
struct Formula {
    base: String,
    radiant: String,
}

/// TS `const FORMULA = { base: formulaIn(def.base.text), radiant: formulaIn(def.radiant.text) }`,
/// read once per build of the script.
fn formula() -> Formula {
    let def = crate::card_def(ID);
    Formula {
        base: formula_in(&def.base.text),
        radiant: formula_in(&def.radiant.text),
    }
}

/// TS `/Fib\([^)]*\)/.exec(text)[0]`, by hand (no regex crate in a pure crate, SURFACE §3): the first
/// "Fib(" that a ")" closes, up to and including that ")".
fn formula_in(text: &str) -> String {
    let mut from = 0;
    while let Some(at) = text[from..].find("Fib(") {
        let start = from + at;
        let open = start + "Fib(".len();
        if let Some(close) = text[open..].find(')') {
            return text[start..open + close + 1].to_string();
        }
        from = open;
    }
    panic!("#31's text names no Fib(…) formula: {text}");
}

/// The plays the Fib index counts: the ones the card has had, this one included. While the card is
/// resolving its play has been counted already (§10.5 step 4); anywhere else — in hand, for the
/// preview — "if it resolved now" is one more play than it has had.
fn plays_if_resolved_now(self_: &CardInstance) -> i32 {
    let counted = times_played_of(self_);
    if self_.zone.z() == ZoneName::Resolving {
        counted
    } else {
        counted + 1
    }
}

/// The damage the card deals if it resolves now — the Cry's amount and the preview's value, one
/// function so the two cannot disagree (R280). R25: `fib` clamps the index at 11, so it tops out at 89.
fn damage_for(self_: &CardInstance, radiant: bool) -> i32 {
    fib(plays_if_resolved_now(self_) + if radiant { FIB_OFFSET.radiant } else { FIB_OFFSET.base })
}

fn blast(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    // TS read the live `ctx.self`: the card as it stands now, its play already counted.
    let amount = match ctx.live_self() {
        None => fib(0),
        Some(self_) => damage_for(self_, ctx.radiant),
    };
    vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": amount })))]
}

/// R65 outside a hand: the card's own cost, `costOverride` or printed, plus `costMod`, floored at 0.
fn own_cost(state: &GameState, self_: &CardInstance) -> i32 {
    (self_.cost_override.unwrap_or_else(|| printed_cost(state, self_)) + self_.cost_mod).max(0)
}

/// "End of turn: Return this to your hand. It costs (1) more, to a maximum of (4)." Only on the turn
/// it was played: the spell stays in the graveyard when its hand is full (`bounce` burns it back,
/// §2.4), and a card that is still there on a later turn is no longer returning, so the turn log —
/// which `startTurn` clears — is the gate.
fn return_to_hand(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(self_) = ctx.live_self().cloned() else {
        return vec![];
    };
    if self_.zone.z() != ZoneName::Graveyard {
        return vec![];
    }
    if !was_played_this_turn(ctx.state, self_.owner, &self_) {
        return vec![];
    }
    // R78: the +1 rides on the instance in the hand, until a graveyard takes it off again (R766). It
    // is the price of the return, so it lands only on a card that reached the hand: a full hand burns
    // the card back to the graveyard (§2.4, R4), which is no return at all. R429: never above (4), so at (4) or more it adds nothing.
    let raise = RETURN_COST_STEP.min(RETURN_COST_CAP - own_cost(ctx.state, &self_)).max(0);
    let mut effects = vec![bounce(json_as(json!({ "target": { "of": "self" } })))];
    if raise > 0 {
        effects.push(set_cost_mod(json_as(json!({ "amount": raise, "inHandOnly": true }))));
    }
    effects
}

/// R81: the target travels in the `play` action; "target" is any unit or hero (§8 Conventions).
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "hero"] }))]
}

/// R280: "Fib(times played + 1) {n}" — the damage it deals if played now, in hand, and as the card
/// stands wherever else it may be read. It reads its own count of plays, which is the card's (§9.1):
/// `viewFor` shows it only where the card itself is shown, never on the opponent's hand.
fn preview(formula: Formula) -> PreviewHook {
    condition_hook(move |ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        vec![PreviewValue {
            label: if ctx.radiant { formula.radiant.clone() } else { formula.base.clone() },
            value: damage_for(ctx.self_, ctx.radiant),
            display: None,
            ids: None,
        }]
    })
}

/// R429: §10.5 step 4 counts this card's plays on its instance.
fn static_flags() -> Option<StaticFlags> {
    Some(StaticFlags {
        counts_plays: Some(true),
        ..StaticFlags::default()
    })
}

pub fn script() -> CardScripts {
    let formula = formula();

    let base = Script {
        static_flags: static_flags(),
        targets: targets(),
        cry: Some(hook(blast)),
        end_of_turn: Some(hook(return_to_hand)),
        preview: Some(preview(formula.clone())),
        ..Script::default()
    };

    // The radiant face changes only the Fibonacci offset (R275), so the return clause is kept.
    let radiant = Script {
        static_flags: static_flags(),
        targets: targets(),
        cry: Some(hook(blast)),
        end_of_turn: Some(hook(return_to_hand)),
        preview: Some(preview(formula)),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// #31 KY's Math Equation — SPEC §8.2 row 31, patch v0.2.0 (R429): "Deal Fib(times played + 1) damage
// to a target. End of turn: Return this to your hand. It costs (1) more, to a maximum of (4)."; radiant
// Fib(times played + 3) (R275).
//
// BUILD M4-T4 must-pass row 31, as R429 rewrites it: "1st play → 1 damage, 2nd → 2, 3rd → 3, 4th → 5,
// 5th → 8; clamps at 89 (R25); its cost — costMod, discounts, the cost paid — changes its price and
// never its damage (R67); the return costs (1) more, never above (4)". Times played counts this card's
// plays, the current one included, on the instance (`timesPlayed`), in every zone. The damage it
// would deal now, its R280 `preview`, is proved in test/preview.test.ts.
//
// Every side gets a unit on the board so §2.5's auto-end-turn does not run the turn on by itself
// (see the harness header): a unit with an unspent exertion is always a meaningful action.
#[cfg(test)]
mod tests {
    use super::script;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// `{ targets: [{ pick: "hero", player: "p2" }] }`: every play here aims at the enemy hero.
    fn at_enemy_hero() -> Value {
        json!({ "targets": [{ "pick": "hero", "player": "p2" }] })
    }

    #[derive(Clone, Copy, Default)]
    struct BoardOptions {
        times_played: Option<i32>,
        radiant: bool,
        cost_mod: Option<i32>,
        health: Option<i32>,
    }

    /// The plays a card has had before the fixture's own (R429): what the harness cannot seed.
    fn set_times_played(s: &mut Scenario, card: &str, times: i32) {
        let id = s.card(card).id.clone();
        find_instance_mut(s.state_mut(), &id).expect("the card is in the state").times_played = Some(times);
    }

    /// A board where p1 can play the equation and neither side auto-ends its turn.
    fn board(options: BoardOptions) -> Scenario {
        let mut p2 = json!({ "field": ["15"], "library": ["15", "15", "15", "15"] });
        if let Some(health) = options.health {
            p2["health"] = json!(health);
        }
        let mut s = scenario(json!({
            "seed": "ky-math",
            "p1": {
                "hand": [{ "def": "31", "radiant": options.radiant, "costMod": options.cost_mod.unwrap_or(0) }],
                "field": ["15"],
                "library": ["15", "15", "15", "15"],
                "mana": 10,
            },
            "p2": p2,
        }));
        // The plays it has had before this fixture's own (R429): what the harness cannot seed.
        if let Some(times) = options.times_played {
            set_times_played(&mut s, "31", times);
        }
        s
    }

    /// Play, end p1's turn (the return), let p2's turn pass, and come back to p1's main phase.
    fn play_and_come_back(mut s: Scenario) -> Scenario {
        s.play("31", at_enemy_hero());
        s.end_turn();
        s.end_turn();
        assert_eq!(s.state().active, P1);
        s
    }

    /// TS `/^p[12]\b/.test(label)`: the label starts with a seat id as a whole word.
    fn starts_with_seat(label: &str) -> bool {
        let Some(rest) = label.strip_prefix("p1").or_else(|| label.strip_prefix("p2")) else {
            return false;
        };
        rest.chars()
            .next()
            .is_none_or(|next| !(next.is_ascii_alphanumeric() || next == '_'))
    }

    mod n31_ky_s_math_equation_base {
        use super::*;

        #[test]
        fn r429_its_1st_play_deals_fib_1_1_fib_2_1_damage_to_the_target() {
            crate::register_all();
            let mut s = board(BoardOptions::default());
            s.play("31", at_enemy_hero());
            s.expect_health(P2, 29);
            s.expect_events(json!(["cardPlayed", "damage"]));
        }

        #[test]
        fn r429_s10_5_step_4_counts_the_play_on_the_instance_and_the_count_rides_it_through_every_zone() {
            crate::register_all();
            let mut s = board(BoardOptions::default());
            let equation = s.card("31").clone();
            assert_eq!(times_played_of(&equation), 0);

            s.play(&equation, at_enemy_hero());
            s.expect_in_zone(&equation, "graveyard");
            assert_eq!(times_played_of(s.card(&equation)), 1);

            s.end_turn(); // the return
            s.expect_in_zone(&equation, "hand");
            assert_eq!(times_played_of(s.card(&equation)), 1);
        }

        #[test]
        fn r429_its_2nd_play_once_it_has_come_back_deals_fib_3_2() {
            crate::register_all();
            let mut s = play_and_come_back(board(BoardOptions::default()));
            s.expect_health(P2, 29);

            s.play("31", at_enemy_hero());

            s.expect_health(P2, 27);
            assert_eq!(times_played_of(s.card("31")), 2);
        }

        #[test]
        fn r429_the_3rd_play_deals_3_the_4th_5_and_the_5th_8() {
            crate::register_all();
            for (before, dealt) in [(2, 3), (3, 5), (4, 8)] {
                let mut s = board(BoardOptions {
                    times_played: Some(before),
                    ..BoardOptions::default()
                });
                s.play("31", at_enemy_hero());
                s.expect_health(P2, 30 - dealt);
            }
        }

        #[test]
        fn r25_the_fib_index_still_clamps_at_11_so_the_damage_clamps_at_89() {
            crate::register_all();
            let mut s = board(BoardOptions {
                times_played: Some(20),
                health: Some(200),
                ..BoardOptions::default()
            });
            s.play("31", at_enemy_hero());
            s.expect_health(P2, 111);
        }

        #[test]
        fn r67_r429_its_cost_plays_no_part_a_4_equation_on_its_1st_play_deals_1_and_a_discount_changes_only_the_price() {
            crate::register_all();
            let mut pricey = board(BoardOptions {
                cost_mod: Some(3),
                ..BoardOptions::default()
            });
            assert_eq!(effective_cost(pricey.state(), pricey.card("31"), Default::default()), 4);
            pricey.play("31", at_enemy_hero());
            pricey.expect_mana(P1, 6);
            pricey.expect_health(P2, 29);

            let mut cheap = board(BoardOptions::default());
            let discount: PlayerModifier = json_as(json!({
                "id": "test-spell-discount",
                "kind": "costDiscount",
                "amount": 1,
                "onlyType": "Spell",
                "expiry": { "until": "never" },
            }));
            cheap.state_mut().players.p1.mods.push(discount);
            cheap.play("31", at_enemy_hero());
            cheap.expect_mana(P1, 10);
            cheap.expect_health(P2, 29);
        }

        #[test]
        fn r429_r78_end_of_turn_it_returns_to_hand_and_costs_1_more_for_good() {
            crate::register_all();
            let mut s = board(BoardOptions::default());
            let equation = s.card("31").clone();
            s.play("31", at_enemy_hero());
            s.expect_in_zone(&equation, "graveyard");

            s.end_turn();

            s.expect_in_zone(&equation, "hand");
            assert_eq!(s.card(&equation).cost_mod, 1);
            assert_eq!(effective_cost(s.state(), s.card(&equation), Default::default()), 2);
        }

        #[test]
        fn r429_r766_the_return_never_lifts_its_cost_above_4_from_3_it_reaches_4_and_at_4_it_adds_nothing() {
            crate::register_all();
            // R766: a price the equation had in hand stays behind in the graveyard it returns from, so a
            // (3) or a (4) equation comes back at its printed (1) plus the return's (1).
            for raised in [2, 3] {
                let mut s = board(BoardOptions {
                    cost_mod: Some(raised),
                    ..BoardOptions::default()
                });
                s.play("31", at_enemy_hero());
                s.end_turn();
                s.expect_in_zone("31", "hand");
                assert_eq!(s.card("31").cost_mod, 1);
                assert_eq!(effective_cost(s.state(), s.card("31"), Default::default()), 2);
            }

            // R429's cap reads the cost the equation has where it lies, as for one whose own cost there
            // is (3) or (4) (a fusion's printed cost, R77): from (3) it reaches (4), and at (4) it adds
            // nothing.
            let mut three = board(BoardOptions::default());
            three.play("31", at_enemy_hero());
            three.card_mut("31").cost_mod = 2;
            three.end_turn();
            three.expect_in_zone("31", "hand");
            assert_eq!(effective_cost(three.state(), three.card("31"), Default::default()), 4);

            let mut four = board(BoardOptions::default());
            four.play("31", at_enemy_hero());
            four.card_mut("31").cost_mod = 3;
            four.end_turn();
            four.expect_in_zone("31", "hand");
            assert_eq!(four.card("31").cost_mod, 3);
            assert_eq!(effective_cost(four.state(), four.card("31"), Default::default()), 4);
        }

        #[test]
        fn r429_r65_r766_a_0_equation_n95_s_cost_override_stays_in_the_graveyard_so_it_comes_back_costing_2() {
            crate::register_all();
            let mut s = board(BoardOptions::default());
            let id = s.card("31").id.clone();
            find_instance_mut(s.state_mut(), &id).expect("the equation in hand").cost_override = Some(0);
            s.play("31", at_enemy_hero());
            s.expect_mana(P1, 10);
            // R766: it reaches the graveyard as the printed (1) card, so the return prices it (1) + (1).
            assert_eq!(s.card("31").cost_override, None);
            s.end_turn();
            assert_eq!(effective_cost(s.state(), s.card("31"), Default::default()), 2);
        }

        #[test]
        fn r429_r30_an_echo_repeat_is_the_same_play_twinspell_s_repeat_deals_the_same_fib_2_again_and_the_count_is_1() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "ky-math-echo",
                "p1": { "hand": ["31"], "field": ["15"], "backrow": ["79"], "library": ["15", "15"], "mana": 10 },
                "p2": { "field": ["15"], "library": ["15", "15"] },
            }));
            s.play("31", at_enemy_hero());
            // The repeat asks its target afresh (§10.6).
            assert_eq!(s.state().pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Target));
            let labels: Vec<String> = s
                .state()
                .pending
                .as_ref()
                .map(|pending| pending.options.iter().map(|option| option.label.clone()).collect())
                .unwrap_or_default();
            assert!(labels.contains(&"Enemy hero".to_string()));
            assert!(!labels.iter().any(|label| starts_with_seat(label)));
            let keys: Vec<String> = s
                .state()
                .pending
                .as_ref()
                .map(|pending| pending.options.iter().map(|option| option.key.clone()).collect())
                .unwrap_or_default();
            assert!(keys.contains(&"hero:p2".to_string()));
            s.answer(json!("hero:p2"));

            s.expect_health(P2, 28);
            assert_eq!(times_played_of(s.card("31")), 1);
        }

        #[test]
        fn the_return_belongs_to_the_turn_it_was_played_on_not_to_every_graveyard_copy() {
            crate::register_all();
            // A copy that was already in the graveyard when the turn began was not played this turn, so
            // ending the turn leaves it there — otherwise it would climb a cost every turn for the game.
            let mut s = scenario(json!({
                "seed": "ky-math-stale",
                "p1": { "hand": ["15"], "field": ["15"], "graveyard": ["31"], "library": ["15", "15"] },
                "p2": { "field": ["15"], "library": ["15", "15"] },
            }));
            let stale = s.card("31").clone();

            s.end_turn();

            s.expect_in_zone(&stale, "graveyard");
            assert_eq!(s.card(&stale).cost_mod, 0);
        }

        #[test]
        fn r429_both_faces_ask_the_engine_to_count_their_plays() {
            let scripts = script();
            assert_eq!(scripts.base.static_flags.as_ref().and_then(|flags| flags.counts_plays), Some(true));
            assert_eq!(scripts.radiant.static_flags.as_ref().and_then(|flags| flags.counts_plays), Some(true));
        }
    }

    mod n31_ky_s_math_equation_radiant {
        use super::*;

        #[test]
        fn r429_radiant_the_1st_play_deals_fib_1_3_fib_4_3() {
            crate::register_all();
            let mut s = board(BoardOptions {
                radiant: true,
                ..BoardOptions::default()
            });
            s.play("31", at_enemy_hero());
            s.expect_health(P2, 27);
        }

        #[test]
        fn r429_radiant_the_2nd_play_deals_fib_5_5_the_3rd_fib_6_8() {
            crate::register_all();
            let mut second = board(BoardOptions {
                radiant: true,
                times_played: Some(1),
                ..BoardOptions::default()
            });
            second.play("31", at_enemy_hero());
            second.expect_health(P2, 25);

            let mut third = board(BoardOptions {
                radiant: true,
                times_played: Some(2),
                ..BoardOptions::default()
            });
            third.play("31", at_enemy_hero());
            third.expect_health(P2, 22);
        }

        #[test]
        fn r25_radiant_clamps_at_89_too() {
            crate::register_all();
            let mut s = board(BoardOptions {
                radiant: true,
                times_played: Some(20),
                health: Some(200),
                ..BoardOptions::default()
            });
            s.play("31", at_enemy_hero());
            s.expect_health(P2, 111);
        }

        #[test]
        fn s8_conventions_the_radiant_face_changes_only_the_offset_so_the_return_clause_and_its_cap_are_kept() {
            crate::register_all();
            let mut s = play_and_come_back(board(BoardOptions {
                radiant: true,
                ..BoardOptions::default()
            }));
            let equation = s.card("31").clone();
            assert!(s.card(&equation).radiant);
            assert_eq!(effective_cost(s.state(), s.card(&equation), Default::default()), 2);
            s.expect_health(P2, 27);

            // The 2nd play: Fib(2 + 3) = 5.
            s.play(&equation, at_enemy_hero());
            s.expect_health(P2, 22);
        }
    }
}
