//! #91 Fed Fauci (SPEC §8.4, BUILD M4-T4 row 91, R63, R78).
//!
//! Base: "Rush. Whenever this takes damage, +1 Plague Counter. Start of turn: +1 mana per Plague
//! Token". Radiant: "Rush; +2 mana per token". The radiant cell lists Rush without "Plus", so Rush
//! is its COMPLETE keyword list (§8 Conventions): the catalog prints it on both faces and nothing
//! here grants it (§10.4 layer 1). The base's damage→token clause is kept, as the cell restates
//! only the mana number.
//!
//! The count is `counters.plague` (§10.1), written only by the `plague` effect (CLAUDE.md rule 5).
//! R63: a 0 hit emits no `damage` event, so the trigger counts EVENTS, one per instance. R78: leaving
//! the field resets counters. The condition is R99's `when` and also a guard in `run`, since queued
//! unit triggers never consult `when`. R280: the preview is the mana it gives at its controller's next
//! start of turn, `mana_now` off its own Plague Counters, which travel on its public view (§10.8); a
//! card in hand holds none (R78).

use jackioh_engine::prelude::*;

pub const ID: &str = "core-091";

/// §8: "+1 Plague Counter" — one per damage instance.
const TOKENS_PER_DAMAGE: i32 = 1;

// §8: base "+1 mana per Plague Counter"; radiant "+2 mana per token" — the declared number `mana`
// (R386), read off the face that is up.

/// "Whenever THIS takes damage": the event carries the target, so a hit this card DEALT — its own
/// strike-back, its Trample overflow — is not a hit it took.
fn is_hit_on_self(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    let Some(self_) = ctx.self_.as_ref() else {
        return false;
    };
    match event {
        GameEvent::Damage { target_id, .. } => *target_id == self_.id,
        _ => false,
    }
}

/// "+1 Plague Counter". `plague` defaults its target to `{ of: "self" }`, which is this card.
fn takes_damage() -> TriggerDef {
    TriggerDef::new("fed-fauci-plague", &[GameEventType::Damage], |ctx, event| {
        if is_hit_on_self(ctx, event) {
            vec![plague(json_as(json!({ "amount": TOKENS_PER_DAMAGE })))]
        } else {
            vec![]
        }
    })
    .with_when(is_hit_on_self)
}

/// R280: the formula as the face prints it, with its rate as it stands, which the preview labels its
/// number with.
fn formula(per_token: i32) -> String {
    format!("+{per_token} mana per Plague Counter")
}

/// The mana the start-of-turn hook gains now: its own Plague Counters times the face's rate.
fn mana_now(self_: Option<&CardInstance>, per_token: i32) -> i32 {
    self_.and_then(|card| card.counters.plague).unwrap_or(0).max(0) * per_token
}

/// "Start of turn: +N mana per Plague Counter" (§2.2, R62: after the refresh, before the draw). With
/// no tokens the card gains nothing and returns no effect at all, so it emits no `manaChanged` for a
/// change of zero.
fn mana_from_tokens() -> Hook {
    hook(|ctx| {
        let amount = mana_now(ctx.self_.as_ref(), param(&*ctx, "mana"));
        if amount <= 0 {
            vec![]
        } else {
            vec![gain_mana(json_as(json!({ "amount": amount })))]
        }
    })
}

fn fed_fauci() -> Script {
    Script {
        triggers: vec![takes_damage()],
        start_of_turn: Some(mana_from_tokens()),
        preview: Some(condition_hook(|ctx| {
            let per_token = param(&ctx, "mana");
            vec![PreviewValue {
                label: formula(per_token),
                value: mana_now(Some(ctx.self_), per_token),
                display: None,
                ids: None,
            }]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: fed_fauci(),
        radiant: fed_fauci(),
    }
}

// #91 Fed Fauci (SPEC §8.4, BUILD M4-T4 row 91: "One Plague Counter per damage instance; +1 mana per
// token at start of turn (radiant +2); counters reset on leaving").
//
// Fauci is 1/6 → 2/12, so #61 Postdoc (2/4, no keywords; its Cry never fires because the harness
// places it, R1) can hit it twice without killing it. #68 (5/5) finishes it for R78, #4 Gary (1/1)
// is R63's zero-hit striker. A test that crosses a turn boundary gives BOTH sides a card in hand, or
// the engine auto-ends the turn (R82) and fires the start-of-turn hook more than once.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FAUCI: &str = "core-091";
    const POSTDOC: &str = "core-061"; // 2/4
    const SOURCERER: &str = "core-068"; // 5/5
    const GARY: &str = "core-004"; // 1/1

    /// The token count the `plague` effect keeps on the instance (§10.1).
    fn plague_on(s: &Scenario, card: &CardInstance) -> i32 {
        s.card(card).counters.plague.unwrap_or(0)
    }

    fn damage_events_on(events: &[GameEvent], id: &str) -> Vec<GameEvent> {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Damage { target_id, .. } if target_id == id))
            .cloned()
            .collect()
    }

    /// The keywords §10.4 computes for a lane, as the client sees them (§10.8).
    fn keywords_in_lane(s: &Scenario, player: PlayerId, lane: usize) -> Vec<Keyword> {
        let view = s.view(Some(P1));
        let side = if player == P1 { view.you } else { view.opponent };
        side.units
            .get(lane - 1)
            .cloned()
            .flatten()
            .map(|unit| unit.keywords)
            .unwrap_or_default()
    }

    fn kinds(keywords: &[Keyword]) -> Vec<String> {
        keywords.iter().map(|keyword| keyword.kind().to_string()).collect()
    }

    fn must_unit(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
        match s.unit(player, lane) {
            Some(card) => card,
            None => panic!("no unit in {player} lane {lane}"),
        }
    }

    /// A board where p2 can attack p1's Fauci, with nothing that could auto-end either turn (R82).
    fn board(radiant: Option<bool>, enemies: &[&str]) -> Scenario {
        let mut fauci = json!({ "def": FAUCI });
        if radiant == Some(true) {
            fauci["radiant"] = json!(true);
        }
        scenario(json!({
            "active": "p2",
            "p1": {
                "field": [fauci],
                "hand": [GARY],
                "library": [GARY, GARY],
            },
            "p2": { "field": enemies, "hand": [GARY], "library": [GARY] },
        }))
    }

    mod n91_fed_fauci_base {
        use super::*;

        #[test]
        fn s8_prints_rush_and_1_6_and_the_script_grants_nothing() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "field": [FAUCI] } }));
            s.expect_stats(FAUCI, json!({ "attack": 1, "health": 6, "maxHealth": 6 }));
            assert_eq!(kinds(&keywords_in_lane(&s, P1, 1)), vec!["Rush".to_string()]);
        }

        #[test]
        fn makes_one_plague_counter_per_damage_instance() {
            crate::register_all();
            let mut s = board(None, &[POSTDOC]);
            let fauci = s.card(FAUCI).clone();

            let attacker = must_unit(&s, P2, 1);
            s.attack(&attacker, &fauci);

            assert_eq!(plague_on(&s, &fauci), 1);
            s.expect_stats(&fauci, json!({ "attack": 1, "health": 4, "maxHealth": 6 }));
            s.expect_events(json!(["damage", "counterChanged"]));
        }

        #[test]
        fn counts_instances_not_attackers_two_separate_hits_are_two_tokens() {
            crate::register_all();
            let mut s = board(None, &[POSTDOC, POSTDOC]);
            let fauci = s.card(FAUCI).clone();

            let first = must_unit(&s, P2, 1);
            s.attack(&first, &fauci);
            assert_eq!(plague_on(&s, &fauci), 1);

            let second = must_unit(&s, P2, 2);
            s.attack(&second, &fauci);
            assert_eq!(plague_on(&s, &fauci), 2);
            // 2 + 2 damage on a 6-health body: still alive, so the second hit really was its own instance.
            s.expect_stats(&fauci, json!({ "attack": 1, "health": 2, "maxHealth": 6 }));
        }

        #[test]
        fn r63_a_hit_reduced_to_0_emits_no_damage_event_so_it_makes_no_token() {
            crate::register_all();
            // Defense Position grants Armor +1 (§4.1), so a 1-attack striker is reduced to 0 at §4.4 step 2.
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "field": [{ "def": FAUCI, "position": "DEF" }], "hand": [GARY] },
                "p2": { "field": [GARY], "hand": [GARY] },
            }));
            let fauci = s.card(FAUCI).clone();

            let attacker = must_unit(&s, P2, 1);
            s.attack(&attacker, &fauci);

            assert_eq!(damage_events_on(s.events(), &fauci.id).len(), 0);
            assert_eq!(plague_on(&s, &fauci), 0);
            s.expect_stats(&fauci, json!({ "attack": 1, "health": 6, "maxHealth": 6 }));
        }

        #[test]
        fn whenever_this_takes_damage_a_hit_it_dealt_is_not_a_hit_it_took() {
            crate::register_all();
            let mut s = board(None, &[POSTDOC]);
            let fauci = s.card(FAUCI).clone();
            let postdoc = must_unit(&s, P2, 1);

            s.attack(&postdoc, &fauci);

            // Fauci struck back for 1, which is a damage event on the Postdoc, and gave no second token.
            assert_eq!(damage_events_on(s.events(), &postdoc.id).len(), 1);
            assert_eq!(plague_on(&s, &fauci), 1);
        }

        #[test]
        fn start_of_turn_1_mana_per_plague_counter() {
            crate::register_all();
            let mut s = board(None, &[POSTDOC, POSTDOC]);
            let fauci = s.card(FAUCI).clone();

            let first = must_unit(&s, P2, 1);
            let second = must_unit(&s, P2, 2);
            s.attack(&first, &fauci).attack(&second, &fauci);
            assert_eq!(plague_on(&s, &fauci), 2);

            // p2 ends; p1's turn starts, so §2.3's refresh runs and then the hook adds 1 per token (R62).
            s.end_turn();

            // turn 9 with p2 active gives p1 four started turns; this start is its fifth, so MAX_MANA 4.
            s.expect_mana(P1, 4 + 2);
        }

        #[test]
        fn no_tokens_is_no_mana_and_no_change_at_all() {
            crate::register_all();
            let mut s = board(None, &[POSTDOC]);
            s.end_turn();
            s.expect_mana(P1, 4);
        }

        #[test]
        fn r78_counters_reset_when_it_leaves_the_field() {
            crate::register_all();
            let mut s = board(None, &[POSTDOC, SOURCERER]);
            let fauci = s.card(FAUCI).clone();

            let first = must_unit(&s, P2, 1);
            s.attack(&first, &fauci);
            assert_eq!(plague_on(&s, &fauci), 1);

            // 2 + 5 on a 6-health body kills it; R78 resets counters on the way to the graveyard.
            let second = must_unit(&s, P2, 2);
            s.attack(&second, &fauci);

            s.expect_in_zone(&fauci, "graveyard");
            assert_eq!(plague_on(&s, &fauci), 0);
        }
    }

    mod n91_fed_fauci_radiant {
        use super::*;

        #[test]
        fn s8_prints_rush_and_2_12_on_the_radiant_face() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "field": [{ "def": FAUCI, "radiant": true }] } }));
            s.expect_stats(FAUCI, json!({ "attack": 2, "health": 12, "maxHealth": 12 }));
            // §8 Conventions: the cell lists "Rush" without "Plus", so that is the complete list.
            assert_eq!(kinds(&keywords_in_lane(&s, P1, 1)), vec!["Rush".to_string()]);
        }

        #[test]
        fn s8_conventions_the_damage_token_clause_the_radiant_cell_does_not_restate_is_kept() {
            crate::register_all();
            let mut s = board(Some(true), &[POSTDOC]);
            let fauci = s.card(FAUCI).clone();

            let attacker = must_unit(&s, P2, 1);
            s.attack(&attacker, &fauci);

            assert_eq!(plague_on(&s, &fauci), 1);
        }

        #[test]
        fn start_of_turn_2_mana_per_plague_counter() {
            crate::register_all();
            let mut s = board(Some(true), &[POSTDOC, POSTDOC]);
            let fauci = s.card(FAUCI).clone();

            let first = must_unit(&s, P2, 1);
            let second = must_unit(&s, P2, 2);
            s.attack(&first, &fauci).attack(&second, &fauci);
            assert_eq!(plague_on(&s, &fauci), 2);

            s.end_turn();

            s.expect_mana(P1, 4 + 2 * 2);
        }

        #[test]
        fn r63_still_holds_on_the_radiant_face_a_0_hit_makes_no_token() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "field": [{ "def": FAUCI, "radiant": true, "position": "DEF" }], "hand": [GARY] },
                "p2": { "field": [GARY], "hand": [GARY] },
            }));
            let fauci = s.card(FAUCI).clone();

            let attacker = must_unit(&s, P2, 1);
            s.attack(&attacker, &fauci);

            assert_eq!(damage_events_on(s.events(), &fauci.id).len(), 0);
            assert_eq!(plague_on(&s, &fauci), 0);
        }
    }

    #[test]
    fn r386_an_upgrade_gains_2_a_counter_and_a_radiant_degrade_1_and_the_preview_says_so() {
        for (radiant, upgrade, rate) in [(false, true, 2), (true, false, 1)] {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "field": [{ "def": FAUCI, "radiant": radiant }], "hand": [GARY] },
                "p2": { "field": [GARY], "hand": [GARY] },
            }));
            s.card_mut(FAUCI).counters.plague = Some(2);
            let moved = if upgrade {
                crate::upgrade_number(&mut s, FAUCI, "mana")
            } else {
                crate::degrade_number(&mut s, FAUCI, "mana")
            };
            assert_eq!(moved, rate);
            let shown = serde_json::to_value(&s.view(P1).you.units[0]).unwrap_or_default();
            assert_eq!(shown["preview"][0]["label"], json!(format!("+{rate} mana per Plague Counter")));
            assert_eq!(shown["preview"][0]["value"], json!(2 * rate));

            s.end_turn();

            s.expect_mana(P1, 4 + 2 * rate);
        }
    }
}
