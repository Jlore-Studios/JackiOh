//! S #4.4 Last Word (SPEC §8.10 row S 4.4, SPEC §7; §3.2, §4.4 step 4a, §6.1 Charge, §6.2
//! Replacement, §6.3 End the turn, Extra turn, Mana, R456, R845, R846, R850, R1181000, R1204,
//! R1704001, R1857002, R1870004): (4) Trap, Terminus, Token (printed Mythic), one of Olivian Expanse's
//! picks.
//!   Base:    "Reveals when your hero would take lethal damage: Prevent it and end the turn. The next
//!            turn is yours: your Units have Charge during it, and you lose the game at its end."
//!   Radiant: "… The next turn is yours: you have {mana} mana and your Units have Charge during it, …"
//!            (100)
//!
//! Replacement on hero lethal damage (C #52, R1204, R1704001): cancels hit, ends turn (R456); grants extra turn if on own turn (R845, R846).
//! Units have Charge during that turn (give_units_keyword_next_turn, R1859007); delayed step at its end wins game for opponent (R1181000, R850).
//! Radiant: sets mana to declared mana at start of turn (R1870004, §6.3 Mana).

use jackioh_engine::effects::{end_turn, gain_mana, give_units_keyword_next_turn, take_extra_turn, win_game};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-004-4";

/// The replacement's follow-up, and the two delayed steps (§10.6).
const AFTER_PREVENT: &str = "afterPrevent";
const LOSE: &str = "lose";
const MANA: &str = "mana";

/// The next turn of the controller's, counted in their turns started (R1181000).
const NEXT_TURN: i32 = 1;

fn after_prevent(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let own = ctx.state.active == ctx.controller;
    let mut effects = vec![end_turn(json_as(
        json!({ "player": if own { "self" } else { "enemy" } }),
    ))];
    if own {
        effects.push(take_extra_turn(json_as(json!({}))));
    }
    effects.push(give_units_keyword_next_turn(json_as(
        json!({ "keyword": { "kind": "Charge" } }),
    )));
    effects.push(delay(json_as(json!({
        "at": { "phase": "end", "player": "self" },
        "step": LOSE,
        "hook": RESUME_HOOK,
        "turns": NEXT_TURN,
    }))));
    if ctx.radiant {
        effects.push(delay(json_as(json!({
            "at": { "phase": "start", "player": "self" },
            "step": MANA,
            "hook": RESUME_HOOK,
            "turns": NEXT_TURN,
            "data": { MANA: param(&*ctx, "mana") },
        }))));
    }
    effects
}

/// "You lose the game at its end."
fn lose(_ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    vec![win_game(json_as(json!({ "player": "enemy" })))]
}

/// "You have {mana} mana": what the refresh left them, topped up to it.
fn mana(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let target = ctx.data.get(MANA).and_then(Value::as_i64).unwrap_or(0) as i32;
    let amount = target - unspent_mana_of(ctx.state, ctx.controller);
    if amount <= 0 {
        return vec![];
    }
    vec![gain_mana(json_as(json!({ "amount": amount })))]
}

pub fn script() -> CardScripts {
    let base = Script {
        replacements: vec![ReplacementDef {
            id: "last-word".to_string(),
            on: ReplacementMoment::LethalHit,
            where_: None,
            when: None,
            instead: ReplacementInstead {
                cancel: Some(true),
                ..ReplacementInstead::default()
            },
            then: Some(AFTER_PREVENT.to_string()),
            by: None,
        }],
        resume: IndexMap::from([
            (AFTER_PREVENT, hook(after_prevent)),
            (LOSE, hook(lose)),
            (MANA, hook(mana)),
        ]),
        ..Script::default()
    };
    // The face is read off the context, so both faces run the one script.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S #4.4 Last Word — SPEC §8.10 row S 4.4, BUILD M12 row S 4.4 (R1870004).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::params::step_param;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const LUNAR: &str = "core-035"; // (1) Spell: deal 3 damage to a target.
    const FILLER: &str = "core-005"; // (1) Spell Stockpile.
    const DECK: [&str; 6] = [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER];
    const MANA: i32 = 100;

    fn last_word(radiant: bool) -> Value {
        json!({ "def": ID, "radiant": radiant, "faceUp": false })
    }

    fn board(active: &str, health: i32, radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": "last-word",
            "active": active,
            "p1": {
                "hand": [VANILLA, LUNAR, FILLER],
                "backrow": [last_word(radiant)],
                "library": DECK,
                "health": health,
            },
            "p2": { "hand": [LUNAR, FILLER], "field": [VANILLA], "library": DECK },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    fn lethal_attack(health: i32, radiant: bool) -> Scenario {
        let mut s = board("p2", health, radiant);
        s.attack(VANILLA, "hero");
        s
    }

    fn fired(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| event.event_type() == GameEventType::TrapFired)
            .count()
    }

    fn last_word_in(s: &mut Scenario, zone: &str) {
        let card = s
            .state()
            .players
            .p1
            .graveyard
            .iter()
            .chain(s.state().players.p1.backrow.iter().flatten())
            .find(|card| card.def_id == ID)
            .cloned()
            .expect("Last Word is set or in the graveyard");
        s.expect_in_zone(&card, zone);
    }

    fn may_attack_hero(s: &Scenario, attacker: &str) -> bool {
        let player = s.card(attacker).controller;
        let hero = format!("hero-{}", opponent_of(player));
        legal_actions(s.state(), player).into_iter().any(|action| {
            matches!(action, ActionBody::Attack { attacker_id, target_id } if attacker_id == attacker && target_id == hero)
        })
    }

    mod r1870004_last_word {
        use super::*;

        #[test]
        fn r1870004_lethal_attack_on_opponents_turn_prevented_and_ends_it() {
            let mut s = lethal_attack(4, false);
            assert_eq!(fired(&s), 1);
            s.expect_health(P1, 4);
            assert!(
                s.events().iter().all(|event| event.event_type() != GameEventType::Damage),
                "the hit deals nothing"
            );
            last_word_in(&mut s, "graveyard");
            assert_eq!(s.state().active, P1);
            assert_eq!(s.state().players.p1.extra_turns, None);
            assert!(s.state().result.is_none());
        }

        #[test]
        fn r1870004_lethal_effect_on_own_turn_gives_an_extra_turn() {
            let mut s = board("p1", 3, false);
            let turn = s.state().turn;
            s.play(LUNAR, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
            assert_eq!(fired(&s), 1);
            s.expect_health(P1, 3);
            assert_eq!(s.state().active, P1, "the turn ended into an extra one of p1's");
            assert_eq!(s.state().turn, turn + 1);
            s.end_turn();
            let result = s.state().result.expect("the game is over");
            assert_eq!(result.winner, Winner::P2);
            assert_eq!(result.reason, GameOverReason::WonByEffect);
        }

        #[test]
        fn r1870004_units_have_charge_that_turn() {
            let mut s = lethal_attack(4, false);
            assert_eq!(s.state().active, P1);
            let vanilla = s.hand(P1).into_iter().find(|card| card.def_id == VANILLA).expect("in hand");
            s.play(vanilla.id.as_str(), json!({}));
            assert!(unit_has(s.state(), s.card(vanilla.id.as_str()), KeywordKind::Charge));
            assert!(may_attack_hero(&s, &vanilla.id), "it may attack the hero the turn it lands");
            s.attack(vanilla.id.as_str(), "hero");
            s.expect_health(P2, HERO_HEALTH - 4);
        }

        #[test]
        fn r1870004_opponent_wins_at_its_end() {
            let mut s = lethal_attack(4, false);
            assert!(s.state().result.is_none(), "the turn plays out");
            s.end_turn();
            let result = s.state().result.expect("the game is over");
            assert_eq!(result.winner, Winner::P2);
            assert_eq!(result.reason, GameOverReason::WonByEffect);
        }

        #[test]
        fn r1870004_the_opponents_owed_extra_turn_comes_first() {
            let mut s = board("p2", 4, false);
            s.state_mut().players.p2.extra_turns = Some(1);
            s.attack(VANILLA, "hero");
            assert_eq!(fired(&s), 1);
            assert_eq!(s.state().active, P2, "p2's owed extra turn first (R846)");
            assert!(s.state().result.is_none());
            s.end_turn();
            assert_eq!(s.state().active, P1, "then p1's next turn");
            let vanilla = s.hand(P1).into_iter().find(|card| card.def_id == VANILLA).expect("in hand");
            s.play(vanilla.id.as_str(), json!({}));
            assert!(may_attack_hero(&s, &vanilla.id), "Charge on that turn");
            s.end_turn();
            assert_eq!(s.state().result.map(|result| result.winner), Some(Winner::P2));
        }

        #[test]
        fn r1870004_non_lethal_hit_leaves_it_set() {
            let mut s = lethal_attack(5, false);
            assert_eq!(fired(&s), 0);
            s.expect_health(P1, 1);
            last_word_in(&mut s, "field");
            assert_eq!(s.state().active, P2, "p2's turn goes on");
        }

        #[test]
        fn r1870004_radiant_has_param_mana_that_turn() {
            let s = lethal_attack(4, true);
            assert_eq!(s.state().players.p1.mana.current, MANA, "100 mana, past MAX_MANA");
            let s = lethal_attack(4, false);
            assert!(s.state().players.p1.mana.current < MANA);
        }

        #[test]
        fn r1870004_radiant_mana_reads_param() {
            let mut s = board("p2", 4, true);
            let set = s
                .state()
                .players
                .p1
                .backrow
                .iter()
                .flatten()
                .find(|card| card.def_id == ID)
                .cloned()
                .expect("Last Word is set");
            step_param(s.card_mut(set.id.as_str()), "mana", -1);
            s.attack(VANILLA, "hero");
            assert_eq!(s.state().players.p1.mana.current, MANA - 25);
        }
    }
}
