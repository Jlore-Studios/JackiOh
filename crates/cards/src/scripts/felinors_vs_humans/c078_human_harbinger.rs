//! FvH #78 Human Harbinger (SPEC §8.14 row 78): (3) Unit, Human, Catalyst, Legendary, 6/6 → 12/12. Built by
//! S08 (#5258).
//!   Base:    "Taunt\nAura: Your other Humans have +{aura}/+{aura}.\nDeath: Shuffle a Human Harbinger Prime
//!            into your deck." — aura 2
//!   Radiant: the same with "a Radiant Human Harbinger Prime" — aura 4
//!
//! C+ #46 Felinor Flagbearer's Catalyst shape with Human for Felinor. Taunt is catalog data. The aura
//! (§10.4 layer 5) reaches its controller's other Units whose tags now hold Human (`tags_of`, a granted
//! tag counting, R923) in the unit row or standing in the backrow (R1081000), "other" on both faces;
//! `applies` reads instance data only. The Death shuffles one FvH #78.1 in at a random place of the deck
//! of the player who controlled it as it died, Radiant on the Radiant face, turned away at the deck cap
//! (R80). Tunes: aura 2 ↑ (Radiant 4).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-078";

/// FvH #78.1 Human Harbinger Prime, the Death's card.
const PRIME: &str = "felinorsvshumans-078-1";

/// "Your other Humans have +N/+N".
fn human_aura() -> AuraHook {
    aura_hook(|ctx| {
        let amount = param(&ctx, "aura");
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                unit.controller == ctx.self_.controller
                    && in_units_row_or_standing(ctx.state, unit)
                    && unit.id != ctx.self_.id
                    && tags_of(ctx.state, unit).contains(&Tag::Human)
            }),
            mod_: StatMod {
                attack: Some(amount),
                max_health: Some(amount),
                ..StatMod::default()
            },
        }]
    })
}

fn harbinger(radiant: bool) -> Script {
    Script {
        aura: Some(human_aura()),
        death: Some(hook(move |_ctx| {
            vec![shuffle_into(json_as(json!({
                "defId": PRIME,
                "count": 1,
                "player": "self",
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: harbinger(false),
        radiant: harbinger(true),
    }
}

// FvH #78 Human Harbinger — SPEC §8.14 row 78.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const HUMAN: &str = "core-008"; // Mr. Vanilla: a Human
    const OTHER: &str = "stand-in (1) 3/3";
    const KILL: &str = "core-016"; // Hit Job: the removal
    const FILLER: &str = "core-005"; // Stockpile: a card in hand

    /// The Harbinger in p1's hand, beside a Human (lane 1) and a non-Human (lane 2) of p1's and a Human of
    /// p2's (lane 1).
    fn rally(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, KILL, FILLER],
                "field": [HUMAN, OTHER],
                "library": [FILLER],
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "field": [HUMAN] },
        }))
    }

    fn stat_line(s: &Scenario, card: &str) -> (i32, i32) {
        let view = s.stats(card);
        (view.attack, view.max_health)
    }

    fn primes(s: &Scenario) -> Vec<CardInstance> {
        s.pile(P1, "library").into_iter().filter(|card| card.def_id == PRIME).collect()
    }

    fn kill_harbinger(s: &mut Scenario) {
        let harbinger = s.card(ID).id.clone();
        s.play(KILL, pick::targets(pick::id(&harbinger)));
        s.expect_in_zone(harbinger.as_str(), "graveyard");
    }

    mod base {
        use super::*;

        #[test]
        fn has_taunt() {
            let mut s = rally(false);
            s.play(ID, json!({}));
            assert!(s.stats(ID).keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Taunt));
        }

        #[test]
        fn your_other_humans_have_2_2_not_itself_not_a_non_human_not_the_opponents() {
            let mut s = rally(false);
            let (human, other, theirs) = (s.unit_at(P1, 1).id, s.unit_at(P1, 2).id, s.unit_at(P2, 1).id);
            let before = [stat_line(&s, &human), stat_line(&s, &other), stat_line(&s, &theirs)];
            let own_before = stat_line(&s, ID);
            s.play(ID, json!({}));
            assert_eq!(stat_line(&s, &human), (before[0].0 + 2, before[0].1 + 2));
            assert_eq!(stat_line(&s, &other), before[1]);
            assert_eq!(stat_line(&s, &theirs), before[2]);
            assert_eq!(stat_line(&s, ID), own_before);
        }

        #[test]
        fn a_granted_human_tag_counts() {
            let mut s = rally(false);
            let other = s.unit_at(P1, 2).id;
            let before = stat_line(&s, &other);
            s.play(ID, json!({}));
            let unit = must(find_instance_mut(s.state_mut(), &other), "the non-Human");
            unit.granted_tags = Some(vec![Tag::Human]);
            assert_eq!(stat_line(&s, &other), (before.0 + 2, before.1 + 2));
        }

        #[test]
        fn the_aura_ends_with_it_and_its_death_shuffles_a_prime_into_your_deck() {
            let mut s = rally(false);
            let human = s.unit_at(P1, 1).id;
            let before = stat_line(&s, &human);
            s.play(ID, json!({}));
            kill_harbinger(&mut s);
            assert_eq!(stat_line(&s, &human), before);
            let shuffled = nonempty(primes(&s), "the Prime in the deck");
            assert_eq!(shuffled.len(), 1);
            assert!(!shuffled[0].radiant);
        }

        #[test]
        fn a_full_deck_turns_the_prime_away() {
            let full: Vec<&str> = vec![FILLER; LIBRARY_CAP as usize];
            let mut s = scenario(json!({
                "p1": { "hand": [ID, KILL, FILLER], "library": full, "mana": 10 },
                "p2": { "hand": [FILLER] },
            }));
            s.play(ID, json!({}));
            kill_harbinger(&mut s);
            assert!(primes(&s).is_empty());
        }

        #[test]
        fn the_aura_tunes() {
            let mut s = rally(false);
            let human = s.unit_at(P1, 1).id;
            let before = stat_line(&s, &human);
            set_param(s.card_mut(ID), "aura", 1);
            s.play(ID, json!({}));
            assert_eq!(stat_line(&s, &human), (before.0 + 1, before.1 + 1));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn your_other_humans_have_4_4_and_not_itself() {
            let mut s = rally(true);
            let human = s.unit_at(P1, 1).id;
            let before = stat_line(&s, &human);
            let own_before = stat_line(&s, ID);
            s.play(ID, json!({}));
            assert_eq!(stat_line(&s, &human), (before.0 + 4, before.1 + 4));
            assert_eq!(stat_line(&s, ID), own_before);
        }

        #[test]
        fn its_death_shuffles_a_radiant_prime_into_your_deck() {
            let mut s = rally(true);
            s.play(ID, json!({}));
            kill_harbinger(&mut s);
            let shuffled = nonempty(primes(&s), "the Prime in the deck");
            assert!(shuffled[0].radiant);
        }
    }
}
