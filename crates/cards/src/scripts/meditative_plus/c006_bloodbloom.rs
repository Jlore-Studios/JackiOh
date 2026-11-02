//! M+ #6 Bloodbloom (SPEC §8.9 row 6, BUILD M11 row M+ 6): (0) Spell, Epic.
//!   Base:    "Your next Spell costs Health equal to {multiple|time|times} its Cost instead of mana."
//!   Radiant: "Your next Spell costs Health equal to {multiple|time|times} its Cost instead of mana,
//!            and has Echo 1."
//!
//! Health as price via price_next_spell_in_health (R1180004): HealthPrice modifier on caster (R65, R18, R70).
//! Radiant price gives that Spell Echo 1 (R30).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-006";

/// The Radiant face's "and has Echo 1": a printed keyword number (§6.2 Echo X), as a card's
/// printed Echo is.
const RADIANT_ECHO: i32 = 1;

fn bloodbloom(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![price_next_spell_in_health(HealthPriceArgs {
                multiple: param(&*ctx, "multiple"),
                echo: radiant.then_some(RADIANT_ECHO),
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: bloodbloom(false),
        radiant: bloodbloom(true),
    }
}

// M+ #6 Bloodbloom — SPEC §8.9 row 6, BUILD M11 row M+ 6 (R65).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const INJECTION: &str = "core-090";
    /// A CN-Viral Injection at (2), the price these cases are written at: it costs (1) since #2657, and
    /// a `costMod` of 1 puts it back.
    fn injection() -> Value {
        json!({ "def": INJECTION, "costMod": 1 })
    }
    const CN_VIRUS: &str = "core-090-1";
    const FULLSEND: &str = "core-078";
    const TWINSPELL: &str = "core-079";
    const MY_PAWN: &str = "core-096";
    const JELLY_BEAN: &str = "core-027";
    const FILLER: &str = "core-008";

    fn game(seed: &str, p1: Value) -> Scenario {
        let mut p1 = p1;
        let side = p1.as_object_mut().expect("a side");
        side.entry("library").or_insert(json!([FILLER, FILLER, FILLER]));
        crate::scenario(json!({
            "seed": seed,
            "p1": p1,
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
        }))
    }

    fn health(s: &Scenario, player: PlayerId) -> i32 {
        s.state().players[player].hero.health
    }

    fn mana(s: &Scenario) -> i32 {
        s.state().players.p1.mana.current
    }

    fn prices(s: &Scenario) -> Vec<ModifierKind> {
        s.state()
            .players
            .p1
            .mods
            .iter()
            .filter(|modifier| matches!(modifier.kind, ModifierKind::HealthPrice { .. }))
            .map(|modifier| modifier.kind.clone())
            .collect()
    }

    fn viruses(s: &Scenario) -> usize {
        s.pile(P2, "library")
            .into_iter()
            .filter(|card| card.def_id == CN_VIRUS)
            .count()
    }

    fn listed(s: &Scenario, def_id: &str) -> bool {
        legal_actions(s.state(), P1).into_iter().any(|action| match action {
            ActionBody::Play { instance_id, .. } => {
                find_instance(s.state(), &instance_id).is_some_and(|card| card.def_id == def_id)
            }
            _ => false,
        })
    }

    #[test]
    fn r1180004_the_next_spell_costs_no_mana_and_three_times_its_cost_in_health() {
        let mut s = game("m+006-price", json!({ "hand": [ID, injection()] }));
        s.play(ID, json!({}));
        let before = mana(&s);
        s.play(INJECTION, json!({}));
        assert_eq!(mana(&s), before);
        assert_eq!(health(&s, P1), 24);
        assert_eq!(viruses(&s), 1);
        assert!(prices(&s).is_empty(), "the Spell used it");
    }

    #[test]
    fn r1180004_legal_actions_offer_a_spell_only_the_health_affords() {
        let mut s = game("m+006-legal", json!({ "hand": [ID, injection()], "mana": 0 }));
        assert!(!listed(&s, INJECTION));
        s.play(ID, json!({}));
        assert!(listed(&s, INJECTION));
        s.play(INJECTION, json!({}));
        assert_eq!(health(&s, P1), 24);
    }

    #[test]
    fn r1180004_a_discount_lowers_the_health() {
        let mut s = game("m+006-discount", json!({ "hand": [FULLSEND, ID, injection()] }));
        // /fullsend's "your cards cost (1) less this turn", read after every modifier (R65).
        s.play(FULLSEND, json!({}));
        s.play(ID, json!({}));
        s.play(INJECTION, json!({}));
        assert_eq!(health(&s, P1), 27);
    }

    #[test]
    fn r1180004_a_field_spell_and_a_trap_do_not_use_it() {
        let mut s = game("m+006-permanents", json!({ "hand": [ID, TWINSPELL, MY_PAWN, FILLER], "mana": 9 }));
        s.play(ID, json!({}));
        let before = mana(&s);
        s.play(TWINSPELL, json!({}));
        s.play(MY_PAWN, json!({}));
        assert_eq!(mana(&s), before - 3);
        assert_eq!(health(&s, P1), 30);
        assert_eq!(prices(&s).len(), 1);
    }

    #[test]
    fn r1180004_a_cast_does_not_use_it() {
        let mut s = game(
            "m+006-cast",
            json!({ "hand": [ID, injection()], "library": [JELLY_BEAN, FILLER, FILLER] }),
        );
        s.play(ID, json!({}));
        s.end_turn();
        s.end_turn();
        // The Jelly Bean was drawn and cast (R70): its own health loss, no price paid.
        let loss = jackioh_engine::prelude::param_value(
            s.state(),
            None,
            "loss",
            jackioh_engine::prelude::ParamValueOptions {
                def_id: Some(JELLY_BEAN.to_string()),
                radiant: Some(false),
            },
        );
        assert_eq!(health(&s, P1), 30 - loss);
        assert_eq!(prices(&s).len(), 1);
    }

    #[test]
    fn r1180004_it_lasts_across_turns_until_a_spell_uses_it() {
        let mut s = game("m+006-turns", json!({ "hand": [ID, injection(), FILLER] }));
        s.play(ID, json!({}));
        s.end_turn();
        s.end_turn();
        assert_eq!(prices(&s).len(), 1);
        s.play(INJECTION, json!({}));
        assert_eq!(health(&s, P1), 24);
        assert!(prices(&s).is_empty());
    }

    #[test]
    fn r1180004_paying_to_0_health_loses_at_the_state_check() {
        let mut s = game("m+006-lose", json!({ "hand": [ID, injection()], "health": 6 }));
        s.play(ID, json!({}));
        s.play(INJECTION, json!({}));
        assert_eq!(s.state().result.expect("the game is over").winner, Winner::P2);
    }

    #[test]
    fn r1180004_a_second_bloodbloom_replaces_the_first() {
        let mut s = game("m+006-twice", json!({ "hand": [ID, { "def": ID, "radiant": true }, injection()] }));
        s.play(s.hand(P1)[0].clone(), json!({}));
        s.play(s.hand(P1)[0].clone(), json!({}));
        assert_eq!(health(&s, P1), 30);
        assert_eq!(
            prices(&s),
            vec![ModifierKind::HealthPrice {
                multiple: 3,
                echo: Some(1)
            }]
        );
    }

    #[test]
    fn multiple_reads_through_param() {
        let mut s = game("m+006-param", json!({ "hand": [ID, injection()] }));
        let bloom = s.card(ID).id.clone();
        set_param(s.card_mut(&bloom), "multiple", 2);
        s.play(bloom.as_str(), json!({}));
        s.play(INJECTION, json!({}));
        assert_eq!(health(&s, P1), 26);
    }

    #[test]
    fn radiant_the_spell_it_prices_resolves_twice() {
        let mut s = game("m+006-radiant", json!({ "hand": [{ "def": ID, "radiant": true }, injection()] }));
        s.play(ID, json!({}));
        s.play(INJECTION, json!({}));
        assert_eq!(health(&s, P1), 24);
        assert_eq!(viruses(&s), 2);
    }

    #[test]
    fn base_the_spell_it_prices_resolves_once() {
        let mut s = game("m+006-base-once", json!({ "hand": [ID, injection()] }));
        s.play(ID, json!({}));
        s.play(INJECTION, json!({}));
        assert_eq!(viruses(&s), 1);
    }
}
