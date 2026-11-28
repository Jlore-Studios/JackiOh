//! FvH #52 Ace Hunter Jay (SPEC §8.14 row 52): (2) Unit, Human, Epic, 2/4 → 4/8. Built by S08 (#5258).
//!   Base:    "Aura: Your Units have Ranged, and +{bonus} Attack while attacking a Felinor." — bonus 3
//!   Radiant: "Aura: Your Units have Ranged and Rush, and +{bonus} Attack while attacking a Felinor." — 6
//!
//! The aura (§10.4 layer 5) lends Ranged (R783000), and Rush on the Radiant face, to every Unit its
//! controller has on the field, itself included; `applies` reads instance data only, as an aura's must.
//! The bonus is a combat-only attack modifier, M #52 Economic Anxiety's shape (`attack_mods`, R1120),
//! summed over every acting card: a strike by one of its controller's Units on a Unit whose tags now
//! hold Felinor (`tags_of`, a granted tag counting, R923) gains the declared `bonus`. Not effect damage,
//! not a strike back, nothing on a hero. It is a harmful effect naming Felinor, so a Felinor immune to
//! tribal hate (R940) or one whose harm passes Felinor by (FvH #88, R5237009) takes no bonus.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-052";

/// The tag the bonus names: its harm, for the tribal-hate reads.
const FELINOR: &[Tag] = &[Tag::Felinor];

fn ace_hunter(keywords: Vec<Keyword>) -> Script {
    Script {
        aura: Some(aura_hook(move |ctx| {
            let keywords = keywords.clone();
            vec![AuraEntry {
                applies: Box::new(move |unit: &CardInstance| {
                    unit.controller == ctx.self_.controller && in_units_row_or_standing(ctx.state, unit)
                }),
                mod_: StatMod {
                    keywords: Some(keywords),
                    ..StatMod::default()
                },
            }]
        })),
        attack_mods: Some(attack_mod_hook(|args| {
            if args.attacker.controller != args.self_.controller
                || !tags_of(args.state, args.defender).contains(&Tag::Felinor)
                || tribal_hate_cannot_reach(args.state, args.defender, Some(FELINOR), None)
            {
                return AttackMod::default();
            }
            AttackMod {
                attack: param(&args, "bonus"),
                poisonous: false,
            }
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: ace_hunter(vec![Keyword::Ranged]),
        radiant: ace_hunter(vec![Keyword::Ranged, Keyword::Rush]),
    }
}

// FvH #52 Ace Hunter Jay — SPEC §8.14 row 52.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FELINOR_UNIT: &str = "core-012"; // Duplicating Felinors: a Felinor
    const HUMAN: &str = "core-008"; // Mr. Vanilla: a Human
    const WHITE_FELINOR: &str = "meditative-025"; // Blue-Eyes White Felinor: immune to tribal hate
    const ALLY: &str = "stand-in (1) 3/9";
    const FILLER: &str = "core-005"; // Stockpile: a card in hand

    /// Jay (lane 1) beside an ally (lane 2), facing `enemies`.
    fn hunt(radiant: bool, enemies: Value) -> Scenario {
        scenario(json!({
            "p1": { "field": [{ "def": ID, "radiant": radiant }, ALLY], "hand": [FILLER] },
            "p2": { "field": enemies, "hand": [FILLER] },
        }))
    }

    /// What the last step's hit from `source` on `target` dealt: its `damage` event's amount.
    fn dealt(s: &Scenario, source: &str, target: &str) -> Option<i32> {
        s.last_events().iter().find_map(|event| match event {
            GameEvent::Damage {
                source_id,
                target_id,
                amount,
                ..
            } if source_id.as_deref() == Some(source) && target_id == target => Some(*amount),
            _ => None,
        })
    }

    fn has(s: &Scenario, card: &str, kind: KeywordKind) -> bool {
        s.stats(card).keywords.iter().any(|keyword| keyword.kind() == kind)
    }

    /// The ally's strike on the enemy in lane 1: what it dealt, and its printed Attack as listed.
    fn ally_strikes(s: &mut Scenario) -> (Option<i32>, i32) {
        let ally = s.unit_at(P1, 2).id;
        let victim = s.unit_at(P2, 1).id;
        let shown = s.stats(ally.as_str()).attack;
        s.attack(ally.as_str(), victim.as_str());
        (dealt(s, &ally, &victim), shown)
    }

    mod base {
        use super::*;

        #[test]
        fn your_units_have_ranged_itself_included_and_the_enemys_do_not() {
            let s = hunt(false, json!([HUMAN]));
            let jay = s.unit_at(P1, 1).id;
            let ally = s.unit_at(P1, 2).id;
            let theirs = s.unit_at(P2, 1).id;
            assert!(has(&s, &jay, KeywordKind::Ranged));
            assert!(has(&s, &ally, KeywordKind::Ranged));
            assert!(!has(&s, &theirs, KeywordKind::Ranged));
            assert!(!has(&s, &ally, KeywordKind::Rush), "the base face lends no Rush");
        }

        #[test]
        fn a_strike_on_a_felinor_gains_the_bonus_and_takes_nothing_back() {
            let mut s = hunt(false, json!([FELINOR_UNIT]));
            let ally = s.unit_at(P1, 2).id;
            let (hit, shown) = ally_strikes(&mut s);
            assert_eq!(hit, Some(shown + 3));
            assert_eq!(s.card(ally.as_str()).damage, 0, "Ranged: no strike back");
            assert_eq!(s.stats(ally.as_str()).attack, shown, "the listed Attack never changes");
        }

        #[test]
        fn a_strike_on_a_non_felinor_gains_nothing() {
            let mut s = hunt(false, json!([HUMAN]));
            let (hit, shown) = ally_strikes(&mut s);
            assert_eq!(hit, Some(shown));
        }

        #[test]
        fn a_granted_felinor_tag_counts() {
            let mut s = hunt(false, json!([HUMAN]));
            let theirs = s.unit_at(P2, 1).id;
            let human = must(find_instance_mut(s.state_mut(), &theirs), "the Human");
            human.granted_tags = Some(vec![Tag::Felinor]);
            let (hit, shown) = ally_strikes(&mut s);
            assert_eq!(hit, Some(shown + 3));
        }

        #[test]
        fn a_felinor_immune_to_tribal_hate_takes_no_bonus() {
            let mut s = hunt(false, json!([WHITE_FELINOR]));
            let (hit, shown) = ally_strikes(&mut s);
            assert_eq!(hit, Some(shown));
        }

        #[test]
        fn the_enemys_strike_on_your_felinor_gains_nothing() {
            let mut s = scenario(json!({
                "p1": { "field": [ID, FELINOR_UNIT], "hand": [FILLER] },
                "p2": { "field": [HUMAN], "hand": [FILLER] },
            }));
            s.end_turn();
            let theirs = s.unit_at(P2, 1).id;
            let ours = s.unit_at(P1, 2).id;
            let shown = s.stats(theirs.as_str()).attack;
            s.attack(theirs.as_str(), ours.as_str());
            assert_eq!(dealt(&s, &theirs, &ours), Some(shown));
        }

        #[test]
        fn an_attack_on_the_hero_gains_nothing() {
            let mut s = hunt(false, json!([]));
            let ally = s.unit_at(P1, 2).id;
            let shown = s.stats(ally.as_str()).attack;
            let before = hero_of(s.state(), P2).health;
            s.attack(ally.as_str(), "hero");
            assert_eq!(hero_of(s.state(), P2).health, before - shown);
        }

        #[test]
        fn the_aura_ends_when_it_leaves_the_field() {
            let mut s = scenario(json!({
                "p1": { "field": [ID, ALLY], "hand": [KILL, FILLER] },
                "p2": { "field": [FELINOR_UNIT], "hand": [FILLER] },
            }));
            let jay = s.unit_at(P1, 1).id;
            s.play(KILL, pick::targets(pick::id(&jay)));
            s.expect_in_zone(jay.as_str(), "graveyard");
            let ally = s.unit_at(P1, 2).id;
            assert!(!has(&s, &ally, KeywordKind::Ranged));
            let (hit, shown) = ally_strikes(&mut s);
            assert_eq!(hit, Some(shown));
        }

        #[test]
        fn the_bonus_tunes() {
            let mut s = hunt(false, json!([FELINOR_UNIT]));
            let jay = s.unit_at(P1, 1).id;
            set_param(s.card_mut(jay.as_str()), "bonus", 1);
            let (hit, shown) = ally_strikes(&mut s);
            assert_eq!(hit, Some(shown + 1));
        }

        const KILL: &str = "core-016"; // Hit Job: the removal
    }

    mod radiant {
        use super::*;

        #[test]
        fn your_units_have_ranged_and_rush() {
            let s = hunt(true, json!([HUMAN]));
            let jay = s.unit_at(P1, 1).id;
            let ally = s.unit_at(P1, 2).id;
            let theirs = s.unit_at(P2, 1).id;
            for id in [&jay, &ally] {
                assert!(has(&s, id, KeywordKind::Ranged));
                assert!(has(&s, id, KeywordKind::Rush));
            }
            assert!(!has(&s, &theirs, KeywordKind::Rush));
        }

        #[test]
        fn a_unit_played_later_has_rush_and_ranged() {
            let mut s = scenario(json!({
                "p1": { "field": [{ "def": ID, "radiant": true }], "hand": [ALLY, FILLER] },
                "p2": { "field": [FELINOR_UNIT], "hand": [FILLER] },
            }));
            s.play(ALLY, json!({}));
            let ally = s.card(ALLY).id.clone();
            assert!(has(&s, &ally, KeywordKind::Rush));
            assert!(has(&s, &ally, KeywordKind::Ranged));
        }

        #[test]
        fn a_strike_on_a_felinor_gains_6() {
            let mut s = hunt(true, json!([FELINOR_UNIT]));
            let (hit, shown) = ally_strikes(&mut s);
            assert_eq!(hit, Some(shown + 6));
        }

        #[test]
        fn a_strike_on_a_non_felinor_gains_nothing() {
            let mut s = hunt(true, json!([HUMAN]));
            let (hit, shown) = ally_strikes(&mut s);
            assert_eq!(hit, Some(shown));
        }
    }
}
