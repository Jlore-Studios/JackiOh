//! C #69 Plague Charger (SPEC §8.6 row 69). (2) Unit, Rare, 4/2 → 8/4.
//!   Base:    "Charge
//!             Has First Strike while it has a Plague Counter.
//!             Has +{attack} Attack for each Plague Counter on it." — +2
//!   Radiant: the same text — +4
//!   Engine:  "A conditional keyword (§6.1) and a self stat layer (§10.4) reading its own
//!            `counters.plague`, both following the count as tokens are placed and consumed. Tunes:
//!            attack per token 2 ↑."
//!
//! Charge is printed on both catalog faces (§10.4 layer 1).
//!
//! "Has First Strike while it has a Plague Counter" is a keyword that holds only while a condition does
//! (§6.1): the card's `conditionalKeywords`, read with its printed keywords on every read (§10.4), so it
//! comes and goes with the tokens. "+{attack} Attack for each Plague Counter on it" is §10.4 layer 5's
//! "stats per Plague Counter": an aura of this card on itself alone, its tokens times the declared
//! `attack`. Both read the card's own counters, which R78 clears when it leaves the field, so a Charger
//! with no token — in hand, or back on the field after a bounce — has neither. A Vanilla Charger has no
//! text, so neither (§6.3). The aura's own attack reaches no other unit.
//!
//! R195: the printed condition is "while it has a Plague Counter", read on the field (`conditionMet`,
//! proved in `test/condition-active.test.ts`): it glows exactly while it has First Strike. A Charger in
//! hand holds no tokens (R78), so it never glows there.
//!
//! The number is the declared `attack` (R386), read through `param` on the face it wears.

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-069";

/// "while it has a Plague Counter": the one predicate the keyword and the glow share.
fn has_plague(self_: &CardInstance) -> bool {
    plague_on(self_) > 0
}

const FIRST_STRIKE: Keyword = Keyword::FirstStrike;

pub fn script() -> CardScripts {
    let base = Script {
        conditional_keywords: Some(read_hook(|args| {
            if has_plague(args.self_) {
                vec![FIRST_STRIKE]
            } else {
                Vec::new()
            }
        })),
        aura: Some(aura_hook(|args| {
            let tokens = plague_on(args.self_);
            if tokens == 0 {
                return Vec::new();
            }
            let per_token = param(&args, "attack");
            let id = args.self_.id.clone();
            vec![AuraEntry {
                applies: Box::new(move |unit: &CardInstance| unit.id == id),
                mod_: StatMod {
                    attack: Some(tokens * per_token),
                    ..StatMod::default()
                },
            }]
        })),
        condition_met: Some(condition_hook(|ctx| {
            ctx.zone == ConditionZone::Field && has_plague(ctx.self_)
        })),
        ..Script::default()
    };
    CardScripts {
        // The same script: the Radiant face's +4 is its declared `attack`, which `param` reads off the face it
        // wears.
        radiant: base.clone(),
        base,
    }
}

// C #69 Plague Charger — SPEC §8.6 row 69, BUILD M9 Classic row C 69: "Charge; +2 Attack for each Plague
// Token on it (a self stat layer, §10.4) and First Strike exactly while it has one (a keyword while a
// condition holds, §6.1), both following the tokens as they come and go and gone when it leaves (R78);
// with no token it has neither; radiant 8/4: +4 per token; its tuned number (attack per token) reads
// through `param()` (R386)".
//
// Its yellow glow (R195, `conditionMet`: "while it has a Plague Counter", on the field) is proved, both
// answers, in `test/condition-active.test.ts`.

/// `describe("C #69 Plague Charger")`.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P2: PlayerId = PlayerId::P2;

    const CHARGER: &str = "classic-069";
    const CRAWLER: &str = "classic-053"; // (1) Unit: Cry: place 1 Plague Counter on another permanent.
    const MUTATE: &str = "classic-078"; // (1) Field Spell: Activate ♾️: remove a Plague Counter from a permanent …
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FLOOD: &str = "core-017"; // (4) Spell: Bounce all Units.
    const FILLER: &str = "core-005"; // (1) Spell (§2.5).
    const ANCHOR: &str = "core-010"; // (0) Spell: keeps a spent turn open (§2.5).

    use crate::scenario;

    use crate::js;

    fn kinds(s: &Scenario, card: &str) -> Vec<String> {
        s.stats(card)
            .keywords
            .iter()
            .map(|keyword| keyword.kind().as_str().to_string())
            .collect()
    }

    fn at(card: &CardInstance) -> Value {
        json!([{ "pick": "instance", "instanceId": card.id }])
    }

    /// declares its one number, a conditional keyword, a self aura and the glow, one script on both faces
    #[test]
    fn declares_its_one_number_a_conditional_keyword_a_self_aura_and_the_glow_one_script_on_both_faces() {
        crate::register_all();
        let def = crate::card_def(CHARGER);
        assert_eq!(def.id, CHARGER);
        assert_eq!(
            js(&def.params),
            json!([{ "key": "attack", "base": 2, "radiant": 4, "better": "up", "step": 1, "min": 1 }])
        );
        assert_eq!(js(&def.base.keywords), json!([{ "kind": "Charge" }]));
        let scripts = script();
        let (base, radiant) = (&scripts.base, &scripts.radiant);
        assert!(base.conditional_keywords.is_some());
        assert!(base.aura.is_some());
        assert!(base.condition_met.is_some());
        // TS `expect(radiant).toBe(base)`: the Radiant face is the base face, every hook the same one.
        assert!(Arc::ptr_eq(
            base.conditional_keywords.as_ref().expect("a conditional keyword"),
            radiant.conditional_keywords.as_ref().expect("a conditional keyword")
        ));
        assert!(Arc::ptr_eq(
            base.aura.as_ref().expect("an aura"),
            radiant.aura.as_ref().expect("an aura")
        ));
        assert!(Arc::ptr_eq(
            base.condition_met.as_ref().expect("a glow"),
            radiant.condition_met.as_ref().expect("a glow")
        ));
    }

    /// `describe("base")`.
    mod base {
        use super::*;

        /// with no token it is a 4/2 with Charge and neither First Strike nor a bonus
        #[test]
        fn with_no_token_it_is_a_4_2_with_charge_and_neither_first_strike_nor_a_bonus() {
            let mut s = scenario(json!({ "p1": { "field": [CHARGER], "hand": [FILLER] } }));

            s.expect_stats(CHARGER, json!({ "attack": 4, "health": 2 }));
            assert_eq!(kinds(&s, CHARGER), vec!["Charge"]);
        }

        /// Charge: it attacks the enemy hero the turn it is played
        #[test]
        fn charge_it_attacks_the_enemy_hero_the_turn_it_is_played() {
            let mut s = scenario(json!({ "p1": { "hand": [CHARGER, FILLER] }, "p2": { "hand": [FILLER], "health": 20 } }));

            s.play(CHARGER, json!({}));
            s.attack(CHARGER, "hero");

            s.expect_health(P2, 16);
        }

        /// §10.4 +2 Attack for each Plague Counter on it, and First Strike while it has one
        #[test]
        fn s10_4_plus_2_attack_for_each_plague_counter_on_it_and_first_strike_while_it_has_one() {
            let mut one = scenario(json!({ "p1": { "field": [{ "def": CHARGER, "counters": { "plague": 1 } }], "hand": [FILLER] } }));
            one.expect_stats(CHARGER, json!({ "attack": 6, "health": 2 }));
            assert_eq!(kinds(&one, CHARGER), vec!["Charge", "First Strike"]);

            let mut three = scenario(json!({ "p1": { "field": [{ "def": CHARGER, "counters": { "plague": 3 } }], "hand": [FILLER] } }));
            three.expect_stats(CHARGER, json!({ "attack": 10, "health": 2, "maxHealth": 2 }));
            assert_eq!(kinds(&three, CHARGER), vec!["Charge", "First Strike"]);
        }

        /// both follow the tokens as they come: a C #53 Plague Crawler's placement gives it +2 and First Strike
        #[test]
        fn both_follow_the_tokens_as_they_come_a_c_n53_plague_crawlers_placement_gives_it_plus_2_and_first_strike() {
            let mut s = scenario(json!({ "p1": { "hand": [CRAWLER, FILLER], "field": [CHARGER] }, "p2": { "hand": [FILLER] } }));
            assert_eq!(kinds(&s, CHARGER), vec!["Charge"]);

            let targets = at(s.card(CHARGER));
            s.play(CRAWLER, json!({ "targets": targets }));

            s.expect_stats(CHARGER, json!({ "attack": 6 }));
            assert_eq!(kinds(&s, CHARGER), vec!["Charge", "First Strike"]);
        }

        /// both follow the tokens as they go: C #78 Mutate Spell's removals take the bonus, then First Strike
        #[test]
        fn both_follow_the_tokens_as_they_go_c_n78_mutate_spells_removals_take_the_bonus_then_first_strike() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [{ "def": CHARGER, "counters": { "plague": 2 } }], "backrow": [MUTATE] },
                "p2": { "hand": [FILLER], "health": 30 },
            }));
            let charger = s.card(CHARGER).clone();
            s.expect_stats(&charger, json!({ "attack": 8 }));

            // Each removal comes before the forced attack it buys, which here can only be at the hero.
            s.activate(MUTATE, json!({ "targets": at(&charger) }));
            s.expect_stats(&charger, json!({ "attack": 6 }));
            assert_eq!(kinds(&s, &charger.id), vec!["Charge", "First Strike"]);
            s.expect_health(P2, 24);

            s.activate(MUTATE, json!({ "targets": at(&charger) }));
            s.expect_stats(&charger, json!({ "attack": 4 }));
            assert_eq!(kinds(&s, &charger.id), vec!["Charge"]);
            s.expect_health(P2, 20);
        }

        /// R78 they are gone when it leaves: bounced to hand and played again, it is a plain 4/2
        #[test]
        fn r78_they_are_gone_when_it_leaves_bounced_to_hand_and_played_again_it_is_a_plain_4_2() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [{ "def": CHARGER, "counters": { "plague": 2 } }], "mana": 10 },
                "p2": { "hand": [FLOOD, ANCHOR] },
                "active": "p2",
            }));
            let charger = s.card(CHARGER).clone();

            s.play(FLOOD, json!({}));
            s.expect_in_zone(&charger, "hand");
            assert!(s.card(&charger).counters.plague.is_none());
            s.end_turn();
            s.play(&charger, json!({}));

            s.expect_stats(&charger, json!({ "attack": 4, "health": 2 }));
            assert_eq!(kinds(&s, &charger.id), vec!["Charge"]);
        }

        /// §4.3 with a token its First Strike kills a 4/4 before it strikes back; with none, both die
        #[test]
        fn s4_3_with_a_token_its_first_strike_kills_a_4_4_before_it_strikes_back_with_none_both_die() {
            let mut plagued = scenario(json!({
                "p1": { "hand": [FILLER], "field": [{ "def": CHARGER, "counters": { "plague": 1 } }] },
                "p2": { "hand": [FILLER], "field": [VANILLA] },
            }));
            let vanilla = plagued.card(VANILLA).clone();
            plagued.attack(CHARGER, &vanilla);
            plagued.expect_in_zone(VANILLA, "graveyard");
            plagued.expect_in_zone(CHARGER, "field");

            let mut clean = scenario(json!({ "p1": { "hand": [FILLER], "field": [CHARGER] }, "p2": { "hand": [FILLER], "field": [VANILLA] } }));
            let vanilla = clean.card(VANILLA).clone();
            clean.attack(CHARGER, &vanilla);
            clean.expect_in_zone(VANILLA, "graveyard");
            clean.expect_in_zone(CHARGER, "graveyard");
        }

        /// the bonus is its own: no other Unit gains from its tokens
        #[test]
        fn the_bonus_is_its_own_no_other_unit_gains_from_its_tokens() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [{ "def": CHARGER, "counters": { "plague": 2 } }, { "def": VANILLA, "counters": { "plague": 2 } }] },
            }));

            s.expect_stats(VANILLA, json!({ "attack": 4 }));
            assert!(kinds(&s, VANILLA).is_empty());
        }

        /// R386 an Upgrade makes it +3 per token; a Degrade +1
        #[test]
        fn r386_an_upgrade_makes_it_plus_3_per_token_a_degrade_plus_1() {
            let mut up = scenario(json!({ "p1": { "field": [{ "def": CHARGER, "counters": { "plague": 2 } }], "hand": [FILLER] } }));
            step_param(up.card_mut(CHARGER), "attack", 1);
            up.expect_stats(CHARGER, json!({ "attack": 10 }));

            let mut down = scenario(json!({ "p1": { "field": [{ "def": CHARGER, "counters": { "plague": 2 } }], "hand": [FILLER] } }));
            step_param(down.card_mut(CHARGER), "attack", -1);
            down.expect_stats(CHARGER, json!({ "attack": 6 }));
        }
    }

    /// `describe("radiant")`.
    mod radiant {
        use super::*;

        /// is an 8/4 with Charge; with no token neither First Strike nor a bonus
        #[test]
        fn is_an_8_4_with_charge_with_no_token_neither_first_strike_nor_a_bonus() {
            let mut s = scenario(json!({ "p1": { "field": [{ "def": CHARGER, "radiant": true }], "hand": [FILLER] } }));

            s.expect_stats(CHARGER, json!({ "attack": 8, "health": 4 }));
            assert_eq!(kinds(&s, CHARGER), vec!["Charge"]);
        }

        /// +4 Attack for each Plague Counter on it, and First Strike while it has one
        #[test]
        fn plus_4_attack_for_each_plague_counter_on_it_and_first_strike_while_it_has_one() {
            let mut s = scenario(json!({ "p1": { "field": [{ "def": CHARGER, "radiant": true, "counters": { "plague": 2 } }], "hand": [FILLER] } }));

            s.expect_stats(CHARGER, json!({ "attack": 16, "health": 4 }));
            assert_eq!(kinds(&s, CHARGER), vec!["Charge", "First Strike"]);
        }

        /// R386 an Upgrade makes it +5 per token
        #[test]
        fn r386_an_upgrade_makes_it_plus_5_per_token() {
            let mut s = scenario(json!({ "p1": { "field": [{ "def": CHARGER, "radiant": true, "counters": { "plague": 1 } }], "hand": [FILLER] } }));
            step_param(s.card_mut(CHARGER), "attack", 1);

            s.expect_stats(CHARGER, json!({ "attack": 13 }));
        }
    }
}
