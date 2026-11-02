//! SPEC §7 Stinkyscape 112.1 Paycheck (§8.10 row 112.1; §2.4, §6.1 Lucky, §6.3 Mana; R81, R1438,
//! R1840005). (0) Spell, a token (printed Legendary) of the Stinkyscape set, S #112 Job
//! Application's Stage 3 reward.
//!   Base:    "Choose one: Gain {mana} mana, draw {draw|card|cards} and gain {credits} Credits; or go all
//!            in on red, with an 18/39 chance to gain twice as much." (mana 2, draw 1, credits 20)
//!   Radiant: "Lucky 1", and the same text.
//!
//! Two declared modes, chosen with the play (R81). Payday: `gain_mana`'s mana this turn (The Coin's,
//! §6.3 Mana), a draw (§2.4) and `gain_market_credits` (R1840005). All in on red: one luck-based roll
//! from the match rng, a win on 18 of 39 pockets (§6.1 Lucky), giving twice each of Payday's three, a
//! loss nothing. Radiant: its Lucky 1 (`lucky_on`) rolls again and keeps the win (R1438), and the
//! controller's Luck does the same on either face (R987).

use jackioh_engine::effects::{
    DrawArgs, GainManaArgs, GainMarketCreditsArgs, chosen_options, draw, gain_mana, gain_market_credits,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-112-1";

/// The two modes (R81).
const PAYDAY: &str = "Payday";
const ALL_IN: &str = "All in on red";
/// "An 18/39 chance": the red pockets, and every pocket the ball may land in.
const RED_POCKETS: i32 = 18;
const POCKETS: i32 = 39;
/// "Twice as much": a win pays Payday this many times over.
const ALL_IN_WIN: i32 = 2;

/// Payday `times` over: mana this turn, draws and Credits, each its declared number.
fn pay(ctx: &EffectContext<'_>, times: i32) -> Vec<Effect> {
    vec![
        gain_mana(GainManaArgs {
            amount: param(ctx, "mana") * times,
            player: None,
        }),
        draw(DrawArgs {
            count: param(ctx, "draw") * times,
            player: None,
        }),
        gain_market_credits(GainMarketCreditsArgs {
            amount: param(ctx, "credits") * times,
            player: None,
        }),
    ]
}

/// "An 18/39 chance", a luck-based roll whose better side is the win, so the card's Lucky (printed and
/// given, R1438) and its controller's Luck (R987) each roll it again.
fn won_on_red(ctx: &mut EffectContext<'_>) -> bool {
    let lucky =
        ctx.live_self().map_or(0, |me| lucky_on(&*ctx.state, me)) + luck_of(&*ctx.state, ctx.controller);
    ctx.rng
        .lucky(lucky, |rng| rng.int(POCKETS) < RED_POCKETS, |a, b| a || b)
}

pub fn script() -> CardScripts {
    let base = Script {
        modes: vec![ModeDecl {
            kind: PromptKind::Mode,
            options: vec![PAYDAY.to_string(), ALL_IN.to_string()],
        }],
        cry: Some(hook(|ctx| {
            match chosen_options(ctx).first().map(String::as_str) {
                Some(PAYDAY) => pay(ctx, 1),
                Some(ALL_IN) if won_on_red(ctx) => pay(ctx, ALL_IN_WIN),
                _ => vec![],
            }
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's Lucky 1 is its printed keyword, which `lucky_on` reads.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S #112.1 Paycheck — SPEC §8.10 row 112.1, BUILD M12 row S 112.1.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const PAYCHECK: &str = "stinkyscape-112-1";
    const FILLER: &str = "core-005";
    const SEEDS: u32 = 200;

    fn setup(seed: &str, radiant: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": PAYCHECK, "radiant": radiant }], "library": [FILLER, FILLER, FILLER, FILLER] },
            "p2": { "hand": [FILLER] },
        }))
    }

    fn paid(s: &mut Scenario, mode: &str) -> (i32, usize, i32) {
        let mana = s.state().players.p1.mana.current;
        let library = s.pile(P1, "library").len();
        s.play(PAYCHECK, json!({ "modes": [mode] }));
        (
            s.state().players.p1.mana.current - mana,
            library - s.pile(P1, "library").len(),
            market_credits_of(s.state(), P1),
        )
    }

    fn wins(radiant: bool) -> u32 {
        let mut won = 0;
        for seed in 0..SEEDS {
            let mut s = setup(&format!("paycheck-red-{radiant}-{seed}"), radiant);
            match paid(&mut s, ALL_IN) {
                (4, 2, 40) => won += 1,
                (0, 0, 0) => {}
                other => panic!("all or nothing: {other:?}"),
            }
        }
        won
    }

    #[test]
    fn declares_payday_and_all_in() {
        crate::register_all();
        for scripts in [script().base, script().radiant] {
            assert_eq!(scripts.modes.len(), 1);
            assert_eq!(scripts.modes[0].kind, PromptKind::Mode);
            assert_eq!(
                scripts.modes[0].options,
                vec![PAYDAY.to_string(), ALL_IN.to_string()]
            );
        }
    }

    #[test]
    fn payday_gives_2_mana_this_turn_a_draw_and_20_credits() {
        let mut s = setup("paycheck-payday", false);
        assert_eq!(paid(&mut s, PAYDAY), (2, 1, 20));
        assert!(
            s.last_events().iter().any(|event| matches!(
                event,
                GameEvent::MarketCreditsChanged {
                    player: P1,
                    value: 20
                }
            )),
            "the Credits are reported"
        );
    }

    #[test]
    fn all_in_on_red_wins_about_18_in_39_and_a_loss_gives_nothing() {
        let won = wins(false);
        assert!((70..=116).contains(&won), "{won} wins in {SEEDS}");
    }

    #[test]
    fn radiant_lucky_1_wins_when_either_roll_wins() {
        let won = wins(true);
        assert!((120..=164).contains(&won), "{won} wins in {SEEDS}");
        assert!(won > wins(false), "the second roll only ever helps");
    }

    #[test]
    fn mana_draw_and_credits_read_through_param() {
        let mut s = setup("paycheck-param", false);
        let id = s.hand(P1)[0].id.clone();
        for key in ["mana", "draw", "credits"] {
            step_param(s.card_mut(id.as_str()), key, 1);
        }
        assert_eq!(paid(&mut s, PAYDAY), (3, 2, 25));
    }
}
